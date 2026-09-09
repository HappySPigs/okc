# U3 upload-client — Code Summary (구현 요약)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U3 Upload Protocol Client** -> Code Generation
**크레이트**: `upload-client` (lib) · **소속 컴포넌트**: `UploadProtocolDriver` (유일 컴포넌트)
**근거 산출물**: FD 3종(`domain-entities.md` / `business-rules.md` / `business-logic-model.md`)
**NFR 참고**: NFR 단계는 사용자 지시로 **SKIP** -> 코드는 FD + 상류 크레이트 실제 공개 API 에 직접 근거한다.

---

## 1. 구현된 컴포넌트 + 공개 API 표면

`UploadProtocolDriver` 하나가 트리거당 **단일 직렬 사이클**(Q2=B)로 다단계 업로드 프로토콜을 구동한다.
`execute_cycle` 오케스트레이션(R-CYCLE-01): 런타임 SafetyLimits 재검사 -> no-op digest 조기 종료(FR-10)
-> have/want 협상 -> want blob 순차 전송(각 blob = 전송 직전 재-읽기+재-해시 재검증 -> 전송) -> 경로->해시
맵 + `manifest_digest` 커밋 -> `Done`. 실패는 `UploadError` 로 즉시 반환하며 재시도/백오프(sleep) 없음
(drive-not-sleep, D-16 — U4 소관).

crate-root 재-export 로 노출되는 공개 표면:

- 드라이버: `UploadProtocolDriver::new(...)`, `execute_cycle(&Manifest) -> Result<CommitOutcome, UploadError>`,
  `execute_cycle_traced(&Manifest) -> CycleReport`. 상태 관측용 `CyclePhase`(`NoOp`/`Negotiate`/`Transfer`/
  `Reverify`/`Commit`/`Done`/`Abort`) + `CycleReport { outcome, phases }`.
- 프로토콜 값 타입 + 순수 함수: `WantSet`, `PathHashMap`, `NegotiateRequest`/`NegotiateResponse`,
  `CommitRequest`/`CommitOutcome`, `VaultContentId`, `ChunkPlan`/`ChunkFrame`,
  `compute_want`/`manifest_hashes`/`negotiate_entries`/`path_hash_map`/`plan_chunks`/`frame_chunks`/`reassemble`,
  경로 상수 `NEGOTIATE_PATH`/`COMMIT_PATH`/`BLOB_PATH`.
- seam 포트: `BlobSource`(+`BlobSourceError`) 재-읽기 seam, `SyncStore` 재개 오프셋/커밋 지속 seam.
- 오류: `UploadError`(`Transport`/`HashMismatch`/`OverLimit`/`Aborted`, taxonomy 보존 반환).

## 2. 모듈 레이아웃

- `driver.rs` — 오케스트레이션 상태머신(협상/전송/재검증/청크/커밋). IO 위임 모듈이라 순수 lint-gate 없음(단 패닉 경로 없음).
- `protocol.rs` — U3 도입 값 타입 + transport 무관 순수 함수. `#![deny(clippy::unwrap_used/expect_used/indexing_slicing/panic)]` 리프.
- `blob_source.rs`, `store.rs` — 주입 seam. `store.rs` 는 프로덕션 배선용 `impl SyncStore for SyncStateStore`(`StateError -> UploadError::Aborted`) 포함.
- `error.rs` — `UploadError` taxonomy(순수 리프 lint-gate).
- `proptest_support/generators.rs` — U3 도메인 제너레이터(`proptest-support` feature 게이트, 런타임 그래프 밖).

## 3. 의존성

**신규 외부 크레이트 0건.** 상류 W2/W0 크레이트의 **공개 API 만 소비**하며 수정하지 않는다(W3 소비자):
`foundation`(값 타입·오류·CBOR `encode`/`decode`·`StatusSink`), `content-core`(`ContentAddressing::hash_stream`
재검증 · `SafetyLimitsValidator::validate` 한도 재검사), `sync-state`(`SyncStateStore` — `SyncStore` seam 경유),
`auth-consent`(`AuthTransport::send` — 유일 아웃바운드 HTTP 경로). `serde`/`thiserror`/`proptest`(optional)는 신규가
아니라 workspace 핀 상속이며 `proptest` 는 비-default feature 뒤에 게이트되어 프로덕션 그래프에 유입되지 않는다(PBT-07).

## 4. MVP 트림 (FD 대비 편차 — 코드 주석에 명시)

- `SyncStore` seam: FD `domain-entities.md` §3 은 `SyncStateStore` concrete 주입을 열거하나, 유일 공개 생성자가 실제 fs IO 를 요구하므로 property 테스트 격리를 위해 seam 트레이트로 감쌌다. 반환도 `Option<&Manifest>` 대신 소유값 `Option<Manifest>`(fake 의 interior-mutability 참여용).
- 단일 요청 실패 시 청크 폴백 미도입(drive-not-sleep — U4 가 사이클 재시도).
- 청크 재개: `Read` 는 seek 불가 -> `[0, resume)` 구간을 읽어 폐기하는 방식(서버가 이미 보유하는 것으로 재조립 동치 성립).
- `WantSet` 은 결정성을 위해 `BTreeSet` 채택(component-methods 의 `HashSet` 은 집합 의미만 승계).
- `plan_chunks` 는 `chunk_size == 0` 을 `1` 로 보정(패닉 회피).
- 프로토콜 봉투는 서버 계약 미확정([blocked-on-server], DEP-03)이라 **잠정 mock 계약** + U0 CBOR 직렬화.

## 5. 테스트 커버리지 (총 16개 통과, 0 실패)

- **예제 앵커 테스트 8개**(`tests/example_upload_client.rs`, 기본 `cargo test`): no-op 조기 종료, 성공 사이클 전이, 서버 보유분 미전송(R-WANT-03/NFR-09), 전송 전 재검증 불일치 -> 커밋 중단(R-REVERIFY-02), 한도 초과 halt(R-LIMIT-RT-02), 협상 전송 실패(R-WANT-04), 대용량 청크 진행률·오프셋(NFR-10/R-STATUS-01), 재개 오프셋 시작(R-RESUME-01/02). fake seam(`ScriptedHttpTransport`/`FakeBlobSource`/`FakeStore`/`RecordingStatusSink`)으로 네트워크·파일시스템·권한 없이 구동.
- **property 테스트 8개**(`tests/prop_upload_client.rs`, `proptest-support` 게이트):
  PROP-U3-01/02(want 차집합 정확성·멱등), PROP-U3-03/04/05(청크 재조립 round-trip·임의 오프셋 재개 동치·해시 정합),
  PROP-U3-06(재검증 불일치 -> 커밋 미발행·last 불변), PROP-U3-07(no-op 조기 종료·왕복 0회),
  PROP-U3-08(상태머신 합법 전이·단일 직렬 사이클).

## 6. 검증 사실 (확인됨)

- 전체 workspace 빌드 성공(8개 크레이트).
- U3 테스트 16개 전부 통과(0 실패).
- `cargo clippy --all-targets --features proptest-support -- -D warnings` CLEAN.

# U3 upload-client — Business Rules (결정 규칙 / 검증 로직 / 제약)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U3 Upload Protocol Client** -> Functional Design -> 산출물 2/3 (`business-rules.md`)
**작성일**: 2026-09-08
**크레이트**: `upload-client` (lib) · **소속 컴포넌트**: `UploadProtocolDriver`
**전제(확정 답변)**: Q2=B · Q6=C · Q8=A · FQ-1=A · FQ-2=A · DEP-03 [blocked-on-server]

> **문서 성격**: U3가 소유하는 **결정 규칙·검증 로직·제약·불변식**을 정의한다. 타입 정의는 `domain-entities.md`가, 알고리즘/시퀀스는 `business-logic-model.md`가 소유하며 이 문서는 그 타입명을 재사용한다(재정의·모순 없음).
>
> **표기 규약**: 기술중립. Rust스러운 표기는 참고용. 화살표는 ASCII(`A -> B`)만. Rust 타입/제네릭은 백틱. Korean 산문.

---

## 1. 사이클 오케스트레이션 순서 규칙 (FR-06/07/09/10, US-E2-05)

**규칙 R-CYCLE-01 (고정 단계 순서)**: `execute_cycle(snapshot, manifest)`는 아래 순서를 **엄격히** 따른다. 어느 단계가 halt/abort를 내면 이후 단계는 실행하지 않는다.

```
(1) 런타임 한도 재검사(R-LIMIT-RT-01)
 -> (2) no-op 조기 종료 판정(R-NOOP-01)
 -> (3) Negotiate: 매니페스트 제시 -> WantSet 수신(R-WANT-*)
 -> (4) Transfer: want blob 순차 전송(각 blob = Reverify -> Send, R-REVERIFY-*/R-CHUNK-*)
 -> (5) Commit: 경로->해시 맵 + manifest_digest 전송(R-COMMIT-*)
 -> (6) Done(CommitOutcome)
```

- **단일 직렬 사이클(Q2=B)**: 한 트리거당 위 흐름을 끝까지 1회 수행하고, blob 병렬 전송은 없다(순차, D-02).
- **drive-not-sleep(D-16)**: 실패는 `UploadError`로 즉시 반환한다. 백오프/재시도 대기(sleep)·재시도 루프는 **U4 소관**이며 U3는 수행하지 않는다.

---

## 2. no-op 조기 종료 규칙 (FR-10, US-E2-06)

**규칙 R-NOOP-01 (digest 동치 조기 종료)**: 사이클 시작 시 현재 `manifest.manifest_digest`가 `SyncStateStore::last_committed_manifest()`의 `manifest_digest`와 **동일**하면(마지막 커밋 존재 시), 사이클은 **no-op**이다:

```
manifest.manifest_digest == last_committed.manifest_digest
  =>  Negotiate/Transfer/Commit 미발생, CommitOutcome{ server_vault_content_id: None, committed: false } 반환
```

- O(1) 판정이며 서버 왕복이 없다. `manifest_digest`는 로컬 no-op 판별자이지 권위 `vault_content_id`가 아니다(FQ-1=A). U0 R-NOOP-02(digest 동치 <=> `ChangeSet::is_empty()`)와 정합한다.
- **엣지**: `last_committed`가 `None`(최초 커밋 이력 없음)이면 no-op이 아니며 정상 진행한다. 빈 매니페스트(0-엔트리)도 digest가 존재하므로 규칙이 그대로 적용된다.

**규칙 R-NOOP-02 (커밋 멱등)**: 이미 서버에 존재하는 상태에 대한 반복 커밋은 서버가 no-op으로 처리하며(`CommitOutcome{ committed: false }`), 클라이언트는 이를 성공으로 간주한다(중복 커밋 안전, FR-10).

---

## 3. 런타임 SafetyLimit 재검사 규칙 (US-E2-08, FR-05/FR-19 케이스3)

**규칙 R-LIMIT-RT-01 (매 사이클 재검사)**: 사이클 시작(단계1)마다 `SafetyLimitsValidator::validate(&manifest)`(U1, 3한도)를 재호출해 `LimitVerdict`를 얻고 `LimitDecision`으로 투영한다. 한도 값·경계 정확성(총<=20 GiB / 파일당<=2 GiB / 수<=100k, `actual <= limit` 수용)은 U1이 소유하며 U3는 재정의하지 않는다.

**규칙 R-LIMIT-RT-02 (Exceeded -> halt + 마지막 정상 커밋 유지)**: `Exceeded(report)`이면:
- `StatusSink::raise_condition(OverLimit)`를 push한다.
- 전송/커밋을 수행하지 않고 `UploadError::OverLimit(report)`로 halt한다.
- **마지막 정상 커밋은 보존**된다 — U3는 `SyncStateStore`의 `last_committed`를 변경하지 않는다(커밋 미발행). FR-19 케이스3 알림·CLI/헬스 표면화는 U6/U8 소관.

**규칙 R-LIMIT-RT-03 (Within -> 자동 재개)**: `Within`이면 진행 전 `StatusSink::clear_condition(OverLimit)`를 push한다(이전 사이클이 over-limit였다면 자동 해제 -> 자동 재개, 사용자 개입 불필요). raise/clear는 멱등(set 의미)이므로 상태 없이 매 사이클 안전하게 호출한다(D-12).

---

## 4. have/want 협상 규칙 (FR-07, NFR-09, US-E2-03, US-E7-02)

**규칙 R-WANT-01 (정확한 차집합)**: 협상 산출 want 집합은 매니페스트 blob 해시에서 서버 보유분을 뺀 **정확한 집합 차집합**이다:

```
WantSet.blobs == { e.raw_sha256 | e ∈ manifest.entries }  \  NegotiateResponse.server_has
```

- 전송 단위는 whole-file(서브파일 델타 없음, §4 okc-core 제약). 다중 경로 동일 콘텐츠는 하나의 해시로 dedup된다.

**규칙 R-WANT-02 (멱등성)**: 서버가 want 집합을 보유한 상태로 재실행하면 결과 want는 **공집합**이다. 특히 `server_has ⊇ manifest_hashes`이면 `WantSet.blobs == ∅`이고, 이 경우 Transfer 단계는 즉시 완료되고 Commit으로 진행한다(전부 보유 -> 커밋만, FR-10 정합).

**규칙 R-WANT-03 (전송 대상 한정)**: `WantSet`에 없는 blob(= 서버 보유분)은 **절대 전송하지 않는다**(NFR-09, 대용량 미디어 재전송 방지).

**규칙 R-WANT-04 (협상 오류)**: `AuthTransport::send`가 `TransportError`를 내면 `UploadError::Transport(err)`로 반환하고 사이클을 중단한다(재시도는 U4).

---

## 5. 전송 전 재검증 규칙 (Q8=A, FR-22, US-E2-01 정합)

**규칙 R-REVERIFY-01 (전송 직전 재-읽기 + 재-해시)**: 각 want blob을 전송하기 **직전**, U3는 `BlobSource::open(path)`로 blob 바이트를 **스스로 재-읽기**하여 `ContentAddressing::hash_stream`(U1, 스트리밍 SHA-256)으로 재-해시한다. 결과가 매니페스트 엔트리의 `raw_sha256`(= `expected`)과 **일치할 때만** 전송한다.

**규칙 R-REVERIFY-02 (불일치 시 커밋 중단)**: 재-해시 결과가 `expected`와 다르면(열거~해시~전송 간 파일 변경, TOCTOU):

```
UploadError::HashMismatch { path, expected, actual } 반환
  =>  이 사이클의 Commit 미발행(그 커밋 중단), last_committed 불변, 다음 사이클이 재스냅샷
```

- 이는 서버가 매니페스트 해시와 어긋난 바이트를 수신하는 것을 원천 차단한다(일관 스냅샷, FR-22). 다음 트리거 사이클이 새 매니페스트로 재스냅샷해 자동 흡수한다(FQ-2=A dirty 재진입).

**규칙 R-REVERIFY-03 (검증-전송 패스 분리, MVP)**: 재검증 패스(재-읽기+재-해시)와 전송 패스(재-읽기+전송)를 **분리**한다(2회 읽기). 단일-패스 hash-while-send 최적화는 도입하지 않는다(정확성 우선, NFR Design 이월, D-05). blob 바이트를 전량 메모리에 적재하지 않고 스트리밍한다.

**규칙 R-REVERIFY-04 (읽기 I/O 실패)**: `BlobSource::open`/스트림 읽기 실패는 패닉 없이 `UploadError::Aborted`(또는 진단 상세 동반)로 표면화하고 사이클을 중단한다(다음 사이클 재시도).

---

## 6. 임계값 S 및 재개 청크 규칙 (FR-08, NFR-10, US-E2-04, US-E7-03)

**규칙 R-XFER-01 (단일 vs 청크 분기)**: blob 크기 `size`에 대해:
- `size <= S` -> **단일 요청** 전송을 허용한다.
- `size > S` **또는** 단일 요청이 실패/타임아웃 -> **재개 가능 청크** 전송으로 전환한다.

여기서 `S`(`chunk_threshold_bytes`)는 주입 config 값이다(D-10). 파일당 상한 2 GiB(U1 한도)를 초과하는 blob은 한도 재검사(R-LIMIT-RT-01)에서 이미 걸러진다.

**규칙 R-CHUNK-01 (바이트 재조립 무결성)**: 청크 프레임을 `offset` 오름차순으로 concat하면 원본 blob을 **바이트 단위로** 재구성한다:

```
concat( frame.bytes for frame in sort_by_offset(frames) ) == original_blob_bytes
```

- 청크 크기는 주입 config의 **고정값**(`chunk_size_bytes`, 적응형 없음, D-07). 마지막 청크만 `<= chunk_size`. 각 프레임은 `chunk_sha256`(per-chunk integrity, FR-08)을 동반한다.

**규칙 R-CHUNK-02 (임의 오프셋 재개 동치)**: 임의의 유효 오프셋 `k`(`0 <= k <= total`, 청크 경계)에서 재개해 재조립한 결과는 중단 없이 전송한 결과와 **동일**하다. 재개는 `[0, k)` 구간을 재전송하지 않는다.

**규칙 R-CHUNK-03 (재조립 해시 == 매니페스트 해시)**: 재조립된 blob의 `raw_sha256`은 매니페스트에 기록된 해시와 일치한다(NFR-10/FR-22). 즉 재검증(R-REVERIFY-01)을 통과한 바이트열과 청크 재조립 결과가 동치다.

---

## 7. 재개 오프셋 지속 규칙 (Q6=C, U4 연계)

**규칙 R-RESUME-01 (오프셋 keying)**: 재개 오프셋은 blob의 `raw_sha256`(`Sha256Digest`)을 키로 `SyncStateStore`에 지속된다(Q6=C). 사이클 시작 시 `resume_offset(&blob)`로 마지막 ack 오프셋을 읽어 청크 `start_offset`으로 삼는다(없으면 0).

**규칙 R-RESUME-02 (ack마다 지속)**: 서버가 청크 오프셋을 ack할 때마다 `persist_resume_offset(blob, new_offset)`로 진행 오프셋을 지속한다. 이로써 크래시/중단 후 재시작이 마지막 ack부터 재개한다(NFR-03 연계). 개념 불변식 `resume_offset <= transferred_bytes <= total`(U0 `TransferResult::Partial` 정합)을 위반하지 않는다.

**규칙 R-RESUME-03 (커밋 성공 시 clear 흡수)**: 커밋 성공 시 오프셋 clear는 U4 `commit_manifest`가 **한 번의 원자 쓰기로 흡수**하므로 U3가 별도 clear를 호출하지 않는다. HashMismatch/Abort로 사이클이 커밋 없이 끝나면 오프셋은 그대로 두며(해시 keying이라 변경 파일의 stale 오프셋은 자연 orphan), 다음 성공 커밋이 정리한다. 필요 시 `clear_resume_offsets()`를 명시 호출할 수 있으나 MVP 기본은 비호출이다(D-09).

---

## 8. 커밋 규칙 (FR-06/09, US-E2-05, DEP-03)

**규칙 R-COMMIT-01 (봉투 구성)**: 모든 want blob이 서버에 존재하면(Transfer 완료) `CommitRequest = { path_hash_map, manifest_digest }`를 전송한다. `path_hash_map`은 `Manifest.entries`에서 `(relative_path -> raw_sha256)`로 투영한 `PathHashMap`(BTreeMap, canonical 정렬)이다.

**규칙 R-COMMIT-02 (서버 책임 경계, [blocked-on-server])**: 바이트 디스크 구체화·경로 바인딩·권위 `vault_content_id` 계산은 **서버 책임**이다(FR-09, DEP-03). 클라이언트는 커밋 요청 조립과 `CommitOutcome` 해석만 검증한다(목 계약). `server_vault_content_id`는 불투명 값으로 로컬 판정에 쓰지 않는다(FQ-1=A).

**규칙 R-COMMIT-03 (원시 볼트)**: 업로드/커밋 대상은 원시 볼트(raw vault)이며 컴파일 산출물이 아니다(FR-06).

---

## 9. 전송 경로·상태 push 규칙 (NFR-06, US-E2-07)

**규칙 R-HTTP-01 (유일 아웃바운드 경로)**: 모든 서버 통신은 주입된 `AuthTransport::send(OkcRequest)`를 통해서만 이루어진다. U3는 소켓/HTTP 클라이언트를 직접 열지 않으며, TLS 강제·토큰 첨부·타임아웃·전송 오류 분류는 U5가 소유한다(U3는 토큰을 취급하지 않는다). 2xx 본문 해석(want/commit 응답 디코드)만 U3가 수행한다.

**규칙 R-STATUS-01 (진행률 push)**: 전송량이 임계값 S를 초과하는 대용량/초기 동기화(또는 청크 전송) 진행 중, `StatusSink::set_resume_progress(transferred, total)`로 진행도를 push한다(US-E2-07, CLI status + 로그 표면; GUI/트레이 비의존). S 이하의 일반 증분 전송에는 진행률 오버레이를 push하지 않는다.

**규칙 R-STATUS-02 (push 범위 한정)**: U3의 `StatusSink` 사용은 `set_resume_progress`(진행률)와 `raise_condition`/`clear_condition(OverLimit)`(§3)로 **한정**한다. 운영 라이프사이클(`set_operational`)·`record_sync_success`·liveness는 U8 코디네이터 소관이다(D-15). 상태 push는 best-effort·infallible(U0 계약)이므로 실패해도 사이클 결과를 바꾸지 않는다.

---

## 10. 오류 처리·전체성 규칙

**규칙 R-ERR-01 (taxonomy 보존 반환)**: 사이클 실패는 `UploadError`{`Transport`/`HashMismatch`/`OverLimit`/`Aborted`}로 표면화하며, `Transport`는 U0 `TransportError`(및 그 `TransportErrorClass`)를 그대로 감싼다. U3는 재시도 분류(`ErrorClass`)를 재계산하지 않고 상위(U4)에 위임한다(관심사 분리).

**규칙 R-ERR-02 (패닉 없음)**: 드라이버 경로는 패닉하지 않는다 — 모든 실패는 `Result`로 표면화한다(전량 적재·인덱싱 회피). 인코딩/디코딩 실패(프로토콜 본문)는 `UploadError::Aborted`(또는 Transport 계열)로 사상하고 재시도 대상 판정은 U4가 수행한다.

---

## 11. Testable Properties (PBT-01, Full 강제)

> **PBT 카테고리 라벨**: [algebraic]=대수적 순수 함수 불변식 · [roundtrip]=직렬화/재조립 왕복 · [model]=상태머신 모델 기반 · [metamorphic]=변형 관계. 제너레이터(PBT-07)는 U3 도메인 생성기(런타임 그래프 밖 test-support)로 정의한다. 프레임워크(PBT-09)는 proptest 유력(NFR Requirements 이월).

| ID | 속성 | 카테고리 | 제너레이터(PBT-07) | 근거 |
|---|---|---|---|---|
| **PROP-U3-01** | `WantSet == manifest_hashes \ server_has` (정확 차집합) | [algebraic] | 랜덤 매니페스트(경로/해시/크기) + 임의 `server_has` 부분집합/초과집합 | NFR-09, FR-07, US-E7-02 |
| **PROP-U3-02** | 서버가 want 보유 후 재실행 시 want == ∅ (멱등); `server_has ⊇ manifest_hashes` -> want == ∅ | [algebraic]/[metamorphic] | 위 + "1차 want를 server_has에 합집합" 변형 | NFR-09, FR-10, US-E7-02 |
| **PROP-U3-03** | 임의 blob 바이트·임의 청크 분할에 대해 `concat(sort_by_offset(frames)) == original` (바이트 일치) | [roundtrip] | 임의 길이 blob 바이트열 + 임의 `chunk_size`/경계 | NFR-10, FR-08, US-E7-03 |
| **PROP-U3-04** | 임의 유효 오프셋 `k`에서 재개 재조립 == 무중단 전송 결과 | [metamorphic] | 위 + 임의 재개 오프셋(청크 경계) | NFR-10, US-E7-03 |
| **PROP-U3-05** | 재조립 blob의 `raw_sha256` == 매니페스트 기록 해시 | [roundtrip] | 위 + `ContentAddressing::hash_stream` 대조 | NFR-10, FR-22, US-E7-03 |
| **PROP-U3-06** | 재-읽기 바이트 해시 != `expected` -> `HashMismatch` 반환 & 커밋 프레임 미발행 & `last_committed` 불변 | [model] | 변조 바이트 주입 mock `BlobSource` + 정상/변조 케이스 | Q8=A, FR-22 |
| **PROP-U3-07** | `manifest_digest == last_committed.digest` -> negotiate/transfer/commit 호출 0회 & `committed == false` | [model] | 동일/상이 digest 매니페스트 쌍 + 호출 카운팅 mock 전송 | FR-10, US-E2-06 |
| **PROP-U3-08** | `CyclePhase` 전이는 합법 순서만 밟는다(`Negotiate -> Transfer -> Commit -> Done` \| `NoOp` \| `Abort`); 단일 직렬 사이클, sleep/retry 0회 | [model] | 성공/실패/변조/over-limit/no-op 시나리오 시퀀스 | Q2=B, US-E7-02/03 |

- **transport 독립성**: PROP-U3-01..05는 순수 함수(차집합/청크 재조립)에 대한 속성이라 전송 없이 검증한다. PROP-U3-06..08은 `HttpTransport` mock seam + mock `BlobSource`로 상태머신을 구동한다.
- 각 속성은 shrinking·고정 시드·CI 통합(PBT-08)을 요구한다. 최종 속성 형태·수치는 본 산출물이 확정하며(PBT-01), 프레임워크/시드 정책은 NFR Requirements로 이월한다.
- **No PBT properties identified**: `VaultContentId`(불투명 서버 식별자), 진행률 push 값(best-effort 관측, 결정 로직 아님).

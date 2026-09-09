# U3 upload-client — Functional Design 계획 및 결정 (AUTOPILOT)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U3 Upload Protocol Client** -> Functional Design (Part: 계획 + 결정)
**작성일**: 2026-09-08
**크레이트**: `upload-client` (lib) · **소속 컴포넌트**: `UploadProtocolDriver`
**입력 아티팩트**: `unit-of-work.md`(§U3 책임·얇은-단위 근거, §0 노트1/2), `component-methods.md`(§UploadProtocolDriver 시그니처·이월 항목 + 노트1 Q8=A), `components.md`(§U3), `unit-of-work-story-map.md`(U3 소유 스토리 행 9~14, 35~36), `stories.md`(US-E2-03..08, US-E7-02/03), `requirements.md`(FR-06/07/08/09/10/22, NFR-06/09/10, DEP-03, §12.2 FQ-2 오버레이, §13 토큰), 실제 의존 크레이트 소스 `crates/foundation/src/**`·`crates/content-core/src/**`·`crates/sync-state/src/**`·`crates/auth-consent/src/**`, U0 Functional Design 3종(스타일·rule-id/property-id 템플릿)
**규칙**: `construction/functional-design.md` · `common/content-validation.md` · 활성 확장 `property-based-testing.md`(PBT-01 Full 강제) · `resiliency-baseline.md`
**모드**: **AUTOPILOT** — 사용자 게이트 면제(2026-09-08 승인). 모든 미결정 항목은 권장안 + MVP 편향으로 저자가 확정하며, 질문을 제기하지 않는다(§3 결정 표가 질문을 대체). 이미 확정된 항목(§5 drop-list)은 재설계·재질문하지 않는다.

---

## 1. 단위 컨텍스트 (Step 1 — 요약)

U3는 컴포넌트 1개(`UploadProtocolDriver`)로 이루어진 얇은 단위지만, **U1·U4·U5가 모이는 합류점의 다단계 업로드 프로토콜 상태머신**을 담는다. W3 웨이브 소비자로서 이미 빌드·테스트·clippy-clean 상태인 W2 크레이트의 실제 공개 API만 소비하며(아래), 그 크레이트를 수정하지 않는다.

- **소비 의존(실제 API, 재정의 금지)**:
  - **U0 `foundation`**: 값 타입(`Manifest`/`ManifestEntry`/`RelativePath`/`Sha256Digest`/`ManifestDigest`/`ByteCount`/`Timestamp`), 오류 taxonomy(`TransportError`/`TransportErrorClass`/`ClassifiedError`/`ErrorClass`), 전송 결과 `TransferResult`, 무손실 코덱 `encode`/`decode`, 상태 push 계약 `StatusSink` 트레이트.
  - **U1 `content-core`**: `ContentAddressing::hash_stream`(전송 전 재검증 재-해시, R-CA), `ContentAddressing::manifest_digest`(no-op 판정용, 참고), `SafetyLimitsValidator::validate(&Manifest) -> LimitVerdict`(매 사이클 런타임 한도 재검사), `LimitVerdict`/`LimitViolation`.
  - **U4 `sync-state`**: `SyncStateStore::last_committed_manifest() -> Option<&Manifest>`(no-op diff 기준), `resume_offset(&Sha256Digest) -> Option<ByteCount>`, `persist_resume_offset(Sha256Digest, ByteCount)`, `clear_resume_offsets()`, `commit_manifest(Manifest)`(성공 시 오프셋 clear 흡수).
  - **U5 `auth-consent`**: `AuthTransport::send(OkcRequest) -> Result<OkcResponse, TransportError>`(**유일 아웃바운드 HTTP 경로**), 봉투 타입 `OkcRequest`/`OkcResponse`/`HttpMethod`/`Headers`/`Body`. 재검증/mock은 하위 `HttpTransport` seam 계약(`crates/auth-consent/src/transport/types.rs`)으로 구동한다.

- **소유 스토리**: US-E2-03(have/want 협상, MVP), US-E2-04(재개 청크 전송, MVP), US-E2-05(원시 볼트 커밋·바인딩, MVP), US-E2-06(멱등 no-op), US-E2-07(대용량 진행률·"재개 중"), US-E2-08(런타임 SafetyLimit 초과 처리), US-E7-02(have/want 정확성·멱등성 속성), US-E7-03(재개 청크 재조립 바이트 일치 속성).

- **비소유(위임) 경계**: 재시도/백오프 스케줄 = U4 `RetryBackoffController`(드라이버는 분류된 결과만 반환하고 **잠들지 않는다**). 동의 게이트 = U5 `ConsentGate`(U8이 사이클 전 검사). 트리거 소싱·사이클 직렬화·운영 라이프사이클 push = U8 `SyncCycleCoordinator`. 파일 열거 = U1 `VaultScanner`(U3는 재-읽기 바이트 소스를 seam으로 주입받아 `VaultScanner`에 직접 의존하지 않음, §0 노트1).

---

## 2. Functional Design 실행 계획 (산출물 체크박스)

아래 3개 산출물을 `aidlc-docs/construction/u3-upload-client/functional-design/` 에 생성한다. 기술중립(Rust스러운 시그니처는 참고용이며 인프라·스레딩·정확 튜닝 수치는 배제). **프런트엔드 파일 없음**(U3는 UI 없는 순수 백엔드 라이브러리; 진행률/상태는 CLI status + 로그로만 노출, 데몬 모델).

- [x] **`domain-entities.md`** — U3가 도입하는 값 타입/포트: `WantSet`(want blob 해시 집합), `PathHashMap`(경로->해시 커밋 맵), `CommitRequest`/`NegotiateRequest`/`CommitOutcome`(프로토콜 봉투, [blocked-on-server] 잠정), `ChunkPlan`/`ChunkFrame`(재개 청크 프레이밍), `CyclePhase`(상태머신 상태), `LimitDecision`, `UploadError`, `BlobSource` 포트 트레이트(재-읽기 바이트 소스 seam). U0/U1/U4/U5 타입(`Manifest`/`Sha256Digest`/`OkcRequest`/`LimitVerdict`/`TransferResult` 등)은 **이름으로 참조만**, 재정의 금지. -> 컴포넌트: `UploadProtocolDriver`
- [x] **`business-rules.md`** — R-* 규칙: no-op 조기 종료(digest 동치), 런타임 한도 재검사 + OverLimit halt/auto-resume, have/want 차집합·멱등 규칙, 임계값 S 기반 단일 vs 청크 분기, 재개 오프셋 keying·지속·clear 시점, per-chunk 무결성·바이트 재조립 불변식, 전송 전 재검증(Q8=A)·HashMismatch 커밋 중단, 커밋 봉투 구성·멱등, 유일 HTTP 경로·재시도 미소유·drive-not-sleep, StatusSink push 범위. + PROP-* Testable Properties(NFR-09/10, US-E7-02/03). -> 규칙: `UploadProtocolDriver`
- [x] **`business-logic-model.md`** — 알고리즘·워크플로: `execute_cycle` 오케스트레이션 순서, `CyclePhase` 상태머신 전이(Negotiate -> Transfer -> Reverify -> Commit -> Done/NoOp/Abort), `compute_want` 순수 차집합, `transfer_blob` 단일/청크 분기 + 재개 + 재검증, 청크 프레이밍/재조립 데이터 흐름, `commit` 봉투 조립, 교차 단위 경계(소유 vs 소비 API 명시), 컴포넌트별 Testable-Properties 노트 + 확장 컴플라이언스 요약. -> `UploadProtocolDriver`
- [x] **PBT-01 "Testable Properties" 섹션**(각 산출물 내, 확장 강제): PROP-U3-01(have/want 차집합 정확성, NFR-09/US-E7-02) · PROP-U3-02(have/want 멱등성, NFR-09/US-E7-02) · PROP-U3-03(청크 재조립 바이트 일치, NFR-10/US-E7-03) · PROP-U3-04(임의 오프셋 재개 동치, NFR-10/US-E7-03) · PROP-U3-05(재조립 blob `raw_sha256` == 매니페스트 해시, NFR-10/FR-22) · PROP-U3-06(재검증 가드: 불일치 시 HashMismatch·커밋 미발행, Q8=A) · PROP-U3-07(no-op 조기 종료: digest 동일 시 협상/전송/커밋 미발생, FR-10) · PROP-U3-08(사이클 상태머신 model-based 합법 전이, US-E7-02/03). 각 속성에 PBT 카테고리 라벨 + 도메인 제너레이터(PBT-07) 요구 기재. 속성 없는 요소는 "No PBT properties identified" 명시.
- [x] **확장 컴플라이언스 요약**(완료 게이트용): PBT ON(Full) / Resiliency ON / Security OFF 표(§4).
- [x] 산출물 작성 전 `content-validation.md` 검증: ASCII 화살표 `->`만(유니코드 화살표 코드포인트 0건), 박스드로잉/유니코드 다이어그램 글리프 0건, Rust 타입/제네릭 백틱, Korean 산문.

---

## 3. AUTOPILOT 결정 표 (질문 대체 — 권장안 + MVP 편향)

| # | 주제 | 채택안 | MVP 트림? | 근거 + 인용 |
|---|---|---|---|---|
| D-01 | 프로토콜 상태 모델 | 명시적 `CyclePhase` enum(`NoOp`/`Negotiate`/`Transfer`/`Reverify`/`Commit`/`Done`/`Abort`)로 모델링해 PBT가 상태머신을 구동 | **예** | MVP 지침 "명시적 state enum으로 PBT 구동". component-methods "오케스트레이션 순서 FD 이월" |
| D-02 | 사이클 동시성 | 트리거당 **1회 직렬 사이클**, blob 병렬 업로드 없음(순차 전송) | **예** | Q2=B(drop-list). MVP 지침 "단일 직렬 전송(병렬 없음)" |
| D-03 | 재-읽기 바이트 소스 | U3 소유 `BlobSource` 포트 트레이트(`open(&RelativePath) -> Read 스트림`)를 **주입**받아 재검증·전송 재-읽기 수행. `VaultScanner`(U1)에 직접 의존하지 않음; U8이 `VaultScanner::open_reader`를 이 포트로 배선 | **예** | §0 노트1 "스스로 재-읽기, `VaultScanner` 의존 불필요(바이트 스트림 해시)". 의존 목록에 U1은 `ContentAddressing`+`SafetyLimitsValidator`만. seam은 mock 재-읽기로 PBT 격리 |
| D-04 | 전송 seam | 유일 HTTP 경로 = 주입된 `AuthTransport`(구체 struct)의 `send(OkcRequest)`. 프로토콜 PBT는 `AuthTransport` + `HttpTransport` **mock seam**(`testing::MockHttpTransport`)으로 구동 | **예** | 태스크 "Transport ONLY via AuthTransport/HttpTransport seam", "design against HttpTransport mock/seam contracts". 별도 래퍼 트레이트 미도입(불필요 추상 회피) |
| D-05 | 재검증 시점·패스 | 각 blob 전송 **직전** `BlobSource`로 재-읽기 -> `ContentAddressing::hash_stream` 재-해시 -> 매니페스트 `raw_sha256` 비교. 불일치 시 `UploadError::HashMismatch`로 그 사이클 커밋 **중단**, 다음 사이클 재스냅샷. 검증 패스와 전송 패스를 **분리**(2회 읽기) | **예** | Q8=A(drop-list, re-verify-before-send). 노트1. 단일-패스 hash-while-send 최적화는 미도입(정확성 우선, NFR Design 이월) |
| D-06 | 단일 vs 청크 분기 | blob 크기 `<= S` -> 단일 요청 허용; `> S` **또는** 단일 요청 실패/타임아웃 -> 재개 가능 청크 전송 | 아니오 | FR-08, US-E2-04 체크리스트 |
| D-07 | 청크 크기 | 주입된 config의 **고정 청크 크기**(적응형/동적 협상 없음) | **예** | MVP 지침 "고정 chunk size from injected config" |
| D-08 | per-chunk 무결성 | 각 청크 프레임에 `(offset, len, chunk raw_sha256)` 동반; 서버가 오프셋 ack. 재조립은 오프셋 순서 concat == 원본 바이트열 | 아니오 | FR-08 "per-chunk integrity". [blocked-on-server] 정확 프레이밍은 서버 프로토콜 확정 시 정합 |
| D-09 | 재개 오프셋 keying·지속 | blob = `raw_sha256`(`Sha256Digest`) 키. 사이클 시작 시 `SyncStateStore::resume_offset`로 마지막 ack 읽기, 청크 ack마다 `persist_resume_offset`, 커밋 성공 시 `commit_manifest`가 clear 흡수 | 아니오 | Q6=C(drop-list). U4 실제 API. `TransferResult::Partial{resume_offset}` |
| D-10 | 임계값 S / 청크 크기 config 키 | U3 소유 federated known-key `chunk_threshold_bytes`(S) + `chunk_size_bytes`; U8 조립루트가 파싱·검증해 드라이버에 하향 주입(U0 `ConfigSnapshot` core 6필드에서 읽지 않음) | 아니오(경계) | U5 `request_timeout_s` 패턴 동형(`AUTH_CONSENT_CONFIG_KEYS`). R-CFG-STRICT(U0) 미지-키 거부 회피 |
| D-11 | no-op 조기 종료 | 사이클 시작 시 현재 `manifest.manifest_digest == last_committed.manifest_digest` -> negotiate/transfer/commit **미발생**, `CommitOutcome{committed:false}` 반환(O(1)) | 아니오 | FR-10, US-E2-06. R-NOOP-02(U0 business-rules) digest 동치 |
| D-12 | 런타임 한도 처리 | 매 사이클 시작 `SafetyLimitsValidator::validate` 재호출. `Exceeded` -> `StatusSink::raise_condition(OverLimit)` + `UploadError::OverLimit` halt(마지막 정상 커밋 유지); `WithinLimits` -> `clear_condition(OverLimit)` 후 진행(자동 재개) | **예** | US-E2-08. 별도 halt 상태 지속 없이 idempotent raise/clear로 auto-resume 실현 |
| D-13 | 커밋 봉투 | `CommitRequest = { path_hash_map: PathHashMap, manifest_digest }`. 서버가 바이트 구체화·경로 바인딩·`vault_content_id` 계산. 반복 커밋 no-op | 아니오 | Q6=C, FR-06/09/10. component-methods `CommitOutcome{server_vault_content_id, committed}` |
| D-14 | 와이어 인코딩 | 요청/응답 본문(`Body` 바이트)은 **U0 CBOR 코덱**(`encode`/`decode`)으로 잠정 직렬화(NFR-13 재사용). 정확 스키마·엔드포인트 경로는 [blocked-on-server] | **예** | DEP-03 목 계약. 신규 와이어 포맷 미도입(NFR-13 코덱 재사용) |
| D-15 | StatusSink push 범위 | U3는 주입된 `StatusSink`로 **진행률**(`set_resume_progress`, US-E2-07)과 **OverLimit raise/clear**(US-E2-08)만 push. 운영 라이프사이클(`set_operational`)·`record_sync_success`는 U8 코디네이터 소관 | **예** | component-methods "진행률·over-limit은 StatusService로 push". U8이 사이클 오케스트레이션 push 소유(§0 노트2 확장) |
| D-16 | 드라이버 반환 형상 | `execute_cycle -> Result<CommitOutcome, UploadError>`. `UploadError`{`Transport(TransportError)`/`HashMismatch`/`OverLimit(LimitReport)`/`Aborted`}를 taxonomy 그대로 상위 반환, **재시도/sleep 없음** | 아니오 | component-methods `UploadError` enum. MVP 지침 "drive-not-sleep, U4가 분류·백오프". |

> 재확인(재-결정 아님): 위 표는 **함수 설계 수준**의 미결정만 확정한다. 인프라·스레딩·정확 튜닝 수치(S/청크 크기 기본값, 서버 엔드포인트 경로)는 NFR Requirements/Design 또는 서버 프로토콜 확정으로 이월한다.

---

## 4. MANDATORY 카테고리 N/A + 확장 컴플라이언스 (완료 게이트용)

### 4.1 MANDATORY 카테고리 적용 판정

| MANDATORY 항목 | 이 단계 적용 | 판정/근거 |
|---|---|---|
| Content Validation | 적용 | ASCII 화살표(`->`)만, 박스드로잉 0건, Rust 타입 백틱, Korean 산문 — 전 산출물 강제 |
| Mermaid/ASCII 다이어그램 검증 | **N/A** | Mermaid 미사용; 흐름은 화살표 표기(`A -> B`) + 순서 목록/표로만 기술 |
| Question File Format | **N/A** | AUTOPILOT — 질문 미발행(§3 결정 표가 대체) |
| Welcome Message | **N/A** | 신규 워크플로 시작 아님(진행 중 per-unit 루프) |
| 감사 로그(audit.md) | 범위 밖 | STRICT WRITE SCOPE상 U3 산출물만 작성; audit.md는 오케스트레이터 소관 |

### 4.2 확장 컴플라이언스

| 확장 | 활성 | 이 단계 적용 | 계획 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제(PBT-01)** | 각 산출물에 Testable Properties 섹션 필수. 핵심: PROP-U3-01/02 have/want 차집합·멱등(NFR-09/US-E7-02), PROP-U3-03/04/05 청크 재조립·재개 동치·해시 일치(NFR-10/US-E7-03), PROP-U3-06 재검증 가드(Q8=A), PROP-U3-07 no-op(FR-10), PROP-U3-08 상태머신 model-based. 카테고리 라벨 + 제너레이터(PBT-07: 매니페스트/blob 바이트/청크 분할/server_has 집합) 요구 기재. 프레임워크(PBT-09)는 Rust=proptest 유력, NFR Requirements 이월. 미준수 시 blocking. |
| **Resiliency Baseline** | ON | **핵심 적용** | RESILIENCY-01: U3 = **High criticality**(zero-loss 업로드의 프로토콜 실행 지점). 재개 청크(FR-08)·재검증(Q8=A)·no-op 멱등(FR-10)이 부분 실패/중단에도 상태 손상 없이 진행/재개를 보장. 타임아웃·백오프 자체는 U5(타임아웃)/U4(백오프) 소관 — U3는 분류된 `UploadError`를 반환만(drive-not-sleep). 크래시-재개 하니스·회복탄력성 테스트(RESILIENCY-14)는 PROP-U3-04에 씨앗을 두되 상세는 NFR Design 이월. RTO/HA/DR·배포는 인프라 소관 N/A. |
| **Security Baseline** | OFF | **N/A** | 미로딩·미강제. 토큰 첨부/TLS 강제(NFR-06)는 U5 `AuthTransport` 소관(U3는 `send`만 호출, 토큰 미취급). 클라이언트측 콘텐츠 필터링 없음은 수용 위험 RISK-01(NFR-07). |

---

## 5. Drop-list (이미 확정 — 재설계·재질문 금지)

| 확정 항목 | 출처 |
|---|---|
| U0가 `Manifest`/`ManifestEntry`/`RelativePath`/`Sha256Digest`/`ManifestDigest`/`ByteCount`/`Timestamp`/`TransferResult`/`ClassifiedError`/`ErrorClass`/`TransportError`/`TransportErrorClass`/`StatusSink` 소유·구현(컴파일+테스트 완료) | crates/foundation/src/core_types/** |
| 무손실 CBOR 코덱 `encode`/`decode`(NFR-13) | crates/foundation/src/core_types/codec.rs |
| U1 `ContentAddressing`(`hash_stream`/`manifest_digest`) + `SafetyLimitsValidator::validate`/`LimitVerdict`/`LimitViolation` (frozen) | crates/content-core/src/** |
| U4 `SyncStateStore`(`last_committed_manifest`/`resume_offset`/`persist_resume_offset`/`clear_resume_offsets`/`commit_manifest`) (frozen) | crates/sync-state/src/store.rs |
| U5 `AuthTransport::send(OkcRequest) -> Result<OkcResponse, TransportError>` + 봉투 타입 + `HttpTransport` mock seam (frozen) | crates/auth-consent/src/transport/** |
| Q2=B(트리거당 단일 직렬 사이클) | requirements/component-methods; 태스크 drop-list |
| Q6=C(have/want·재개·커밋 keying = `raw_sha256`; 커밋 = 경로->해시 맵 + `manifest_digest`) | requirements §12.2; component-methods |
| Q8=A(전송 전 재검증, 불일치 시 커밋 중단·재스냅샷) | component-methods 노트1; 태스크 drop-list |
| FQ-1=A(서버가 `vault_content_id` 소유; `ManifestDigest`는 로컬 no-op 판별자) | requirements §12.1 |
| FQ-2=A(최신 상태 대체, 이벤트 큐 없음) | requirements §12.2 |
| 안전 한도 상수(총<=20 GiB, 파일당<=2 GiB, 수<=100k) | crates/foundation config/model.rs |
| TLS-only(NFR-06), 토큰 §13 저장 = config 평문(+env) | requirements §12/§13 |
| 서버 프로토콜 = [blocked-on-server] -> mock/seam 계약으로 설계 | requirements DEP-03 |

---

## 6. 다음 단계(참고)
U3 Functional Design 산출물 3종 완료 -> **U3 NFR Requirements**(임계값 S·청크 크기 기본값, 타임아웃/백오프 정합, 대용량 전송 성능·재개 하니스, 서버 프로토콜 목 계약 정밀화). 이후 U3 NFR Design -> (Infrastructure Design SKIP: U3는 인프라 자원 없음) -> U3 Code Generation.

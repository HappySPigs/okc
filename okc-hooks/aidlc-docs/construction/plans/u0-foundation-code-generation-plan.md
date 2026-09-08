# U0 Foundation — Code Generation 계획 (PART 1: PLANNING ONLY)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U0 Foundation** -> Code Generation (Part 1: 계획)
**작성일**: 2026-09-08
**크레이트**: `foundation` (lib) · **공개 애그리게이트**: `CoreTypes`(순수 값/함수/계약 트레이트, 무상태·무IO), `ConfigProvider`(JSON 로드·검증 + 활성 스냅샷 + 관찰자 팬아웃)
**입력 아티팩트**: `u0-foundation/functional-design/`(domain-entities·business-rules·business-logic-model) · `u0-foundation/nfr-requirements/`(nfr-requirements.md·tech-stack-decisions.md) · `u0-foundation/nfr-design/`(nfr-design-patterns.md·logical-components.md) · `inception/application-design/`(components·component-methods·component-dependency·unit-of-work-story-map)
**규칙**: `construction/code-generation.md` Part-1 Steps 2–4 · `common/content-validation.md` · 활성 확장 `property-based-testing.md`(ON, Full)·`resiliency-baseline.md`(ON) · `security-baseline.md`(OFF, RISK-01/02 수용)

---

## 0. 이 문서의 지위 (SINGLE SOURCE OF TRUTH)

> **이 계획서는 U0 Foundation Code Generation Part-2(생성)의 유일한 진실 원천(single source of truth)이다.**
> Part-2는 이 문서에 적힌 단계를 **정확히 그대로** 순서대로 실행하며, 하드코딩된 로직을 추가하지 않는다.
> 각 단계를 완료하는 **동일 상호작용에서 즉시** 해당 체크박스를 `[x]`로 갱신한다(Plan-Level Checkbox Enforcement).
> 이 계획서는 **계획만** 담는다. 어떤 Rust/Cargo/애플리케이션 코드도 이 Part-1에서 생성하지 않는다.

> **[Part-2 실행 완료 — 2026-09-08]** 이 계획의 STEP 1~23 이 워크플로우 `wf_01a34bb1-d7f` 로 전량 실행되어 `crates/foundation` 크레이트(소스 21 파일 + 매니페스트 2 + 테스트 2, 약 2,322 LOC)가 워크스페이스 루트에 생성되었다. 정적 Rust 일관성 크리틱 판정 = **PASS**(would_compile / would_compile_tests / missing_docs / lint_gates / plan_adherence / no_extra_deps 전부 통과, blocking finding 0, non-blocking 4). 후속 트리아지: sink/observer 트레이트에 `Send + Sync` 상위트레이트 추가(데몬 스레드 공유 요건 — 하류 공개 트레이트 파괴적 변경 예방) + `ConfigStore` 의 `ArcSwapOption` 대체 근거 주석 추가. **이연 항목**(이 단계에서 결정/기록 완료로 표시되나 실제 이행은 Build-and-Test): PBT-08 케이스수/시드/CI 통합, 정확 patch 핀 + MSRV CI 검증, USE-01 did-you-mean, 그리고 **`cargo build`/`test`/`clippy` 실제 실행**(현 환경에 Rust 툴체인 미설치 — 정적 크리틱으로 대체). 수용된 비-치명 편차: generators.rs 잔여 identity `prop_map`(비-default feature, 컴파일 정상), validator token `.trim().is_empty()`(자격증명 안전측 엄격화).

**코드 위치 규칙(확정)**:
- 애플리케이션 코드: 워크스페이스 루트 `/Users/sihun/workspace/projects/okc-hooks` (절대 `aidlc-docs/` 아래 아님).
- 마크다운 코드 요약: `/Users/sihun/workspace/projects/okc-hooks/aidlc-docs/construction/u0-foundation/code/` 에만.

**콘텐츠 검증(필수)**: ASCII 화살표 `A -> B` 만 사용, 유니코드 화살표/박스-드로잉 글리프 금지, Rust 제네릭/타입/식별자는 백틱으로 감싼다(`Vec<u8>`, `Arc<WatcherConfig>`, `dyn ConfigReloadObserver`). 모든 doc 주석은 한국어로 작성.

---

## 1. 단위 컨텍스트 (code-generation.md Step 3)

- **구현 스토리**: U0의 유일한 PRIMARY 스토리 = **US-E7-06**(직렬화 무손실 round-trip, NFR-13). 그 외 U0 커버리지는 전부 cross-cutting foundation.
- **하위 단위 의존**: U0는 의존성 DAG의 **루트**로서 어떤 단위(U1..U8)에도 의존하지 않으며 어떤 하위 단위도 import 하지 않는다(비순환 유지). 하류 단위 U1..U8 및 `watcher-bin`이 U0를 링크한다.
  - 이 크레이트를 링크하는 하류 단위(선언): **U1** Content Core, **U2** Change Detect, **U3** Upload Client, **U4** Resilience&Retry, **U5** Auth&Consent, **U6** Observability, **U7a** Deploy&Update, **U7b** Ops Control, **U8** Orchestration + thin `watcher-bin`.
- **U0가 계약만 정의하고 실행/구현은 하류가 소유하는 경계**:
  - `manifest_digest` 계산 + `ChangeSet` diff 계산 -> **U1**
  - `SafetyLimits` 검사(경계/단조성) -> **U1**
  - `SyncState` 영속/복구 + stateful PBT 실행 -> **U4**
  - 토큰 최종 해석(secure-store>config>env) -> **U5**
  - Sink 구현체 + `LogRecord`/`UploadHistoryRecord` 스키마 + condition->state 파생 + `health_check`/`update_probe` 임계값 -> **U6**
  - federated known-key `const &[&str]` 정의 및 union 주입 -> 각 하위 단위 + `watcher-bin`
- **U0 시그니처에 등장하나 소유는 타 단위인 참조 전용 타입**: `LogRecord`(U6), `UploadHistoryRecord`(U6), `ConsentGrant`(U5), `ConsentState`(U5, `StatusSnapshot` 필드).

---

## 2. 제안 물리 레이아웃 (Step 2 — greenfield 구조; doc-granularity guard 하 최소 매핑)

NFR Design의 논리 분해는 **추적성 맵**이며 물리 모듈/크레이트 증식을 강제하지 않는다. 아래는 12개 런타임 논리 컴포넌트를 **최소** 모듈로 매핑한 물리 레이아웃(복수 논리 컴포넌트가 한 모듈을 공유할 수 있음)이다.

```text
/Users/sihun/workspace/projects/okc-hooks/
  Cargo.toml                       # [workspace] 루트 매니페스트: members, [workspace.package], [workspace.dependencies]
  crates/
    foundation/
      Cargo.toml                   # foundation lib 크레이트 멤버 매니페스트 + [features] proptest-support
      src/
        lib.rs                     # 크레이트 루트: #![deny(missing_docs)], 모듈 선언, 공개 재-export
        core_types/
          mod.rs                   # CoreTypes 애그리게이트 모듈 루트 + 순수 모듈 clippy lint-gate 적용
          path.rs                  # PathNormalizer: `RelativePath`
          primitives.rs            # `Sha256Digest`,`ManifestDigest`,`Timestamp`,`ByteCount`
          manifest.rs              # 값 모델: `ManifestEntry`,`Manifest`,`ChangeSet`
          error.rs                 # ErrorTaxonomy: `ErrorClass`,`TransportErrorClass`,`TransportError`,`ClassifiedError`,`TransferResult`,`CodecError`
          token.rs                 # TokenSecret: redacting newtype
          sync_state.rs            # SyncStateModel: `SyncState` + T1..T8 전이 모델(문서/모델만)
          status.rs                # StatusVocab: `OperationalState`,`ActiveCondition`,`LivenessSignal`,`StatusSnapshot`
          sink.rs                  # SinkContracts: `Logger`,`StatusSink`,`HistorySink`,`CriticalEventSink`,`ConfigReloadObserver`(+`LogRecord` 참조 자리표시)
          codec.rs                 # Codec: `encode`/`decode` (ciborium)
        config/
          mod.rs                   # ConfigProvider 애그리게이트 공개 표면: load/current/reload/subscribe
          model.rs                 # `WatcherConfig`,`ConfigSnapshot`
          url_validator.rs         # UrlValidator: url 파싱 + https-only
          validator.rs             # ConfigValidator: 2-pass, `ConfigError`(thiserror, Vec of issues)
          loader.rs                # ConfigLoader: R-DISCOVER-01 경로 우선순위 + 2-pass orchestration
          observer.rs              # ObserverRegistry: 불변 순서 `Vec<Arc<dyn ConfigReloadObserver>>`
          store.rs                 # ConfigStore: `ArcSwap<Arc<WatcherConfig>>` + reload_mutex
        proptest_support/
          mod.rs                   # ProptestGenerators (feature = "proptest-support" 게이트)
          generators.rs            # 도메인 제너레이터
      tests/                       # 통합/속성 테스트 (proptest-support 활성)
```

> ProptestGenerators는 런타임 의존성 그래프 **밖**의 test-support 논리 단위이며 비-default cargo feature `proptest-support` 뒤에 게이트된다(릴리스 런타임 그래프에서 제외).

---

## STEP 1 — 워크스페이스/크레이트 구조 셋업 (greenfield)

- [x] 워크스페이스 루트 `Cargo.toml` 생성: `[workspace]` `resolver = "2"`, `members = ["crates/foundation"]`(하류 단위는 이후 웨이브에서 추가; U0는 루트).
- [x] `[workspace.package]` 에 **Edition 2024** (`edition = "2024"`) + **pinned MSRV** (`rust-version = "1.85"`, Edition 2024와 정합) 지정 — 멤버는 `edition.workspace = true` / `rust-version.workspace = true` 로 상속.
- [x] `[workspace.dependencies]` 에 확정 크레이트 major/minor 시리즈 핀 등록: `serde = { version = "1", features = ["derive"] }`, `ciborium = "0.2"`, `serde_json = "1"`, `url = "2"`, `thiserror = "2"`, `arc_swap = "1"`, `proptest = "1"`. (정확한 patch 핀 + MSRV CI 검증은 Build-and-Test로 이연.)
- [x] `crates/foundation/Cargo.toml` 생성: `[package]` 이름 `foundation`, edition/rust-version workspace 상속; `[dependencies]` 에 `serde`/`ciborium`/`serde_json`/`url`/`thiserror`/`arc_swap` workspace 상속(`.workspace = true`), `proptest` 는 **optional dependency**로 등록.
- [x] **`proptest-support` feature 배선**: `[features] proptest-support = ["dep:proptest"]` (NON-DEFAULT). `[dev-dependencies]` 에서 자기 크레이트 테스트가 이 feature를 켤 수 있게 구성. proptest 는 default/프로덕션 빌드 그래프에 유입되지 않음(MNT-02/PBT-07).
- [x] `src/lib.rs` 뼈대 생성: 크레이트-레벨 lint gate `#![deny(missing_docs)]` (MNT-03/Q13=A), 모듈 선언(`core_types`, `config`, `#[cfg(feature = "proptest-support")] proptest_support`), 공개 재-export 자리(STEP 18에서 채움).
- [x] **순수 모듈 clippy lint-gate 배치 계획 확정**: 순수 모듈(`core_types/codec.rs`, `core_types/path.rs`, `core_types/error.rs` 및 기타 순수 CoreTypes 표면)에 `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]` 적용. **상태 보유 모듈(`config/store.rs` 등 `ArcSwap` 포인터 로드 사용)에는 적용 안 함**(순수 표면 아님). 이는 REL-02 panic-free-total(Q8=A)을 컴파일타임으로 승격.
- [x] 모든 doc 주석 한국어 + ASCII 화살표 + 백틱 타입 규약을 크레이트 전반에 적용한다는 규약 명시.

---

## STEP 2..16 — Business-Logic Generation (leaf-first, 논리 컴포넌트별 분해)

**생성 순서 근거(REL-EDGE-order)**: `ConfigProvider.* -> CoreTypes.*` 단방향(역방향 엣지 없음) => 비순환, U0 = DAG 루트. 순수 CoreTypes 리프(내부 의존 0)를 먼저 생성한 뒤 Codec/SinkContracts, 그다음 ConfigProvider 체인.

각 STEP은 realize 하는 rule(R-*)과 NFR id를 인용한다.

### STEP 2 — PathNormalizer (순수 리프) [`core_types/path.rs`]
- [x] `struct RelativePath(String)` + 정규화 생성자 `RelativePath::normalize(...) -> Result<RelativePath, _>` 생성. 정규화는 **생성 시 1회**, 멱등.
- [x] 정규화 규칙: 백슬래시 `\` -> `/`; 절대경로(선행 `/`, 드라이브 접두사 `C:`) REJECT; `..` escape REJECT(생성 실패); `.` 세그먼트/`//`/선행·후행 슬래시 제거; **바이트-보존 대소문자**(케이스 폴딩 없음); **Unicode NFC/NFD 정규화 없음**.
- [x] 바이트-사전식(UTF-8) `Ord`/`Eq`(로케일 독립) — Manifest 1차 정렬 키. serde (De)Serialize (CBOR round-trip 대상).
- [x] **실현**: R-DISCOVER-01 무관; realizes U0-NFR-REL-02(panic-free, `Result` 반환) + PROP-DE-03 멱등성. 순수 모듈 clippy lint-gate 대상.

### STEP 3 — 원시 값 타입 [`core_types/primitives.rs`]
- [x] `struct Sha256Digest([u8;32])` — 표준 SHA-256(=`sha256sum`, FQ-1=A; 실제 계산은 U1 소유). 바이트 `Eq`; 소문자 hex 표시.
- [x] `struct ManifestDigest([u8;32])` — 로컬 no-op 판별자(정규-정렬된 `(relative_path, raw_sha256, size)` 3-튜플 시퀀스 위; 계산은 U1). **두 다이제스트는 컴파일타임 비호환 newtype로 분리.**
- [x] `struct Timestamp(...)` — UTC 전용, 결정적 무손실 직렬화(epoch nanos 또는 RFC3339), 자연 시간순 `Ord`(`--since` 사용).
- [x] `struct ByteCount(u64)` — 비음수; 모든 바이트-카운트 필드 통일(`ManifestEntry.size`, `TransferResult` bytes/resume_offset, `StatusSnapshot.resume` 쌍). 무손실 round-trip(NFR-13).
- [x] serde (De)Serialize 전부. **실현**: U0-NFR-REL-01 round-trip 대상, US-E7-06.

### STEP 4 — ErrorTaxonomy (순수 리프) [`core_types/error.rs`]
- [x] `enum ErrorClass { Retryable, AuthAborted, Backpressure, Fatal }` (retry gate, U4 소비).
- [x] `ErrorClass::is_retryable(&self) -> bool` — **total 함수**(모든 변형 소진, panic 경로 없음): `Retryable` yes, `Backpressure` yes, `AuthAborted` no, `Fatal` no (R-CLASS-02/03, PROP-BR-04).
- [x] `enum TransportErrorClass { AuthFailed, ServerError, Backpressure, Timeout, Network }` (component-methods의 `ErrorClass`에서 이름 충돌 해소 위해 RENAMED).
- [x] `struct TransportError { class: TransportErrorClass, http_status: Option<u16>, detail: String }` — detail 무손실(unicode/newline), `http_status` 는 Network/Timeout 시 부재.
- [x] `struct ClassifiedError { class: ErrorClass, code: Option<String>, detail: String }` (detail/code 무손실).
- [x] `enum TransferResult { Success { bytes: u64 }, Partial { bytes: u64, resume_offset: u64 }, Failed { error: ClassifiedError } }`; 개념 불변식 `Partial.resume_offset <= bytes`.
- [x] R-CLASS-01 매핑(문서/구현): `AuthFailed -> AuthAborted`(retry no), `ServerError -> Retryable`(yes), `Backpressure -> Backpressure`(yes), `Timeout -> Retryable`(yes), `Network -> Retryable`(yes) — 각 `TransportErrorClass` 는 정확히 하나의 `ErrorClass` 로 매핑(gap/dup 없음, R-CLASS-03).
- [x] `CodecError` 정의 — encode/decode 실패, `ErrorClass::Fatal` 매핑(`is_retryable()==false`); **thiserror** derive.
- [x] 분류 값 타입(`ErrorClass`/`ClassifiedError`/`TransportError`/`TransportErrorClass`/`TransferResult`)은 **순수 serde** 유지(round-trip 대상). `CodecError`/`ConfigError`(STEP 12) 는 thiserror(operational). (MNT-01, anyhow 미채택.)
- [x] **실현**: R-CLASS-01/02/03, U0-NFR-REL-01(round-trip), U0-NFR-REL-02(is_retryable total), U0-NFR-MNT-01. 순수 모듈 clippy lint-gate 대상.

### STEP 5 — TokenSecret (순수 리프) [`core_types/token.rs`]
- [x] redacting newtype(자체 구현, ~15-20줄, 외부 의존 0; secrecy 미채택): `Debug`/`Display` -> `"***"`, 실제 값은 `.expose() -> &str` 로만.
- [x] `Serialize`/`Deserialize` 는 **값-보존**(CBOR round-trip 전용; PROP-BR-02). Config `token` 을 감쌈.
- [x] **실현**: U0-NFR-SEC-02(Debug/Display 한정 redaction + Q7=A no-Serialize-to-log 계약), R-TOKEN-01/02. Security-Baseline OFF 하 잔여 위생(RISK-01 수용).

### STEP 6 — SyncStateModel (순수 리프) [`core_types/sync_state.rs`]
- [x] `enum SyncState { Idle, Dirty, Uploading, Committed }` (Q8=A) + **별도 dirty boolean 플래그**(단일 신호, 큐 아님). `Failed`/`Paused` 변형 없음.
- [x] 전이 표 T1..T8 문서화(모델만; 합법성 검사·영속은 U4): T1 `Idle`+change->`Dirty`; T2 `Dirty`+cycle-start->`Uploading`(진입 시 dirty CLEAR); T3 `Uploading`+success->`Committed`; T4 `Committed`+persist-done & dirty-UNSET->`Idle`; T5 `Uploading`+new-change->`Uploading`(dirty SET); T6 `Committed`+new-change->`Committed`(dirty SET); T7 `Committed`+persist-done & dirty-SET->`Dirty`(consume, 재진입); T8 `Uploading`+upload-FAILURE->`Dirty`(Failed 상태 없음, U4 retry/backoff 대기).
- [x] serde (De)Serialize (round-trip 대상). **실현**: U0-NFR-REL-01 round-trip(PROP-DE-01); PROP-DE-04/PROP-BL-05 stateful 실행은 U4로 위임(모델·제너레이터만 U0).

### STEP 7 — StatusVocab (순수 리프) [`core_types/status.rs`]
- [x] `enum OperationalState { Idle, Syncing, Offline, Paused }` (axis1 단일).
- [x] `enum ActiveCondition { AuthFailed, ConsentBlocked, OverLimit, VaultUnavailable, UpdateRolledBack }` (axis2 공존 집합).
- [x] `enum LivenessSignal { IdleReached, CredentialReadable }`.
- [x] `struct StatusSnapshot { operational: OperationalState, conditions: Set<ActiveCondition>, last_success: Option<Timestamp>, dirty: bool, resume: Option<(ByteCount, ByteCount)>, consent: ConsentState, offline: bool }` (`ConsentState` 는 U5 정의 참조 타입).
- [x] serde (De)Serialize (round-trip 대상). **실현**: U0-NFR-REL-01 round-trip(PROP-DE-01). condition->state 파생은 U6 소유(독립 PBT 없음).

### STEP 8 — Manifest 값 모델 [`core_types/manifest.rs`]
- [x] `struct ManifestEntry { relative_path: RelativePath, raw_sha256: Sha256Digest, size: ByteCount }` — **mtime 필드 없음**.
- [x] `struct Manifest { entries: Vec<ManifestEntry>, manifest_digest: ManifestDigest }`. entries 는 `relative_path` asc(byte-lex) 정규 정렬, tie-break `(raw_sha256, size)`; 경로 유일; 0-entry 유효; 불변식 `manifest_digest == digest(canonical(entries))`(digest 계산은 U1; U0는 타입 + 결정성 계약만).
- [x] `struct ChangeSet { added: Vec<ManifestEntry>, modified: Vec<ManifestEntry>, deleted: Vec<RelativePath> }`. `ChangeSet::is_empty(&self) -> bool` (세 리스트 모두 비면 true). 각 리스트 `relative_path` 정렬. diff 계산은 U1.
- [x] serde (De)Serialize 전부. **실현**: U0-NFR-REL-01 round-trip(PROP-DE-01/PROP-BL-01), R-NOOP-01/02 근거 타입, NFR-12 정합성 계약. `is_empty` 는 순수 헬퍼.

### STEP 9 — Codec [`core_types/codec.rs`]  (의존: STEP 4 + STEP 2/3/6/7/8 값 모델)
- [x] `fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, CodecError>` (ciborium/CBOR, 버퍼드 `Vec<u8>`, 파일 IO 없음).
- [x] `fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, CodecError>` — 잘림/손상/적대적 CBOR 바이트 입력에도 **panic 금지**, 실패는 오직 `CodecError`(Fatal). (corruption 복구는 U4.)
- [x] **실현**: R-CODEC-01(round-trip 불변식), U0-NFR-REL-01, U0-NFR-REL-02(panic-free-total), U0-NFR-PERF-01(선형·유계, 수치 게이트 N/A — 문서화). US-E7-06 primary. 순수 모듈 clippy lint-gate 대상.

### STEP 10 — SinkContracts [`core_types/sink.rs`]  (의존: STEP 4 ErrorTaxonomy + STEP 7 StatusVocab)
- [x] push-only, 하류 주입 트레이트 **정의만**(구현 U6, 주입 U8; U0는 back-reference 없음):
  - `trait Logger { fn log(&self, record: LogRecord); fn event(&self, level, event, cycle_id, fields); }`
  - `trait StatusSink { set_operational; raise_condition; clear_condition; record_sync_success; set_dirty; set_resume_progress; set_liveness; }` (in-memory, infallible)
  - `trait HistorySink { fn append(&self, record: UploadHistoryRecord); }`
  - `trait CriticalEventSink { report_auth_failure; report_cycle_result; report_preflight_exceeded; report_update_rollback; }`
  - `trait ConfigReloadObserver { fn on_config_reload(&self, ...); }`
- [x] `LogRecord`/`UploadHistoryRecord`/`ConsentGrant` 스키마는 U5/U6로 이연 — 참조 자리표시만.
- [x] 관측 push 는 best-effort/infallible/non-fatal. **실현**: U0-NFR-SEC-02 CONSUMER 계약(`Logger` 는 config 유래 필드를 redacted `Debug`/`Display` 또는 명시적 per-field projection 으로만 방출), R-OBSERVER-02. 독립 PBT 없음.

### STEP 11 — WatcherConfig 모델 [`config/model.rs`]
- [x] `struct WatcherConfig` U0-확정 필드: `vault_path: String`, `server_endpoint: String`, `token: Option<TokenSecret>`, `secure_store_enabled: bool`(default false), `log_level`(enum, default info), `notify_consecutive_failures`(default 3). `SafetyLimits` 는 필드 아님(R-LIMIT-01, 컴파일타임 상수).
- [x] 타 단위 섹션(debounce_ms, reconciliation_interval_s, exclude_patterns, chunk_threshold_bytes, backoff, request_timeout_s, log_rotation, service, tray_enabled, confirm_empty)은 federated known-key 집합의 일부이나 스키마는 해당 단위로 이연 — U0에서 필드로 만들지 않음.
- [x] `struct ConfigSnapshot`(= `Arc<WatcherConfig>` 스냅샷 래퍼) 정의. serde Deserialize(typed pass).
- [x] **R-LIMIT-01 상수**: 컴파일타임 상수(vault 총 <= 20 GiB, per-file <= 2 GiB, file count <= 100,000) 값 정의(경계/단조성 검사는 U1). 이 값들에 대응하는 config 키는 unknown 으로 REJECT.
- [x] **U0 core-key const 노출**(DEC-FEDERATED-KEYS, Q6=A): U0가 소유하는 6개 core config 키를 공개 컴파일타임 상수 `pub const FOUNDATION_CONFIG_KEYS: &[&str] = &["vault_path","server_endpoint","token","secure_store_enabled","log_level","notify_consecutive_failures"]` 로 노출한다. `watcher-bin` 이 이 상수를 federated union 에 집계 -> `ConfigProvider::new(known_keys)` 로 주입(STEP 14/16 이 union 을 CONSUME). 이 상수가 union 에서 누락되면 R-CFG-STRICT-01 이 U0 자신의 키를 unknown 으로 거부하므로 필수. (U0는 하위 단위 키를 import 하지 않음 — 자기 키만 노출 -> DAG 루트 비순환 유지.)
- [x] **실현**: US-E7-06 round-trip(PROP-BR-02), R-LIMIT-01 상수 소유, DEC-FEDERATED-KEYS(U0 core-key 기여).

### STEP 12 — UrlValidator [`config/url_validator.rs`]  (의존: STEP 4 ErrorTaxonomy) — ConfigProvider 체인 첫 컴포넌트
- [x] `url::Url::parse` 로 `server_endpoint` 파싱 후 `scheme() == "https"` assert(`starts_with` 아님; http/scheme-less/malformed authority+port REJECT).
- [x] 위반은 `ConfigError` field 위반으로 표면화. 파싱된 `url::Url` 은 U5 AuthTransport base-URL join 에 재사용(하류 재파싱 없음).
- [x] **실현**: U0-NFR-SEC-01(TLS/https 유일 잔여 통제, NFR-06), business-rules §4.1 `server_endpoint` 규칙. 독립 PBT 없음(PROP-BR-05에 흡수).

### STEP 13 — ConfigValidator [`config/validator.rs`]  (의존: STEP 12 UrlValidator, STEP 5 TokenSecret, STEP 4 ErrorTaxonomy)
- [x] `ConfigError` 정의 — **thiserror** derive, `Vec` of issues 를 실어 나름.
- [x] **2-pass 검증(Q5=C 확정)**: (1) 1st-pass `serde_json::Value` 에서 주입된 federated known-key 집합 밖의 **모든** unknown 키를 전수 수집(중단 없음, R-CFG-STRICT-01), 이름 + JSON pointer 위치; (2) unknown 키가 clean 판정된 **후에만** typed deserialize 실행, **첫** 구조화 per-field 위반만 보고(field 이름 + expected format/range + JSON pointer). **bespoke full-field 누적 pass 없음.**
- [x] `ConfigError` 이슈 `Vec` = (다수) unknown-key 이슈 + 단일 first field 위반.
- [x] per-field 규칙(business-rules §4.1): `vault_path` REQUIRED·non-empty·**절대경로**(존재 검사 안 함, U2 런타임 관심사; 상대/빈값 REJECT); `server_endpoint` REQUIRED·유효 URL·**HTTPS-only**(UrlValidator 위임); `token` OPTIONAL·present 시 non-empty·형식 검사 없음; `secure_store_enabled` OPTIONAL bool(default false); `log_level` OPTIONAL in {trace,debug,info,warn,error}(default info); `notify_consecutive_failures` OPTIONAL 양의 정수 >= 1(default 3; 0/음수/비정수 실패).
- [x] `SafetyLimits` 키는 known-key 집합 밖 -> unknown 으로 REJECT(R-LIMIT-01).
- [x] **실현**: R-CFG-STRICT-01, U0-NFR-USE-01(구조화 메시지, did-you-mean 이연), U0-NFR-MNT-01, PROP-BR-05.

### STEP 14 — ConfigLoader [`config/loader.rs`]  (의존: STEP 13 ConfigValidator, STEP 4 ErrorTaxonomy)
- [x] `load(path) -> Result<ConfigSnapshot, ConfigError>` — 2-pass orchestration 진입점; 주입된 known-key 집합을 검증 경로에 전달.
- [x] **R-DISCOVER-01** config 경로 우선순위(로드타임 관심사): `--config` 플래그 > `OKC_WATCHER_CONFIG` env > 플랫폼 기본 경로.
- [x] 초기 로드 실패 = abort(R-RELOAD-04, non-zero exit, last-good 없음); 런타임 로드 실패 = keep-last-good 경로(R-RELOAD-03).
- [x] **federated known-key 소비 계약**(Q6=A): `ConfigProvider::new(known_keys: &[&str])` 형태로 주입된 union 을 CONSUME 만; 하위 단위 import 없음(DAG 루트 비순환).
- [x] **실현**: R-DISCOVER-01, R-RELOAD-03/04, U0-NFR-USE-01, DEC-serde-json-2pass, DEC-Q6-federated-keys.

### STEP 15 — ObserverRegistry [`config/observer.rs`]  (내부 의존 없음; ConfigStore 전에 독립 생성 가능)
- [x] `subscribe(observer: &dyn ConfigReloadObserver)` — U8 assembly 시, 데몬 스레드 기동 **전**, 1회 등록.
- [x] 내부 저장 = **불변 순서** `Vec<Arc<dyn ConfigReloadObserver>>` (기동 시 고정, 런타임 변경/해제 없음 -> 결정적 순서 + registry 락 없음).
- [x] 성공 swap 후 순서대로 `on_config_reload()` fan-out — per-observer `catch_unwind` 격리(패닉 observer 는 나머지 K-1 진행에 영향 없음, `current()` 손상·롤백 없음).
- [x] fan-out 대상 매핑(R-OBSERVER-02): `token`/`secure_store_enabled` -> U5 `CredentialProvider`/`ConsentGate`, `log_level` -> U6 `StructuredLogger`.
- [x] **실현**: R-OBSERVER-01/02/03, R-RELOAD-05, U0-NFR-REL-04, DEC-observer-vec-catchunwind.

### STEP 16 — ConfigStore + ConfigProvider 애그리게이트 표면 [`config/store.rs` + `config/mod.rs`]  (의존: STEP 14 ConfigLoader, STEP 15 ObserverRegistry, STEP 5 TokenSecret)
- [x] `current() -> ConfigSnapshot` — **lock-free** `arc_swap::ArcSwap<Arc<WatcherConfig>>` 로드. 읽기는 reload_mutex 취득 안 함.
- [x] `reload() -> Result<(), ConfigError>` — 쓰기 경로는 내부 `reload_mutex` 로 시퀀스 `load -> validate -> swap -> notify` 전체를 직렬화.
- [x] R-RELOAD-01 validate-before-swap; R-RELOAD-02 all-or-nothing(부분 적용 없음); R-RELOAD-05 성공 swap **후에만** observer fan-out; R-TRIGGER-01 reload 는 CLI `reload` 로만(SIGHUP/파일-watch 자동 reload 없음).
- [x] **keep-last-good**(R-RELOAD-03, U0-NFR-REL-05): 검증 실패 시 swap 미도달 -> 이전 스냅샷 유지, `Err(ConfigError)` 만 표면화. 초기 load 실패 -> non-zero exit(observer fan-out 없음, R-RELOAD-04).
- [x] 불변식: 동시 reader 는 항상 완료된 old 또는 완료된 new 스냅샷 중 정확히 하나만 관측(중간 상태 없음). 동일 유효 config 2회 적용 == 1회 관측(멱등, PROP-BR-03).
- [x] `config/mod.rs` 에 애그리게이트 공개 표면 노출: `load(path)`, `current() -> ConfigSnapshot`, `reload() -> Result<(), ConfigError>`, `subscribe(observer: &dyn ConfigReloadObserver)`.
- [x] **실현**: R-RELOAD-01/02/03/04/05, R-TRIGGER-01, R-OBSERVER-03, U0-NFR-REL-03/REL-05, DEC-reload-mutex, DEC-arcswap-reads, DEC-keep-last-good.

---

## STEP 17 — Property-Based Testing (PBT, proptest-support) [`proptest_support/` + `tests/`]

**확장 PBT ON/Full.** ProptestGenerators 는 feature `proptest-support` 게이트. 도메인 제약 준수 제너레이터 + PROP-* 속성 테스트 생성.

- [x] **제너레이터 모듈**(PBT-07): valid-normalized `RelativePath`; `Manifest`/correlated pairs/permutations(empty/single/many); detail 문자열(unicode/newline/empty 경계); `ByteCount`/offset(0/경계/large); valid+invalid `WatcherConfig`(모든 `log_level` 값, `notify_consecutive_failures >= 1` 경계, unknown/typo 키 주입, bad scheme, 상대 `vault_path`, empty required); 전-variant `ErrorClass`/`TransportErrorClass` 열거.
- [x] **PROP-DE-01 / PROP-BR-01 / PROP-BL-01 (codec round-trip, US-E7-06 매핑)**: `decode(encode(v)) == v` — 모든 CBOR 타입(`Manifest`+`ManifestEntry`/`RelativePath`/`Sha256Digest`/`ManifestDigest`, `Timestamp`, `ByteCount`, `ChangeSet`, `SyncState`, `ErrorClass`, `TransportError`, `ClassifiedError`, `TransferResult`) incl unicode/newline/empty detail, `None`, empty/single/many collection, 경계 수(0/max).
- [x] **PROP-DE-03 (RelativePath 멱등)**: `normalize(normalize(p)) == normalize(p)`; 결과 항상 POSIX 구분자/상대/no `..`/no `.`. 혼합 구분자·중복 슬래시·`.` 세그먼트·유니코드 + reject-case(`..`/절대 -> 정규화 실패).
- [x] **no-panic 불변식**(U0-NFR-REL-02): `decode<T>` over arbitrary bytes(valid CBOR/truncated/bitflip/random) 무패닉; `RelativePath::normalize` 무패닉; `ErrorClass::is_retryable()` total(PROP-BR-04, 유한 domain 소진).
- [x] **PROP-BR-05 (strict unknown-key reject)**: 주입된 모든 unknown 키 + typo 변형을 전수 나열하며 REJECT.
- [x] **PROP-BR-02 (config parse round-trip)**: `parse(serialize(cfg)) == cfg` (TokenSecret 값-보존 포함).
- [x] **PROP-BR-03 / PROP-BL-04 (reload 멱등 + keep-last-good)**: 동일 유효 config 2회 적용 == 1회(`current()` + observer 상태 동일); invalid reload 는 이전 스냅샷 보존 + `Err(ConfigError)` 만, 부분 적용 없음.
- [x] **위임 명시(생성/실행 하류)**: PROP-DE-02/PROP-BL-02(digest shuffle-invariance) + PROP-BL-03(diff apply oracle) -> **U1**; PROP-DE-04/PROP-BR-06/PROP-BL-05(SyncState stateful) -> **U4**. U0는 제너레이터/모델 계약만 제공(proptest-support 아래 위치 가능), stateful 실행 property 없음.
- [x] PBT-08(case count/shrink tuning/fixed-seed vs seed-logging/CI 통합)은 Build-and-Test로 이연.

---

## STEP 18 — Non-Property Unit Tests (example anchors) [`tests/` 또는 모듈 내 `#[cfg(test)]`]

MNT-03(Q13=A): 비즈니스-크리티컬 경로는 property + example 테스트 **둘 다** 보유(코어 경로 PBT-only 금지).

- [x] **config 2-pass Q5=C 동작**: unknown 키 다수 주입 시 **모든** unknown 키 나열 + typed deserialize 의 **첫** field 오류가 함께 담김을 검증.
- [x] **keep-last-good**: 유효 config 로드 후 invalid reload -> `current()` 이전 스냅샷 유지 + `Err(ConfigError)` 검증.
- [x] **observer catch_unwind 격리**: 한 observer 가 패닉해도 나머지 K-1 이 통지되고 `current()` 무손상 검증.
- [x] **https-only 거부**: `http://`/scheme-less/malformed 를 `ConfigError` 로 거부, `https://` 통과 검증.
- [x] **token redaction**: `Debug`/`Display` 에 실제 토큰 미노출(`***`) + `Serialize` 는 값-보존(round-trip) 검증.
- [x] **is_retryable 매핑**: R-CLASS-01/02 매핑 표를 example 로 고정.
- [x] **RelativePath reject-case**: 절대경로/`..` escape 생성 실패 example.
- [x] **Manifest 정규 정렬/`ChangeSet::is_empty`** example.

---

## STEP 19 — Public API Surface [`src/lib.rs`]

- [x] `lib.rs` 에서 공개 재-export: `CoreTypes` 표면(값 타입 `RelativePath`/`Sha256Digest`/`ManifestDigest`/`Timestamp`/`ByteCount`/`ManifestEntry`/`Manifest`/`ChangeSet`/`SyncState`/`ErrorClass`/`TransportErrorClass`/`TransportError`/`ClassifiedError`/`TransferResult`/`OperationalState`/`ActiveCondition`/`LivenessSignal`/`StatusSnapshot`/`TokenSecret`, 함수 `encode`/`decode`, 헬퍼 `ErrorClass::is_retryable`/`ChangeSet::is_empty`/`RelativePath::normalize`, 오류 `CodecError`/`ConfigError`, 계약 트레이트 `Logger`/`StatusSink`/`HistorySink`/`CriticalEventSink`/`ConfigReloadObserver`).
- [x] `ConfigProvider` 애그리게이트 공개 메서드 표면 재-export: `load(path)`, `current() -> ConfigSnapshot`, `reload() -> Result<(), ConfigError>`, `subscribe(observer: &dyn ConfigReloadObserver)`, + `WatcherConfig`/`ConfigSnapshot`.
- [x] `#![deny(missing_docs)]` 하 모든 공개 아이템 한국어 doc 주석 완비(미문서화 시 컴파일 실패).
- [x] read-judgment intent 계약 시그니처(return-shape only, 임계값 U6): `health_check()`(운영 건강, CLI exit-code, US-E5-02) vs `update_probe()`(순수 liveness: startup + IdleReached + CredentialReadable; U7a AutoUpdater 만 소비) — 시그니처 노출.

---

## STEP 20 — N/A 레이어 명시 (근거 포함; N/A 는 gap 아님)

- [x] **Repository/Persistence 레이어 = N/A** — 영속(파일 IO, atomic temp+rename, CBOR 저장/복구)은 **U4** 소유. U0는 무IO 순수 값 + config 로드(읽기 전용 JSON)만; codec 은 바이트 변환만 하고 파일에 쓰지 않음.
- [x] **Frontend = N/A** — U0는 lib 크레이트. UI/tray 는 U6(TrayIndicator, optional). data-testid 등 automation-friendly 규칙 무관.
- [x] **Database migrations = N/A** — DB 없음(로컬 파일 상태만, 스키마 진화 내성은 CBOR 선택으로 흡수).
- [x] **Deployment artifacts(크레이트 매니페스트 초과분) = N/A** — 서비스 설치/자동 업데이트/패키징은 **U7** 소유. U0는 workspace/crate 매니페스트만.
- [x] **성능 수치 게이트 = N/A** — U0-NFR-PERF-01 은 정성 계약(선형·유계, 100k entries/free-form detail); throughput/latency/peak-memory 수치 게이트 없음(명시적 N/A). 스트리밍-hash 메모리 경계(NFR-02)는 U1.
- [x] **Enforced Security Controls = N/A** — Security-Baseline OFF(RISK-01/02 수용); 유일 잔여 통제는 TLS/https-only(U0-NFR-SEC-01) + 토큰 로그 위생(U0-NFR-SEC-02).

---

## STEP 21 — Code SUMMARY 문서화 (markdown; 애플리케이션 코드는 워크스페이스 루트 유지)

- [x] `aidlc-docs/construction/u0-foundation/code/` 아래에 마크다운 요약 생성(코드 아님):
  - `code-summary.md` — 생성 파일 목록(경로) + 각 모듈 책임 + 공개 표면 요약.
  - `coretypes-summary.md` — CoreTypes 값 타입/함수/계약 트레이트 요약 + realize 한 R-*/NFR id.
  - `configprovider-summary.md` — ConfigProvider load/validate/snapshot/observer fan-out + reload 동시성 모델 요약.
  - `test-summary.md` — PBT(PROP-* 매핑) + example 테스트 커버리지 요약.
- [x] 모든 요약 한국어 + ASCII 화살표 + 백틱 타입; content-validation grep(유니코드 화살표/박스 글리프 0건) 통과.

---

## STEP 22 — Story Traceability & 매핑 표

### 22.1 Story Traceability
- [x] **US-E7-06**(직렬화 무손실 round-trip, NFR-13) -> **codec round-trip proptest**(PROP-DE-01/PROP-BR-01/PROP-BL-01, STEP 17) + example 테스트(STEP 18). 이것이 이 스토리의 수용 증거. 그 외 U0 커버리지는 cross-cutting foundation.

### 22.2 매핑 표 (논리 컴포넌트 -> 파일/모듈 -> 규칙 -> NFR id -> PBT 속성)

| 논리 컴포넌트 | 파일/모듈 | 규칙(R-*) | NFR id | PBT 속성 |
|---|---|---|---|---|
| PathNormalizer | `core_types/path.rs` | (정규화 규칙) | REL-02 | PROP-DE-03 |
| (원시 값 타입) | `core_types/primitives.rs` | R-CODEC-01 | REL-01 | PROP-DE-01 |
| ErrorTaxonomy | `core_types/error.rs` | R-CLASS-01/02/03 | REL-01, REL-02, MNT-01 | PROP-BR-04 |
| TokenSecret | `core_types/token.rs` | R-TOKEN-01/02 | SEC-02 | PROP-BR-02 |
| SyncStateModel | `core_types/sync_state.rs` | T1..T8 | REL-01 | PROP-DE-01 (DE-04/BL-05 -> U4) |
| StatusVocab | `core_types/status.rs` | (Q9 2축) | REL-01 | PROP-DE-01 (독립 없음) |
| (Manifest 값 모델) | `core_types/manifest.rs` | R-NOOP-01/02 | REL-01 | PROP-DE-01/BL-01 (DE-02/BL-02/BL-03 -> U1) |
| Codec | `core_types/codec.rs` | R-CODEC-01 | REL-01, REL-02, PERF-01 | PROP-DE-01/BR-01/BL-01 |
| SinkContracts | `core_types/sink.rs` | R-OBSERVER-02 | SEC-02 | 없음 |
| ConfigLoader | `config/loader.rs` | R-DISCOVER-01, R-RELOAD-03/04 | USE-01 | (PROP-BR-05 흐름) |
| ConfigValidator | `config/validator.rs` | R-CFG-STRICT-01, R-LIMIT-01 | USE-01, MNT-01 | PROP-BR-05 |
| UrlValidator | `config/url_validator.rs` | (§4.1 https) | SEC-01 | 없음(BR-05 흡수) |
| ConfigStore | `config/store.rs` | R-RELOAD-01/02/05, R-TRIGGER-01, R-OBSERVER-03 | REL-03, REL-05 | PROP-BR-02/BR-03/BL-04 |
| ObserverRegistry | `config/observer.rs` | R-OBSERVER-01/02/03, R-RELOAD-05 | REL-04 | 없음 |
| ProptestGenerators | `proptest_support/` | (feature `proptest-support`) | MNT-02 | (전 property 지원) |

- [x] 위 표를 `code/code-summary.md` 에 복제 반영.

---

## STEP 23 — Extension-Compliance 계획 행

- [x] **Property-Based-Testing (ON, Full)**: PROP-DE-01/BR-01/BL-01(codec round-trip, US-E7-06), PROP-DE-03(normalize 멱등), PROP-BR-02(config round-trip), PROP-BR-03/BL-04(reload 멱등 + keep-last-good), PROP-BR-04(is_retryable total), PROP-BR-05(strict unknown-key) 모두 STEP 17 에서 생성. 제너레이터는 `proptest-support` feature 게이트(MNT-02/PBT-07). 코어 경로 property + example 동반(MNT-03). PBT-09(proptest 채택) 충족. **COMPLIANT.**
- [x] **Resiliency-Baseline (ON)**: U0 = Critical DAG 루트(RESILIENCY-01, 단일 배포 바이너리 보존). keep-last-good(REL-05), reload 원자성(REL-03), observer catch_unwind 격리(REL-04), panic-free-total 순수 표면(REL-02), 초기 load 실패 abort(REL-04). **COMPLIANT.**
- [x] **Security-Baseline (OFF / N-A)**: RISK-01/02 수용. 잔여 통제만 = TLS/https-only(SEC-01) + 토큰 로그 위생 redacting newtype(SEC-02). 강제 보안 통제는 N/A(비-blocking). **N/A(근거 명시).**

---

## 4. 총괄 요약 (code-generation.md Step 5)

- **접근**: leaf-first(순수 CoreTypes 리프 -> Codec/SinkContracts -> ConfigProvider 체인), 비순환 DAG 루트, 무IO 순수 lib.
- **단계 수**: 총 23개 STEP (구조 셋업 1 + 컴포넌트 생성 15(STEP 2-16) + PBT 1 + unit 1 + public API 1 + N/A 1 + 요약 1 + traceability 1 + 확장 컴플라이언스 1).
- **스토리 커버리지**: US-E7-06(무손실 round-trip) 유일 primary — codec round-trip PBT + example 로 증거화. 그 외 cross-cutting foundation.
- **크레이트/결정 인코딩(재-질문 없음)**: ciborium/serde+serde_json(2-pass)/url(https-only)/thiserror/arc_swap/Edition 2024+pinned MSRV/redacting TokenSecret/proptest(+`proptest-support` feature). clippy 순수-모듈 lint-gate + `#![deny(missing_docs)]`. reload_mutex 직렬화 + lock-free 읽기 + keep-last-good + 순서 observer catch_unwind + Q5=C config-error + federated known-key 주입.
- **미채택/이연**: secrecy·fuzzy-match·minicbor·serde_cbor·anyhow·streaming codec·`deny_unknown_fields` 미채택; did-you-mean(USE-01)·PBT-08 세부·MSRV/coverage CI 이연(Build-and-Test).

> **다음(Part-2)**: 이 계획서를 그대로 실행하며 각 STEP 완료 시 즉시 `[x]` 갱신. 애플리케이션 코드는 워크스페이스 루트, 마크다운 요약은 `aidlc-docs/construction/u0-foundation/code/`.

# CoreTypes 애그리게이트 요약

**모듈**: `crates/foundation/src/core_types/` · **성격**: 무상태·무IO 순수 값 타입/함수/계약 트레이트, DAG 루트 값 계층.
**공통 lint-gate**: 각 순수 서브모듈은 module-level `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]` 를 적용해 panic-free-total(U0-NFR-REL-02)을 컴파일타임 강제한다.

---

## 1. 값 타입

### `path.rs` — PathNormalizer
- `RelativePath(String)` — 정규화된 POSIX 상대경로. `#[serde(transparent)]` Serialize, `Deserialize` 는 재정규화로 불변식 재확립.
  - `RelativePath::normalize(input: &str) -> Result<RelativePath, PathError>` — `\` -> `/`, 절대/드라이브 접두 REJECT, `..` REJECT, `.`/중복 슬래시/선행·후행 슬래시 제거, 바이트-보존(케이스 폴딩·NFC/NFD 없음). 멱등.
  - `as_str(&self) -> &str`, `into_string(self) -> String`.
  - `Ord`/`Eq` 는 바이트-사전식(로케일 독립) — Manifest 1차 정렬 키.
- `PathError { Absolute, Escape, Empty }` — thiserror.
- **realize**: U0-NFR-REL-02(panic-free), PROP-DE-03(멱등).

### `primitives.rs` — 원시 값 타입
- `Sha256Digest([u8;32])` — `from_bytes`/`as_bytes`; `Display` 소문자 hex. 표준 SHA-256(FQ-1=A, 계산은 U1).
- `ManifestDigest([u8;32])` — 로컬 no-op 판별자. `Sha256Digest` 와 컴파일타임 비호환 newtype.
- `Timestamp(i64)` — UTC epoch nanos, `from_unix_nanos`/`as_unix_nanos`, 자연 시간순 `Ord`.
- `ByteCount(u64)` — `new`/`get`; 모든 바이트-카운트 필드 일원화.
- 전부 serde (De)Serialize. **realize**: U0-NFR-REL-01 round-trip, US-E7-06.

### `error.rs` — ErrorTaxonomy
- `ErrorClass { Retryable, AuthAborted, Backpressure, Fatal }` — retry gate(U4 소비).
  - `is_retryable(&self) -> bool` — total: `Retryable`/`Backpressure` -> true, `AuthAborted`/`Fatal` -> false (R-CLASS-02/03, PROP-BR-04).
- `TransportErrorClass { AuthFailed, ServerError, Backpressure, Timeout, Network }`.
  - `to_error_class(&self) -> ErrorClass` — R-CLASS-01 매핑(gap/dup 없음): `AuthFailed -> AuthAborted`, `ServerError -> Retryable`, `Backpressure -> Backpressure`, `Timeout -> Retryable`, `Network -> Retryable`.
- `TransportError { class, http_status: Option<u16>, detail: String }` — detail 무손실.
- `ClassifiedError { class: ErrorClass, code: Option<String>, detail: String }`.
- `TransferResult { Success{bytes: ByteCount}, Partial{bytes, resume_offset}, Failed{error: ClassifiedError} }` — 개념 불변식 `resume_offset <= bytes`.
- 위 분류 값 타입은 순수 serde(round-trip 대상).
- `CodecError { Encode(String), Decode(String) }` — thiserror(운영 오류, round-trip 대상 아님). `error_class(&self) -> ErrorClass` == `Fatal`.
- **realize**: R-CLASS-01/02/03, REL-01/REL-02, MNT-01.

### `token.rs` — TokenSecret
- `TokenSecret(String)` — `#[serde(transparent)]`, `new`/`expose() -> &str`. `Debug` -> `TokenSecret(***)`, `Display` -> `***`. `PartialEq`/`Eq` 는 내부 값 비교(round-trip 검증). Serialize/Deserialize 값-보존(PROP-BR-02).
- **realize**: U0-NFR-SEC-02(redaction + Q7=A no-Serialize-to-log 계약), R-TOKEN-01/02. 외부 secrecy 미채택(자체 구현).

### `sync_state.rs` — SyncStateModel
- `SyncState { Idle, Dirty, Uploading, Committed }` — `Failed`/`Paused` 없음. dirty boolean 은 별도 신호(모델 doc T1..T8, 지속·가드는 U4). serde (De)Serialize.
- **realize**: REL-01 round-trip(PROP-DE-01); stateful(DE-04/BL-05) -> U4.

### `status.rs` — StatusVocab
- `OperationalState { Idle, Syncing, Offline, Paused }`(축1 단일).
- `ActiveCondition { AuthFailed, ConsentBlocked, OverLimit, VaultUnavailable, UpdateRolledBack }`(축2 집합).
- `LivenessSignal { IdleReached, CredentialReadable }`.
- `ConsentState { Granted, Blocked, Unknown }` — U5 소유 참조 타입의 최소 자리표시.
- `StatusSnapshot { operational, conditions: Vec<ActiveCondition>, last_success: Option<Timestamp>, dirty: bool, resume: Option<(ByteCount, ByteCount)>, consent: ConsentState, offline: bool }`.
- `Health { Healthy, Unhealthy{reasons: Vec<HealthReason>} }`, `HealthReason(String)`, `Liveness { Alive, NotReady{missing: Vec<LivenessSignal>} }` — read-judgment 반환 형상(임계값 U6).
- 전부 serde (De)Serialize. **realize**: REL-01 round-trip(PROP-DE-01).

### `manifest.rs` — Manifest 값 모델
- `ManifestEntry { relative_path: RelativePath, raw_sha256: Sha256Digest, size: ByteCount }` — mtime 없음; 파생 `Ord` = canonical 정렬 키.
- `Manifest { entries: Vec<ManifestEntry>, manifest_digest: ManifestDigest }` — canonical 정렬·경로 유일·0-entry 유효 계약(강제는 U1).
- `ChangeSet { added, modified: Vec<ManifestEntry>, deleted: Vec<RelativePath> }`.
  - `is_empty(&self) -> bool` — 세 목록 모두 비면 true(no-op 판정).
- 전부 serde (De)Serialize. **realize**: R-NOOP-01/02, REL-01 round-trip(PROP-DE-01/BL-01).

---

## 2. 순수 함수

### `codec.rs` — Codec (ciborium)
- `encode<T: Serialize>(value: &T) -> Result<Vec<u8>, CodecError>` — 버퍼드 `Vec<u8>`, 파일 IO 없음, 실패 -> `CodecError::Encode`.
- `decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, CodecError>` — 잘림/손상/적대적 입력에도 패닉 없음, 실패 -> `CodecError::Decode`.
- **realize**: R-CODEC-01(round-trip 불변식 `decode(encode(v)) == v`), REL-01, REL-02(panic-free-total), PERF-01(정성 계약). US-E7-06 primary 실행 지점.

---

## 3. 계약 트레이트 (`sink.rs` — SinkContracts, push-only, 구현 U6/주입 U8)

- `Logger { log(&self, record: LogRecord); event(&self, level: LogLevel, event: &str, cycle_id: Option<CycleId>, fields: LogFields); }`
- `StatusSink { set_operational; raise_condition; clear_condition; record_sync_success; set_dirty; set_resume_progress; set_liveness; }`(in-memory, infallible)
- `HistorySink { append(&self, record: UploadHistoryRecord); }`
- `CriticalEventSink { report_auth_failure(TransportError); report_cycle_result(CycleOutcome); report_preflight_exceeded(LimitReport); report_update_rollback(Version, Version, RollbackReason); }`
- `ConfigReloadObserver { on_config_reload(&self); }` — 페이로드 없음, 관찰자가 `current()` 로 재조회.
- `ReadJudgment { health_check(&self) -> Health; update_probe(&self) -> Liveness; }` — 반환 형상 계약만(임계값 U6). `update_probe` 는 U7a AutoUpdater 전용.
- placeholder 레코드 타입(`LogLevel`/`LogRecord`/`LogFields`/`CycleId`/`UploadHistoryRecord`/`CycleOutcome`/`LimitReport`/`Version`/`RollbackReason`) — 트레이트 시그니처 컴파일용 최소 정의, 최종 스키마는 U5/U6/U7 확정.
- **realize**: U0-NFR-SEC-02 CONSUMER 계약, R-OBSERVER-02. 독립 PBT 없음.

---

## 4. 실현한 R-*/NFR id 집계

- **R-***: R-CODEC-01, R-CLASS-01/02/03, R-TOKEN-01/02, R-NOOP-01/02, R-OBSERVER-02(계약), T1..T8(모델).
- **NFR**: U0-NFR-REL-01(무손실 round-trip), U0-NFR-REL-02(panic-free-total), U0-NFR-SEC-02(토큰 로그 위생), U0-NFR-MNT-01(thiserror vs 순수 serde 분리), U0-NFR-PERF-01(정성 선형·유계).
- **Story**: US-E7-06(직렬화 무손실 round-trip) — `encode`/`decode` + 값 타입 serde 파생.

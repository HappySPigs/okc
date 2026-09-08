#![deny(missing_docs)]
//! `foundation` 크레이트 (U0) — okc-hooks 의존성 DAG 의 루트.
//!
//! 두 개의 공개 애그리게이트를 노출한다:
//! - `CoreTypes`: 무상태·무IO 순수 값 타입/함수/계약 트레이트 (모듈 `core_types`).
//! - `ConfigProvider`: JSON 로드·검증 + 활성 스냅샷 + 관찰자 팬아웃 (모듈 `config`).
//!
//! 이 크레이트는 어떤 하류 단위(U1..U8)에도 의존하지 않으며(비순환 DAG 루트),
//! 하류 단위와 `watcher-bin` 이 이 크레이트를 링크한다.
//!
//! 규약: 모든 doc 주석은 한국어, 코드 내 화살표는 ASCII `A -> B` 만 사용,
//! Rust 타입/식별자는 doc 주석에서 백틱으로 감싼다(`Vec<u8>`, `Arc<WatcherConfig>`).

/// CoreTypes 애그리게이트: 순수 값 타입, 함수, 계약 트레이트 (무상태·무IO).
pub mod core_types;

/// ConfigProvider 애그리게이트: config 로드·검증·활성 스냅샷·관찰자 팬아웃.
pub mod config;

/// ProptestGenerators: 도메인 제약 준수 제너레이터 (test-support, 런타임 그래프 밖).
///
/// 비-default cargo feature `proptest-support` 뒤에 게이트되어 릴리스 빌드에서 제외된다.
#[cfg(feature = "proptest-support")]
pub mod proptest_support;

/// U0(`foundation`)가 소유하는 6개 core config 키 목록(DEC-FEDERATED-KEYS, Q6=A).
///
/// 단일 정의는 `config::model` 이 소유하며, 여기서는 crate-root 경로
/// `foundation::FOUNDATION_CONFIG_KEYS` 로 재-export 한다(중복 정의 회피).
pub use config::model::FOUNDATION_CONFIG_KEYS;

// ---------------------------------------------------------------------------
// STEP 19 — Public API Surface (공개 재-export)
//
// `CoreTypes` 값 타입/함수/오류/계약 트레이트 및 `ConfigProvider` 애그리게이트 표면을
// crate-root 로 재-export 한다. 재-export 된 아이템은 원본 정의의 doc 주석을 그대로 승계하므로
// `#![deny(missing_docs)]` 하에서도 별도 doc 이 필요 없다.
// ---------------------------------------------------------------------------

// CoreTypes 표면: 값 타입 + 순수 함수(`encode`/`decode`) + 오류 + 계약 트레이트 +
// read-judgment 반환 형상(`Health`/`Liveness`/`ReadJudgment`, return-shape only; 임계값 U6).
pub use core_types::{
    ActiveCondition, ByteCount, ChangeSet, ClassifiedError, CodecError, ConfigReloadObserver,
    ConsentState, CriticalEventSink, CycleId, CycleOutcome, ErrorClass, Health, HealthReason,
    HistorySink, LimitReport, Liveness, LivenessSignal, LogFields, LogLevel, LogRecord, Logger,
    Manifest, ManifestDigest, ManifestEntry, OperationalState, PathError, ReadJudgment,
    RelativePath, RollbackReason, Sha256Digest, StatusSink, StatusSnapshot, SyncState, Timestamp,
    TokenSecret, TransferResult, TransportError, TransportErrorClass, UploadHistoryRecord, Version,
    decode, encode,
};

// ConfigProvider 애그리게이트 표면: 프로바이더 + 값 모델 + 검증/로더/URL 진입점 + R-LIMIT-01 상수.
pub use config::{
    ConfigError, ConfigIssue, ConfigProvider, ConfigSnapshot, MAX_FILE_BYTES, MAX_FILE_COUNT,
    MAX_VAULT_TOTAL_BYTES, ObserverRegistry, UrlValidationError, WatcherConfig, read_and_validate,
    resolve_config_path, validate, validate_https_url,
};

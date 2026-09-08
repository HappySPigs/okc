//! `CoreTypes` 애그리게이트 — 순수 값 타입, 순수 함수, 계약 트레이트(무상태·무IO).
//!
//! U0(및 전 단위)의 DAG 루트 값 계층이다. 순수 리프 서브모듈(내부 의존 0)을 먼저 두고,
//! `codec`(-> `error` + 값 모델)과 `sink`(-> `error`/`status`)가 그 위에 얹힌다.
//! 각 서브모듈은 순수 표면으로서 module-level clippy lint-gate
//! (`deny(clippy::unwrap_used, expect_used, indexing_slicing, panic)`)를 개별 적용해
//! panic-free-total(U0-NFR-REL-02)을 컴파일타임으로 강제한다.

pub mod codec;
pub mod error;
pub mod manifest;
pub mod path;
pub mod primitives;
pub mod sink;
pub mod status;
pub mod sync_state;
pub mod token;

pub use codec::{decode, encode};
pub use error::{
    ClassifiedError, CodecError, ErrorClass, TransferResult, TransportError, TransportErrorClass,
};
pub use manifest::{ChangeSet, Manifest, ManifestEntry};
pub use path::{PathError, RelativePath};
pub use primitives::{ByteCount, ManifestDigest, Sha256Digest, Timestamp};
pub use sink::{
    ConfigReloadObserver, CriticalEventSink, CycleId, CycleOutcome, HistorySink, LimitReport,
    LogFields, LogLevel, LogRecord, Logger, ReadJudgment, RollbackReason, StatusSink,
    UploadHistoryRecord, Version,
};
pub use status::{
    ActiveCondition, ConsentState, Health, HealthReason, Liveness, LivenessSignal, OperationalState,
    StatusSnapshot,
};
pub use sync_state::SyncState;
pub use token::TokenSecret;

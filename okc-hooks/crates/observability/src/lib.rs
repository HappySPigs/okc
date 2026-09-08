#![deny(missing_docs)]
//! `observability` 크레이트 (U6) — okc-hooks 의 push-only 관측 계층.
//!
//! U0 `foundation` 이 계약으로 소유한 SinkContracts 트레이트를 구현하는 4개 실체 싱크와
//! 선택적 no-op 트레이 어댑터를 제공한다. 하위 단위(U1~U5/U8)는 이 싱크를 `Arc<dyn _>` 로
//! 주입받아 값을 **push** 하며, U6 는 값을 받아 방출/집계/판정/지속할 뿐이다(R-PUSH-01).
//!
//! - `StructuredLogger` (`logger`): `Logger` + `ConfigReloadObserver` 구현 — `serde_json`
//!   JSON-line 방출, 방출 시각 stamping, 레벨 필터, 크기 기반 rotation.
//! - `StatusService` (`status`): `StatusSink` + `ReadJudgment` 구현 — 2축 상태 집계 +
//!   `health_check`/`update_probe` 판정(단일 `Mutex` 집계 지점).
//! - `UploadHistoryStore` (`history`): `HistorySink` 구현 — 무손실 CBOR 프레임 append-only
//!   지속 + truncated-tail 관용 + `query` 필터.
//! - `CriticalErrorNotifier` (`critical`): `CriticalEventSink` 구현 — 닫힌 4조건 3중 fan-out.
//! - `TrayIndicator` (`tray`): MVP no-op 어댑터(D1).
//!
//! 관측 push 표면은 모두 best-effort·infallible·비차단이며(R-PUSH-02), 실패를 표면화하는 것은
//! 읽기/재적용 경로(`query`/`reload`)뿐이다(R-PUSH-03).
//!
//! 규약: 모든 doc 주석은 한국어, 코드 내 화살표는 ASCII `A -> B` 만, Rust 타입/식별자는 백틱.

/// 방출 시각 stamping 을 위한 now-source seam(`Clock`) + `std::time` 기반 `SystemClock`.
pub mod clock;
/// federated 주입 config 뷰(`ObservabilityConfig`/`LoggerConfig`) + `OBSERVABILITY_CONFIG_KEYS`.
pub mod config;
/// `CriticalErrorNotifier` — 닫힌 4조건 3중 fan-out + 연속실패 escalation 상태머신.
pub mod critical;
/// U6 운영 오류 타입(`LogError`/`HistoryError`, `thiserror` 파생).
pub mod error;
/// `UploadHistoryStore` — append-only 무손실 CBOR 프레임 히스토리 + `query`.
pub mod history;
/// `StructuredLogger` — JSON-line 구조화 로거 + config 리로드 재적용.
pub mod logger;
/// `StatusService` — 2축 상태 집계 + `health_check`/`update_probe` 판정.
pub mod status;
/// `TrayIndicator` — MVP no-op 트레이 어댑터(D1).
pub mod tray;

/// ProptestGenerators: U6 고유 타입 도메인 제너레이터(test-support, 런타임 그래프 밖).
///
/// 비-default cargo feature `proptest-support` 뒤에 게이트되어 릴리스 빌드에서 제외된다(PBT-07).
#[cfg(feature = "proptest-support")]
pub mod proptest_support;

// ---------------------------------------------------------------------------
// 공개 API 표면(재-export) — 재-export 아이템은 원본 doc 주석을 승계한다.
// ---------------------------------------------------------------------------
pub use clock::{Clock, SystemClock};
pub use config::{LoggerConfig, OBSERVABILITY_CONFIG_KEYS, ObservabilityConfig};
pub use critical::CriticalErrorNotifier;
pub use error::{HistoryError, LogError};
pub use history::{HistoryQuery, UploadHistoryStore};
pub use logger::{EmittedLogLine, StructuredLogger};
pub use status::StatusService;
pub use tray::{NotificationMessage, TrayConfig, TrayError, TrayHandle, TrayIndicator};

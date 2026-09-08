#![deny(missing_docs)]
//! `change-detect` 크레이트 (U2) — 변경 감지·트리거 발행·가드 판정 순수 lib.
//!
//! 세 공개 애그리게이트를 노출한다:
//! - `FilesystemWatcher`: OS 네이티브 감시(`notify`) + 볼트-전체 단일 디바운스 타이머 ->
//!   편집 버스트당 1 트리거(모듈 `watcher` + 순수 `debounce`).
//! - `ReconciliationScheduler`: 시작 1회 + 주기 백스톱 트리거를 발행-시각 앵커로 스케줄해
//!   미관측 변경 검출 지연을 `<= T_recon` 으로 상한 짓는 순수 판정자(모듈 `scheduler`).
//! - `VaultAvailabilityGuard`: 볼트 가용성 분류 + 파괴적-빈-커밋 방지 total-function 판정
//!   (모듈 `guard`).
//!
//! U2 는 **순수 판정/신호 반환자**다 — `StatusSink`/`Logger` 등 U0 싱크를 주입받지 않으며
//! 상태를 디스크에 지속하지 않는다(R-PURE-01/02). 실제 관측 push·오케스트레이션은 U8 소관이다.
//! 크레이트 의존은 `foundation`(U0, 참조만) + `notify`(어댑터 뒤 캡슐화) + `thiserror` 뿐이며
//! U1~U8 어느 단위도 역참조하지 않는다(비순환 DAG).
//!
//! 규약: 모든 doc 주석은 한국어, 코드 내 화살표는 ASCII `A -> B` 만 사용,
//! Rust 타입/식별자는 doc 주석에서 백틱으로 감싼다(`Option<Instant>`, `Vec<TriggerSignal>`).

/// 공용 값/전송 규약: `TriggerSignal`/`TriggerKind`/`ReconPhase`/`TriggerStream`.
pub mod trigger;

/// 감시 계층 오류 taxonomy: `WatchError`/`BackendCause`.
pub mod error;

/// U2 config 뷰(U0 `WatcherConfig` 의 read-only 투영) + 검증.
pub mod config_views;

/// 순수 디바운스 상태 기계 + 결정적 시뮬레이션(버스트당 1 트리거 로직).
pub mod debounce;

/// `FilesystemWatcher` 애그리게이트 + `WatchBackend` 어댑터 경계.
pub mod watcher;

/// `ReconciliationScheduler` 애그리게이트(발행-시각 앵커 백스톱).
pub mod scheduler;

/// `VaultAvailabilityGuard` 애그리게이트(가용성 분류 + 파괴적-빈-커밋 가드).
pub mod guard;

/// ProptestGenerators: U2 도메인 제너레이터 (test-support, 런타임 그래프 밖).
///
/// 비-default cargo feature `proptest-support` 뒤에 게이트되어 릴리스 빌드에서 제외된다.
#[cfg(feature = "proptest-support")]
pub mod proptest_support;

// ---------------------------------------------------------------------------
// 공개 API 표면(crate-root 재-export). 재-export 아이템은 원본 doc 을 승계한다.
// ---------------------------------------------------------------------------

pub use trigger::{ReconPhase, TriggerKind, TriggerSignal, TriggerStream};

pub use error::{BackendCause, WatchError};

pub use config_views::{ConfigViewError, ReconConfig, VaultConfig, WatchConfig};

pub use debounce::{Debouncer, simulate_debounce};

pub use watcher::{
    DegradedBackend, FilesystemWatcher, NotifyBackend, RawFsEvent, Subscription, WatchBackend,
    WatchState,
};

pub use scheduler::{CycleOutcome, ReconResult, ReconciliationScheduler};

pub use guard::{Availability, GuardVerdict, HoldReason, VaultAvailabilityGuard};

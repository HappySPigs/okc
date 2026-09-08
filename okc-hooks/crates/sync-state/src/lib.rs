#![deny(missing_docs)]
//! `sync-state` 크레이트 (U4) — okc-hooks Watcher 의 회복력/재시도 단위.
//!
//! 두 개의 상태를 공유하지 않는 공개 애그리게이트를 노출한다:
//! - `SyncStateStore`: 마지막 커밋 `Manifest` + dirty 플래그 + 재개 오프셋을 crash-atomic
//!   하게 지속/복구하는 유일한 I/O·상태 보유 컴포넌트(모듈 `store`).
//! - `RetryBackoffController`: `TransportError` 를 분류하고 지수 백오프 + full jitter +
//!   상한 + 에스컬레이션을 계산하는 순수·결정적 인메모리 상태머신(모듈 `retry`).
//!
//! 이 크레이트는 DAG 상 U0 `foundation` 에만 의존하는 소비자 말단이며(비순환), 어떤 U0
//! sink/observer 계약도 구현하지 않는다 — `RetryDecision` 을 반환만 하고 status/notify push 는
//! 상위(U8)가 소유한다.
//!
//! 규약: 모든 doc 주석은 한국어, 코드 내 화살표는 ASCII `A -> B` 만 사용,
//! Rust 타입/식별자는 doc 주석에서 백틱으로 감싼다(`BTreeMap<Sha256Digest, ByteCount>`).

/// `SyncStateStore` 애그리게이트: crash-atomic 상태 지속/복구(유일 I/O 소유자).
pub mod store;

/// `RetryBackoffController` 애그리게이트: 순수·결정적 분류/백오프/에스컬레이션 상태머신.
pub mod retry;

/// ProptestGenerators: U4 도메인 제너레이터(test-support, 런타임 그래프 밖).
///
/// 비-default cargo feature `proptest-support` 뒤에 게이트되어 릴리스 빌드에서 제외된다.
#[cfg(feature = "proptest-support")]
pub mod proptest_support;

// ---------------------------------------------------------------------------
// 공개 API 표면(crate-root 재-export). 재-export 된 아이템은 원본 doc 을 승계한다.
// ---------------------------------------------------------------------------

pub use store::{
    PersistedState, RecoveredState, ResumeOffsetMap, StateConfig, StateError, SyncStateStore,
};

pub use retry::{BackoffConfig, JitterMode, RetryBackoffController, RetryClass, RetryDecision};

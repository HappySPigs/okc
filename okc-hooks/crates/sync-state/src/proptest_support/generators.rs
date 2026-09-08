//! U4 도메인 제약 준수 `proptest` 제너레이터.
//!
//! U0 제너레이터(`foundation::proptest_support::generators`)를 재사용해 `Manifest`/
//! `Sha256Digest`/`ByteCount`/`TransportError` 를 조합하고, U4 지속/재시도 타입의 불변식을
//! 위반하지 않는 값을 생성한다(빈/다수·경계값 포괄).

use std::time::Duration;

use foundation::proptest_support::generators as fg;
use proptest::prelude::*;

use crate::retry::{BackoffConfig, JitterMode};
use crate::store::{PersistedState, ResumeOffsetMap};

/// 빈/다수·경계 오프셋을 포괄하는 `ResumeOffsetMap` 을 생성한다.
pub fn arb_resume_offsets() -> impl Strategy<Value = ResumeOffsetMap> {
    prop::collection::vec((fg::arb_sha256_digest(), fg::arb_byte_count()), 0..6)
        .prop_map(|pairs| pairs.into_iter().collect())
}

/// `last_committed`(None/빈/다수 엔트리) + `dirty` 양값 + `resume_offsets`(빈/다수·경계)를
/// 포괄하는 `PersistedState` 를 생성한다(PROP-U4-03 round-trip 대상).
pub fn arb_persisted_state() -> impl Strategy<Value = PersistedState> {
    (
        prop::option::of(fg::arb_manifest()),
        any::<bool>(),
        arb_resume_offsets(),
    )
        .prop_map(|(last_committed, dirty, resume_offsets)| PersistedState {
            last_committed,
            dirty,
            resume_offsets,
        })
}

/// 전 변형(2종)을 열거하는 `JitterMode` 를 생성한다.
pub fn arb_jitter_mode() -> impl Strategy<Value = JitterMode> {
    prop::sample::select(vec![JitterMode::Full, JitterMode::None])
}

/// 경계(작은 cap, `multiplier==1.0`, full/none jitter)를 포괄하는 `BackoffConfig` 를 생성한다.
///
/// `cap >= initial_delay` 불변식을 보정한다(`cap_ms = max(cap_ms, initial_ms)`).
pub fn arb_backoff_config() -> impl Strategy<Value = BackoffConfig> {
    (
        prop::sample::select(vec![1u64, 10, 1000]),
        prop::sample::select(vec![1.0f64, 1.5, 2.0]),
        prop::sample::select(vec![1u64, 100, 8_000, 300_000]),
        arb_jitter_mode(),
    )
        .prop_map(|(initial_ms, multiplier, cap_ms, jitter)| BackoffConfig {
            initial_delay: Duration::from_millis(initial_ms),
            multiplier,
            cap: Duration::from_millis(cap_ms.max(initial_ms)),
            jitter,
        })
}

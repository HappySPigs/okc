//! PBT 도메인 제너레이터 — U8 고유 값 타입용(비-default `proptest-support` feature 뒤 게이트).
//!
//! `foundation::proptest_support::generators`(매니페스트/상태 등)를 재사용하고, U8 이 도입한
//! `CycleTrigger`/게이트 입력 조합 제너레이터를 추가로 노출한다. 런타임 그래프 밖 test-support 다.

use std::time::Instant;

use auth_consent::{BlockReason, ConsentDecision};
use change_detect::{Availability, GuardVerdict, HoldReason, TriggerKind, TriggerSignal};
use proptest::prelude::*;

use crate::coordinator::CycleTrigger;

/// 4개 `Availability` 변형을 균등 생성한다.
pub fn arb_availability() -> impl Strategy<Value = Availability> {
    prop::sample::select(vec![
        Availability::Reachable,
        Availability::RootMissing,
        Availability::Unmounted,
        Availability::Inaccessible,
    ])
}

/// 3개 `BlockReason` 변형을 균등 생성한다.
pub fn arb_block_reason() -> impl Strategy<Value = BlockReason> {
    prop::sample::select(vec![
        BlockReason::NeedsAcknowledgment,
        BlockReason::NotGranted,
        BlockReason::Withdrawn,
    ])
}

/// `Permitted`/`Blocked(reason)` 를 포괄하는 `ConsentDecision` 을 생성한다.
pub fn arb_consent_decision() -> impl Strategy<Value = ConsentDecision> {
    prop_oneof![
        Just(ConsentDecision::Permitted),
        arb_block_reason().prop_map(ConsentDecision::Blocked),
    ]
}

/// 3개 `GuardVerdict` 변형을 포괄한다(hold 사유는 고정 진단 문구).
pub fn arb_guard_verdict() -> impl Strategy<Value = GuardVerdict> {
    prop_oneof![
        Just(GuardVerdict::Proceed),
        Just(GuardVerdict::HoldDestructiveEmpty),
        Just(GuardVerdict::HoldVaultUnavailable(HoldReason(
            "test hold".to_string()
        ))),
    ]
}

/// 진단 요약이 임의인 디바운스 `TriggerSignal` 을 생성한다(단조 시점은 생성 시각으로 고정).
fn arb_trigger_signal() -> impl Strategy<Value = TriggerSignal> {
    "[a-z ]{0,16}".prop_map(|summary| {
        TriggerSignal::new(TriggerKind::Debounced, summary, Instant::now())
    })
}

/// 세 트리거 소스를 포괄하는 `CycleTrigger` 를 생성한다.
pub fn arb_cycle_trigger() -> impl Strategy<Value = CycleTrigger> {
    prop_oneof![
        Just(CycleTrigger::SyncNow),
        arb_trigger_signal().prop_map(CycleTrigger::Filesystem),
        arb_trigger_signal().prop_map(CycleTrigger::Reconcile),
    ]
}

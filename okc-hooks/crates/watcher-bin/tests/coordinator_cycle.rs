//! `SyncCycleCoordinator::run_cycle` 분기 예제 테스트(always-compiled) — 배타적 종결 + 조건부 구동.

mod common;

use std::sync::atomic::Ordering;

use auth_consent::{BlockReason, ConsentDecision};
use change_detect::{Availability, GuardVerdict};
use foundation::ActiveCondition;

use watcher_bin::coordinator::{CoordinatorOutcome, CycleTrigger};

use common::{Scenario, build, empty_manifest, nonempty_manifest};

/// paused -> `SkippedPaused`, 드라이버 미호출.
#[test]
fn paused_skips_and_never_drives() {
    let (mut coord, handles) = build(Scenario {
        paused: true,
        availability: Availability::Reachable,
        verdict: GuardVerdict::Proceed,
        last: None,
        current: nonempty_manifest(),
        consent: ConsentDecision::Permitted,
        driver_succeeds: true,
    });
    let outcome = coord.run_cycle(CycleTrigger::SyncNow);
    assert!(matches!(outcome, CoordinatorOutcome::SkippedPaused));
    assert_eq!(handles.driver_calls.load(Ordering::SeqCst), 0);
}

/// vault 도달 불가 -> `HeldVaultUnavailable` + status 조건 raise, 드라이버 미호출.
#[test]
fn unreachable_holds_and_raises_condition() {
    let (mut coord, handles) = build(Scenario {
        paused: false,
        availability: Availability::RootMissing,
        verdict: GuardVerdict::Proceed,
        last: None,
        current: nonempty_manifest(),
        consent: ConsentDecision::Permitted,
        driver_succeeds: true,
    });
    let outcome = coord.run_cycle(CycleTrigger::SyncNow);
    assert!(matches!(outcome, CoordinatorOutcome::HeldVaultUnavailable));
    assert_eq!(handles.driver_calls.load(Ordering::SeqCst), 0);
    assert!(handles.status.raised().contains(&ActiveCondition::VaultUnavailable));
}

/// 파괴적-빈-커밋 -> `HeldDestructiveEmpty`, 드라이버 미호출.
#[test]
fn destructive_empty_holds() {
    let (mut coord, handles) = build(Scenario {
        paused: false,
        availability: Availability::Reachable,
        verdict: GuardVerdict::HoldDestructiveEmpty,
        last: Some(nonempty_manifest()),
        current: empty_manifest(),
        consent: ConsentDecision::Permitted,
        driver_succeeds: true,
    });
    let outcome = coord.run_cycle(CycleTrigger::SyncNow);
    assert!(matches!(outcome, CoordinatorOutcome::HeldDestructiveEmpty));
    assert_eq!(handles.driver_calls.load(Ordering::SeqCst), 0);
}

/// 빈 diff -> `NoOp`, 드라이버 미호출.
#[test]
fn empty_diff_is_noop() {
    let (mut coord, handles) = build(Scenario {
        paused: false,
        availability: Availability::Reachable,
        verdict: GuardVerdict::Proceed,
        last: None,
        current: empty_manifest(),
        consent: ConsentDecision::Permitted,
        driver_succeeds: true,
    });
    let outcome = coord.run_cycle(CycleTrigger::SyncNow);
    assert!(matches!(outcome, CoordinatorOutcome::NoOp));
    assert_eq!(handles.driver_calls.load(Ordering::SeqCst), 0);
}

/// consent 차단 -> `BlockedConsent`, 드라이버 미호출.
#[test]
fn blocked_consent_does_not_drive() {
    let (mut coord, handles) = build(Scenario {
        paused: false,
        availability: Availability::Reachable,
        verdict: GuardVerdict::Proceed,
        last: None,
        current: nonempty_manifest(),
        consent: ConsentDecision::Blocked(BlockReason::NotGranted),
        driver_succeeds: true,
    });
    let outcome = coord.run_cycle(CycleTrigger::SyncNow);
    assert!(matches!(
        outcome,
        CoordinatorOutcome::BlockedConsent(BlockReason::NotGranted)
    ));
    assert_eq!(handles.driver_calls.load(Ordering::SeqCst), 0);
}

/// diff 있음 + Permitted -> 드라이버 정확히 1회 + 성공 표면화.
#[test]
fn permitted_change_drives_once_and_records_success() {
    let (mut coord, handles) = build(Scenario {
        paused: false,
        availability: Availability::Reachable,
        verdict: GuardVerdict::Proceed,
        last: None,
        current: nonempty_manifest(),
        consent: ConsentDecision::Permitted,
        driver_succeeds: true,
    });
    let outcome = coord.run_cycle(CycleTrigger::SyncNow);
    assert!(matches!(outcome, CoordinatorOutcome::Uploaded(_)));
    assert_eq!(handles.driver_calls.load(Ordering::SeqCst), 1);
    assert_eq!(handles.dirty_calls.load(Ordering::SeqCst), 1);
    assert_eq!(handles.status.sync_success_count(), 1);
    assert_eq!(handles.history.append_count(), 1);
    assert_eq!(handles.critical.results(), vec![true]);
}

/// 드라이버 실패 -> `Failed` + 실패 표면화, 드라이버 1회.
#[test]
fn driver_failure_surfaces_as_failed() {
    let (mut coord, handles) = build(Scenario {
        paused: false,
        availability: Availability::Reachable,
        verdict: GuardVerdict::Proceed,
        last: None,
        current: nonempty_manifest(),
        consent: ConsentDecision::Permitted,
        driver_succeeds: false,
    });
    let outcome = coord.run_cycle(CycleTrigger::SyncNow);
    assert!(matches!(outcome, CoordinatorOutcome::Failed(_)));
    assert_eq!(handles.driver_calls.load(Ordering::SeqCst), 1);
    assert_eq!(handles.history.append_count(), 1);
    assert_eq!(handles.critical.results(), vec![false]);
    assert_eq!(handles.status.sync_success_count(), 0);
}

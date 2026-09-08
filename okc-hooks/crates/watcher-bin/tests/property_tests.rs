//! U8 Testable Properties(PROP-U8-01..07) — `proptest-support` feature 뒤에서만 컴파일된다.
//!
//! 네트워크/파일시스템/실제 스레드 없이 in-memory fake seam + 순수 판정 함수로 사이클 분기·
//! 직렬화·표면화·단일 인스턴스·배선 비순환·config round-trip 을 결정적으로 검증한다.
#![cfg(feature = "proptest-support")]

mod common;

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc;

use proptest::prelude::*;

use auth_consent::ConsentDecision;
use change_detect::{Availability, GuardVerdict};
use foundation::proptest_support::generators::arb_valid_watcher_config;
use foundation::{ActiveCondition, ConfigSnapshot};

use watcher_bin::config::{RawFederatedConfig, known_config_keys};
use watcher_bin::coordinator::{
    CoordinatorOutcome, CycleGate, CycleTrigger, WIRING_EDGES, WiringEdge, coalesce_to_latest,
    decide_cycle_gate, wiring_is_acyclic,
};
use watcher_bin::instance_lock::{LockConfig, LockError, SingleInstanceLock};
use watcher_bin::proptest_support::{
    arb_availability, arb_consent_decision, arb_cycle_trigger, arb_guard_verdict,
};

use common::{Scenario, build, empty_manifest, nonempty_manifest};

// ---------------------------------------------------------------------------
// PROP-U8-01 — SingleInstanceLock 상호 배제
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_single_instance_mutual_exclusion(tag in 0u64..5_000) {
        let path = std::env::temp_dir().join(format!(
            "okc_u8_lock_prop_{}_{}",
            std::process::id(),
            tag
        ));
        let _ = std::fs::remove_file(&path);
        let cfg = LockConfig { path: path.clone() };

        // free -> 획득 성공 + 현재 pid.
        let guard = SingleInstanceLock::acquire(&cfg).expect("free acquire");
        prop_assert_eq!(guard.record().pid, std::process::id());

        // live holder -> 두 번째 획득 실패.
        let second = SingleInstanceLock::acquire(&cfg);
        prop_assert!(
            matches!(second, Err(LockError::AlreadyRunning { .. })),
            "second acquire must fail while held"
        );
        drop(second);
        drop(guard);

        // drop 후 재획득 성공(정리 보장).
        let reacquired = SingleInstanceLock::acquire(&cfg);
        prop_assert!(reacquired.is_ok(), "reacquire after drop must succeed");
        drop(reacquired);
        let _ = std::fs::remove_file(&path);
    }
}

// ---------------------------------------------------------------------------
// PROP-U8-03 — 배선 그래프 비순환
// ---------------------------------------------------------------------------

/// 실제 배선 간선 집합은 비순환이다(역엣지 0).
#[test]
fn wiring_edges_are_acyclic() {
    assert!(wiring_is_acyclic(WIRING_EDGES));
}

/// 역엣지를 하나 주입하면 사이클 탐지기가 이를 검출한다(탐지기 건전성).
#[test]
fn wiring_detects_injected_back_edge() {
    let mut edges = WIRING_EDGES.to_vec();
    edges.push(WiringEdge {
        from: "config",
        to: "daemon",
    });
    assert!(!wiring_is_acyclic(&edges));
}

// ---------------------------------------------------------------------------
// PROP-U8-04 — 배타적 게이트 + 조건부 1회 구동
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_gate_precedence_is_total_and_exclusive(
        paused in any::<bool>(),
        availability in arb_availability(),
        verdict in arb_guard_verdict(),
        change_empty in any::<bool>(),
        consent in arb_consent_decision(),
    ) {
        let gate = decide_cycle_gate(paused, availability, &verdict, change_empty, &consent);
        match gate {
            CycleGate::SkipPaused => prop_assert!(paused, "skip only when paused"),
            CycleGate::HoldVaultUnavailable => prop_assert!(
                !paused
                    && (availability != Availability::Reachable
                        || matches!(verdict, GuardVerdict::HoldVaultUnavailable(_))),
                "hold-unavailable precondition"
            ),
            CycleGate::HoldDestructiveEmpty => prop_assert!(
                !paused
                    && availability == Availability::Reachable
                    && matches!(verdict, GuardVerdict::HoldDestructiveEmpty),
                "hold-destructive precondition"
            ),
            CycleGate::NoOp => prop_assert!(
                !paused
                    && availability == Availability::Reachable
                    && matches!(verdict, GuardVerdict::Proceed)
                    && change_empty,
                "noop precondition"
            ),
            CycleGate::BlockConsent(_) => prop_assert!(
                !paused && matches!(consent, ConsentDecision::Blocked(_)),
                "block-consent precondition"
            ),
            CycleGate::Proceed => prop_assert!(
                matches!(consent, ConsentDecision::Permitted),
                "proceed requires permitted consent"
            ),
        }
    }
}

proptest! {
    #[test]
    fn prop_run_cycle_drives_exactly_once_when_permitted(
        availability in arb_availability(),
        verdict in arb_guard_verdict(),
        consent in arb_consent_decision(),
        use_empty in any::<bool>(),
        driver_succeeds in any::<bool>(),
    ) {
        let current = if use_empty { empty_manifest() } else { nonempty_manifest() };
        let (mut coord, handles) = build(Scenario {
            paused: false,
            availability,
            verdict: verdict.clone(),
            last: None,
            current,
            consent: consent.clone(),
            driver_succeeds,
        });
        let outcome = coord.run_cycle(CycleTrigger::SyncNow);
        let drives = handles.driver_calls.load(Ordering::SeqCst);

        match outcome {
            CoordinatorOutcome::Uploaded(_) | CoordinatorOutcome::Failed(_) => {
                prop_assert_eq!(drives, 1);
            }
            _ => prop_assert_eq!(drives, 0),
        }

        // 드라이버 구동은 오직 reachable + Proceed + non-empty diff + Permitted 조합에서만.
        if drives == 1 {
            prop_assert_eq!(availability, Availability::Reachable);
            prop_assert!(matches!(verdict, GuardVerdict::Proceed), "verdict must be proceed");
            prop_assert!(
                matches!(consent, ConsentDecision::Permitted),
                "consent must be permitted"
            );
            prop_assert!(!use_empty, "diff must be non-empty");
        }
    }
}

// ---------------------------------------------------------------------------
// PROP-U8-05 — drain-to-latest 합류
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_coalesce_keeps_latest_and_drains(extra in prop::collection::vec(arb_cycle_trigger(), 0..8)) {
        let (tx, rx) = mpsc::channel::<CycleTrigger>();
        let first = CycleTrigger::SyncNow;
        tx.send(first.clone()).expect("send first");
        for trigger in &extra {
            tx.send(trigger.clone()).expect("send extra");
        }

        let mut current = rx.recv().expect("recv first");
        coalesce_to_latest(&rx, &mut current);

        // 합류 후 채널은 비어 있다(폭주 전부 흡수).
        prop_assert!(rx.try_recv().is_err(), "channel fully drained");
        // 최신 트리거만 유효(추가분 없으면 first 유지).
        let expected = extra.last().cloned().unwrap_or(first);
        prop_assert_eq!(current, expected);
    }
}

// ---------------------------------------------------------------------------
// PROP-U8-06 — U2 판정 push 1회
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_hold_raises_condition_once(
        availability in arb_availability(),
        verdict in arb_guard_verdict(),
    ) {
        let (mut coord, handles) = build(Scenario {
            paused: false,
            availability,
            verdict: verdict.clone(),
            last: Some(nonempty_manifest()),
            current: nonempty_manifest(),
            consent: ConsentDecision::Permitted,
            driver_succeeds: true,
        });
        let outcome = coord.run_cycle(CycleTrigger::SyncNow);

        if matches!(
            outcome,
            CoordinatorOutcome::HeldVaultUnavailable | CoordinatorOutcome::HeldDestructiveEmpty
        ) {
            let raises = handles
                .status
                .raised()
                .into_iter()
                .filter(|cond| *cond == ActiveCondition::VaultUnavailable)
                .count();
            prop_assert_eq!(raises, 1);
        }
    }
}

/// 성공 사이클은 `record_sync_success` 를 정확히 1회 수행한다(PROP-U8-06 성공 경로).
#[test]
fn success_records_sync_success_once() {
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
    assert_eq!(handles.status.sync_success_count(), 1);
    assert_eq!(handles.critical.results(), vec![true]);
}

// ---------------------------------------------------------------------------
// PROP-U8-07 — known-key union + federated round-trip + timeout 검증
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_federated_round_trip_and_timeout_rule(
        config in arb_valid_watcher_config(),
        timeout in prop_oneof![Just(-5i64), Just(0i64), 1i64..3600],
    ) {
        // union 은 네 소스의 대표 키를 모두 포함한다.
        let keys = known_config_keys();
        prop_assert!(keys.contains(&"request_timeout_s"), "u5 key present");
        prop_assert!(keys.contains(&"debounce_ms"), "u8 key present");
        prop_assert!(keys.contains(&"log_file"), "u6 key present");
        prop_assert!(keys.contains(&"vault_path"), "u0 key present");

        // raw federated 투영은 JSON round-trip 에서 값 보존.
        let raw = RawFederatedConfig {
            request_timeout_s: Some(timeout),
            ..Default::default()
        };
        let value = raw.to_json_value().expect("to json");
        let restored = RawFederatedConfig::from_json_value(value).expect("from json");
        prop_assert_eq!(raw.clone(), restored);

        // 비양수 timeout 은 U5 규칙대로 거부, 양수는 해소 성공.
        let core = ConfigSnapshot::new(Arc::new(config));
        let resolved = raw.resolve_with_default_data_dir(&core, std::path::PathBuf::from("/data"));
        if timeout >= 1 {
            prop_assert!(resolved.is_ok(), "positive timeout resolves");
        } else {
            prop_assert!(resolved.is_err(), "non-positive timeout rejected");
        }
    }
}

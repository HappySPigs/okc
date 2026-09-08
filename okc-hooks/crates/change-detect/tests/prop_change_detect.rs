//! Property-Based Testing (PBT, `proptest-support` feature 게이트).
//!
//! feature 가 꺼진 기본 빌드에서는 빈 테스트 크레이트로 컴파일되고,
//! `cargo test --features proptest-support` 로만 실제 property 가 실행된다(proptest 프로덕션
//! 그래프 미유입, T9/PBT-07). 규칙 계층 속성 PROP-U2-01~08 을 구현한다.
#![cfg(feature = "proptest-support")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::time::{Duration, Instant};

use change_detect::proptest_support::generators::{self as g, SchedCmd};
use change_detect::{
    Availability, CycleOutcome, GuardVerdict, RawFsEvent, ReconciliationScheduler,
    VaultAvailabilityGuard, simulate_debounce,
};
use foundation::Timestamp;
use proptest::prelude::*;

/// `(경과 ms, 이벤트)` 타임라인을 기준 시각으로부터 절대 `Instant` 시퀀스로 누적한다.
fn to_absolute(base: Instant, timeline: &[(u64, RawFsEvent)]) -> Vec<(Instant, RawFsEvent)> {
    let mut cur = base;
    let mut out = Vec::with_capacity(timeline.len());
    for (gap, ev) in timeline {
        cur += Duration::from_millis(*gap);
        out.push((cur, ev.clone()));
    }
    out
}

proptest! {
    /// PROP-U2-01 / PROP-U2-02 — exactly-1-trigger-per-burst + 디바운스 지연 상한.
    ///
    /// 각 버스트(인접 간격 `< T_debounce`)마다 정확히 1 트리거, 발행 시각 = 버스트 마지막
    /// 이벤트 + `T_debounce`. 총 트리거 수 == 버스트 수.
    #[test]
    fn prop_exactly_one_trigger_per_burst(timeline in g::arb_event_timeline()) {
        let t = Duration::from_millis(2000);
        let base = Instant::now();
        let abs = to_absolute(base, &timeline);

        let triggers = simulate_debounce(t, &abs);

        if abs.is_empty() {
            prop_assert!(triggers.is_empty());
            return Ok(());
        }

        // 오라클: 간격 >= T_debounce 에서 버스트 분할, 버스트별 마지막 이벤트 시각 수집.
        let mut burst_last: Vec<Instant> = Vec::new();
        for (i, (arrival, _)) in abs.iter().enumerate() {
            if i == 0 {
                burst_last.push(*arrival);
            } else {
                let gap = *arrival - abs[i - 1].0;
                if gap >= t {
                    burst_last.push(*arrival);
                } else if let Some(last) = burst_last.last_mut() {
                    *last = *arrival;
                }
            }
        }

        prop_assert_eq!(triggers.len(), burst_last.len());
        for (trig, last) in triggers.iter().zip(burst_last.iter()) {
            // PROP-U2-02: 발행 시각 = 버스트 마지막 이벤트 + T_debounce.
            prop_assert_eq!(trig.observed_at, *last + t);
        }
    }

    /// PROP-U2-03 — 오버플로가 트리거 없이 삼켜지지 않는다.
    ///
    /// `Overflow` 를 포함한 비어있지 않은 타임라인은 정적 구간 후 최소 1 트리거를 낸다.
    #[test]
    fn prop_overflow_yields_trigger(timeline in g::arb_event_timeline()) {
        let t = Duration::from_millis(2000);
        let base = Instant::now();
        let abs = to_absolute(base, &timeline);
        let has_overflow = abs.iter().any(|(_, e)| *e == RawFsEvent::Overflow);
        let triggers = simulate_debounce(t, &abs);
        if has_overflow {
            prop_assert!(!triggers.is_empty());
        }
    }

    /// PROP-U2-05 — 백엔드 무관 트리거 등가(구성적).
    ///
    /// 디바운스 로직은 정규화된 `RawFsEvent` 만 소비하므로, 동일 타임라인은 백엔드 종류와
    /// 무관하게 동일 트리거 시퀀스를 낳는다(결정성으로 관측). 여기서는 두 "백엔드"가 동일
    /// 정규화 이벤트를 방출한 경우로 재생해 트리거 시퀀스 동일성을 확인한다.
    #[test]
    fn prop_backend_agnostic_equivalence(timeline in g::arb_event_timeline()) {
        let t = Duration::from_millis(2000);
        let base = Instant::now();
        let abs = to_absolute(base, &timeline);
        let a = simulate_debounce(t, &abs);
        let b = simulate_debounce(t, &abs);
        prop_assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            prop_assert_eq!(x.observed_at, y.observed_at);
            prop_assert_eq!(x.kind, y.kind);
        }
    }

    /// PROP-U2-04 / PROP-U2-06 — 검출 지연 상한(`next_due <= 마지막 발행 + T_recon`) + 직렬화.
    ///
    /// 임의 스케줄러 명령 시퀀스에 대해 (a) busy 중 `tick` 은 절대 `Some` 을 반환하지 않고,
    /// (b) `next_due` 는 항상 `마지막 재조정 발행 + T_recon` 을 넘지 않으며,
    /// (c) `record_result` 는 `next_due` 를 변경하지 않는다.
    #[test]
    fn prop_recon_backstop_and_serialization(cmds in g::arb_sched_commands()) {
        let t = Duration::from_secs(900);
        let base = Instant::now();
        let mut sched = ReconciliationScheduler::new(t);
        sched.run_startup_scan(base);

        let mut now = base;
        let mut last_emit = base; // 시작 재조정 발행 시각.

        for cmd in cmds {
            match cmd {
                SchedCmd::Tick { advance_ms, busy } => {
                    now += Duration::from_millis(advance_ms);
                    let out = sched.tick(now, busy);
                    if busy {
                        // PROP-U2-06: busy 중 동시 재조정 없음.
                        prop_assert!(out.is_none());
                    }
                    if out.is_some() {
                        last_emit = now;
                        // 발행 직후 앵커.
                        prop_assert_eq!(sched.next_recon_due(), Some(now + t));
                    }
                    // PROP-U2-04 오라클: next_due <= 마지막 발행 + T_recon.
                    if let Some(due) = sched.next_recon_due() {
                        prop_assert!(due <= last_emit + t);
                    }
                }
                SchedCmd::Record(outcome) => {
                    let before = sched.next_recon_due();
                    sched.record_result(outcome, Timestamp::from_unix_nanos(0));
                    // record_result 는 next_due 를 재계산하지 않는다(R-RECON-04).
                    prop_assert_eq!(sched.next_recon_due(), before);
                }
            }
        }
    }

    /// PROP-U2-07 — 파괴적-빈-커밋 불가 불변식.
    ///
    /// `availability != Reachable` 이거나 (`empty(new)` AND `last_committed` 비어있지 않음 AND
    /// `!confirm_empty`) 이면 `guard_diff` 는 절대 `Proceed` 를 반환하지 않는다.
    #[test]
    fn prop_no_destructive_empty_commit(
        (new_m, last_m, availability, confirm) in g::arb_guard_case()
    ) {
        let guard = VaultAvailabilityGuard::new(confirm);
        let verdict = guard.guard_diff(&new_m, last_m.as_ref(), availability);

        let empty_new = new_m.entries.is_empty();
        let last_non_empty = matches!(&last_m, Some(m) if !m.entries.is_empty());
        let must_hold =
            availability != Availability::Reachable || (empty_new && last_non_empty && !confirm);

        if must_hold {
            prop_assert_ne!(verdict, GuardVerdict::Proceed);
        }
    }

    /// PROP-U2-08 — 자동 재개(무상태) + 멱등.
    ///
    /// `Reachable` AND `!empty(new)` 이면 이력과 무관하게 항상 `Proceed`, 동일 입력 반복은 동일
    /// 판정.
    #[test]
    fn prop_auto_resume_idempotent(
        (new_m, last_m, _availability, confirm) in g::arb_guard_case()
    ) {
        let guard = VaultAvailabilityGuard::new(confirm);
        prop_assume!(!new_m.entries.is_empty());

        let first = guard.guard_diff(&new_m, last_m.as_ref(), Availability::Reachable);
        let second = guard.guard_diff(&new_m, last_m.as_ref(), Availability::Reachable);
        prop_assert_eq!(&first, &second); // 멱등.
        prop_assert_eq!(first, GuardVerdict::Proceed); // 자동 재개.
    }
}

/// PROP-U2-04 보강 — 예제 기반 emit-anchor 불변식(연속 발행 간격 == T_recon, busy skip 무-스택).
#[test]
fn example_emit_anchor_invariant() {
    let t = Duration::from_secs(900);
    let base = Instant::now();
    let mut sched = ReconciliationScheduler::new(t);
    sched.run_startup_scan(base);
    assert_eq!(sched.next_recon_due(), Some(base + t));

    // 만료 후 발행 -> 재-앵커.
    let due1 = base + t;
    let s1 = sched.tick(due1, false).expect("발행");
    assert!(matches!(
        s1.kind,
        change_detect::TriggerKind::Reconciliation(change_detect::ReconPhase::Periodic)
    ));
    assert_eq!(sched.next_recon_due(), Some(due1 + t));

    // busy skip 은 스택을 쌓지 않는다(next_due 불변).
    assert!(sched.tick(due1 + t + Duration::from_secs(1), true).is_none());
    assert_eq!(sched.next_recon_due(), Some(due1 + t));

    // 완료 통지는 next_due 를 재계산하지 않는다.
    sched.record_result(CycleOutcome::NoOp, Timestamp::from_unix_nanos(1));
    assert_eq!(sched.next_recon_due(), Some(due1 + t));
}

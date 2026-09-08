//! U2 도메인 제너레이터 — 이벤트 도착 타임라인, 스케줄러 명령 시퀀스, 가드 입력 조합.
//!
//! `Instant` 는 `Arbitrary` 가 아니므로 타임라인/스케줄러 시각은 **직전 대비 경과 ms** 로 생성하고
//! 테스트가 기준 `Instant` 로부터 누적해 절대 시각을 구성한다(결정적 재생).

use proptest::prelude::*;

use foundation::Manifest;
use foundation::proptest_support::generators as fg;

use crate::guard::Availability;
use crate::scheduler::CycleOutcome;
use crate::watcher::RawFsEvent;

/// 단일 정규화 `RawFsEvent`(경로 이벤트 3종 + `Overflow`)를 생성한다.
pub fn arb_raw_fs_event() -> impl Strategy<Value = RawFsEvent> {
    prop_oneof![
        fg::arb_relative_path().prop_map(RawFsEvent::Created),
        fg::arb_relative_path().prop_map(RawFsEvent::Modified),
        fg::arb_relative_path().prop_map(RawFsEvent::Deleted),
        Just(RawFsEvent::Overflow),
    ]
}

/// `(직전 대비 경과 ms, 이벤트)` 타임라인 — 단일/밀집 버스트/경계 간격/다중 버스트/`Overflow`.
///
/// 경과 ms 는 0..5000 범위로, 다양한 `T_debounce`(예: 2000ms)에 대해 버스트 내/버스트 경계를
/// 모두 커버한다.
pub fn arb_event_timeline() -> impl Strategy<Value = Vec<(u64, RawFsEvent)>> {
    prop::collection::vec((0u64..5000, arb_raw_fs_event()), 0..14)
}

/// `Availability` 전 변이(유한 도메인 — 근사 전수 검증).
pub fn arb_availability() -> impl Strategy<Value = Availability> {
    prop_oneof![
        Just(Availability::Reachable),
        Just(Availability::RootMissing),
        Just(Availability::Unmounted),
        Just(Availability::Inaccessible),
    ]
}

/// U2 `CycleOutcome` 전 변이.
pub fn arb_cycle_outcome() -> impl Strategy<Value = CycleOutcome> {
    prop_oneof![
        Just(CycleOutcome::NoOp),
        Just(CycleOutcome::Committed),
        ".*".prop_map(CycleOutcome::Held),
        ".*".prop_map(CycleOutcome::Failed),
    ]
}

/// 스케줄러 명령 — `(now 증가, busy/idle 전이, record_result)` 시퀀스의 원소.
#[derive(Debug, Clone)]
pub enum SchedCmd {
    /// `tick(now, busy)` 호출. `advance_ms` 는 직전 대비 단조 시점 경과.
    Tick {
        /// 직전 시각 대비 경과 ms(단조 증가).
        advance_ms: u64,
        /// 진행 중 사이클 여부.
        busy: bool,
    },
    /// `record_result(outcome, at)` 통지.
    Record(CycleOutcome),
}

/// 스케줄러 명령 시퀀스 — 긴 busy 구간·조밀 tick·경계(now == next_due)를 포함하도록 생성.
pub fn arb_sched_commands() -> impl Strategy<Value = Vec<SchedCmd>> {
    let cmd = prop_oneof![
        (0u64..2000, any::<bool>()).prop_map(|(advance_ms, busy)| SchedCmd::Tick { advance_ms, busy }),
        arb_cycle_outcome().prop_map(SchedCmd::Record),
    ];
    prop::collection::vec(cmd, 0..30)
}

/// 가드 입력 조합 — `Availability` x 매니페스트(빈/비빈) x `last_committed`(None/빈/비빈) x
/// `confirm_empty`. U0 `arb_manifest` 를 재사용한다(단일 출처).
pub fn arb_guard_case() -> impl Strategy<Value = (Manifest, Option<Manifest>, Availability, bool)> {
    (
        fg::arb_manifest(),
        prop::option::of(fg::arb_manifest()),
        arb_availability(),
        any::<bool>(),
    )
}

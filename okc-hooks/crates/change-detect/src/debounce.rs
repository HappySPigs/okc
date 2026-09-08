//! 순수 디바운스 상태 기계 — 볼트-전체 단일 정적-구간 타이머(D1).
//!
//! `Debouncer` 는 `FilesystemWatcher` 의 감시 스레드가 소유하는 in-memory 상태이며,
//! change/overflow 이벤트 도착마다 타이머를 리셋하고 무이벤트 `T_debounce` 경과 시 트리거 1개를
//! 발행한다(R-DEB-01). `Overflow` 도 합성 change 로 동일 투입한다(R-DEB-04).
//!
//! `simulate_debounce` 는 감시 스레드의 `recv_timeout` 루프와 **동일한 결정 로직**을 결정적으로
//! 재생하는 순수 함수다 — 이벤트 타임라인에 대한 트리거 시퀀스를 계산해 PROP-U2-01/02/03/05 를
//! 검증한다. 상위 디바운스 로직은 정규화된 `RawFsEvent` 만 소비하므로 백엔드 종류에 불변이다.
//!
//! 순수 리프 모듈로서 panic-free 를 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::time::{Duration, Instant};

use crate::trigger::{TriggerKind, TriggerSignal};
use crate::watcher::RawFsEvent;

/// 볼트-전체 단일 디바운스 타이머 상태(버스트 카운트 + 정적-구간 데드라인).
///
/// 파일별 디바운스·rename 추적 없이(D2) 볼트 전체를 하나의 타이머로 합친다. 지속하지 않는다.
#[derive(Debug, Clone)]
pub struct Debouncer {
    t_debounce: Duration,
    deadline: Option<Instant>,
    burst_count: u64,
}

impl Debouncer {
    /// 주어진 정적 구간(`T_debounce`)으로 디바운서를 구성한다(타이머 미가동).
    pub fn new(t_debounce: Duration) -> Self {
        Debouncer {
            t_debounce,
            deadline: None,
            burst_count: 0,
        }
    }

    /// 현재 정적-구간 데드라인(가동 중이면 `Some`, 미가동이면 `None`).
    pub fn deadline(&self) -> Option<Instant> {
        self.deadline
    }

    /// 진행 중 버스트에 누적된 이벤트 수(진단용).
    pub fn burst_count(&self) -> u64 {
        self.burst_count
    }

    /// change/overflow 이벤트 도착 -> 데드라인을 `now + T_debounce` 로 리셋(버스트 합치기).
    ///
    /// 이미 가동 중이면 재설정되어 다수 이벤트가 1개 트리거로 합쳐진다(R-DEB-01).
    pub fn on_event(&mut self, now: Instant) {
        self.deadline = Some(now + self.t_debounce);
        self.burst_count = self.burst_count.saturating_add(1);
    }

    /// 정적 구간 만료 -> 트리거 1개 생성 + 버스트 종료(카운트/데드라인 리셋).
    ///
    /// 반환 `TriggerSignal` 의 `observed_at` 은 만료 시각(`now`)이며 kind 는 `Debounced` 다.
    pub fn fire(&mut self, now: Instant) -> TriggerSignal {
        let count = self.burst_count;
        self.deadline = None;
        self.burst_count = 0;
        TriggerSignal::new(
            TriggerKind::Debounced,
            format!("{count} events in burst"),
            now,
        )
    }
}

/// 감시 스레드의 `recv_timeout` 루프와 동일한 로직을 결정적으로 재생하는 순수 함수.
///
/// `events` 는 `(도착 시각, 이벤트)` 시퀀스이며 시각 오름차순으로 정렬되어 처리된다. 각 버스트
/// (인접 간격 `< T_debounce`)마다 정확히 1개 트리거를 `t_last + T_debounce` 시점에 발행한다
/// (R-DEB-01/02). `Overflow` 도 change 로 취급되어 동일하게 타이머를 리셋한다(R-DEB-04).
///
/// 반환 벡터는 발행 순서를 보존한 트리거 시퀀스다. 백엔드 종류와 무관하게 동일한 정규화
/// `RawFsEvent` 타임라인은 동일한 트리거 시퀀스를 낳는다(PROP-U2-05, 구성적 등가).
pub fn simulate_debounce(
    t_debounce: Duration,
    events: &[(Instant, RawFsEvent)],
) -> Vec<TriggerSignal> {
    let mut sorted: Vec<(Instant, RawFsEvent)> = events.to_vec();
    sorted.sort_by_key(|e| e.0);

    let mut deb = Debouncer::new(t_debounce);
    let mut out: Vec<TriggerSignal> = Vec::new();

    for (arrival, _ev) in &sorted {
        // 다음 이벤트 도착 전에 데드라인이 만료되면 트리거를 먼저 발행한다.
        while let Some(deadline) = deb.deadline() {
            if deadline <= *arrival {
                out.push(deb.fire(deadline));
            } else {
                break;
            }
        }
        // 모든 RawFsEvent(change 또는 Overflow)는 합성 change 로 타이머를 리셋한다.
        deb.on_event(*arrival);
    }
    // 마지막 버스트의 잔여 데드라인 flush.
    if let Some(deadline) = deb.deadline() {
        out.push(deb.fire(deadline));
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )]
    use super::*;
    use foundation::RelativePath;

    fn rel(p: &str) -> RelativePath {
        RelativePath::normalize(p).expect("고정 테스트 경로는 정규화된다")
    }

    #[test]
    fn single_event_yields_one_trigger_at_last_plus_debounce() {
        let t = Duration::from_millis(100);
        let base = Instant::now();
        let events = vec![(base, RawFsEvent::Created(rel("a.txt")))];
        let triggers = simulate_debounce(t, &events);
        assert_eq!(triggers.len(), 1);
        assert_eq!(triggers[0].kind, TriggerKind::Debounced);
        assert_eq!(triggers[0].observed_at, base + t);
    }

    #[test]
    fn dense_burst_coalesces_to_one_trigger() {
        let t = Duration::from_millis(100);
        let base = Instant::now();
        // 간격 50ms(< 100ms) 인 3 이벤트 = 단일 버스트.
        let events = vec![
            (base, RawFsEvent::Created(rel("a"))),
            (base + Duration::from_millis(50), RawFsEvent::Modified(rel("a"))),
            (base + Duration::from_millis(100), RawFsEvent::Modified(rel("a"))),
        ];
        let triggers = simulate_debounce(t, &events);
        assert_eq!(triggers.len(), 1);
        // 마지막 이벤트(base+100) + T_debounce(100) = base+200.
        assert_eq!(triggers[0].observed_at, base + Duration::from_millis(200));
    }

    #[test]
    fn gap_at_least_debounce_splits_into_two_bursts() {
        let t = Duration::from_millis(100);
        let base = Instant::now();
        let events = vec![
            (base, RawFsEvent::Created(rel("a"))),
            // 간격 정확히 100ms(>= T_debounce) -> 새 버스트.
            (base + Duration::from_millis(100), RawFsEvent::Created(rel("b"))),
        ];
        let triggers = simulate_debounce(t, &events);
        assert_eq!(triggers.len(), 2);
    }

    #[test]
    fn overflow_alone_yields_one_trigger() {
        let t = Duration::from_millis(100);
        let base = Instant::now();
        let events = vec![(base, RawFsEvent::Overflow)];
        let triggers = simulate_debounce(t, &events);
        assert_eq!(triggers.len(), 1);
    }

    #[test]
    fn empty_timeline_yields_no_trigger() {
        let t = Duration::from_millis(100);
        let triggers = simulate_debounce(t, &[]);
        assert!(triggers.is_empty());
    }
}

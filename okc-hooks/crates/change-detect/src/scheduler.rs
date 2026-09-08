//! `ReconciliationScheduler` — 미관측 변경 검출 지연을 `<= T_recon` 으로 상한 짓는 백스톱
//! 트리거 소스(FR-04 / NFR-03).
//!
//! 자체 타이머·스레드를 갖지 않는 **순수 `tick(now)` 판정자**다 — 상위 데몬 스케줄러가 단조
//! 시점(`Instant`)을 공급한다(D7, async 미도입). `next_due` 는 재조정 트리거 **발행 시각 앵커**로
//! 잡히며(`발행 시각 + T_recon`, R-RECON-04), `record_result` 는 `last_result` 만 갱신하고
//! `next_due` 를 완료 시각으로 재계산하지 않는다 — 완료 앵커는 간격을 `사이클 시간 + T_recon` 으로
//! 늘려 상한을 깨뜨리기 때문이다.
//!
//! 순수 리프 모듈로서 panic-free 를 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::time::{Duration, Instant};

use foundation::Timestamp;

use crate::trigger::{ReconPhase, TriggerKind, TriggerSignal};

/// 코디네이터가 완료한 사이클 결과. `record_result` 로 통지되어 `last_result` 갱신에 소비된다.
///
/// 어느 결과든 재스냅샷이 수행됐으므로 **재조정 커버로 간주**된다 — `next_due` 는 발행 시각 앵커로
/// 이미 설정되어 있고 완료 시각으로 재계산되지 않는다(R-RECON-04).
///
/// 주의: U0 `foundation::CycleOutcome`(U6 placeholder, `Success`/`Failure`)와는 의미가 다른
/// U2 고유 타입이다(도메인 4-변이, `domain-entities.md` §4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CycleOutcome {
    /// diff 가 비어 업로드 미발생(변경 없음).
    NoOp,
    /// 업로드·커밋 성공.
    Committed,
    /// 가드 보류(vault-unavailable / destructive-empty).
    Held(String),
    /// 사이클 실패(재시도는 U4 소관).
    Failed(String),
}

/// 마지막 재조정의 조회 가능한 요약. 진단·표면화 전용(스케줄 기준은 단조 시점, 표면화는 UTC).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconResult {
    /// Startup / Periodic.
    pub phase: ReconPhase,
    /// 그 재조정 사이클의 결과.
    pub outcome: CycleOutcome,
    /// 재조정 완료 시각(UTC, 사람 표면화용).
    pub at: Timestamp,
}

/// 발행-시각 앵커 백스톱 스케줄러. in-memory 상태(`next_due`/`last_result`)만 보유하며
/// 지속하지 않는다 — 재시작 시 시작 재조정이 초기화한다.
#[derive(Debug, Clone)]
pub struct ReconciliationScheduler {
    t_recon: Duration,
    /// 다음 주기 재조정 발행 예정 시각(단조). 시작 재조정 전에는 `None`.
    next_due: Option<Instant>,
    /// 마지막으로 발행한 재조정의 국면(`record_result` 시 `ReconResult.phase` 로 사용).
    last_phase: Option<ReconPhase>,
    last_result: Option<ReconResult>,
}

impl ReconciliationScheduler {
    /// 주어진 백스톱 간격(`T_recon`)으로 스케줄러를 구성한다(시작 재조정 전).
    pub fn new(t_recon: Duration) -> Self {
        ReconciliationScheduler {
            t_recon,
            next_due: None,
            last_phase: None,
            last_result: None,
        }
    }

    /// 데몬 기동 시 1회 전체 재조정을 발행한다(R-RECON-01).
    ///
    /// `next_due = now + T_recon` 으로 초기화(발행 시각 앵커)하고 `Reconciliation(Startup)`
    /// 트리거를 반환한다. 정상 라이브 감시 진입 전에 호출된다.
    pub fn run_startup_scan(&mut self, now: Instant) -> TriggerSignal {
        self.next_due = Some(now + self.t_recon);
        self.last_phase = Some(ReconPhase::Startup);
        TriggerSignal::new(
            TriggerKind::Reconciliation(ReconPhase::Startup),
            "startup reconciliation".to_string(),
            now,
        )
    }

    /// 주기 백스톱 판정(R-RECON-02). 다음을 **모두** 만족할 때만 `Some` 을 반환한다:
    /// (1) `now >= next_due`(발행 시각 앵커 만료), (2) 진행 중 사이클 없음(`!busy`).
    ///
    /// `Some` 반환 시 `next_due = now + T_recon`(발행 시각 앵커, R-RECON-04) 후
    /// `Reconciliation(Periodic)` 트리거를 낸다. 시작 재조정 전(`next_due` 미설정)이거나 busy
    /// 이거나 아직 만료 전이면 `None`.
    pub fn tick(&mut self, now: Instant, busy: bool) -> Option<TriggerSignal> {
        if busy {
            return None; // 진행 중 사이클이 재스냅샷 커버(R-RECON-06, 스택 없음).
        }
        let due = self.next_due?;
        if now < due {
            return None;
        }
        self.next_due = Some(now + self.t_recon);
        self.last_phase = Some(ReconPhase::Periodic);
        Some(TriggerSignal::new(
            TriggerKind::Reconciliation(ReconPhase::Periodic),
            "periodic recon due".to_string(),
            now,
        ))
    }

    /// 재조정 완료 통지 -> `last_result` 만 갱신(`next_due` 재계산 안 함, R-RECON-04).
    ///
    /// `at` 은 완료 시각(UTC, 표면화용)이며 스케줄 기준(단조 `next_due`)에 영향을 주지 않는다.
    /// `phase` 는 마지막으로 발행한 재조정의 국면을 승계한다(발행 이력이 없으면 `Periodic`).
    pub fn record_result(&mut self, outcome: CycleOutcome, at: Timestamp) {
        let phase = self.last_phase.unwrap_or(ReconPhase::Periodic);
        self.last_result = Some(ReconResult {
            phase,
            outcome,
            at,
        });
    }

    /// 다음 주기 재조정 발행 예정 시각(단조). 시작 재조정 전에는 `None`.
    pub fn next_recon_due(&self) -> Option<Instant> {
        self.next_due
    }

    /// 마지막 재조정 결과 요약(없으면 `None`).
    pub fn last_result(&self) -> Option<&ReconResult> {
        self.last_result.as_ref()
    }
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

    #[test]
    fn startup_anchors_next_due_at_emit_plus_t_recon() {
        let t = Duration::from_secs(900);
        let base = Instant::now();
        let mut sched = ReconciliationScheduler::new(t);
        let sig = sched.run_startup_scan(base);
        assert_eq!(sig.kind, TriggerKind::Reconciliation(ReconPhase::Startup));
        // 발행 시각 앵커: next_due == emit + T_recon.
        assert_eq!(sched.next_recon_due(), Some(base + t));
    }

    #[test]
    fn tick_before_startup_returns_none() {
        let t = Duration::from_secs(900);
        let mut sched = ReconciliationScheduler::new(t);
        assert!(sched.tick(Instant::now(), false).is_none());
    }

    #[test]
    fn tick_before_due_returns_none_then_fires_when_due() {
        let t = Duration::from_secs(900);
        let base = Instant::now();
        let mut sched = ReconciliationScheduler::new(t);
        sched.run_startup_scan(base);
        // 아직 만료 전.
        assert!(sched.tick(base + Duration::from_secs(100), false).is_none());
        // 만료 시점 발행 + 재-앵커.
        let due = base + t;
        let sig = sched.tick(due, false).expect("만료 시 발행");
        assert_eq!(sig.kind, TriggerKind::Reconciliation(ReconPhase::Periodic));
        assert_eq!(sched.next_recon_due(), Some(due + t));
    }

    #[test]
    fn busy_tick_never_fires() {
        let t = Duration::from_secs(900);
        let base = Instant::now();
        let mut sched = ReconciliationScheduler::new(t);
        sched.run_startup_scan(base);
        // 만료를 넘겼어도 busy 이면 절대 발행하지 않는다(R-RECON-06 / PROP-U2-06).
        assert!(sched.tick(base + t + Duration::from_secs(10), true).is_none());
        // next_due 도 재상승하지 않는다(발행 없음).
        assert_eq!(sched.next_recon_due(), Some(base + t));
    }

    #[test]
    fn record_result_updates_last_result_only_not_next_due() {
        let t = Duration::from_secs(900);
        let base = Instant::now();
        let mut sched = ReconciliationScheduler::new(t);
        sched.run_startup_scan(base);
        let before = sched.next_recon_due();
        sched.record_result(CycleOutcome::Committed, Timestamp::from_unix_nanos(42));
        // next_due 는 완료 통지로 재계산되지 않는다(R-RECON-04).
        assert_eq!(sched.next_recon_due(), before);
        let res = sched.last_result().expect("결과 기록됨");
        assert_eq!(res.phase, ReconPhase::Startup);
        assert_eq!(res.outcome, CycleOutcome::Committed);
        assert_eq!(res.at, Timestamp::from_unix_nanos(42));
    }
}

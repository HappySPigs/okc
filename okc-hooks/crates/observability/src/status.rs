//! StatusService — 2축 상태 단일 집계 지점 + `health_check`/`update_probe` 판정.
//!
//! `StatusSink`(push 뮤테이터) + `ReadJudgment`(`health_check`/`update_probe`) + 고유
//! `snapshot()` 을 구현한다. 전 상태를 `std::sync::Mutex<StatusInner>` 단일 락으로 보호하며,
//! 모든 읽기는 락 하에서 완결 값을 복제/파생해 부분 갱신을 노출하지 않는다(원자 관측).
//!
//! `Mutex` 를 보유하는 상태 모듈이므로 순수 lint-gate 는 적용하지 않는다(U0 store.rs 정합).

use std::collections::BTreeSet;
use std::sync::{Mutex, MutexGuard};

use foundation::{
    ActiveCondition, ByteCount, ConsentState, Health, HealthReason, Liveness, LivenessSignal,
    OperationalState, ReadJudgment, StatusSink, StatusSnapshot, Timestamp,
};

/// health-critical 조건의 닫힌 집합(US-E5-04 케이스1/3/4 — 케이스2 는 escalation 플래그).
///
/// `health_check()` 가 `Unhealthy` 로 flip 하는 유일한 `ActiveCondition` 들이다. `VaultUnavailable`/
/// `ConsentBlocked` 는 이 집합 밖이라 health 를 flip 하지 않는다(snapshot/로그에는 계속 노출).
const HEALTH_CRITICAL: [ActiveCondition; 3] = [
    ActiveCondition::AuthFailed,
    ActiveCondition::OverLimit,
    ActiveCondition::UpdateRolledBack,
];

/// 단일 락으로 보호되는 2축 상태 + 부가 필드 + 연속실패 escalation 플래그.
#[derive(Debug)]
struct StatusInner {
    /// 축1: 운영 라이프사이클(단일 값, 초기 `Idle`).
    operational: OperationalState,
    /// 축2: 동시 성립 활성 조건(중복 없는 집합, 결정적 순서).
    conditions: BTreeSet<ActiveCondition>,
    /// 마지막 성공 시각.
    last_success: Option<Timestamp>,
    /// 미커밋 변경 대기 표시.
    dirty: bool,
    /// 진행 중 전송 `(전송량, 전체)`.
    resume: Option<(ByteCount, ByteCount)>,
    /// startup liveness 신호 `IdleReached` 관측 여부(단조 누적).
    idle_reached: bool,
    /// startup liveness 신호 `CredentialReadable` 관측 여부(단조 누적).
    credential_readable: bool,
    /// 연속실패 escalation 플래그(`CriticalErrorNotifier` 가 push, D6).
    escalation: bool,
}

impl StatusInner {
    fn new() -> Self {
        StatusInner {
            operational: OperationalState::Idle,
            conditions: BTreeSet::new(),
            last_success: None,
            dirty: false,
            resume: None,
            idle_reached: false,
            credential_readable: false,
            escalation: false,
        }
    }
}

/// 2축 상태 단일 집계 지점(`StatusSink` + `ReadJudgment` 구현체).
#[derive(Debug)]
pub struct StatusService {
    inner: Mutex<StatusInner>,
}

impl Default for StatusService {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusService {
    /// 초기 상태(`operational = Idle`, 조건 없음)로 서비스를 생성한다.
    pub fn new() -> Self {
        StatusService {
            inner: Mutex::new(StatusInner::new()),
        }
    }

    /// 뮤텍스를 취득한다. poison 시 패닉 대신 내부 상태를 회수한다(회복력, U0 store.rs 정합).
    fn lock(&self) -> MutexGuard<'_, StatusInner> {
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// 연속실패 escalation 플래그를 세팅/해제한다(U6-내부 고유 입력, `CriticalErrorNotifier` 가 push).
    ///
    /// 활성 시 `health_check()` 가 `Unhealthy` 사유에 포함한다(R-STATUS-04, D6). `StatusSink`
    /// 트레이트 밖의 U6-내부 상태이며 두 U6 컴포넌트가 조립 루트(U8)에서 배선된다.
    pub fn set_failure_escalation(&self, active: bool) {
        self.lock().escalation = active;
    }

    /// 현재 2축 상태 + 부가 필드로부터 `StatusSnapshot` 을 파생한다(in-memory, infallible).
    ///
    /// `offline`/`consent` 는 push 신호로부터 결정론적으로 파생된다(R-STATUS-03).
    pub fn snapshot(&self) -> StatusSnapshot {
        let inner = self.lock();
        let conditions: Vec<ActiveCondition> = inner.conditions.iter().copied().collect();
        let offline = inner.operational == OperationalState::Offline;
        let consent = if inner.conditions.contains(&ActiveCondition::ConsentBlocked) {
            ConsentState::Blocked
        } else if inner.last_success.is_some() {
            ConsentState::Granted
        } else {
            ConsentState::Unknown
        };
        StatusSnapshot {
            operational: inner.operational,
            conditions,
            last_success: inner.last_success,
            dirty: inner.dirty,
            resume: inner.resume,
            consent,
            offline,
        }
    }
}

impl StatusSink for StatusService {
    fn set_operational(&self, state: OperationalState) {
        self.lock().operational = state;
    }

    fn raise_condition(&self, cond: ActiveCondition) {
        self.lock().conditions.insert(cond);
    }

    fn clear_condition(&self, cond: ActiveCondition) {
        self.lock().conditions.remove(&cond);
    }

    fn record_sync_success(&self, at: Timestamp) {
        self.lock().last_success = Some(at);
    }

    fn set_dirty(&self, dirty: bool) {
        self.lock().dirty = dirty;
    }

    fn set_resume_progress(&self, transferred: ByteCount, total: ByteCount) {
        self.lock().resume = Some((transferred, total));
    }

    fn set_liveness(&self, signal: LivenessSignal) {
        let mut inner = self.lock();
        match signal {
            LivenessSignal::IdleReached => inner.idle_reached = true,
            LivenessSignal::CredentialReadable => inner.credential_readable = true,
        }
    }
}

impl ReadJudgment for StatusService {
    fn health_check(&self) -> Health {
        let inner = self.lock();
        let mut reasons: Vec<HealthReason> = Vec::new();
        for cond in HEALTH_CRITICAL {
            if inner.conditions.contains(&cond) {
                reasons.push(HealthReason(format!("condition:{cond:?}")));
            }
        }
        if inner.escalation {
            reasons.push(HealthReason("consecutive_failure_escalation".to_string()));
        }
        if reasons.is_empty() {
            Health::Healthy
        } else {
            Health::Unhealthy { reasons }
        }
    }

    fn update_probe(&self) -> Liveness {
        // 격리(R-STATUS-05): 오직 두 liveness 신호에만 의존하며 축1/축2 를 읽지 않는다.
        let inner = self.lock();
        let mut missing: Vec<LivenessSignal> = Vec::new();
        if !inner.idle_reached {
            missing.push(LivenessSignal::IdleReached);
        }
        if !inner.credential_readable {
            missing.push(LivenessSignal::CredentialReadable);
        }
        if missing.is_empty() {
            Liveness::Alive
        } else {
            Liveness::NotReady { missing }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_flips_only_on_closed_four_set() {
        // 비목록 조건 단독은 Healthy 유지(알림 피로 회피).
        let svc = StatusService::new();
        svc.raise_condition(ActiveCondition::VaultUnavailable);
        svc.raise_condition(ActiveCondition::ConsentBlocked);
        svc.set_operational(OperationalState::Offline);
        svc.set_dirty(true);
        assert!(matches!(svc.health_check(), Health::Healthy));

        // health-critical 조건은 flip.
        svc.raise_condition(ActiveCondition::AuthFailed);
        assert!(matches!(svc.health_check(), Health::Unhealthy { .. }));
        svc.clear_condition(ActiveCondition::AuthFailed);
        assert!(matches!(svc.health_check(), Health::Healthy));

        // escalation 플래그(케이스2)도 flip.
        svc.set_failure_escalation(true);
        assert!(matches!(svc.health_check(), Health::Unhealthy { .. }));
    }

    #[test]
    fn update_probe_isolated_from_conditions() {
        let svc = StatusService::new();
        // 어떤 운영 조건도 probe 에 영향 없음.
        svc.raise_condition(ActiveCondition::AuthFailed);
        svc.raise_condition(ActiveCondition::OverLimit);
        assert!(matches!(svc.update_probe(), Liveness::NotReady { .. }));
        svc.set_liveness(LivenessSignal::IdleReached);
        svc.set_liveness(LivenessSignal::CredentialReadable);
        assert!(matches!(svc.update_probe(), Liveness::Alive));
    }

    #[test]
    fn snapshot_derives_offline_and_consent() {
        let svc = StatusService::new();
        assert_eq!(svc.snapshot().consent, ConsentState::Unknown);
        svc.record_sync_success(Timestamp::from_unix_nanos(1));
        assert_eq!(svc.snapshot().consent, ConsentState::Granted);
        svc.raise_condition(ActiveCondition::ConsentBlocked);
        assert_eq!(svc.snapshot().consent, ConsentState::Blocked);
        svc.set_operational(OperationalState::Offline);
        assert!(svc.snapshot().offline);
    }

    #[test]
    fn conditions_are_idempotent_set() {
        let svc = StatusService::new();
        svc.raise_condition(ActiveCondition::AuthFailed);
        svc.raise_condition(ActiveCondition::AuthFailed);
        svc.clear_condition(ActiveCondition::OverLimit); // 없는 것 제거 = 무변화
        assert_eq!(svc.snapshot().conditions, vec![ActiveCondition::AuthFailed]);
    }
}

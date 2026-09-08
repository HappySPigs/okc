//! CriticalErrorNotifier — 닫힌 4조건 3중 fan-out + 연속실패 escalation 상태머신.
//!
//! `CriticalEventSink`(U0)를 구현한다. 능동 표면화는 정확히 4개 진입점으로만 발생하며(R-CRIT-01,
//! 닫힌 집합), 각 표면화는 3중 fan-out 한다(R-CRIT-03): (1) 상향 심각도 구조화 로그(`Logger`),
//! (2) `StatusService` 반영(`raise_condition` 또는 escalation 플래그), (3) `TrayIndicator.notify`
//! (no-op). 앞 2개는 트레이 비의존 항상 성립.
//!
//! 케이스2(연속실패)는 `report_cycle_result(Failure)` 마다 카운트 +1, `Success` 시 0 리셋,
//! 임계 `N`(주입된 `notify_consecutive_failures`, 기본 3) 도달 순간 **1회** 발화 + escalation
//! 세팅(후속 실패 재발화 없음, R-CRIT-02).
//!
//! `Mutex` 를 보유하는 상태 모듈이므로 순수 lint-gate 는 적용하지 않는다.

use std::sync::{Arc, Mutex, MutexGuard};

use foundation::{
    ActiveCondition, CriticalEventSink, CycleOutcome, LimitReport, LogFields, LogLevel, Logger,
    RollbackReason, StatusSink, TransportError, Version,
};

use crate::status::StatusService;
use crate::tray::{NotificationMessage, TrayIndicator};

/// 연속실패 상태머신 내부 상태.
#[derive(Debug, Default)]
struct EscalationState {
    /// 연속 실패 카운트(Success 시 0 리셋).
    consecutive: u32,
    /// escalation 발화 여부(임계 도달 후 유지, Success 시 해제).
    escalated: bool,
}

/// 중대 이벤트 3중 fan-out 라우터(`CriticalEventSink` 구현체).
pub struct CriticalErrorNotifier {
    /// 상향 심각도 구조화 로그 대상(fan-out #1).
    logger: Arc<dyn Logger>,
    /// 상태 반영 대상 — `raise_condition` + escalation 플래그(fan-out #2).
    status: Arc<StatusService>,
    /// 트레이 알림 대상(fan-out #3, no-op).
    tray: Arc<TrayIndicator>,
    /// 연속실패 임계 `N`(= `notify_consecutive_failures`, U0 코어 config, 기본 3).
    threshold: u32,
    /// 연속실패 상태머신(카운트 + escalation 플래그).
    state: Mutex<EscalationState>,
}

impl CriticalErrorNotifier {
    /// 노티파이어를 생성한다. `threshold` 는 주입된 `notify_consecutive_failures`(>= 1).
    pub fn new(
        logger: Arc<dyn Logger>,
        status: Arc<StatusService>,
        tray: Arc<TrayIndicator>,
        threshold: u32,
    ) -> Self {
        CriticalErrorNotifier {
            logger,
            status,
            tray,
            threshold,
            state: Mutex::new(EscalationState::default()),
        }
    }

    /// 상태머신 뮤텍스를 취득한다. poison 시 패닉 대신 내부 상태를 회수한다.
    fn lock(&self) -> MutexGuard<'_, EscalationState> {
        match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// 트레이 알림을 시도한다(no-op 이면 무연산, 비의존 보증).
    fn notify_tray(&self, title: &str, body: String) {
        self.tray.notify(NotificationMessage {
            title: title.to_string(),
            body,
        });
    }
}

impl CriticalEventSink for CriticalErrorNotifier {
    fn report_auth_failure(&self, detail: TransportError) {
        self.logger.event(
            LogLevel::Error,
            "critical.auth_failure",
            None,
            LogFields(vec![("detail".to_string(), detail.detail.clone())]),
        );
        self.status.raise_condition(ActiveCondition::AuthFailed);
        self.notify_tray("인증 실패", detail.detail);
    }

    fn report_cycle_result(&self, outcome: CycleOutcome) {
        match outcome {
            CycleOutcome::Success => {
                let mut state = self.lock();
                state.consecutive = 0;
                state.escalated = false;
                drop(state);
                // escalation 해제 반영(R-CRIT-02, R-CRIT-04).
                self.status.set_failure_escalation(false);
            }
            CycleOutcome::Failure(error) => {
                let mut state = self.lock();
                state.consecutive = state.consecutive.saturating_add(1);
                let fire = state.consecutive >= self.threshold && !state.escalated;
                if fire {
                    state.escalated = true;
                }
                drop(state);
                if fire {
                    // 임계 도달 순간 1회 발화 + escalation 세팅.
                    self.logger.event(
                        LogLevel::Error,
                        "critical.consecutive_failures",
                        None,
                        LogFields(vec![
                            ("threshold".to_string(), self.threshold.to_string()),
                            ("detail".to_string(), error.detail),
                        ]),
                    );
                    self.status.set_failure_escalation(true);
                    self.notify_tray("연속 실패", "임계 도달".to_string());
                }
            }
        }
    }

    fn report_preflight_exceeded(&self, report: LimitReport) {
        self.logger.event(
            LogLevel::Warn,
            "critical.preflight_exceeded",
            None,
            LogFields(vec![("report".to_string(), report.0.clone())]),
        );
        self.status.raise_condition(ActiveCondition::OverLimit);
        self.notify_tray("한도 초과", report.0);
    }

    fn report_update_rollback(&self, from: Version, to: Version, reason: RollbackReason) {
        self.logger.event(
            LogLevel::Error,
            "critical.update_rollback",
            None,
            LogFields(vec![
                ("from".to_string(), from.0),
                ("to".to_string(), to.0.clone()),
                ("reason".to_string(), reason.0),
            ]),
        );
        self.status.raise_condition(ActiveCondition::UpdateRolledBack);
        self.notify_tray("업데이트 롤백", to.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use foundation::{ClassifiedError, ErrorClass, Health, ReadJudgment, TransportErrorClass};
    use std::sync::Mutex as StdMutex;

    /// 방출된 이벤트명을 기록하는 테스트 로거.
    #[derive(Default)]
    struct RecordingLogger {
        events: StdMutex<Vec<String>>,
    }
    impl Logger for RecordingLogger {
        fn log(&self, record: foundation::LogRecord) {
            self.events.lock().unwrap().push(record.event);
        }
        fn event(
            &self,
            _level: LogLevel,
            event: &str,
            _cycle_id: Option<foundation::CycleId>,
            _fields: LogFields,
        ) {
            self.events.lock().unwrap().push(event.to_string());
        }
    }

    fn failure() -> CycleOutcome {
        CycleOutcome::Failure(ClassifiedError {
            class: ErrorClass::Retryable,
            code: None,
            detail: "x".to_string(),
        })
    }

    fn setup(threshold: u32) -> (Arc<RecordingLogger>, Arc<StatusService>, CriticalErrorNotifier) {
        let logger = Arc::new(RecordingLogger::default());
        let status = Arc::new(StatusService::new());
        let notifier = CriticalErrorNotifier::new(
            logger.clone(),
            status.clone(),
            Arc::new(TrayIndicator::new()),
            threshold,
        );
        (logger, status, notifier)
    }

    #[test]
    fn auth_failure_fans_out_to_log_and_status() {
        let (logger, status, notifier) = setup(3);
        notifier.report_auth_failure(TransportError {
            class: TransportErrorClass::AuthFailed,
            http_status: Some(401),
            detail: "denied".to_string(),
        });
        assert!(logger.events.lock().unwrap().contains(&"critical.auth_failure".to_string()));
        assert!(status.snapshot().conditions.contains(&ActiveCondition::AuthFailed));
        assert!(matches!(status.health_check(), Health::Unhealthy { .. }));
    }

    #[test]
    fn escalation_fires_once_at_threshold_and_resets_on_success() {
        let (logger, status, notifier) = setup(3);
        notifier.report_cycle_result(failure());
        notifier.report_cycle_result(failure());
        assert!(matches!(status.health_check(), Health::Healthy)); // 아직 임계 미만
        notifier.report_cycle_result(failure()); // 임계 도달 -> 발화
        assert!(matches!(status.health_check(), Health::Unhealthy { .. }));
        notifier.report_cycle_result(failure()); // 후속 실패 재발화 없음
        let count = logger
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| *e == "critical.consecutive_failures")
            .count();
        assert_eq!(count, 1);
        notifier.report_cycle_result(CycleOutcome::Success); // 리셋
        assert!(matches!(status.health_check(), Health::Healthy));
    }
}

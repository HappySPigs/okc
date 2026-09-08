//! `AutoUpdater` — 채널 스테이징 -> 재기동 -> bounded-time 헬스 게이트 -> 자동 롤백 상태머신.
//!
//! apply 흐름: `hold` 검사(R-AU-04) -> `UpdateSource.stage`(fetch+verify, R-AU-05) ->
//! `ServiceManager.restart()`(위임, D-U7A-12) -> 주입 `Clock` 기반 `update_probe()` 폴링 게이트
//! (R-AU-01/02) -> 통과 시 `Committed`(last-good 승계) / 실패·타임아웃 시 `RolledBack`(last-good
//! 복원 + `report_update_rollback` 1회 + hold 마커, R-AU-03/04).
//!
//! 게이트는 **오직 `ReadJudgment::update_probe()`(순수 liveness)만** 소비하며 운영
//! `health_check()` 는 소비하지 않는다(D-U7A-04) — 일시 운영 조건(`AuthFailed`/`OverLimit`)이
//! 좋은 새 버전을 오판 롤백시키는 롤백 루프를 방지한다.
//!
//! 순수 오케스트레이션 모듈로서 panic-free-total 을 컴파일타임으로 강제한다.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::sync::Arc;
use std::time::Duration;

use foundation::{CriticalEventSink, Liveness, ReadJudgment, RollbackReason, Timestamp, Version};

use crate::clock::Clock;
use crate::service::ServiceManager;

/// 업데이트 채널 config(D-U7A-03). `Disabled` 이면 `check_for_update()` 는 항상 `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateChannel {
    /// 안정 채널.
    Stable,
    /// 베타 채널.
    Beta,
    /// 자동 업데이트 비활성(check 항상 `None`).
    Disabled,
}

/// `UpdateSource` seam 이 해석하는 불투명 아티팩트 참조.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRef(pub String);

/// 스테이징 무결성 검증용 체크섬(불투명).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checksum(pub String);

/// 업데이트 후보 — `check_for_update()` 결과 / `apply_update()` 입력.
///
/// **불변식**: `version` 은 현재 실행 버전보다 상위(단조). MVP 는 문자열 동등 비교만으로
/// no-op(현재와 동일 버전)을 판정하고, semver 정렬 비교는 이연한다(트림).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateInfo {
    /// 대상 버전(U0 타입 참조).
    pub version: Version,
    /// `UpdateSource` seam 이 해석하는 불투명 아티팩트 참조.
    pub artifact_ref: ArtifactRef,
    /// 스테이징 검증용 체크섬.
    pub checksum: Checksum,
}

/// last-good 단일 슬롯(D-U7A-02) + 롤백 루프 방지 hold 마커(D-U7A-06)를 지속하는 상태.
///
/// **불변식**: `hold_until` 이 미래이면 apply 는 `Refused`(no-op). `last_good` 는 항상 정상
/// 통과한 적 있는 버전.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackState {
    /// 롤백 복원 대상(단일 슬롯).
    pub last_good: Version,
    /// 세팅 시 이 시각까지 재-apply 거부.
    pub hold_until: Option<Timestamp>,
    /// 마지막으로 롤백된(거부된) 버전 — 재시도 억제 진단.
    pub last_rollback: Option<Version>,
}

/// apply 거부 사유. MVP 는 롤백 직후 hold 유효 구간 거부 한 가지(`Backoff`)뿐(지수 백오프 미도입).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldReason {
    /// 롤백 직후 `hold_until` 유효 구간 -> 재-apply 즉시 거부(R-AU-04, 롤백 루프 방지).
    Backoff,
}

/// apply 종료 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateOutcome {
    /// 게이트 통과 -> 새 버전 확정.
    Committed {
        /// 확정된 새 버전.
        version: Version,
    },
    /// 롤백 완료 — `from` = 시도 버전, `to` = 복원된 last-good, `reason` = 롤백 사유.
    RolledBack {
        /// 롤백을 유발한 시도(신규) 버전.
        from: Version,
        /// 복원된 last-good 버전.
        to: Version,
        /// 롤백 사유(GateTimeout/RestartFailed 서술).
        reason: RollbackReason,
    },
    /// hold 유효 구간 내 재-apply 거부(루프 방지, 스테이징/재기동 부수효과 없음).
    Refused {
        /// 거부 사유.
        reason: HoldReason,
    },
}

/// AutoUpdater 오류 taxonomy(U7a 소유, D-U7A-11).
///
/// **참고**: `RestartFailed`/`GateTimeout` 은 apply 흐름 내에서 롤백으로 귀결되어 최종 결과가
/// `UpdateOutcome::RolledBack` 이 되며, `Err` 로 반환되지 않는다. 순수 `Err(UpdateError)` 는
/// 재기동 이전 스테이징 실패(`Download`/`Verify`)에만 반환된다(상태 변경 없음, R-AU-05).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UpdateError {
    /// `UpdateSource` fetch 실패.
    #[error("업데이트 다운로드 실패")]
    Download,
    /// checksum/스테이징 검증 실패.
    #[error("업데이트 검증 실패")]
    Verify,
    /// `ServiceManager.restart()` 실패(내부 롤백 트리거로만 사용).
    #[error("재기동 실패")]
    RestartFailed,
    /// bounded-time 내 `update_probe` Alive 미도달(내부 롤백 트리거로만 사용).
    #[error("헬스 게이트 타임아웃")]
    GateTimeout,
    /// 하위 seam I/O 실패.
    #[error("업데이트 I/O 실패: {0}")]
    Io(String),
}

/// 업데이트 아티팩트 획득/검증 seam(D-U7A-03). 실 네트워크/서명검증은 이 seam 뒤로 이연된다.
pub trait UpdateSource: Send + Sync {
    /// 채널/현재 버전에 대해 업데이트 후보를 조회한다(없으면 `None`).
    fn check_for_update(
        &self,
        channel: UpdateChannel,
        current: &Version,
    ) -> Result<Option<UpdateInfo>, UpdateError>;

    /// 후보를 스테이징한다(fetch + checksum 검증). 실패 시 `Download`/`Verify`.
    fn stage(&self, info: &UpdateInfo) -> Result<(), UpdateError>;
}

/// bounded-time 헬스 게이트의 시간 파라미터(입력값; config 배선은 U8/NFR 이월, D-U7A-05).
///
/// **주의**: `t_poll` 은 `> 0` 이어야 결정적 fake clock 에서 게이트가 종료한다(진행 보장).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateConfig {
    /// 게이트 상한 시간 `T_gate`.
    pub t_gate: Duration,
    /// 폴링 간격 `T_poll`(`> 0` 필수).
    pub t_poll: Duration,
    /// 롤백 후 재-apply 거부 hold 기간 `T_hold`.
    pub t_hold: Duration,
}

impl Default for GateConfig {
    fn default() -> Self {
        // MVP 제안 기본치: 30s 상한, 1s 폴링, 5m hold(U8/NFR 에서 config 배선 가능).
        GateConfig {
            t_gate: Duration::from_secs(30),
            t_poll: Duration::from_secs(1),
            t_hold: Duration::from_secs(300),
        }
    }
}

/// 롤백 트리거 사유(내부) — 게이트 타임아웃 / 재기동 실패.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RollbackTrigger {
    /// bounded-time 내 Alive 미도달.
    GateTimeout,
    /// `ServiceManager.restart()` 실패.
    RestartFailed,
}

impl RollbackTrigger {
    fn reason(self) -> RollbackReason {
        let text = match self {
            RollbackTrigger::GateTimeout => "GateTimeout",
            RollbackTrigger::RestartFailed => "RestartFailed",
        };
        RollbackReason(text.to_string())
    }
}

/// 헬스 게이트 폴링 결과.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GateResult {
    /// bounded-time 내 `Alive` 관측.
    Passed,
    /// bounded-time 경과까지 `Alive` 미관측.
    TimedOut,
}

/// 채널 config 스테이징 업데이트 + 헬스 게이트 + 자동 롤백 오케스트레이터.
pub struct AutoUpdater {
    source: Box<dyn UpdateSource>,
    service: ServiceManager,
    probe: Arc<dyn ReadJudgment>,
    critical: Arc<dyn CriticalEventSink>,
    clock: Arc<dyn Clock>,
    channel: UpdateChannel,
    gate: GateConfig,
    current: Version,
    state: RollbackState,
}

impl AutoUpdater {
    /// 주입 의존으로 업데이터를 구성한다. 초기 `last_good` 은 현재 실행 버전으로 설정된다
    /// (기동 시점의 현재 버전은 정상 통과한 것으로 간주).
    // 의존성 주입 생성자 — 5개 seam/싱크 + 채널/게이트/현재버전 입력이 모두 필수라 인자가 많다
    // (조립 루트 U8 이 배선). 별도 config 구조체 도입은 MVP 범위 밖이므로 의도적으로 허용한다.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source: Box<dyn UpdateSource>,
        service: ServiceManager,
        probe: Arc<dyn ReadJudgment>,
        critical: Arc<dyn CriticalEventSink>,
        clock: Arc<dyn Clock>,
        channel: UpdateChannel,
        gate: GateConfig,
        current: Version,
    ) -> Self {
        let state = RollbackState {
            last_good: current.clone(),
            hold_until: None,
            last_rollback: None,
        };
        AutoUpdater {
            source,
            service,
            probe,
            critical,
            clock,
            channel,
            gate,
            current,
            state,
        }
    }

    /// 현재 롤백 상태 스냅샷(진단/테스트용).
    pub fn rollback_state(&self) -> &RollbackState {
        &self.state
    }

    /// 현재 실행 버전.
    pub fn current_version(&self) -> &Version {
        &self.current
    }

    /// 업데이트 후보를 조회한다. `Disabled` 채널은 항상 `None`, 현재와 동일 버전도 no-op(`None`).
    pub fn check_for_update(&self) -> Result<Option<UpdateInfo>, UpdateError> {
        if self.channel == UpdateChannel::Disabled {
            return Ok(None);
        }
        let candidate = self.source.check_for_update(self.channel, &self.current)?;
        Ok(candidate.filter(|info| info.version != self.current))
    }

    /// 업데이트를 적용한다(상태머신 1회). 종료 상태: `Committed`/`RolledBack`/`Refused`.
    ///
    /// 스테이징 실패(`Download`/`Verify`)만 `Err(UpdateError)` 로 조기 반환하며(재기동 이전 -> 상태
    /// 변경 없음). 재기동 실패/게이트 타임아웃은 롤백으로 귀결되어 `Ok(RolledBack)` 이 된다.
    pub fn apply_update(&mut self, info: UpdateInfo) -> Result<UpdateOutcome, UpdateError> {
        // (R-AU-04) hold 유효 구간이면 스테이징/재기동 없이 즉시 거부.
        if let Some(hold) = self.state.hold_until
            && self.clock.now() < hold {
                return Ok(UpdateOutcome::Refused {
                    reason: HoldReason::Backoff,
                });
            }

        // (R-AU-05) 스테이징(fetch+verify) 실패는 재기동 이전 -> 상태 변경 없이 조기 반환.
        self.source.stage(&info)?;

        let attempted = info.version.clone();

        // (D-U7A-12) 재기동은 ServiceManager 에 위임. 재기동 실패 = 즉시 롤백 트리거.
        if self.service.restart().is_err() {
            return Ok(self.rollback(attempted, RollbackTrigger::RestartFailed));
        }

        // (R-AU-01/02) bounded-time update_probe 폴링 게이트.
        match run_gate(
            self.probe.as_ref(),
            self.clock.as_ref(),
            self.gate.t_gate,
            self.gate.t_poll,
        ) {
            GateResult::Passed => Ok(self.commit(attempted)),
            GateResult::TimedOut => Ok(self.rollback(attempted, RollbackTrigger::GateTimeout)),
        }
    }

    /// (R-AU-05) 커밋 — 새 버전 확정 + 직전 버전을 last-good 으로 승계 + hold 해제.
    fn commit(&mut self, attempted: Version) -> UpdateOutcome {
        let previous = std::mem::replace(&mut self.current, attempted.clone());
        self.state.last_good = previous;
        self.state.hold_until = None;
        UpdateOutcome::Committed { version: attempted }
    }

    /// (R-AU-03/04) 롤백 — last-good 복원 + 통지 1회 + hold 마커 세팅.
    ///
    /// last-good 아티팩트 재활성(실 스왑)은 `UpdateSource` seam 뒤로 이연되었으므로 MVP 는
    /// best-effort 로 `restart()` 만 재호출하고 그 결과는 무시한다(롤백은 최후 경로라 완결이 우선).
    fn rollback(&mut self, attempted: Version, trigger: RollbackTrigger) -> UpdateOutcome {
        let to = self.state.last_good.clone();
        let _ = self.service.restart();

        let reason = trigger.reason();
        self.critical
            .report_update_rollback(attempted.clone(), to.clone(), reason.clone());

        let hold_nanos = self
            .clock
            .now()
            .as_unix_nanos()
            .saturating_add(dur_to_nanos(self.gate.t_hold));
        self.state.hold_until = Some(Timestamp::from_unix_nanos(hold_nanos));
        self.state.last_rollback = Some(attempted.clone());
        self.current = to.clone();

        UpdateOutcome::RolledBack {
            from: attempted,
            to,
            reason,
        }
    }
}

/// `Duration` 을 `i64` 나노초로 포화 변환한다(오버플로 시 `i64::MAX`).
fn dur_to_nanos(d: Duration) -> i64 {
    i64::try_from(d.as_nanos()).unwrap_or(i64::MAX)
}

/// (R-AU-02) bounded-time 게이트 폴링 루프 — `Alive` 관측 시 통과, 상한 경과 시 타임아웃.
///
/// `update_probe()` 만 소비하며(D-U7A-04) 운영 `health_check()` 는 호출하지 않는다. 매 `NotReady`
/// 관측 후 데드라인을 검사하고, 미도달이면 `T_poll` 만큼 대기한다(fake clock 은 이를 즉시 진행).
fn run_gate(
    probe: &dyn ReadJudgment,
    clock: &dyn Clock,
    t_gate: Duration,
    t_poll: Duration,
) -> GateResult {
    let start = clock.now().as_unix_nanos();
    let deadline = start.saturating_add(dur_to_nanos(t_gate));
    loop {
        match probe.update_probe() {
            Liveness::Alive => return GateResult::Passed,
            Liveness::NotReady { .. } => {
                if clock.now().as_unix_nanos() >= deadline {
                    return GateResult::TimedOut;
                }
                clock.sleep(t_poll);
            }
        }
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
    use crate::testing::{
        FakeClock, FakeController, FakeUpdateSource, RecordingCritical, ScriptedProbe,
    };

    fn v(s: &str) -> Version {
        Version(s.to_string())
    }

    fn info(version: &str) -> UpdateInfo {
        UpdateInfo {
            version: v(version),
            artifact_ref: ArtifactRef("ref".to_string()),
            checksum: Checksum("sum".to_string()),
        }
    }

    fn fast_gate() -> GateConfig {
        GateConfig {
            t_gate: Duration::from_secs(10),
            t_poll: Duration::from_secs(1),
            t_hold: Duration::from_secs(300),
        }
    }

    fn build(
        source: FakeUpdateSource,
        controller: FakeController,
        probe: ScriptedProbe,
        critical: Arc<RecordingCritical>,
        clock: Arc<FakeClock>,
    ) -> AutoUpdater {
        // 서비스는 설치+실행 상태로 시작해 restart 가 성공하도록 한다(별도 실패 주입은 controller 로).
        let mgr = ServiceManager::new(Box::new(controller));
        AutoUpdater::new(
            Box::new(source),
            mgr,
            Arc::new(probe),
            critical,
            clock,
            UpdateChannel::Stable,
            fast_gate(),
            v("1.0.0"),
        )
    }

    #[test]
    fn gate_pass_commits_and_succeeds_last_good() {
        let critical = Arc::new(RecordingCritical::default());
        let mut au = build(
            FakeUpdateSource::ok(),
            FakeController::running(1),
            ScriptedProbe::alive_now(),
            critical.clone(),
            Arc::new(FakeClock::new(0)),
        );
        let out = au.apply_update(info("2.0.0")).unwrap();
        assert_eq!(out, UpdateOutcome::Committed { version: v("2.0.0") });
        assert_eq!(au.current_version(), &v("2.0.0"));
        assert_eq!(au.rollback_state().last_good, v("1.0.0"));
        assert!(critical.rollback_calls().is_empty());
    }

    #[test]
    fn gate_timeout_rolls_back_and_notifies_once() {
        let critical = Arc::new(RecordingCritical::default());
        let mut au = build(
            FakeUpdateSource::ok(),
            FakeController::running(1),
            ScriptedProbe::never_alive(),
            critical.clone(),
            Arc::new(FakeClock::new(0)),
        );
        let out = au.apply_update(info("2.0.0")).unwrap();
        assert_eq!(
            out,
            UpdateOutcome::RolledBack {
                from: v("2.0.0"),
                to: v("1.0.0"),
                reason: RollbackReason("GateTimeout".to_string()),
            }
        );
        assert_eq!(critical.rollback_calls().len(), 1);
        assert_eq!(au.current_version(), &v("1.0.0"));
    }

    #[test]
    fn restart_failure_rolls_back() {
        let critical = Arc::new(RecordingCritical::default());
        let mut au = build(
            FakeUpdateSource::ok(),
            FakeController::running(1).failing_start(),
            ScriptedProbe::alive_now(),
            critical.clone(),
            Arc::new(FakeClock::new(0)),
        );
        let out = au.apply_update(info("2.0.0")).unwrap();
        match out {
            UpdateOutcome::RolledBack { reason, .. } => {
                assert_eq!(reason, RollbackReason("RestartFailed".to_string()));
            }
            other => panic!("예상: RolledBack, 실제: {other:?}"),
        }
        assert_eq!(critical.rollback_calls().len(), 1);
    }

    #[test]
    fn staging_failure_returns_error_no_state_change() {
        let critical = Arc::new(RecordingCritical::default());
        let mut au = build(
            FakeUpdateSource::verify_fails(),
            FakeController::running(1),
            ScriptedProbe::alive_now(),
            critical.clone(),
            Arc::new(FakeClock::new(0)),
        );
        assert_eq!(au.apply_update(info("2.0.0")), Err(UpdateError::Verify));
        assert_eq!(au.current_version(), &v("1.0.0"));
        assert!(au.rollback_state().hold_until.is_none());
        assert!(critical.rollback_calls().is_empty());
    }

    #[test]
    fn hold_after_rollback_refuses_reapply_without_staging() {
        let critical = Arc::new(RecordingCritical::default());
        let source = FakeUpdateSource::ok();
        let stage_handle = source.clone();
        let mut au = build(
            source,
            FakeController::running(1),
            ScriptedProbe::never_alive(),
            critical.clone(),
            Arc::new(FakeClock::new(0)),
        );
        // 1회차: 롤백 -> hold 세팅.
        let first = au.apply_update(info("2.0.0")).unwrap();
        assert!(matches!(first, UpdateOutcome::RolledBack { .. }));
        let staged_after_first = stage_handle.stage_count();
        // 2회차(hold 유효 구간): Refused, 추가 스테이징 없음.
        let second = au.apply_update(info("2.0.1")).unwrap();
        assert_eq!(
            second,
            UpdateOutcome::Refused {
                reason: HoldReason::Backoff
            }
        );
        assert_eq!(stage_handle.stage_count(), staged_after_first);
    }

    #[test]
    fn disabled_channel_never_finds_update() {
        let critical = Arc::new(RecordingCritical::default());
        let mgr = ServiceManager::new(Box::new(FakeController::running(1)));
        let au = AutoUpdater::new(
            Box::new(FakeUpdateSource::offers(info("9.9.9"))),
            mgr,
            Arc::new(ScriptedProbe::alive_now()),
            critical,
            Arc::new(FakeClock::new(0)),
            UpdateChannel::Disabled,
            fast_gate(),
            v("1.0.0"),
        );
        assert_eq!(au.check_for_update().unwrap(), None);
    }
}

//! 테스트 더블(fake seam 구현) — OS/프로세스/시계/파일시스템/업데이트-소스/관측 싱크를 특권·
//! 네트워크·실시간 없이 결정적으로 대체한다.
//!
//! 비-default `proptest-support` feature 또는 `test` 에서만 빌드되어 프로덕션 그래프에 유입되지
//! 않는다. 크레이트 내부 단위테스트와 하류 property 테스트(`tests/prop_deploy.rs`)가 공유한다.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use foundation::{
    CriticalEventSink, CycleOutcome, Health, LimitReport, Liveness, LivenessSignal, ReadJudgment,
    RollbackReason, Timestamp, TransportError, Version,
};

use crate::clock::Clock;
use crate::service::{ServiceController, ServiceError, ServiceRegistration, ServiceSpec};
use crate::uninstall::{FileSystem, FsError};
use crate::update::{ArtifactRef, Checksum, UpdateChannel, UpdateError, UpdateInfo, UpdateSource};

// ---------------------------------------------------------------------------
// FakeController — 인메모리 등록/실행 상태 + 실패 주입.
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct ControllerState {
    registered: bool,
    running: bool,
    pid: Option<u32>,
    autostart_enabled: bool,
    fail_start: bool,
    start_count: u32,
}

/// 인메모리 `ServiceController` — 임의 등록/실행 초기상태 + `start` 실패 주입 지원.
///
/// `Clone` 은 내부 상태(`Arc<Mutex>`)를 공유하므로, 매니저에 주입한 뒤에도 핸들로 상태를 관측할 수 있다.
#[derive(Debug, Clone)]
pub struct FakeController {
    state: Arc<Mutex<ControllerState>>,
}

impl FakeController {
    fn from_state(state: ControllerState) -> Self {
        FakeController {
            state: Arc::new(Mutex::new(state)),
        }
    }

    /// 미등록 초기상태.
    pub fn empty() -> Self {
        Self::from_state(ControllerState::default())
    }

    /// 등록 + 실행(주어진 PID) 초기상태.
    pub fn running(pid: u32) -> Self {
        Self::from_state(ControllerState {
            registered: true,
            running: true,
            pid: Some(pid),
            ..ControllerState::default()
        })
    }

    /// 등록되었으나 정지 상태.
    pub fn stopped() -> Self {
        Self::from_state(ControllerState {
            registered: true,
            ..ControllerState::default()
        })
    }

    /// 임의 초기상태(불변식 정규화: `running` 이면 `registered` 강제, `running` 아니면 `pid` 없음).
    pub fn custom(registered: bool, running: bool, pid: Option<u32>) -> Self {
        let registered = registered || running;
        let (running, pid) = if running {
            (true, Some(pid.unwrap_or(1000)))
        } else {
            (false, None)
        };
        Self::from_state(ControllerState {
            registered,
            running,
            pid,
            ..ControllerState::default()
        })
    }

    /// `start()` 가 항상 `PermissionDenied` 로 실패하도록 만든다(재기동 실패 주입).
    pub fn failing_start(self) -> Self {
        self.state.lock().unwrap().fail_start = true;
        self
    }

    /// 자동시작 활성화 여부(관측).
    pub fn autostart_enabled(&self) -> bool {
        self.state.lock().unwrap().autostart_enabled
    }

    /// 누적 `start` 호출 수(restart 관측 proxy).
    pub fn restart_count(&self) -> u32 {
        self.state.lock().unwrap().start_count
    }

    /// 등록 여부(관측).
    pub fn is_registered(&self) -> bool {
        self.state.lock().unwrap().registered
    }
}

impl ServiceController for FakeController {
    fn register(&self, _spec: &ServiceSpec) -> Result<(), ServiceError> {
        self.state.lock().unwrap().registered = true;
        Ok(())
    }

    fn deregister(&self) -> Result<(), ServiceError> {
        let mut s = self.state.lock().unwrap();
        s.registered = false;
        s.running = false;
        s.pid = None;
        Ok(())
    }

    fn enable_autostart(&self) -> Result<(), ServiceError> {
        self.state.lock().unwrap().autostart_enabled = true;
        Ok(())
    }

    fn start(&self) -> Result<(), ServiceError> {
        let mut s = self.state.lock().unwrap();
        if !s.registered {
            return Err(ServiceError::NotInstalled);
        }
        if s.fail_start {
            return Err(ServiceError::PermissionDenied);
        }
        s.running = true;
        s.pid = Some(s.pid.unwrap_or(1000));
        s.start_count = s.start_count.saturating_add(1);
        Ok(())
    }

    fn stop(&self) -> Result<(), ServiceError> {
        let mut s = self.state.lock().unwrap();
        if !s.registered {
            return Err(ServiceError::NotInstalled);
        }
        s.running = false;
        s.pid = None;
        Ok(())
    }

    fn query(&self) -> Result<ServiceRegistration, ServiceError> {
        let s = self.state.lock().unwrap();
        if !s.registered {
            return Err(ServiceError::NotInstalled);
        }
        // 불변식 정규화: running 이면 pid 존재, 아니면 pid 없음.
        let (running, pid) = if s.running {
            (true, Some(s.pid.unwrap_or(1000)))
        } else {
            (false, None)
        };
        Ok(ServiceRegistration {
            registered: true,
            running,
            pid,
        })
    }
}

// ---------------------------------------------------------------------------
// FakeClock — 인메모리 단조 시계(sleep = 시각 진행).
// ---------------------------------------------------------------------------

/// 결정적 인메모리 clock — `sleep(dur)` 은 실제 대기 없이 내부 시각을 `dur` 만큼 진행한다.
#[derive(Debug, Clone)]
pub struct FakeClock {
    nanos: Arc<Mutex<i64>>,
}

impl FakeClock {
    /// 주어진 epoch 나노초에서 시작하는 clock 을 만든다.
    pub fn new(start_nanos: i64) -> Self {
        FakeClock {
            nanos: Arc::new(Mutex::new(start_nanos)),
        }
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_unix_nanos(*self.nanos.lock().unwrap())
    }

    fn sleep(&self, dur: Duration) {
        let add = i64::try_from(dur.as_nanos()).unwrap_or(i64::MAX);
        let mut n = self.nanos.lock().unwrap();
        *n = n.saturating_add(add);
    }
}

// ---------------------------------------------------------------------------
// ScriptedProbe — 사전 지정 liveness 시퀀스를 반환하는 ReadJudgment.
// ---------------------------------------------------------------------------

/// 사전 지정된 `Liveness` 시퀀스를 순서대로 반환하고, 소진 후에는 `default` 를 반복하는 probe.
///
/// `health_check()` 는 게이트가 소비하지 않으므로 항상 `Healthy` 를 반환한다(D-U7A-04 격리 확인용).
#[derive(Debug, Clone)]
pub struct ScriptedProbe {
    seq: Arc<Mutex<std::collections::VecDeque<Liveness>>>,
    default: Liveness,
}

impl ScriptedProbe {
    /// 임의 시퀀스 + 소진 후 기본값으로 구성한다.
    pub fn new(seq: Vec<Liveness>, default: Liveness) -> Self {
        ScriptedProbe {
            seq: Arc::new(Mutex::new(seq.into_iter().collect())),
            default,
        }
    }

    /// 첫 폴링부터 `Alive`.
    pub fn alive_now() -> Self {
        Self::new(vec![Liveness::Alive], Liveness::Alive)
    }

    /// 항상 `NotReady`(영원히 게이트 미통과 -> 타임아웃).
    pub fn never_alive() -> Self {
        Self::new(Vec::new(), not_ready())
    }

    /// `n` 회 `NotReady` 후 `Alive`.
    pub fn alive_after(n: usize) -> Self {
        let mut seq = vec![not_ready(); n];
        seq.push(Liveness::Alive);
        Self::new(seq, Liveness::Alive)
    }
}

impl ReadJudgment for ScriptedProbe {
    fn health_check(&self) -> Health {
        Health::Healthy
    }

    fn update_probe(&self) -> Liveness {
        let mut q = self.seq.lock().unwrap();
        q.pop_front().unwrap_or_else(|| self.default.clone())
    }
}

/// `IdleReached` 미충족을 나타내는 표준 `NotReady` 값.
pub fn not_ready() -> Liveness {
    Liveness::NotReady {
        missing: vec![LivenessSignal::IdleReached],
    }
}

// ---------------------------------------------------------------------------
// RecordingCritical — report_update_rollback 호출을 기록하는 CriticalEventSink.
// ---------------------------------------------------------------------------

/// `report_update_rollback` 호출을 기록하는 `CriticalEventSink`(그 외 케이스는 no-op).
#[derive(Debug, Default)]
pub struct RecordingCritical {
    rollbacks: Mutex<Vec<(Version, Version, RollbackReason)>>,
}

impl RecordingCritical {
    /// 기록된 롤백 통지 호출 목록의 사본을 반환한다.
    pub fn rollback_calls(&self) -> Vec<(Version, Version, RollbackReason)> {
        self.rollbacks.lock().unwrap().clone()
    }
}

impl CriticalEventSink for RecordingCritical {
    fn report_auth_failure(&self, _detail: TransportError) {}
    fn report_cycle_result(&self, _outcome: CycleOutcome) {}
    fn report_preflight_exceeded(&self, _report: LimitReport) {}
    fn report_update_rollback(&self, from: Version, to: Version, reason: RollbackReason) {
        self.rollbacks.lock().unwrap().push((from, to, reason));
    }
}

// ---------------------------------------------------------------------------
// FakeUpdateSource — 스테이징 결과/후보 주입 + 스테이징 호출 카운트.
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct SourceInner {
    stage_error: Option<UpdateError>,
    candidate: Option<UpdateInfo>,
    stage_count: Mutex<usize>,
}

/// 주입된 스테이징 결과/후보를 돌려주고 스테이징 호출 수를 기록하는 `UpdateSource`.
#[derive(Debug, Clone)]
pub struct FakeUpdateSource {
    inner: Arc<SourceInner>,
}

impl FakeUpdateSource {
    fn build(stage_error: Option<UpdateError>, candidate: Option<UpdateInfo>) -> Self {
        FakeUpdateSource {
            inner: Arc::new(SourceInner {
                stage_error,
                candidate,
                stage_count: Mutex::new(0),
            }),
        }
    }

    /// 스테이징 성공 + 후보 없음.
    pub fn ok() -> Self {
        Self::build(None, None)
    }

    /// 스테이징이 항상 `Verify` 로 실패.
    pub fn verify_fails() -> Self {
        Self::build(Some(UpdateError::Verify), None)
    }

    /// 스테이징이 항상 `Download` 로 실패.
    pub fn download_fails() -> Self {
        Self::build(Some(UpdateError::Download), None)
    }

    /// `check_for_update` 가 주어진 후보를 제안(스테이징은 성공).
    pub fn offers(candidate: UpdateInfo) -> Self {
        Self::build(None, Some(candidate))
    }

    /// 누적 `stage` 호출 수.
    pub fn stage_count(&self) -> usize {
        *self.inner.stage_count.lock().unwrap()
    }
}

impl UpdateSource for FakeUpdateSource {
    fn check_for_update(
        &self,
        _channel: UpdateChannel,
        _current: &Version,
    ) -> Result<Option<UpdateInfo>, UpdateError> {
        Ok(self.inner.candidate.clone())
    }

    fn stage(&self, _info: &UpdateInfo) -> Result<(), UpdateError> {
        *self.inner.stage_count.lock().unwrap() += 1;
        match &self.inner.stage_error {
            Some(e) => Err(e.clone()),
            None => Ok(()),
        }
    }
}

/// 테스트 편의: 임의 버전 문자열로 `UpdateInfo` 를 만든다.
pub fn make_update_info(version: &str) -> UpdateInfo {
    UpdateInfo {
        version: Version(version.to_string()),
        artifact_ref: ArtifactRef("ref".to_string()),
        checksum: Checksum("sum".to_string()),
    }
}

// ---------------------------------------------------------------------------
// FakeFileSystem — 인메모리 존재 집합 + per-path 권한 실패 주입.
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct FsInner {
    existing: HashSet<PathBuf>,
    denied: HashSet<PathBuf>,
}

/// 인메모리 파일시스템 fake — 존재 집합을 관리하고 per-path 권한 실패를 주입한다.
///
/// `Clone` 은 내부 상태를 공유한다(주입 후 핸들로 관측).
#[derive(Debug, Clone, Default)]
pub struct FakeFileSystem {
    inner: Arc<Mutex<FsInner>>,
}

impl FakeFileSystem {
    /// 빈 파일시스템을 만든다.
    pub fn new() -> Self {
        FakeFileSystem::default()
    }

    /// 경로가 존재하도록 등록한다.
    pub fn add_existing(&self, path: &Path) {
        self.inner.lock().unwrap().existing.insert(path.to_path_buf());
    }

    /// 경로 삭제 시 `PermissionDenied` 를 반환하도록 주입한다.
    pub fn deny_permission(&self, path: &Path) {
        self.inner.lock().unwrap().denied.insert(path.to_path_buf());
    }

    /// 경로 존재 여부(관측).
    pub fn contains(&self, path: &Path) -> bool {
        self.inner.lock().unwrap().existing.contains(path)
    }

    /// 현재 존재하는 경로 수(관측).
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().existing.len()
    }

    /// 존재 집합이 비었는지 여부(관측).
    pub fn is_empty(&self) -> bool {
        self.inner.lock().unwrap().existing.is_empty()
    }
}

impl FileSystem for FakeFileSystem {
    fn remove(&self, path: &Path) -> Result<bool, FsError> {
        let mut inner = self.inner.lock().unwrap();
        if inner.denied.contains(path) {
            return Err(FsError::PermissionDenied);
        }
        Ok(inner.existing.remove(path))
    }
}

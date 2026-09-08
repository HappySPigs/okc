//! U8 코디네이터 property/예제 테스트용 in-memory fake seam + 기록 싱크.
//!
//! 스레드/네트워크/파일시스템 없이 사이클 분기·순서·표면화를 결정적으로 검증한다(PBT-01).
// 여러 테스트 바이너리가 이 지원 모듈을 각각 컴파일하므로(전부가 모든 fake 를 쓰지 않음)
// dead_code 는 의도된 것이다.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use auth_consent::ConsentDecision;
use change_detect::{Availability, GuardVerdict};
use foundation::{
    ActiveCondition, ByteCount, CriticalEventSink, CycleId, CycleOutcome, HistorySink, LimitReport,
    LivenessSignal, LogFields, LogLevel, LogRecord, Logger, Manifest, ManifestDigest, ManifestEntry,
    OperationalState, RelativePath, RollbackReason, Sha256Digest, StatusSink, Timestamp,
    TransportError, UploadHistoryRecord, Version,
};
use ops_control::{RunMode, RunState, StopMode};
use sync_state::{BackoffConfig, RetryBackoffController};
use upload_client::{CommitOutcome, CyclePhase, CycleReport, UploadError};

use watcher_bin::adapters::{
    AvailabilityGuard, ConsentPort, CoordinatorStore, CycleDriver, ManifestSource,
    ManifestSourceError, RunStatePort, StoreAdapterError,
};
use watcher_bin::coordinator::{CoordinatorPorts, CoordinatorSinks, SyncCycleCoordinator};

// ---------------------------------------------------------------------------
// 매니페스트 헬퍼
// ---------------------------------------------------------------------------

/// 0-엔트리 매니페스트(digest [0;32]).
pub fn empty_manifest() -> Manifest {
    Manifest {
        entries: Vec::new(),
        manifest_digest: ManifestDigest::from_bytes([0u8; 32]),
    }
}

/// 1-엔트리 매니페스트(digest [9;32]) — 빈 기준선 대비 non-empty diff 를 유발한다.
pub fn nonempty_manifest() -> Manifest {
    let entry = ManifestEntry {
        relative_path: RelativePath::normalize("a.txt").expect("valid path"),
        raw_sha256: Sha256Digest::from_bytes([1u8; 32]),
        size: ByteCount::new(3),
    };
    Manifest {
        entries: vec![entry],
        manifest_digest: ManifestDigest::from_bytes([9u8; 32]),
    }
}

// ---------------------------------------------------------------------------
// port fake
// ---------------------------------------------------------------------------

/// 고정 run-state 를 반환하는 fake `RunStatePort`.
pub struct FakeRunState {
    pub mode: RunMode,
    pub stop: Option<StopMode>,
}

impl RunStatePort for FakeRunState {
    fn current(&self) -> RunState {
        RunState {
            mode: self.mode,
            sync_requested: false,
            stop_requested: self.stop,
        }
    }
}

/// 고정 가용성/판정을 반환하는 fake `AvailabilityGuard`.
pub struct FakeGuard {
    pub availability: Availability,
    pub verdict: GuardVerdict,
}

impl AvailabilityGuard for FakeGuard {
    fn check_reachable(&self, _root: &Path) -> Availability {
        self.availability
    }

    fn guard_diff(
        &self,
        _new_manifest: &Manifest,
        _last_committed: Option<&Manifest>,
        _availability: Availability,
    ) -> GuardVerdict {
        self.verdict.clone()
    }
}

/// 고정 매니페스트를 반환하는 fake `ManifestSource`.
pub struct FakeManifestSource {
    pub manifest: Manifest,
}

impl ManifestSource for FakeManifestSource {
    fn rebuild(&self) -> Result<Manifest, ManifestSourceError> {
        Ok(self.manifest.clone())
    }
}

/// 고정 판정을 반환하는 fake `ConsentPort`.
pub struct FakeConsent {
    pub decision: ConsentDecision,
}

impl ConsentPort for FakeConsent {
    fn is_upload_permitted(&self) -> ConsentDecision {
        self.decision.clone()
    }
}

/// 호출 횟수를 기록하고 성공/실패를 반환하는 fake `CycleDriver`.
pub struct FakeDriver {
    pub calls: Arc<AtomicUsize>,
    pub succeed: bool,
}

impl CycleDriver for FakeDriver {
    fn execute_cycle_traced(&mut self, _manifest: &Manifest) -> CycleReport {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let outcome = if self.succeed {
            Ok(CommitOutcome {
                server_vault_content_id: None,
                committed: true,
            })
        } else {
            Err(UploadError::Aborted)
        };
        CycleReport {
            outcome,
            phases: vec![CyclePhase::Done],
        }
    }
}

/// 마지막 커밋을 반환하고 `mark_dirty` 호출을 기록하는 fake `CoordinatorStore`.
pub struct FakeStore {
    pub last: Option<Manifest>,
    pub dirty_calls: Arc<AtomicUsize>,
}

impl CoordinatorStore for FakeStore {
    fn last_committed_manifest(&self) -> Option<Manifest> {
        self.last.clone()
    }

    fn mark_dirty(&self) -> Result<(), StoreAdapterError> {
        self.dirty_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 기록 싱크
// ---------------------------------------------------------------------------

/// raise/clear/record_sync_success/operational 을 기록하는 `StatusSink`.
#[derive(Default)]
pub struct RecordingStatus {
    pub raised: Mutex<Vec<ActiveCondition>>,
    pub cleared: Mutex<Vec<ActiveCondition>>,
    pub sync_successes: AtomicUsize,
    pub operationals: Mutex<Vec<OperationalState>>,
}

impl RecordingStatus {
    pub fn new() -> Self {
        RecordingStatus {
            raised: Mutex::new(Vec::new()),
            cleared: Mutex::new(Vec::new()),
            sync_successes: AtomicUsize::new(0),
            operationals: Mutex::new(Vec::new()),
        }
    }

    pub fn raised(&self) -> Vec<ActiveCondition> {
        self.raised.lock().expect("lock").clone()
    }

    pub fn sync_success_count(&self) -> usize {
        self.sync_successes.load(Ordering::SeqCst)
    }
}

impl StatusSink for RecordingStatus {
    fn set_operational(&self, state: OperationalState) {
        self.operationals.lock().expect("lock").push(state);
    }

    fn raise_condition(&self, cond: ActiveCondition) {
        self.raised.lock().expect("lock").push(cond);
    }

    fn clear_condition(&self, cond: ActiveCondition) {
        self.cleared.lock().expect("lock").push(cond);
    }

    fn record_sync_success(&self, _at: Timestamp) {
        self.sync_successes.fetch_add(1, Ordering::SeqCst);
    }

    fn set_dirty(&self, _dirty: bool) {}

    fn set_resume_progress(&self, _transferred: ByteCount, _total: ByteCount) {}

    fn set_liveness(&self, _signal: LivenessSignal) {}
}

/// 방출된 이벤트명을 기록하는 `Logger`.
#[derive(Default)]
pub struct RecordingLogger {
    pub events: Mutex<Vec<String>>,
}

impl RecordingLogger {
    pub fn new() -> Self {
        RecordingLogger {
            events: Mutex::new(Vec::new()),
        }
    }

    pub fn events(&self) -> Vec<String> {
        self.events.lock().expect("lock").clone()
    }
}

impl Logger for RecordingLogger {
    fn log(&self, record: LogRecord) {
        self.events.lock().expect("lock").push(record.event);
    }

    fn event(&self, _level: LogLevel, event: &str, _cycle_id: Option<CycleId>, _fields: LogFields) {
        self.events.lock().expect("lock").push(event.to_string());
    }
}

/// append 횟수를 기록하는 `HistorySink`.
#[derive(Default)]
pub struct RecordingHistory {
    pub appends: AtomicUsize,
}

impl RecordingHistory {
    pub fn new() -> Self {
        RecordingHistory {
            appends: AtomicUsize::new(0),
        }
    }

    pub fn append_count(&self) -> usize {
        self.appends.load(Ordering::SeqCst)
    }
}

impl HistorySink for RecordingHistory {
    fn append(&self, _record: UploadHistoryRecord) {
        self.appends.fetch_add(1, Ordering::SeqCst);
    }
}

/// `report_cycle_result` 결과(성공=true)를 기록하는 `CriticalEventSink`.
#[derive(Default)]
pub struct RecordingCritical {
    pub results: Mutex<Vec<bool>>,
}

impl RecordingCritical {
    pub fn new() -> Self {
        RecordingCritical {
            results: Mutex::new(Vec::new()),
        }
    }

    pub fn results(&self) -> Vec<bool> {
        self.results.lock().expect("lock").clone()
    }
}

impl CriticalEventSink for RecordingCritical {
    fn report_auth_failure(&self, _detail: TransportError) {}

    fn report_cycle_result(&self, outcome: CycleOutcome) {
        self.results
            .lock()
            .expect("lock")
            .push(matches!(outcome, CycleOutcome::Success));
    }

    fn report_preflight_exceeded(&self, _report: LimitReport) {}

    fn report_update_rollback(&self, _from: Version, _to: Version, _reason: RollbackReason) {}
}

// ---------------------------------------------------------------------------
// 코디네이터 조립 하네스
// ---------------------------------------------------------------------------

/// 한 사이클 시나리오 입력.
pub struct Scenario {
    pub paused: bool,
    pub availability: Availability,
    pub verdict: GuardVerdict,
    pub last: Option<Manifest>,
    pub current: Manifest,
    pub consent: ConsentDecision,
    pub driver_succeeds: bool,
}

/// 조립 후 검사 가능한 기록 핸들 묶음.
pub struct Handles {
    pub status: Arc<RecordingStatus>,
    pub logger: Arc<RecordingLogger>,
    pub history: Arc<RecordingHistory>,
    pub critical: Arc<RecordingCritical>,
    pub driver_calls: Arc<AtomicUsize>,
    pub dirty_calls: Arc<AtomicUsize>,
}

/// 시나리오로 코디네이터를 조립하고 검사 핸들을 반환한다.
pub fn build(scenario: Scenario) -> (SyncCycleCoordinator, Handles) {
    let status = Arc::new(RecordingStatus::new());
    let logger = Arc::new(RecordingLogger::new());
    let history = Arc::new(RecordingHistory::new());
    let critical = Arc::new(RecordingCritical::new());
    let driver_calls = Arc::new(AtomicUsize::new(0));
    let dirty_calls = Arc::new(AtomicUsize::new(0));

    let ports = CoordinatorPorts {
        store: Arc::new(FakeStore {
            last: scenario.last,
            dirty_calls: dirty_calls.clone(),
        }),
        guard: Arc::new(FakeGuard {
            availability: scenario.availability,
            verdict: scenario.verdict,
        }),
        root: PathBuf::from("/okc-nonexistent-test-root"),
        manifest_source: Arc::new(FakeManifestSource {
            manifest: scenario.current,
        }),
        consent: Arc::new(FakeConsent {
            decision: scenario.consent,
        }),
        driver: Box::new(FakeDriver {
            calls: driver_calls.clone(),
            succeed: scenario.driver_succeeds,
        }),
        run_state: Arc::new(FakeRunState {
            mode: if scenario.paused {
                RunMode::Paused
            } else {
                RunMode::Running
            },
            stop: None,
        }),
    };
    let sinks = CoordinatorSinks {
        status: status.clone(),
        logger: logger.clone(),
        history: history.clone(),
        critical: critical.clone(),
    };
    let retry = RetryBackoffController::new(&BackoffConfig::default(), 3, 0);
    let coordinator =
        SyncCycleCoordinator::new(ports, sinks, retry, Arc::new(AtomicBool::new(false)));

    (
        coordinator,
        Handles {
            status,
            logger,
            history,
            critical,
            driver_calls,
            dirty_calls,
        },
    )
}

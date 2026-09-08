//! 트리거 합류 + 트리거당 단일 직렬 동기화 사이클(Q2=B, DEC-U8-02).
//!
//! `SyncCycleCoordinator` 는 세 트리거 소스(U2 `FilesystemWatcher` 디바운스 / U2
//! `ReconciliationScheduler` 백스톱 / U7b `ControlPlane` sync-now)를 단일 `mpsc` 스트림으로 받아
//! 한 번에 하나씩 처리한다 — 단일 소비자 == 암묵적 사이클 잠금. 각 사이클은 paused-skip ->
//! vault 가용성 -> 매니페스트 재계산 -> guard_diff -> diff -> consent 게이트 -> `UploadProtocolDriver`
//! 위임 -> 결과 push 순서로 진행한다(R-U8-06/07/08/09). preflight limits / no-op digest /
//! have-want / 전송 / 재검증 / commit 은 동결 드라이버 내부가 소유하므로 중복하지 않는다.
//!
//! 순수 판정 함수(`decide_cycle_gate`/`coalesce_to_latest`/`wiring_is_acyclic`)와 `WIRING_EDGES` 는
//! 스레드/네트워크/파일시스템 없이 결정적으로 property 검증된다(PROP-U8-03/04/05).
//!
//! 순수-이하 오케스트레이션 표면으로서 panic-free 를 컴파일타임으로 강제한다(어떤 경로에서도
//! `unwrap`/`expect`/`panic`/인덱싱을 쓰지 않는다).
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use auth_consent::{BlockReason, ConsentDecision};
use change_detect::{Availability, GuardVerdict, TriggerSignal};
use content_core::ManifestDiffer;
use foundation::{
    ActiveCondition, ByteCount, ClassifiedError, CriticalEventSink, CycleId, CycleOutcome,
    ErrorClass, HistorySink, LogFields, LogLevel, Logger, Manifest, ManifestDigest,
    OperationalState, StatusSink, Timestamp, UploadHistoryRecord,
};
use ops_control::RunMode;
use sync_state::RetryBackoffController;
use upload_client::{CommitOutcome, UploadError};

use crate::adapters::{
    AvailabilityGuard, ConsentPort, CoordinatorStore, CycleDriver, ManifestSource, RunStatePort,
};

/// 합류된 트리거 원인(U8 소유). 세 소스를 하나의 직렬 스트림으로 표현한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CycleTrigger {
    /// U2 `FilesystemWatcher` 디바운스 버스트(mpsc 재전송).
    Filesystem(TriggerSignal),
    /// U2 `ReconciliationScheduler` 시작/주기 tick.
    Reconcile(TriggerSignal),
    /// `ControlPlane` sync-now 엣지 이벤트(핸들러가 직접 push, run_state 폴링 아님).
    SyncNow,
}

/// graceful 종료 플래그 — 신호 핸들러가 set 하고 루프들이 관측한다(공유 `AtomicBool`).
#[derive(Debug, Clone, Default)]
pub struct ShutdownFlag(Arc<AtomicBool>);

impl ShutdownFlag {
    /// 미설정 상태의 종료 플래그를 생성한다.
    pub fn new() -> Self {
        ShutdownFlag(Arc::new(AtomicBool::new(false)))
    }

    /// 종료를 요청한다(신호 핸들러/제어면이 호출).
    pub fn set(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// 종료가 요청되었는지 관측한다.
    pub fn is_set(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// 사이클 진입 게이트 판정(순수). 신호 집합을 정확히 하나의 배타적 분기로 사상한다(PROP-U8-04).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CycleGate {
    /// run-state paused -> 사이클 skip.
    SkipPaused,
    /// vault 도달 불가 -> 보류.
    HoldVaultUnavailable,
    /// 파괴적-빈-커밋 -> 보류.
    HoldDestructiveEmpty,
    /// diff 비었음 -> no-op.
    NoOp,
    /// consent 차단 -> 업로드 차단(사유 동반).
    BlockConsent(BlockReason),
    /// 드라이버 실행 진행.
    Proceed,
}

/// 사이클 종결 사유(U8 내부 분류). 정확히 하나로 종결하며 분기는 배타적이다(PROP-U8-04).
#[derive(Debug)]
pub enum CoordinatorOutcome {
    /// run-state paused 로 사이클 skip(감시·상태 유지).
    SkippedPaused,
    /// vault 도달 불가로 보류.
    HeldVaultUnavailable,
    /// 파괴적-빈-커밋 방지 보류.
    HeldDestructiveEmpty,
    /// 변경 없음(idle).
    NoOp,
    /// consent 차단(사유 동반).
    BlockedConsent(BlockReason),
    /// 드라이버 커밋 성공.
    Uploaded(CommitOutcome),
    /// 드라이버 실패 — `Transport` 만 백오프 재예약 대상(그 외는 다음 트리거 재스냅샷).
    Failed(UploadError),
}

/// 배선 그래프의 방향 간선(consumer -> provider). 역엣지가 없어야 비순환이다(PROP-U8-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WiringEdge {
    /// 의존/보유하는 상위 컴포넌트.
    pub from: &'static str,
    /// 주입되는 하위 컴포넌트.
    pub to: &'static str,
}

/// U8 데몬 배선 그래프의 하향 주입 간선 집합(domain-entities §6). 전부 상위 -> 하위 방향이다.
pub const WIRING_EDGES: &[WiringEdge] = &[
    WiringEdge { from: "daemon", to: "coordinator" },
    WiringEdge { from: "daemon", to: "control_plane" },
    WiringEdge { from: "daemon", to: "watcher" },
    WiringEdge { from: "daemon", to: "scheduler" },
    WiringEdge { from: "coordinator", to: "store" },
    WiringEdge { from: "coordinator", to: "guard" },
    WiringEdge { from: "coordinator", to: "manifest_source" },
    WiringEdge { from: "coordinator", to: "consent" },
    WiringEdge { from: "coordinator", to: "driver" },
    WiringEdge { from: "coordinator", to: "run_state" },
    WiringEdge { from: "coordinator", to: "retry" },
    WiringEdge { from: "coordinator", to: "logger" },
    WiringEdge { from: "coordinator", to: "status" },
    WiringEdge { from: "coordinator", to: "history" },
    WiringEdge { from: "coordinator", to: "critical" },
    WiringEdge { from: "driver", to: "transport" },
    WiringEdge { from: "driver", to: "blob_source" },
    WiringEdge { from: "driver", to: "store" },
    WiringEdge { from: "driver", to: "status" },
    WiringEdge { from: "transport", to: "credential" },
    WiringEdge { from: "transport", to: "config" },
    WiringEdge { from: "credential", to: "config" },
    WiringEdge { from: "consent", to: "status" },
    WiringEdge { from: "run_state", to: "status" },
    WiringEdge { from: "critical", to: "logger" },
    WiringEdge { from: "critical", to: "status" },
    WiringEdge { from: "critical", to: "tray" },
    WiringEdge { from: "history", to: "logger" },
    WiringEdge { from: "logger", to: "config" },
    WiringEdge { from: "control_plane", to: "handlers" },
    WiringEdge { from: "handlers", to: "status" },
    WiringEdge { from: "handlers", to: "history" },
    WiringEdge { from: "handlers", to: "consent" },
    WiringEdge { from: "handlers", to: "config" },
    WiringEdge { from: "handlers", to: "run_state" },
    WiringEdge { from: "handlers", to: "logger" },
];

/// 코디네이터가 의존하는 하위 seam(port) 묶음 — 조립 편의 + 생성자 인자 수 축소.
pub struct CoordinatorPorts {
    /// 공유 상태 저장소(마지막 커밋 읽기 + dirty 표시).
    pub store: Arc<dyn CoordinatorStore>,
    /// vault 가용성 + 파괴적-빈-커밋 가드.
    pub guard: Arc<dyn AvailabilityGuard>,
    /// 볼트 루트 경로(가용성 사전 점검 대상).
    pub root: PathBuf,
    /// 현재 매니페스트 재계산 소스.
    pub manifest_source: Arc<dyn ManifestSource>,
    /// 업로드 동의 게이트.
    pub consent: Arc<dyn ConsentPort>,
    /// 업로드 프로토콜 드라이버.
    pub driver: Box<dyn CycleDriver>,
    /// run-state 읽기 전용 관측.
    pub run_state: Arc<dyn RunStatePort>,
}

/// 코디네이터가 사이클 결과를 push 하는 U6 싱크 묶음(U0 계약 트레이트로 주입).
pub struct CoordinatorSinks {
    /// 2축 상태 push.
    pub status: Arc<dyn StatusSink>,
    /// 구조화 로그 push.
    pub logger: Arc<dyn Logger>,
    /// 업로드 히스토리 append.
    pub history: Arc<dyn HistorySink>,
    /// 중대 이벤트(연속 실패 에스컬레이션) 보고.
    pub critical: Arc<dyn CriticalEventSink>,
}

/// 트리거당 단일 직렬 사이클을 구동하는 코디네이터.
pub struct SyncCycleCoordinator {
    store: Arc<dyn CoordinatorStore>,
    guard: Arc<dyn AvailabilityGuard>,
    root: PathBuf,
    manifest_source: Arc<dyn ManifestSource>,
    consent: Arc<dyn ConsentPort>,
    driver: Box<dyn CycleDriver>,
    run_state: Arc<dyn RunStatePort>,
    status: Arc<dyn StatusSink>,
    logger: Arc<dyn Logger>,
    history: Arc<dyn HistorySink>,
    critical: Arc<dyn CriticalEventSink>,
    retry: RetryBackoffController,
    cycle_in_progress: Arc<AtomicBool>,
    next_cycle_id: u64,
}

impl SyncCycleCoordinator {
    /// port/sink 묶음 + 재시도 컨트롤러 + busy 신호로 코디네이터를 구성한다.
    pub fn new(
        ports: CoordinatorPorts,
        sinks: CoordinatorSinks,
        retry: RetryBackoffController,
        cycle_in_progress: Arc<AtomicBool>,
    ) -> Self {
        SyncCycleCoordinator {
            store: ports.store,
            guard: ports.guard,
            root: ports.root,
            manifest_source: ports.manifest_source,
            consent: ports.consent,
            driver: ports.driver,
            run_state: ports.run_state,
            status: sinks.status,
            logger: sinks.logger,
            history: sinks.history,
            critical: sinks.critical,
            retry,
            cycle_in_progress,
            next_cycle_id: 0,
        }
    }

    /// 합류 채널에서 트리거를 하나씩 수신해 사이클을 직렬 실행한다(단일 소비자 == 사이클 잠금).
    ///
    /// 각 반복은 종료 플래그와 run-state stop 신호를 관측해 정지하며(R-U8-10), 폭주 트리거는
    /// drain-to-latest 로 합쳐 최신 하나만 사이클을 유발한다(DEC-U8-02).
    pub fn run_loop(&mut self, rx: Receiver<CycleTrigger>, shutdown: &ShutdownFlag) {
        loop {
            if shutdown.is_set() || self.stop_requested() {
                break;
            }
            match rx.recv_timeout(Duration::from_millis(250)) {
                Ok(mut trigger) => {
                    coalesce_to_latest(&rx, &mut trigger);
                    // 종료가 합류 대기 중 도착했으면 새 사이클을 시작하지 않는다.
                    if shutdown.is_set() {
                        break;
                    }
                    self.cycle_in_progress.store(true, Ordering::SeqCst);
                    let outcome = self.run_cycle(trigger);
                    self.cycle_in_progress.store(false, Ordering::SeqCst);
                    self.after_cycle(&outcome);
                }
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    /// 한 트리거에 대한 단일 사이클을 처음부터 끝까지 실행한다(R-U8-06..09).
    pub fn run_cycle(&mut self, trigger: CycleTrigger) -> CoordinatorOutcome {
        let cycle_id = self.allocate_cycle_id();
        self.logger.event(
            LogLevel::Info,
            "cycle.start",
            Some(cycle_id),
            LogFields(vec![("cause".to_string(), trigger_cause(&trigger))]),
        );

        // C2: run-state paused -> skip(감시·상태 유지).
        if matches!(self.run_state.current().mode, RunMode::Paused) {
            self.logger.event(
                LogLevel::Info,
                "cycle.skipped.paused",
                Some(cycle_id),
                LogFields::default(),
            );
            return CoordinatorOutcome::SkippedPaused;
        }

        // C3: vault 가용성 사전 점검.
        let availability = self.guard.check_reachable(&self.root);
        if availability != Availability::Reachable {
            self.raise_hold(cycle_id, "cycle.hold.vault_unavailable");
            return CoordinatorOutcome::HeldVaultUnavailable;
        }

        // C4: 매니페스트 재계산(폴더 = 진실 원천). scan/hash 실패는 vault-unavailable 로 흡수.
        let new_manifest = match self.manifest_source.rebuild() {
            Ok(manifest) => manifest,
            Err(_) => {
                self.raise_hold(cycle_id, "cycle.hold.rebuild_failed");
                return CoordinatorOutcome::HeldVaultUnavailable;
            }
        };

        // C5: 마지막 커밋 대비 파괴적-빈-커밋 가드.
        let last = self.store.last_committed_manifest();
        match self.guard.guard_diff(&new_manifest, last.as_ref(), availability) {
            GuardVerdict::HoldVaultUnavailable(_) => {
                self.raise_hold(cycle_id, "cycle.hold.vault_unavailable");
                return CoordinatorOutcome::HeldVaultUnavailable;
            }
            GuardVerdict::HoldDestructiveEmpty => {
                self.raise_hold(cycle_id, "cycle.hold.destructive_empty");
                return CoordinatorOutcome::HeldDestructiveEmpty;
            }
            GuardVerdict::Proceed => {}
        }

        // C6: diff. 빈 변경 -> no-op(드라이버 digest no-op 과 이중 방어).
        let baseline = last.unwrap_or_else(empty_manifest);
        let change = ManifestDiffer::diff(&baseline, &new_manifest);
        if change.is_empty() {
            self.status.set_operational(OperationalState::Idle);
            self.logger
                .event(LogLevel::Info, "cycle.noop", Some(cycle_id), LogFields::default());
            return CoordinatorOutcome::NoOp;
        }

        // C7: consent 게이트(드라이버 실행 직전). 차단 시 감시·상태 유지.
        if let ConsentDecision::Blocked(reason) = self.consent.is_upload_permitted() {
            self.logger.event(
                LogLevel::Warn,
                "cycle.blocked.consent",
                Some(cycle_id),
                LogFields(vec![("reason".to_string(), format!("{reason:?}"))]),
            );
            return CoordinatorOutcome::BlockedConsent(reason);
        }

        // C8: 커밋 전 유실 방지 dirty 표시.
        let _ = self.store.mark_dirty();

        // C9: 드라이버 위임(사이클 본체는 동결 U3 내부가 수행).
        self.status.set_operational(OperationalState::Syncing);
        let report = self.driver.execute_cycle_traced(&new_manifest);

        // C10: 결과 표면화(R-U8-09).
        match report.outcome {
            Ok(commit) => {
                let now = now_timestamp();
                self.status.record_sync_success(now);
                self.status.set_dirty(false);
                self.status.set_operational(OperationalState::Idle);
                self.history.append(UploadHistoryRecord {
                    timestamp: now,
                    bytes_transferred: ByteCount::new(0),
                    error_detail: None,
                });
                self.critical.report_cycle_result(CycleOutcome::Success);
                self.retry.on_success();
                self.logger.event(
                    LogLevel::Info,
                    "cycle.uploaded",
                    Some(cycle_id),
                    LogFields::default(),
                );
                CoordinatorOutcome::Uploaded(commit)
            }
            Err(error) => {
                let now = now_timestamp();
                self.status.set_operational(OperationalState::Offline);
                self.history.append(UploadHistoryRecord {
                    timestamp: now,
                    bytes_transferred: ByteCount::new(0),
                    error_detail: Some(error.to_string()),
                });
                self.critical
                    .report_cycle_result(CycleOutcome::Failure(classify_upload_error(&error)));
                self.logger.event(
                    LogLevel::Error,
                    "cycle.failed",
                    Some(cycle_id),
                    LogFields(vec![("error".to_string(), error.to_string())]),
                );
                CoordinatorOutcome::Failed(error)
            }
        }
    }

    /// 실패 결과에 대한 변형별 백오프 재예약(FIX4, R-U8-09). `Transport` 만 재시도 분류·백오프한다.
    fn after_cycle(&mut self, outcome: &CoordinatorOutcome) {
        if let CoordinatorOutcome::Failed(UploadError::Transport(transport_error)) = outcome {
            let decision = self.retry.on_failure(transport_error, Instant::now());
            if let Some(delay) = decision.retry_after {
                // 다음 재시도 시각까지 전체 사이클을 백오프한다(재예약 사이 편집은 다음 재스냅샷 흡수).
                std::thread::sleep(delay);
            }
        }
        // HashMismatch/OverLimit/Aborted 및 AuthFailed 클래스는 백오프 없이 종료(다음 트리거 재스냅샷).
    }

    /// run-state stop 신호 관측(휘발성). set 되어 있으면 graceful 종료 대상.
    fn stop_requested(&self) -> bool {
        self.run_state.current().stop_requested.is_some()
    }

    /// 다음 사이클 상관관계 id 를 발급한다.
    fn allocate_cycle_id(&mut self) -> CycleId {
        let id = self.next_cycle_id;
        self.next_cycle_id = self.next_cycle_id.wrapping_add(1);
        CycleId(id)
    }

    /// vault-unavailable 계열 보류를 status 조건 + 로그로 1회 push 한다(U2 는 판정만, push 는 U8).
    fn raise_hold(&self, cycle_id: CycleId, event: &str) {
        self.status.raise_condition(ActiveCondition::VaultUnavailable);
        self.logger
            .event(LogLevel::Warn, event, Some(cycle_id), LogFields::default());
    }
}

/// paused/availability/verdict/diff/consent 신호를 정확히 하나의 배타적 게이트로 사상한다(순수).
///
/// 실제 사이클은 이 순서로 단락 평가하지만, 이 함수는 모든 신호가 주어졌을 때의 총-함수 분기를
/// 규정해 배타성·완전성을 property 로 검증하게 한다(PROP-U8-04).
pub fn decide_cycle_gate(
    paused: bool,
    availability: Availability,
    verdict: &GuardVerdict,
    change_is_empty: bool,
    consent: &ConsentDecision,
) -> CycleGate {
    if paused {
        return CycleGate::SkipPaused;
    }
    if availability != Availability::Reachable {
        return CycleGate::HoldVaultUnavailable;
    }
    match verdict {
        GuardVerdict::HoldVaultUnavailable(_) => return CycleGate::HoldVaultUnavailable,
        GuardVerdict::HoldDestructiveEmpty => return CycleGate::HoldDestructiveEmpty,
        GuardVerdict::Proceed => {}
    }
    if change_is_empty {
        return CycleGate::NoOp;
    }
    match consent {
        ConsentDecision::Blocked(reason) => CycleGate::BlockConsent(*reason),
        ConsentDecision::Permitted => CycleGate::Proceed,
    }
}

/// 대기 중인 트리거를 모두 drain 해 최신 하나로 합친다(폭주 흡수, DEC-U8-02, PROP-U8-05).
pub fn coalesce_to_latest(rx: &Receiver<CycleTrigger>, current: &mut CycleTrigger) {
    while let Ok(next) = rx.try_recv() {
        *current = next;
    }
}

/// 배선 간선 집합에 사이클(역엣지)이 없는지 검사한다(DFS, PROP-U8-03).
pub fn wiring_is_acyclic(edges: &[WiringEdge]) -> bool {
    use std::collections::{HashMap, HashSet};

    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut nodes: Vec<&str> = Vec::new();
    for edge in edges {
        adjacency.entry(edge.from).or_default().push(edge.to);
        nodes.push(edge.from);
        nodes.push(edge.to);
    }

    let mut visited: HashSet<&str> = HashSet::new();
    let mut in_stack: HashSet<&str> = HashSet::new();
    for node in nodes {
        if !visited.contains(node) && has_cycle_from(node, &adjacency, &mut visited, &mut in_stack) {
            return false;
        }
    }
    true
}

/// `node` 에서 시작하는 DFS 로 back-edge(사이클)를 탐지한다.
fn has_cycle_from<'a>(
    node: &'a str,
    adjacency: &std::collections::HashMap<&'a str, Vec<&'a str>>,
    visited: &mut std::collections::HashSet<&'a str>,
    in_stack: &mut std::collections::HashSet<&'a str>,
) -> bool {
    visited.insert(node);
    in_stack.insert(node);
    if let Some(neighbors) = adjacency.get(node) {
        for next in neighbors {
            if in_stack.contains(next) {
                return true;
            }
            if !visited.contains(next) && has_cycle_from(next, adjacency, visited, in_stack) {
                return true;
            }
        }
    }
    in_stack.remove(node);
    false
}

/// 트리거 원인 요약을 로그 필드용 문자열로 추출한다(US-E1-01 진단).
fn trigger_cause(trigger: &CycleTrigger) -> String {
    match trigger {
        CycleTrigger::Filesystem(signal) | CycleTrigger::Reconcile(signal) => {
            signal.cause_summary.clone()
        }
        CycleTrigger::SyncNow => "sync-now (operator)".to_string(),
    }
}

/// 마지막 커밋이 없을 때 diff 기준선으로 쓰는 빈 매니페스트.
fn empty_manifest() -> Manifest {
    Manifest {
        entries: Vec::new(),
        manifest_digest: ManifestDigest::from_bytes([0u8; 32]),
    }
}

/// `UploadError` 를 U6 히스토리/critical 이 소비하는 `ClassifiedError` 로 사상한다.
fn classify_upload_error(error: &UploadError) -> ClassifiedError {
    let (class, detail) = match error {
        UploadError::Transport(transport_error) => {
            (transport_error.class.to_error_class(), transport_error.detail.clone())
        }
        UploadError::HashMismatch { .. } => (ErrorClass::Retryable, error.to_string()),
        UploadError::OverLimit(_) => (ErrorClass::Fatal, error.to_string()),
        UploadError::Aborted => (ErrorClass::Fatal, error.to_string()),
    };
    ClassifiedError {
        class,
        code: None,
        detail,
    }
}

/// 현재 벽시계 시각을 U0 `Timestamp` 로 반환한다(포화 처리, panic 없음).
fn now_timestamp() -> Timestamp {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_nanos()).unwrap_or(i64::MAX))
        .unwrap_or(0);
    Timestamp::from_unix_nanos(nanos)
}

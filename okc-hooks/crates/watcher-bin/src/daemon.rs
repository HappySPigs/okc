//! 전체 컴포넌트 조립 + 트리거 스레드 배선 + graceful 종료 수명주기(WatcherDaemon).
//!
//! 기동 순서는 R-U8-05 로 고정된다: config 로드(+federated 해소) -> `SingleInstanceLock::acquire`
//! -> `SyncStateStore::open_and_recover` -> U6 싱크 구축(먼저) -> 하위 단위 생성·하향 주입 ->
//! `ControlPlane` 배선 -> `SyncCycleCoordinator` 조립 -> 시작 스캔 트리거 enqueue ->
//! `FilesystemWatcher::start` + 트리거 소스 스레드 배선 -> 소비자 루프(현 스레드) -> graceful 종료.
//! 모든 주입은 하위 -> 상위 방향이며 역엣지가 없다(비순환, PROP-U8-03).
//!
//! 스레드/소켓/파일시스템 I/O 를 수행하는 조립 모듈이므로 순수 lint-gate 는 두지 않는다(단, 어떤
//! 경로에서도 `unwrap`/`expect`/`panic` 을 쓰지 않는다).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use auth_consent::{AuthTransport, ConfigSource, ConsentGate, CredentialProvider, UreqAdapter};
use change_detect::{
    FilesystemWatcher, NotifyBackend, ReconciliationScheduler, TriggerSignal,
    VaultAvailabilityGuard,
};
use foundation::{CriticalEventSink, HistorySink, LogFields, LogLevel, Logger, StatusSink};
use observability::{
    Clock, CriticalErrorNotifier, StatusService, StructuredLogger, SystemClock, TrayIndicator,
    UploadHistoryStore,
};
use ops_control::{ControlPlane, RunStateController, WatcherHandlers, bind_native};
use sync_state::{RetryBackoffController, StateError, SyncStateStore};
use upload_client::UploadProtocolDriver;

use crate::adapters::{
    AvailabilityGuard, ConsentPort, CoordinatorStore, CycleDriver, ManifestSource, RunStatePort,
    SharedSyncStore, VaultBlobSource, VaultManifestSource,
};
use crate::config::{
    FederatedConfig, LoadedRuntimeConfig, RuntimeConfigError, load_runtime_config,
};
use crate::control::{DaemonControlHandlers, TriggerSender};
use crate::coordinator::{
    CoordinatorPorts, CoordinatorSinks, CycleTrigger, ShutdownFlag, SyncCycleCoordinator,
};
use crate::instance_lock::{LockConfig, LockError, SingleInstanceLock};
use crate::target_binding::{ensure_target_binding, target_fingerprint};

/// 트리거 소스 스레드가 종료 플래그를 관측하는 폴링 간격.
const THREAD_POLL: Duration = Duration::from_millis(250);

/// 데몬 기동/조립/종료 실패 taxonomy.
#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    /// 지속 sync 상태가 다른 vault/업로드 목적지에 속함.
    #[error("sync target binding failed: {0}")]
    TargetBinding(String),
    /// config 로드/federated 해소 실패(lock 이전 즉시 종료).
    #[error(transparent)]
    Config(#[from] RuntimeConfigError),
    /// 단일 인스턴스 획득 실패(live holder 보고 후 비정상 종료).
    #[error(transparent)]
    Lock(#[from] LockError),
    /// 상태 저장소 복구 실패.
    #[error("상태 저장소 복구 실패: {0}")]
    State(String),
    /// 파일시스템 감시 시작 실패(볼트 루트 부재 등).
    #[error("파일시스템 감시 시작 실패: {0}")]
    Watch(String),
}

/// 조립 루트 — 전 컴포넌트를 생성·하향 주입하고 직렬 사이클을 구동하는 데몬.
pub struct WatcherDaemon;

impl WatcherDaemon {
    /// 데몬을 전경 실행한다(S1~S9). 소비자 루프가 종료될 때까지 블로킹한다.
    pub fn run(config_path: Option<PathBuf>) -> Result<(), DaemonError> {
        // S1: config 로드 + federated 해소(무효 config -> lock 이전 즉시 종료).
        let LoadedRuntimeConfig {
            provider,
            core,
            federated,
            ..
        } = load_runtime_config(config_path.as_deref())?;

        // 종료 플래그 + 신호 핸들러(SIGINT/SIGTERM -> graceful shutdown).
        let shutdown = ShutdownFlag::new();
        install_signal_handler(&shutdown);

        // S2: 단일 인스턴스 락(live holder -> AlreadyRunning 반환).
        let lock = SingleInstanceLock::acquire(&LockConfig {
            path: federated.lock_file.clone(),
        })?;

        // S3: 상태 저장소 crash-atomic 복구.
        let recovered_store = open_state(&federated)?;
        let target = target_fingerprint(&core)
            .map_err(|error| DaemonError::TargetBinding(error.to_string()))?;
        let binding_path = federated
            .state
            .state_path
            .as_ref()
            .map(|path| path.with_extension("target"))
            .unwrap_or_else(|| federated.data_dir.join("sync-state.target"));
        ensure_target_binding(
            &binding_path,
            &target,
            recovered_store.last_committed_manifest().is_some(),
        )
        .map_err(|error| DaemonError::TargetBinding(error.to_string()))?;
        let store = Arc::new(Mutex::new(recovered_store));

        // S4: U6 싱크(먼저 구축해 U0 트레이트 핸들 확보).
        let status = Arc::new(StatusService::new());
        let clock: Arc<dyn Clock> = Arc::new(SystemClock);
        let logger = Arc::new(StructuredLogger::new(
            federated.logger_config(&core),
            clock,
            provider.clone(),
        ));
        let logger_sink: Arc<dyn Logger> = logger.clone();
        let status_sink: Arc<dyn StatusSink> = status.clone();
        let history = Arc::new(UploadHistoryStore::new(
            federated.observability.history_file.clone(),
            Some(logger_sink.clone()),
        ));
        let tray = Arc::new(TrayIndicator::new());
        let critical = Arc::new(CriticalErrorNotifier::new(
            logger.clone(),
            status.clone(),
            tray,
            core.notify_consecutive_failures,
        ));

        // S5: 하위 단위 생성 + 하향 주입(U0 트레이트로 주입, no back-reference).
        let vault_root = PathBuf::from(core.vault_path.clone());
        let config_source: Arc<dyn ConfigSource> = provider.clone();
        let credential = Arc::new(CredentialProvider::with_defaults(config_source.clone()));
        let transport = AuthTransport::new(
            credential,
            Arc::new(UreqAdapter::new()),
            config_source,
            federated.request_timeout,
        );
        let consent = Arc::new(ConsentGate::open_with_system_clock(
            federated.consent_file.clone(),
            Some(status_sink.clone()),
        ));
        let guard_port: Arc<dyn AvailabilityGuard> =
            Arc::new(VaultAvailabilityGuard::new(federated.confirm_empty));
        let manifest_source: Arc<dyn ManifestSource> =
            Arc::new(VaultManifestSource::new(vault_root.clone()));
        let driver = UploadProtocolDriver::new(
            transport,
            Box::new(VaultBlobSource::new(vault_root.clone())),
            Box::new(SharedSyncStore::new(store.clone())),
            status_sink.clone(),
            federated.chunk_threshold_bytes,
            federated.chunk_size_bytes,
        );
        let run_state = Arc::new(RunStateController::new(
            federated.run_state_file.clone(),
            status_sink.clone(),
        ));
        let retry = RetryBackoffController::new(
            &federated.backoff,
            core.notify_consecutive_failures,
            jitter_seed(),
        );

        // S6: ControlPlane 배선 — U8 고유 핸들러 주입, 전용 스레드에서 serve.
        let (trigger_tx, trigger_rx) = mpsc::channel::<CycleTrigger>();
        let trigger_sender = Arc::new(TriggerSender::new(trigger_tx.clone()));
        let inner_handlers = WatcherHandlers::new(
            status.clone(),
            history.clone(),
            consent.clone(),
            provider.clone(),
            run_state.clone(),
        );
        let handlers = Arc::new(DaemonControlHandlers::new(
            inner_handlers,
            trigger_sender,
            logger.clone(),
            provider.clone(),
            target,
        ));
        // 리스너는 `Box<dyn IpcListener>`(non-Send)이라 스레드로 이동할 수 없으므로 전용 스레드
        // 안에서 bind + serve 한다. 바인딩 실패는 데몬을 중단시키지 않고(제어면 부재로 계속) 로그로
        // 남긴다(RESILIENCY: 제어 소켓 부재가 동기화를 막지 않음).
        let control_endpoint = federated.ipc_endpoint.clone();
        let control_logger = logger.clone();
        let _control_handle = thread::spawn(move || match bind_native(&control_endpoint) {
            Ok(listener) => ControlPlane::new(handlers).serve_forever(listener.as_ref()),
            Err(error) => control_logger.event(
                LogLevel::Error,
                "control_plane.bind_failed",
                None,
                LogFields(vec![("error".to_string(), error.to_string())]),
            ),
        });

        // S7: coordinator 조립.
        let cycle_in_progress = Arc::new(AtomicBool::new(false));
        let coord_store: Arc<dyn CoordinatorStore> = Arc::new(SharedSyncStore::new(store.clone()));
        let consent_port: Arc<dyn ConsentPort> = consent.clone();
        let run_state_port: Arc<dyn RunStatePort> = run_state.clone();
        let driver_box: Box<dyn CycleDriver> = Box::new(driver);
        let history_sink: Arc<dyn HistorySink> = history.clone();
        let critical_sink: Arc<dyn CriticalEventSink> = critical;
        let ports = CoordinatorPorts {
            store: coord_store,
            guard: guard_port,
            root: vault_root.clone(),
            manifest_source,
            consent: consent_port,
            driver: driver_box,
            run_state: run_state_port,
        };
        let sinks = CoordinatorSinks {
            status: status_sink,
            logger: logger_sink,
            history: history_sink,
            critical: critical_sink,
        };
        let mut coordinator =
            SyncCycleCoordinator::new(ports, sinks, retry, cycle_in_progress.clone());

        // 시작 스캔 트리거를 라이브 감시 진입 전에 enqueue(DEC-U8-16, NFR-03 백스톱).
        let mut scheduler = ReconciliationScheduler::new(federated.reconciliation_interval);
        let startup_signal = scheduler.run_startup_scan(Instant::now());
        let _ = trigger_tx.send(CycleTrigger::Reconcile(startup_signal));

        // S8: 트리거 소스 배선(디바운스 forwarder + 주기 timer).
        let (watcher, fs_rx) =
            FilesystemWatcher::start(&vault_root, &NotifyBackend, federated.debounce)
                .map_err(|err| DaemonError::Watch(err.to_string()))?;
        let fs_handle = spawn_forwarder(fs_rx, trigger_tx.clone(), shutdown.clone());
        let timer_handle = spawn_timer(scheduler, trigger_tx, cycle_in_progress, shutdown.clone());

        // 소비자 루프(현 스레드) — 단일 소비자 == 사이클 직렬화.
        coordinator.run_loop(trigger_rx, &shutdown);

        // S9: graceful 종료 — 감시 정지 -> 트리거 스레드 종료 -> 락 해제.
        shutdown.set();
        drop(watcher);
        let _ = fs_handle.join();
        let _ = timer_handle.join();
        drop(lock);
        Ok(())
    }
}

/// SIGINT/SIGTERM 핸들러를 등록해 종료 플래그를 set 한다(best-effort — 중복 등록 실패는 무시).
fn install_signal_handler(shutdown: &ShutdownFlag) {
    let handler_flag = shutdown.clone();
    let _ = ctrlc::set_handler(move || {
        handler_flag.set();
    });
}

/// 상태 저장소를 복구한다. 손상 복구 신호(`CorruptRecovered`)는 빈 상태로 재개한다(재조정 백스톱).
fn open_state(federated: &FederatedConfig) -> Result<SyncStateStore, DaemonError> {
    match SyncStateStore::open_and_recover(&federated.state) {
        Ok(store) => Ok(store),
        Err(StateError::CorruptRecovered { .. }) => {
            // 손상 파일은 이미 빈 상태 + dirty 로 재기록됐으므로 재-open 은 성공한다.
            SyncStateStore::open_and_recover(&federated.state)
                .map_err(|err| DaemonError::State(err.to_string()))
        }
        Err(err) => Err(DaemonError::State(err.to_string())),
    }
}

/// 디바운스 트리거 forwarder 스레드 — U2 `TriggerStream` 을 합류 채널로 재전송한다.
fn spawn_forwarder(
    fs_rx: Receiver<TriggerSignal>,
    tx: Sender<CycleTrigger>,
    shutdown: ShutdownFlag,
) -> JoinHandle<()> {
    thread::spawn(move || {
        while let Ok(signal) = fs_rx.recv() {
            if shutdown.is_set() {
                break;
            }
            if tx.send(CycleTrigger::Filesystem(signal)).is_err() {
                break;
            }
        }
    })
}

/// 주기 재조정 timer 스레드 — busy 신호를 전달해 중복 억제하며 tick 트리거를 발행한다.
fn spawn_timer(
    mut scheduler: ReconciliationScheduler,
    tx: Sender<CycleTrigger>,
    cycle_in_progress: Arc<AtomicBool>,
    shutdown: ShutdownFlag,
) -> JoinHandle<()> {
    thread::spawn(move || {
        loop {
            if shutdown.is_set() {
                break;
            }
            thread::sleep(THREAD_POLL);
            let busy = cycle_in_progress.load(Ordering::SeqCst);
            if let Some(signal) = scheduler.tick(Instant::now(), busy)
                && tx.send(CycleTrigger::Reconcile(signal)).is_err()
            {
                break;
            }
        }
    })
}

/// full jitter 시드 — 현재 시각 나노초에서 파생한다(암호학적 품질 불필요).
fn jitter_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0)
}

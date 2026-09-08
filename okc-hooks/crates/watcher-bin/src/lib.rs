#![deny(missing_docs)]
//! `watcher-bin` 크레이트 (U8) - okc-hooks의 단일 배포 바이너리 조립 루트.
//!
//! 하위 아홉 라이브러리 크레이트를 한 방향으로 조립하고, 단일 인스턴스 잠금, federated config
//! 해소, 트리거 합류, 직렬 동기화 사이클, 운영 제어, graceful shutdown을 제공한다. 실제 실행
//! 진입점은 `main.rs`이며 이 라이브러리 표면은 동일 로직을 예제/property 테스트에서 사용한다.

/// Federated config 키 집계, raw JSON 검증, 타입드 런타임 설정 해소.
pub mod config;
/// OS advisory 파일 잠금 기반 단일 인스턴스 보호.
pub mod instance_lock;
/// 하위 크레이트의 공개 API를 U8 seam에 연결하는 얇은 어댑터.
pub mod adapters;
/// U7b 제어 핸들러 위임과 U8 고유 sync-now/reload 처리.
pub mod control;
/// 트리거 합류 및 단일 직렬 동기화 사이클.
pub mod coordinator;
/// 전체 컴포넌트 조립, 트리거 스레드, 종료 수명주기.
pub mod daemon;
/// 동기화 상태와 목적지 identity의 지속 바인딩.
pub mod target_binding;
/// 설정 주도 `setup` — config 검증·배치(0600) + 자동시작 서비스 바인딩(LIR-H1..H4).
pub mod setup;

/// PBT 도메인 제너레이터. 비기본 feature에서만 노출된다.
#[cfg(feature = "proptest-support")]
pub mod proptest_support;

pub use adapters::{
    AvailabilityGuard, ConsentPort, CoordinatorStore, CycleDriver, ManifestSource, RunStatePort,
    SharedSyncStore, VaultBlobSource, VaultManifestSource,
};
pub use config::{
    FederatedConfig, FederatedConfigError, LoadedRuntimeConfig, RawBackoffConfig,
    RawFederatedConfig, RawUpdateGateConfig, RuntimeConfigError, U8_CONFIG_KEYS,
    UpdateRuntimeConfig, known_config_keys, load_runtime_config,
};
pub use control::{DaemonControlHandlers, TriggerSender};
pub use coordinator::{
    CoordinatorOutcome, CycleGate, CycleTrigger, ShutdownFlag, SyncCycleCoordinator, WIRING_EDGES,
    WiringEdge, coalesce_to_latest, decide_cycle_gate, wiring_is_acyclic,
};
pub use daemon::{DaemonError, WatcherDaemon};
pub use instance_lock::{
    InstanceInfo, LockConfig, LockError, LockGuard, LockRecord, SingleInstanceLock,
};
pub use setup::{
    NativeServiceRegistrar, ServiceRegistrar, SetupError, SetupReport, perform_setup, run_setup,
    user_config_path,
};

//! ConfigStore + ConfigProvider — lock-free 읽기 `ArcSwap` + reload 쓰기 경로 직렬화.
//!
//! `current()` 는 `arc_swap::ArcSwapOption<WatcherConfig>` 를 lock-free 로드하며 reload_mutex 를
//! 취득하지 않는다. `reload()` 는 내부 `reload_mutex` 로 `load -> validate -> swap -> notify` 전체
//! 시퀀스를 직렬화한다(R-RELOAD-01/02, R-OBSERVER-03). 검증 실패 시 스왑에 도달하지 않아 이전
//! 스냅샷이 그대로 유지되고(keep-last-good, R-RELOAD-03/U0-NFR-REL-05) `Err(ConfigError)` 만
//! 표면화된다. 관찰자 fan-out 은 성공 스왑 직후 동일 임계구역 안에서만 이루어진다(R-RELOAD-05).
//! reload 는 명시적 CLI 호출로만 트리거된다(R-TRIGGER-01, SIGHUP/파일-watch 없음).
//!
//! 구현 노트: 셀 타입은 계획서 약칭 `ArcSwap<Arc<WatcherConfig>>` 대신
//! `arc_swap::ArcSwapOption<WatcherConfig>` 를 사용한다 — 전자는 `Arc<Arc<_>>` 로 중첩되며 STEP
//! 16 이 요구하는 "최초 load 전 빈 상태"를 표현하지 못한다. `ArcSwapOption` 이 동일한 lock-free
//! 읽기 + 원자 스왑 + keep-last-good 의미를 제공하면서 empty 상태를 자연스럽게 표현한다.
//!
//! `ArcSwap` 포인터 로드를 사용하는 상태 보유 모듈이므로 순수 lint-gate 를 적용하지 않는다.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use arc_swap::ArcSwapOption;

use crate::core_types::{ConfigReloadObserver, LogLevel};

use super::loader;
use super::model::{ConfigSnapshot, WatcherConfig};
use super::observer::ObserverRegistry;
use super::validator::{ConfigError, ConfigIssue};

/// reload_mutex 로 보호되는 쓰기 경로 상태(최초 load 시 해소된 경로).
struct ReloadState {
    /// 최초 load 로 해소된 config 경로(reload 는 동일 경로를 재-읽기).
    resolved_path: Option<PathBuf>,
}

/// ConfigProvider 애그리게이트 — config 로드·검증 + 활성 스냅샷 + 관찰자 팬아웃의 구현체.
///
/// 사용 순서: `new(known_keys)` -> `subscribe(...)`(기동 전, 가변 참조) -> `load(...)`(최초 로드)
/// -> 이후 `current()`(lock-free 읽기) / `reload()`(CLI 트리거). federated known-key union 을
/// 주입받아 CONSUME 만 하며 하위 단위를 import 하지 않는다(DAG 루트 비순환).
pub struct ConfigProvider {
    /// 활성 config 의 lock-free 스왑 셀(최초 load 전에는 비어 있음).
    active: ArcSwapOption<WatcherConfig>,
    /// 쓰기 경로(reload)를 직렬화하는 뮤텍스 + 해소된 경로.
    reload_mutex: Mutex<ReloadState>,
    /// 기동 시 고정되는 불변 순서 관찰자 레지스트리.
    observers: ObserverRegistry,
    /// 주입된 federated known-key 집합(미지 키 판정 기준).
    known_keys: Vec<String>,
}

impl ConfigProvider {
    /// 주입된 federated known-key 집합으로 (아직 로드되지 않은) 프로바이더를 생성한다(Q6=A).
    pub fn new(known_keys: &[&str]) -> Self {
        ConfigProvider {
            active: ArcSwapOption::empty(),
            reload_mutex: Mutex::new(ReloadState {
                resolved_path: None,
            }),
            observers: ObserverRegistry::new(),
            known_keys: known_keys.iter().map(|key| (*key).to_string()).collect(),
        }
    }

    /// 관찰자를 등록한다(데몬 스레드 기동 전, 조립 단계에서만 호출).
    ///
    /// 불변 순서 레지스트리에 보존하기 위해 소유권 있는 `Arc<dyn ConfigReloadObserver>` 를 받는다.
    pub fn subscribe(&mut self, observer: Arc<dyn ConfigReloadObserver>) {
        self.observers.subscribe(observer);
    }

    /// 최초 config 로드 — 경로를 해소(R-DISCOVER-01)하고 읽기·검증 후 활성 스냅샷을 세운다.
    ///
    /// 성공 시 로드된 스냅샷을 반환하고(조립 루트가 초기 config 로 사용) 이후 reload 를 위해
    /// 해소된 경로를 기억한다. 실패 시 `Err(ConfigError)` 를 반환하며(활성은 비어 있는 채 유지),
    /// 호출측 `watcher-bin` 이 non-zero exit 으로 abort 한다(R-RELOAD-04). 최초 load 는 관찰자
    /// fan-out 을 트리거하지 않는다(관찰자는 `current()` 로 초기 config 를 읽는다).
    pub fn load(&self, cli_path: Option<&Path>) -> Result<ConfigSnapshot, ConfigError> {
        let resolved = loader::resolve_config_path(cli_path);
        let config = loader::read_and_validate(&resolved, &self.known_keys_refs())?;
        let arc = Arc::new(config);

        let mut guard = self.lock_reload();
        guard.resolved_path = Some(resolved);
        self.active.store(Some(arc.clone()));
        drop(guard);

        Ok(ConfigSnapshot::new(arc))
    }

    /// 현재 활성 config 스냅샷을 lock-free 로 반환한다(reload_mutex 미취득).
    ///
    /// 계약상 최초 `load` 성공 이후에 호출된다. 로드 전 호출(계약 위반)은 패닉 대신 구조적
    /// 기본 스냅샷(빈 필수 필드)을 반환한다 — 이 값은 검증을 통과할 수 없는 자리표시일 뿐이다.
    pub fn current(&self) -> ConfigSnapshot {
        match self.active.load_full() {
            Some(config) => ConfigSnapshot::new(config),
            None => ConfigSnapshot::new(Arc::new(unloaded_placeholder())),
        }
    }

    /// CLI 트리거 config 리로드 — 쓰기 경로 전체를 직렬화한다(R-TRIGGER-01).
    ///
    /// `reload_mutex` 임계구역 안에서 기억된 경로를 재-읽기·검증하고, 성공 시에만 활성 스냅샷을
    /// 원자적으로 스왑한 뒤 동일 구역에서 관찰자에게 순서대로 fan-out 한다(R-RELOAD-05,
    /// R-OBSERVER-03). 검증 실패 시 스왑에 도달하지 않아 이전 스냅샷을 유지하며(keep-last-good)
    /// `Err(ConfigError)` 만 반환한다. 최초 load 미완료 상태에서의 호출은 `Discovery` 이슈로 거부.
    pub fn reload(&self) -> Result<(), ConfigError> {
        let guard = self.lock_reload();
        let path = match &guard.resolved_path {
            Some(path) => path.clone(),
            None => {
                return Err(ConfigError::single(ConfigIssue::Discovery(
                    "reload 이전에 최초 load 가 완료되지 않았습니다".to_string(),
                )));
            }
        };

        // 검증 실패 시 여기서 조기 반환 -> 활성 스냅샷 불변(keep-last-good).
        let config = loader::read_and_validate(&path, &self.known_keys_refs())?;

        // 검증 성공: 원자적 스왑 후 동일 임계구역 안에서 결정적 fan-out.
        self.active.store(Some(Arc::new(config)));
        self.observers.notify_all();
        drop(guard);

        Ok(())
    }

    /// 저장된 `Vec<String>` known-key 를 `&[&str]` 형태로 빌려 검증 경로에 전달한다.
    fn known_keys_refs(&self) -> Vec<&str> {
        self.known_keys.iter().map(|key| key.as_str()).collect()
    }

    /// reload_mutex 를 취득한다. 뮤텍스 poison 시 패닉 대신 내부 상태를 회수한다(회복력).
    fn lock_reload(&self) -> MutexGuard<'_, ReloadState> {
        match self.reload_mutex.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

/// 최초 load 전 `current()` 호출(계약 위반)에 대한 구조적 자리표시 config.
///
/// 필수 필드는 빈 문자열이라 어떤 검증도 통과하지 못하는 no-op 기본값이며, 패닉 회피용이다.
fn unloaded_placeholder() -> WatcherConfig {
    WatcherConfig {
        vault_path: String::new(),
        server_endpoint: String::new(),
        token: None,
        secure_store_enabled: false,
        log_level: LogLevel::Info,
        notify_consecutive_failures: 3,
    }
}

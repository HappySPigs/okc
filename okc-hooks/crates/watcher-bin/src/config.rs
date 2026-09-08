//! U8 federated config 해소.
//!
//! U0 `ConfigProvider`는 core 6필드를 검증하고, 이 모듈은 같은 JSON의 비-core 필드를 각 하위
//! 생성자가 요구하는 타입으로 해소한다. 비-core 값은 기동 시 고정되고 변경은 재시작 후 반영된다.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use auth_consent::{AUTH_CONSENT_CONFIG_KEYS, DEFAULT_REQUEST_TIMEOUT_S, validate_request_timeout_s};
use change_detect::{ReconConfig, WatchConfig};
use foundation::{
    ByteCount, ConfigError, ConfigProvider, ConfigSnapshot, FOUNDATION_CONFIG_KEYS,
};
use lifecycle_deploy::{GateConfig, UpdateChannel};
use observability::{
    LoggerConfig, OBSERVABILITY_CONFIG_KEYS, ObservabilityConfig,
};
use ops_control::IpcEndpoint;
use serde::{Deserialize, Serialize};
use sync_state::{BackoffConfig, JitterMode, StateConfig};

/// U8이 집계하는 나머지 federated top-level 키.
///
/// U1은 MVP에서 federated 키가 없고, U2/U3/U4/U7a의 키는 U8 조립 루트가 이 목록으로 소유한다.
pub const U8_CONFIG_KEYS: &[&str] = &[
    "debounce_ms",
    "reconciliation_interval_s",
    "confirm_empty",
    "state_path",
    "backoff",
    "chunk_threshold_bytes",
    "chunk_size_bytes",
    "data_dir",
    "socket_path",
    "update_channel",
    "update_gate",
];

/// 기본 로그 회전 임계(10 MiB).
pub const DEFAULT_LOG_MAX_SIZE_BYTES: u64 = 10 * 1024 * 1024;
/// 기본 로그 회전 보존 개수.
pub const DEFAULT_LOG_MAX_RETAINED: usize = 3;
/// 기본 단일 요청/청크 경계(8 MiB).
pub const DEFAULT_CHUNK_THRESHOLD_BYTES: u64 = 8 * 1024 * 1024;
/// 기본 청크 크기(1 MiB).
pub const DEFAULT_CHUNK_SIZE_BYTES: u64 = 1024 * 1024;
/// 기본 디바운스 정적 구간(ms).
pub const DEFAULT_DEBOUNCE_MS: u64 = 2_000;
/// 기본 재조정 간격(초).
pub const DEFAULT_RECONCILIATION_INTERVAL_S: u64 = 900;

/// `backoff` raw JSON 투영.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RawBackoffConfig {
    /// 첫 재시도 지연(초).
    pub initial_delay_s: Option<i64>,
    /// 지수 승수.
    pub multiplier: Option<f64>,
    /// 지연 상한(초).
    pub cap_s: Option<i64>,
    /// `full` 또는 `none`.
    pub jitter: Option<String>,
}

/// `update_gate` raw JSON 투영. AutoUpdater는 MVP에서 실행 배선하지 않지만 설정 의미는 보존한다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RawUpdateGateConfig {
    /// liveness 게이트 상한(초).
    pub timeout_s: Option<i64>,
    /// liveness 폴링 간격(초).
    pub poll_s: Option<i64>,
    /// 롤백 뒤 재적용 보류(초).
    pub hold_s: Option<i64>,
}

/// 단일 JSON에서 U8이 읽는 raw federated 필드.
///
/// U0 core 필드는 serde 기본 unknown-field 허용으로 무시된다. top-level 미지 키의 엄격 거부는
/// 먼저 실행되는 `ConfigProvider::load`가 담당한다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RawFederatedConfig {
    /// U5 요청 타임아웃(초).
    pub request_timeout_s: Option<i64>,
    /// U2 디바운스 구간(ms).
    pub debounce_ms: Option<i64>,
    /// U2 재조정 간격(초).
    pub reconciliation_interval_s: Option<i64>,
    /// U2 파괴적 빈 매니페스트 명시 허용.
    pub confirm_empty: Option<bool>,
    /// U4 상태 파일 절대 경로.
    pub state_path: Option<String>,
    /// U4 백오프 설정.
    pub backoff: Option<RawBackoffConfig>,
    /// U3 단일 전송 임계(바이트).
    pub chunk_threshold_bytes: Option<i64>,
    /// U3 청크 크기(바이트).
    pub chunk_size_bytes: Option<i64>,
    /// U6 로그 파일 경로.
    pub log_file: Option<String>,
    /// U6 로그 회전 임계(바이트).
    pub log_max_size_bytes: Option<i64>,
    /// U6 회전 파일 보존 개수.
    pub log_max_retained: Option<i64>,
    /// U6 히스토리 파일 경로.
    pub history_file: Option<String>,
    /// U6 트레이 활성 플래그(MVP no-op).
    pub tray_enabled: Option<bool>,
    /// U8 데이터 디렉터리 절대 경로.
    pub data_dir: Option<String>,
    /// U7b 로컬 IPC 소켓 경로 또는 Windows pipe 이름.
    pub socket_path: Option<String>,
    /// U7a 업데이트 채널(`stable`, `beta`, `disabled`).
    pub update_channel: Option<String>,
    /// U7a bounded liveness 게이트 설정.
    pub update_gate: Option<RawUpdateGateConfig>,
}

/// AutoUpdater에 전달 가능한 해소된 설정. MVP 데몬은 source가 없어 실행 배선하지 않는다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateRuntimeConfig {
    /// 업데이트 채널.
    pub channel: UpdateChannel,
    /// bounded liveness 게이트 파라미터.
    pub gate: GateConfig,
}

/// 각 하위 생성자에 하향 주입할 타입드 federated 설정.
#[derive(Debug, Clone, PartialEq)]
pub struct FederatedConfig {
    /// 모든 로컬 런타임 아티팩트의 기본 디렉터리.
    pub data_dir: PathBuf,
    /// U4 상태 파일 설정.
    pub state: StateConfig,
    /// U4 백오프 설정.
    pub backoff: BackoffConfig,
    /// U5 요청 데드라인.
    pub request_timeout: Duration,
    /// U2 디바운스 구간.
    pub debounce: Duration,
    /// U2 주기 재조정 간격.
    pub reconciliation_interval: Duration,
    /// U2 파괴적 빈 매니페스트 명시 허용.
    pub confirm_empty: bool,
    /// U3 단일 전송 임계.
    pub chunk_threshold_bytes: u64,
    /// U3 청크 크기.
    pub chunk_size_bytes: u64,
    /// U6 관측 설정.
    pub observability: ObservabilityConfig,
    /// U7b 로컬 IPC 주소.
    pub ipc_endpoint: IpcEndpoint,
    /// U8 단일 인스턴스 락 파일.
    pub lock_file: PathBuf,
    /// U5 동의 지속 파일.
    pub consent_file: PathBuf,
    /// U7b run-state 지속 파일.
    pub run_state_file: PathBuf,
    /// U7a 업데이트 설정(MVP 실행 미배선).
    pub update: UpdateRuntimeConfig,
}

/// U0 core 스냅샷, U8 federated 설정, 실제 config 경로의 한 번 로드 결과.
pub struct LoadedRuntimeConfig {
    /// 공유 U0 config provider.
    pub provider: Arc<ConfigProvider>,
    /// 최초 검증된 U0 core 스냅샷.
    pub core: ConfigSnapshot,
    /// 해소된 비-core 값.
    pub federated: FederatedConfig,
    /// 실제 읽은 config 파일 경로.
    pub config_path: PathBuf,
}

/// Federated per-field 검증 오류.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FederatedConfigError {
    /// raw JSON 필드 타입/중첩 스키마 오류.
    #[error("federated config 타입 오류: {0}")]
    Type(String),
    /// 필드 값 범위/형식 오류.
    #[error("federated config 필드 `{field}` 위반: {expected}")]
    Field {
        /// 위반 필드.
        field: &'static str,
        /// 기대 형식/범위.
        expected: &'static str,
    },
}

/// 전체 runtime config 로드 오류.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeConfigError {
    /// U0 core/strict-known-key 검증 오류.
    #[error("core config 오류: {0}")]
    Core(#[from] ConfigError),
    /// config 파일 재읽기 오류.
    #[error("config 파일 읽기 실패 `{path}`: {source}")]
    Read {
        /// 대상 경로.
        path: PathBuf,
        /// 원본 I/O 오류.
        #[source]
        source: std::io::Error,
    },
    /// raw federated JSON 타입 오류.
    #[error(transparent)]
    Federated(#[from] FederatedConfigError),
}

/// U0/U5/U6/U8 네 소스의 정확한 top-level known-key 합집합을 반환한다.
pub fn known_config_keys() -> Vec<&'static str> {
    FOUNDATION_CONFIG_KEYS
        .iter()
        .chain(AUTH_CONSENT_CONFIG_KEYS)
        .chain(OBSERVABILITY_CONFIG_KEYS)
        .chain(U8_CONFIG_KEYS)
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// config를 U0 strict 검증과 U8 raw typed 해소로 한 번에 읽는다.
pub fn load_runtime_config(cli_path: Option<&Path>) -> Result<LoadedRuntimeConfig, RuntimeConfigError> {
    let config_path = foundation::resolve_config_path(cli_path);
    let known_keys = known_config_keys();
    let provider = Arc::new(ConfigProvider::new(&known_keys));
    let core = provider.load(Some(&config_path))?;
    let raw_text = std::fs::read_to_string(&config_path).map_err(|source| RuntimeConfigError::Read {
        path: config_path.clone(),
        source,
    })?;
    let raw: RawFederatedConfig = serde_json::from_str(&raw_text)
        .map_err(|error| FederatedConfigError::Type(error.to_string()))?;
    let federated = raw.resolve(&core)?;
    Ok(LoadedRuntimeConfig {
        provider,
        core,
        federated,
        config_path,
    })
}

impl RawFederatedConfig {
    /// raw federated 값을 플랫폼 기본 data-dir 기준의 타입드 설정으로 해소한다.
    pub fn resolve(self, core: &ConfigSnapshot) -> Result<FederatedConfig, FederatedConfigError> {
        self.resolve_with_default_data_dir(core, platform_default_data_dir())
    }

    /// 테스트/내장 환경이 지정한 기본 data-dir로 타입드 설정을 해소한다.
    pub fn resolve_with_default_data_dir(
        self,
        core: &ConfigSnapshot,
        default_data_dir: PathBuf,
    ) -> Result<FederatedConfig, FederatedConfigError> {
        let data_dir = resolve_path("data_dir", self.data_dir, default_data_dir, true)?;

        let request_timeout_s = self
            .request_timeout_s
            .unwrap_or(DEFAULT_REQUEST_TIMEOUT_S as i64);
        let request_timeout = validate_request_timeout_s(request_timeout_s).map_err(|_| {
            FederatedConfigError::Field {
                field: "request_timeout_s",
                expected: "1 이상의 정수 초",
            }
        })?;

        let debounce_ms = positive_u64(
            "debounce_ms",
            self.debounce_ms.unwrap_or(DEFAULT_DEBOUNCE_MS as i64),
        )?;
        let debounce = WatchConfig {
            vault_path: core.vault_path.clone(),
            debounce_ms,
        }
        .debounce()
        .map_err(|_| FederatedConfigError::Field {
            field: "debounce_ms",
            expected: "1 이상의 정수 밀리초",
        })?;

        let reconciliation_interval_s = positive_u64(
            "reconciliation_interval_s",
            self.reconciliation_interval_s
                .unwrap_or(DEFAULT_RECONCILIATION_INTERVAL_S as i64),
        )?;
        let reconciliation_interval = ReconConfig {
            reconciliation_interval_s,
        }
        .interval()
        .map_err(|_| FederatedConfigError::Field {
            field: "reconciliation_interval_s",
            expected: "1 이상의 정수 초",
        })?;

        let chunk_threshold_bytes = positive_u64(
            "chunk_threshold_bytes",
            self.chunk_threshold_bytes
                .unwrap_or(DEFAULT_CHUNK_THRESHOLD_BYTES as i64),
        )?;
        let chunk_size_bytes = positive_u64(
            "chunk_size_bytes",
            self.chunk_size_bytes
                .unwrap_or(DEFAULT_CHUNK_SIZE_BYTES as i64),
        )?;

        let log_max_size_bytes = positive_u64(
            "log_max_size_bytes",
            self.log_max_size_bytes
                .unwrap_or(DEFAULT_LOG_MAX_SIZE_BYTES as i64),
        )?;
        let retained_raw = self
            .log_max_retained
            .unwrap_or(DEFAULT_LOG_MAX_RETAINED as i64);
        let log_max_retained = non_negative_usize("log_max_retained", retained_raw)?;

        let log_file = resolve_path(
            "log_file",
            self.log_file,
            data_dir.join("watcher.log"),
            true,
        )?;
        let history_file = resolve_path(
            "history_file",
            self.history_file,
            data_dir.join("history.cbor"),
            true,
        )?;
        let state_path = resolve_path(
            "state_path",
            self.state_path,
            data_dir.join("sync-state.cbor"),
            true,
        )?;

        let socket_value = match self.socket_path {
            Some(value) if value.trim().is_empty() => {
                return Err(FederatedConfigError::Field {
                    field: "socket_path",
                    expected: "비어있지 않은 경로 또는 pipe 이름",
                });
            }
            Some(value) => value,
            None if cfg!(windows) => r"\\.\pipe\okc-watcher".to_string(),
            None => data_dir.join("control.sock").display().to_string(),
        };
        let ipc_endpoint = if cfg!(windows) {
            IpcEndpoint::NamedPipe(socket_value)
        } else {
            let socket_path = PathBuf::from(socket_value);
            if !socket_path.is_absolute() {
                return Err(FederatedConfigError::Field {
                    field: "socket_path",
                    expected: "절대 파일시스템 경로",
                });
            }
            IpcEndpoint::SocketPath(socket_path)
        };

        let backoff = resolve_backoff(self.backoff)?;
        let update = resolve_update(self.update_channel, self.update_gate)?;
        let observability = ObservabilityConfig {
            log_file,
            log_max_size_bytes: ByteCount::new(log_max_size_bytes),
            log_max_retained,
            history_file,
            tray_enabled: self.tray_enabled.unwrap_or(false),
        };

        Ok(FederatedConfig {
            lock_file: data_dir.join("watcher.lock"),
            consent_file: data_dir.join("consent.cbor"),
            run_state_file: data_dir.join("run-state.cbor"),
            state: StateConfig {
                state_path: Some(state_path),
            },
            backoff,
            request_timeout,
            debounce,
            reconciliation_interval,
            confirm_empty: self.confirm_empty.unwrap_or(false),
            chunk_threshold_bytes,
            chunk_size_bytes,
            observability,
            ipc_endpoint,
            data_dir,
            update,
        })
    }

    /// 직렬화 가능한 raw 투영을 JSON 값으로 변환한다(PROP-U8-07 round-trip 보조).
    pub fn to_json_value(&self) -> Result<serde_json::Value, FederatedConfigError> {
        serde_json::to_value(self).map_err(|error| FederatedConfigError::Type(error.to_string()))
    }

    /// JSON 값에서 raw 투영을 복원한다(PROP-U8-07 round-trip 보조).
    pub fn from_json_value(value: serde_json::Value) -> Result<Self, FederatedConfigError> {
        serde_json::from_value(value)
            .map_err(|error| FederatedConfigError::Type(error.to_string()))
    }
}

impl FederatedConfig {
    /// `StructuredLogger` 생성자용 설정을 U0 core log level과 결합한다.
    pub fn logger_config(&self, core: &ConfigSnapshot) -> LoggerConfig {
        LoggerConfig {
            log_file: self.observability.log_file.clone(),
            log_max_size_bytes: self.observability.log_max_size_bytes,
            log_max_retained: self.observability.log_max_retained,
            log_level: core.log_level,
        }
    }
}

fn positive_u64(field: &'static str, value: i64) -> Result<u64, FederatedConfigError> {
    u64::try_from(value)
        .ok()
        .filter(|number| *number > 0)
        .ok_or(FederatedConfigError::Field {
            field,
            expected: "1 이상의 정수",
        })
}

fn non_negative_usize(field: &'static str, value: i64) -> Result<usize, FederatedConfigError> {
    usize::try_from(value).map_err(|_| FederatedConfigError::Field {
        field,
        expected: "0 이상의 정수",
    })
}

fn resolve_path(
    field: &'static str,
    raw: Option<String>,
    default: PathBuf,
    require_absolute: bool,
) -> Result<PathBuf, FederatedConfigError> {
    let path = match raw {
        Some(value) if value.trim().is_empty() => {
            return Err(FederatedConfigError::Field {
                field,
                expected: "비어있지 않은 경로",
            });
        }
        Some(value) => PathBuf::from(value),
        None => default,
    };
    if require_absolute && !path.is_absolute() {
        return Err(FederatedConfigError::Field {
            field,
            expected: "절대 파일시스템 경로",
        });
    }
    Ok(path)
}

fn resolve_backoff(raw: Option<RawBackoffConfig>) -> Result<BackoffConfig, FederatedConfigError> {
    let raw = raw.unwrap_or_default();
    let initial = positive_u64(
        "backoff.initial_delay_s",
        raw.initial_delay_s.unwrap_or(1),
    )?;
    let cap = positive_u64("backoff.cap_s", raw.cap_s.unwrap_or(300))?;
    if cap < initial {
        return Err(FederatedConfigError::Field {
            field: "backoff.cap_s",
            expected: "initial_delay_s 이상",
        });
    }
    let multiplier = raw.multiplier.unwrap_or(2.0);
    if !multiplier.is_finite() || multiplier < 1.0 {
        return Err(FederatedConfigError::Field {
            field: "backoff.multiplier",
            expected: "1.0 이상의 유한 실수",
        });
    }
    let jitter = match raw.jitter.as_deref().unwrap_or("full") {
        "full" => JitterMode::Full,
        "none" => JitterMode::None,
        _ => {
            return Err(FederatedConfigError::Field {
                field: "backoff.jitter",
                expected: "full 또는 none",
            });
        }
    };
    Ok(BackoffConfig {
        initial_delay: Duration::from_secs(initial),
        multiplier,
        cap: Duration::from_secs(cap),
        jitter,
    })
}

fn resolve_update(
    channel: Option<String>,
    gate: Option<RawUpdateGateConfig>,
) -> Result<UpdateRuntimeConfig, FederatedConfigError> {
    let channel = match channel.as_deref().unwrap_or("disabled") {
        "stable" => UpdateChannel::Stable,
        "beta" => UpdateChannel::Beta,
        "disabled" => UpdateChannel::Disabled,
        _ => {
            return Err(FederatedConfigError::Field {
                field: "update_channel",
                expected: "stable, beta, disabled 중 하나",
            });
        }
    };
    let gate = gate.unwrap_or_default();
    let timeout = positive_u64("update_gate.timeout_s", gate.timeout_s.unwrap_or(30))?;
    let poll = positive_u64("update_gate.poll_s", gate.poll_s.unwrap_or(1))?;
    let hold_value = gate.hold_s.unwrap_or(300);
    let hold = u64::try_from(hold_value).map_err(|_| FederatedConfigError::Field {
        field: "update_gate.hold_s",
        expected: "0 이상의 정수 초",
    })?;
    Ok(UpdateRuntimeConfig {
        channel,
        gate: GateConfig {
            t_gate: Duration::from_secs(timeout),
            t_poll: Duration::from_secs(poll),
            t_hold: Duration::from_secs(hold),
        },
    })
}

// 플랫폼별 cfg 암이 각각 `return` 으로 값을 돌려주는 대칭 구조라 명시적 return 을 유지한다
// (한 암에서만 return 을 빼면 세 플랫폼 암의 형태가 어긋난다).
#[allow(clippy::needless_return)]
fn platform_default_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        return std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("okc-watcher");
    }
    #[cfg(target_os = "macos")]
    {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Library")
            .join("Application Support")
            .join("okc-watcher");
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(xdg) = std::env::var_os("XDG_STATE_HOME") {
            return PathBuf::from(xdg).join("okc-watcher");
        }
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(".local")
            .join("state")
            .join("okc-watcher")
    }
    #[cfg(not(any(unix, target_os = "windows")))]
    {
        std::env::temp_dir().join("okc-watcher")
    }
}

//! 설정 주도 `setup` — 사용자 작성 config 를 검증·배치하고 자동시작 서비스를 그 config 에 바인딩한다.
//!
//! `setup` 흐름(LIR-H1..H4): (1) 사용자 작성 config 소스를 읽어 foundation 의 기존 검증으로
//! **hard-gate**(strict 스키마 + https-only 엔드포인트 + non-blank 토큰)를 수행하고,
//! (2) 사용자 레벨 경로(`user_config_path`)에 원본 내용을 그대로 기록하며 Unix 에서 `0600` 을 강제하고,
//! (3) 기존 `install` 서비스 등록 경로(U7a `ServiceManager`)를 재사용하되 생성된 유닛이 이 config 에
//! 명시 바인딩되도록 한다(`ServiceSpec::with_config_path`). 코어 발견 기본값(`/etc/...`)은 건드리지
//! 않는다. DNS 도달성 확인은 best-effort 경고일 뿐 setup 을 중단하지 않는다(LIR-H3).
//!
//! 토큰은 어떤 출력/오류에도 노출되지 않는다(SECURITY-03): 검증 오류는 `ConfigError::report()` 로만
//! 표면화되며 토큰 값을 포함하지 않고, 원본 내용은 재직렬화 없이 그대로 복사되어 로그 경로를 타지 않는다.
//!
//! `std::fs`/서비스 I/O 를 수행하므로 순수 lint-gate 는 두지 않는다(단, `unwrap`/`expect`/`panic` 미사용).
//! 서비스 등록은 `ServiceRegistrar` seam 뒤에 격리되어 단위테스트가 특권/OS 없이 검증한다.

use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};

use lifecycle_deploy::{ServiceManager, ServiceSpec, native_controller};

use crate::config::known_config_keys;

/// `setup` 실패 taxonomy. 어떤 변형도 토큰 값을 담지 않는다(SECURITY-03/09).
#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    /// config 소스 파일 읽기 실패.
    #[error("config 소스 읽기 실패 `{path}`: {detail}")]
    Read {
        /// 읽기를 시도한 소스 경로.
        path: String,
        /// I/O 오류 상세.
        detail: String,
    },
    /// config 검증 실패(strict 스키마/https-only/non-blank 토큰). 메시지는 토큰 값을 포함하지 않는다.
    #[error("config 검증 실패: {}", _0.report())]
    Invalid(foundation::ConfigError),
    /// 대상 경로 쓰기(또는 디렉터리 생성) 실패.
    #[error("config 쓰기 실패 `{path}`: {detail}")]
    Write {
        /// 쓰기를 시도한 대상 경로.
        path: String,
        /// I/O 오류 상세.
        detail: String,
    },
    /// 자동시작 서비스 등록 실패(seam 반환 오류 문자열).
    #[error("서비스 등록 실패: {0}")]
    Registration(String),
    /// 실행 파일 경로 확인 실패(`current_exe`).
    #[error("실행 파일 경로 확인 실패: {0}")]
    Exec(String),
}

/// `setup` 성공 결과 요약(비밀 값 없음).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupReport {
    /// config 가 기록된 사용자 레벨 경로.
    pub dest_path: PathBuf,
    /// 검증된 서버 엔드포인트(비밀 아님 — 사용자 메시지/도달성 확인용).
    pub server_endpoint: String,
}

/// 자동시작 서비스 등록 seam — 생성될 유닛을 명시 config 경로에 바인딩한다.
///
/// 프로덕션 구현(`NativeServiceRegistrar`)은 U7a `ServiceManager` 를 재사용하고, 테스트는 호출을
/// 기록하는 fake 를 주입해 특권/OS 없이 setup 로직을 검증한다.
pub trait ServiceRegistrar {
    /// 자동시작 서비스를 등록하고 데몬을 `config_path` 에 바인딩한다. 재등록(upsert)은 멱등하다.
    fn register(&self, config_path: &Path) -> Result<(), String>;
}

/// 프로덕션 `ServiceRegistrar` — U7a `ServiceManager`/`ServiceSpec` 로 자동시작 서비스를 등록한다.
pub struct NativeServiceRegistrar {
    /// 배포 바이너리 절대 경로(`current_exe` 유래).
    exec_path: PathBuf,
    /// 서비스 작업 디렉터리(절대). 데몬은 바인딩된 config 에서 실제 경로를 해소하므로 CWD 용도다.
    working_dir: PathBuf,
}

impl NativeServiceRegistrar {
    /// 실행 파일 경로와 작업 디렉터리로 등록기를 구성한다.
    pub fn new(exec_path: PathBuf, working_dir: PathBuf) -> Self {
        NativeServiceRegistrar {
            exec_path,
            working_dir,
        }
    }
}

impl ServiceRegistrar for NativeServiceRegistrar {
    fn register(&self, config_path: &Path) -> Result<(), String> {
        let controller = native_controller().map_err(|err| err.to_string())?;
        let manager = ServiceManager::new(controller);
        let spec = ServiceSpec::new(self.exec_path.clone(), self.working_dir.clone())
            .with_config_path(config_path.to_path_buf());
        manager.install(&spec).map_err(|err| err.to_string())
    }
}

/// 사용자 레벨 config 경로를 플랫폼 규약대로 반환한다(코어 발견 기본값과 별개, loader.rs 미변경).
///
/// Linux: `$XDG_CONFIG_HOME/okc-watcher/config.json` 또는 `~/.config/okc-watcher/config.json`.
/// macOS: `~/Library/Application Support/okc-watcher/config.json`.
/// Windows: `%APPDATA%\okc-watcher\config.json`. HOME/APPDATA 부재 시 임시 디렉터리로 폴백한다.
#[allow(clippy::needless_return)]
pub fn user_config_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        return std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("okc-watcher")
            .join("config.json");
    }
    #[cfg(target_os = "macos")]
    {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Library")
            .join("Application Support")
            .join("okc-watcher")
            .join("config.json");
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            let xdg = PathBuf::from(xdg);
            if xdg.is_absolute() {
                return xdg.join("okc-watcher").join("config.json");
            }
        }
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(".config")
            .join("okc-watcher")
            .join("config.json");
    }
    #[cfg(not(any(unix, target_os = "windows")))]
    {
        std::env::temp_dir().join("okc-watcher").join("config.json")
    }
}

/// setup 코어 — 검증 -> 사용자 레벨 경로에 `0600` 기록 -> 서비스 등록(seam). 결정적/테스트 가능.
///
/// 원본 내용을 재직렬화 없이 그대로 기록해 core+federated 필드를 모두 보존한다(코어 모델만
/// 재직렬화하면 `data_dir` 등 federated 필드가 유실됨). 재실행 시 덮어쓰기 + 재등록으로 멱등하다(NFR-3).
pub fn perform_setup(
    source_path: &Path,
    dest_path: &Path,
    registrar: &dyn ServiceRegistrar,
) -> Result<SetupReport, SetupError> {
    // (1) 소스 읽기.
    let raw = std::fs::read_to_string(source_path).map_err(|error| SetupError::Read {
        path: source_path.display().to_string(),
        detail: error.to_string(),
    })?;

    // (2) hard-gate 검증: strict 스키마 + https-only 엔드포인트 + non-blank 토큰(foundation 재사용).
    //     오류 메시지는 토큰 값을 포함하지 않는다(SECURITY-03/05).
    let config = foundation::validate(&raw, &known_config_keys()).map_err(SetupError::Invalid)?;

    // (3) 대상 부모 디렉터리 생성(사용자 소유 하위 디렉터리만 생성; 상위 사용자 디렉터리 권한은 미변경).
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| SetupError::Write {
            path: parent.display().to_string(),
            detail: error.to_string(),
        })?;
    }

    // (4) 원본 내용을 그대로 기록 + Unix 0600 강제(LIR-H2, SECURITY-06).
    write_private(dest_path, raw.as_bytes()).map_err(|detail| SetupError::Write {
        path: dest_path.display().to_string(),
        detail,
    })?;

    // (5) 자동시작 서비스 등록(bound config path). 재등록은 upsert(멱등).
    registrar
        .register(dest_path)
        .map_err(SetupError::Registration)?;

    Ok(SetupReport {
        dest_path: dest_path.to_path_buf(),
        server_endpoint: config.server_endpoint,
    })
}

/// 프로덕션 진입점 — 사용자 레벨 경로를 해소하고 네이티브 서비스 등록으로 setup 을 수행한다.
///
/// 성공 후 best-effort DNS 도달성 경고를 출력한다(LIR-H3, 중단 없음).
pub fn run_setup(source_path: &Path) -> Result<SetupReport, SetupError> {
    let dest = user_config_path();
    let exec = std::env::current_exe().map_err(|error| SetupError::Exec(error.to_string()))?;
    // 작업 디렉터리는 config 디렉터리(절대)로 둔다 — 데몬은 바인딩된 config 에서 data_dir 을 해소한다.
    let working_dir = dest
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| dest.clone());
    let registrar = NativeServiceRegistrar::new(exec, working_dir);
    let report = perform_setup(source_path, &dest, &registrar)?;
    warn_if_unreachable(&report.server_endpoint);
    Ok(report)
}

/// best-effort DNS 도달성 확인(LIR-H3). 실패해도 경고만 출력하고 setup 을 중단하지 않는다.
///
/// 엔드포인트 호스트만 사용하므로 토큰 유출 위험이 없다(SECURITY-03). 런타임 인증은 이미 fail-closed 다.
fn warn_if_unreachable(endpoint: &str) {
    let Ok(url) = foundation::validate_https_url(endpoint) else {
        return; // 이미 hard-gate 를 통과했으므로 정상 경로에서는 도달하지 않는다.
    };
    let Some(host) = url.host_str() else {
        return;
    };
    let port = url.port_or_known_default().unwrap_or(443);
    let reachable = (host, port)
        .to_socket_addrs()
        .map(|mut addrs| addrs.next().is_some())
        .unwrap_or(false);
    if !reachable {
        eprintln!(
            "warning: 엔드포인트 '{host}' 의 DNS 확인에 실패했습니다(설치는 계속됩니다). \
             호스트명과 네트워크 연결을 확인하세요."
        );
    }
}

/// 파일을 기록하고 Unix 에서 `0600` 을 강제한다. 실패는 사람이 읽는 오류 문자열로 반환한다.
///
/// 생성 시 `mode(0o600)` 으로 열고, 재실행(덮어쓰기) 시 기존 파일 권한이 다를 수 있으므로 기록 후
/// 명시적으로 `0600` 을 재설정한다(멱등 보장, SECURITY-06).
fn write_private(path: &Path, contents: &[u8]) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::io::Write as _;
        use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .map_err(|error| error.to_string())?;
        file.write_all(contents).map_err(|error| error.to_string())?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|error| error.to_string())?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, contents).map_err(|error| error.to_string())
    }
}

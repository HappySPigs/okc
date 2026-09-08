//! `ServiceManager` — 3-OS 동일 계약 서비스 수명주기 오케스트레이션 + `ServiceController` seam.
//!
//! 상위 메서드(`install/uninstall/start/stop/restart/status`)는 OS-불변 순수 로직으로
//! idempotency(R-SM-02)·상태 정합(R-SM-03)·`ServiceSpec` 검증(R-SM-01)만 담당하고, 실제
//! launchd plist / systemd unit / Windows SCM 호출은 `ServiceController` 어댑터에 격리한다
//! (D-U7A-01). per-OS 어댑터는 특권/OS 의존이라 단위테스트 대상이 아니며(root 필요, Infra 이월),
//! 오케스트레이션은 `testing::FakeController` 로 완전 검증된다.
//!
//! 이 모듈은 `std::process::Command` I/O 를 수행하므로 U0 `store.rs`/`loader.rs` 처럼 순수 리프
//! clippy lint-gate 를 두지 않는다(단, 어떤 경로에서도 `unwrap`/`expect`/`panic`/인덱싱을 쓰지 않는다).

use std::path::{Path, PathBuf};
use std::process::Command;

/// U7a 배포 서비스의 고정 식별 라벨/이름(MVP: 단일 서비스, 커스터마이징 이연).
pub const SERVICE_LABEL: &str = "com.okc.watcher";

/// 서비스 실행 계정 — MVP 기본은 기동/현재 사용자(계정 커스터마이징은 명시 이연, D-U7A-10).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ServiceAccount {
    /// 현재 사용자(로그인 세션) 계정으로 실행(MVP 기본).
    #[default]
    CurrentUser,
    /// 명시된 계정 이름으로 실행(이연 기능 — 어댑터가 지원 시 사용).
    Named(String),
}

/// 서비스 등록에 필요한 배포 파라미터(입력값). 조립 루트(U8)가 조립해 주입한다(D-U7A-10).
///
/// **불변식**: `exec_path`/`working_dir` 는 절대 경로여야 한다(설치 시 검증, R-SM-01). MVP 는
/// 전용 `AbsolutePath` newtype 대신 `PathBuf` + 설치 시점 검증으로 절대성 불변식을 강제한다(트림).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSpec {
    /// 배포 바이너리 절대 경로(U8: `current_exe` 유래).
    pub exec_path: PathBuf,
    /// 데몬 작업 디렉터리 절대 경로(data-dir).
    pub working_dir: PathBuf,
    /// 실행 계정(현재 사용자 기본).
    pub account: ServiceAccount,
    /// 부팅/로그인 자동시작 여부(기본 `true`, FR-21 core).
    pub autostart: bool,
    /// 생성될 서비스 유닛이 데몬을 바인딩할 config 경로(선택). `Some` 이면 유닛이
    /// `<exec> run <config>` 로 데몬을 그 config 에 명시 바인딩해 코어 기본 발견(`/etc/...`)에
    /// 의존하지 않는다(LIR-H1). `None` 이면 인자 없이 기동해 기존 `install` 동작(기본 발견)을 유지한다.
    pub config_path: Option<PathBuf>,
}

impl ServiceSpec {
    /// 기본값(`account = CurrentUser`, `autostart = true`, `config_path = None`)으로 스펙을 구성한다.
    pub fn new(exec_path: PathBuf, working_dir: PathBuf) -> Self {
        ServiceSpec {
            exec_path,
            working_dir,
            account: ServiceAccount::CurrentUser,
            autostart: true,
            config_path: None,
        }
    }

    /// 생성될 서비스 유닛이 데몬을 바인딩할 명시 config 경로를 설정한다(`setup` 경로, LIR-H1).
    pub fn with_config_path(mut self, config_path: PathBuf) -> Self {
        self.config_path = Some(config_path);
        self
    }
}

/// 상태 질의 결과 — 실제 OS 상태를 진실하게 반영한다(R-SM-03 불변식).
///
/// **불변식**: `running == true` 이면 `registered == true`, `pid.is_some()` 이면 `running == true`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceRegistration {
    /// OS 서비스 매니저에 유닛이 등록됨.
    pub registered: bool,
    /// 현재 실행 중.
    pub running: bool,
    /// 실행 중일 때의 PID.
    pub pid: Option<u32>,
}

/// `ServiceManager`/`ServiceController` 오류 taxonomy(U7a 소유, D-U7A-11).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ServiceError {
    /// 알 수 없는/미지원 OS.
    #[error("미지원 플랫폼")]
    UnsupportedPlatform,
    /// 특권 부족(등록/해제).
    #[error("권한 부족")]
    PermissionDenied,
    /// `start`/`stop`/`status` 대상 미등록(`uninstall` 은 idempotent no-op).
    #[error("서비스가 설치되어 있지 않음")]
    NotInstalled,
    /// 하위 seam I/O 실패(또는 스펙 검증 실패 — taxonomy 고정이라 검증 실패도 여기 매핑).
    #[error("서비스 I/O 실패: {0}")]
    Io(String),
}

/// per-OS 서비스 매니저 seam — `ServiceManager` 오케스트레이션이 호출하는 저수준 계약.
///
/// 프로덕션 구현은 `launchctl`/`systemctl`/`sc.exe` 로 shell-out 하는 얇은 best-effort 어댑터이며,
/// 테스트는 `testing::FakeController`(인메모리 등록/실행 상태)를 주입한다.
pub trait ServiceController: Send + Sync {
    /// 유닛을 등록한다(설치). 이미 등록되어 있으면 덮어쓰기(재등록)로 취급한다.
    fn register(&self, spec: &ServiceSpec) -> Result<(), ServiceError>;

    /// 유닛을 등록 해제한다. **미등록이어도 성공**(idempotent, R-SM-02).
    fn deregister(&self) -> Result<(), ServiceError>;

    /// 부팅/로그인 자동시작을 활성화한다(FR-21).
    fn enable_autostart(&self) -> Result<(), ServiceError>;

    /// 서비스를 시작한다. 이미 실행 중이면 no-op 성공. 미등록이면 `NotInstalled`.
    fn start(&self) -> Result<(), ServiceError>;

    /// 서비스를 정지한다. 이미 정지 상태면 no-op 성공. 미등록이면 `NotInstalled`.
    fn stop(&self) -> Result<(), ServiceError>;

    /// 현재 등록/실행 상태를 질의한다. 미등록이면 `NotInstalled`.
    fn query(&self) -> Result<ServiceRegistration, ServiceError>;
}

/// 3-OS 동일 계약 서비스 수명주기 오케스트레이터. per-OS 차이는 주입된
/// `ServiceController` 뒤에 격리된다(OS-불변 상위 로직).
pub struct ServiceManager {
    controller: Box<dyn ServiceController>,
}

impl ServiceManager {
    /// 주입된 `ServiceController` 로 매니저를 구성한다.
    pub fn new(controller: Box<dyn ServiceController>) -> Self {
        ServiceManager { controller }
    }

    /// 설치 — 스펙 검증(R-SM-01) 후 등록하고, `autostart` 이면 자동시작을 활성화한다.
    pub fn install(&self, spec: &ServiceSpec) -> Result<(), ServiceError> {
        validate_spec(spec)?;
        self.controller.register(spec)?;
        if spec.autostart {
            self.controller.enable_autostart()?;
        }
        Ok(())
    }

    /// 제거(등록 해제) — **미등록이어도 성공**(idempotent, R-SM-02).
    pub fn uninstall(&self) -> Result<(), ServiceError> {
        self.controller.deregister()
    }

    /// 시작 — 미등록이면 `NotInstalled`.
    pub fn start(&self) -> Result<(), ServiceError> {
        self.controller.start()
    }

    /// 정지 — 미등록이면 `NotInstalled`.
    pub fn stop(&self) -> Result<(), ServiceError> {
        self.controller.stop()
    }

    /// 재기동 — `stop()` 후 `start()`(AutoUpdater 재기동 진입점, R-AU-02/D-U7A-12).
    pub fn restart(&self) -> Result<(), ServiceError> {
        self.controller.stop()?;
        self.controller.start()
    }

    /// 상태 질의 — `ServiceRegistration`(R-SM-03 불변식 만족).
    pub fn status(&self) -> Result<ServiceRegistration, ServiceError> {
        self.controller.query()
    }
}

/// `ServiceSpec` 검증(R-SM-01): `exec_path`/`working_dir` 는 절대 경로여야 한다.
///
/// taxonomy(D-U7A-11)에 전용 Validation 변형이 없으므로 검증 실패는 `Io(detail)` 로 매핑한다.
fn validate_spec(spec: &ServiceSpec) -> Result<(), ServiceError> {
    if !spec.exec_path.is_absolute() {
        return Err(ServiceError::Io(
            "exec_path 는 절대 경로여야 합니다".to_string(),
        ));
    }
    if !spec.working_dir.is_absolute() {
        return Err(ServiceError::Io(
            "working_dir 는 절대 경로여야 합니다".to_string(),
        ));
    }
    Ok(())
}

/// 현재 OS 에 맞는 프로덕션 `ServiceController` 를 생성한다(미지원 OS 는 `UnsupportedPlatform`).
///
/// 반환된 어댑터는 `launchctl`/`systemctl`/`sc.exe` 로 shell-out 하는 얇은 best-effort 구현이며
/// 특권을 요구할 수 있다(root/관리자). 단위테스트는 이 함수 대신 fake 를 주입한다.
pub fn native_controller() -> Result<Box<dyn ServiceController>, ServiceError> {
    match std::env::consts::OS {
        "macos" => Ok(Box::new(LaunchdController)),
        "linux" => Ok(Box::new(SystemdController)),
        "windows" => Ok(Box::new(WindowsScmController)),
        _ => Err(ServiceError::UnsupportedPlatform),
    }
}

// ---------------------------------------------------------------------------
// per-OS best-effort 어댑터 (untested; root/OS 의존 — Infra/통합테스트 이월).
// 공통 실행 헬퍼: 프로세스 실행 실패 -> Io, 비정상 종료 -> stderr 휴리스틱으로 Permission/Io 분류.
// ---------------------------------------------------------------------------

/// 명령을 실행하고 종료 상태를 `ServiceError` 로 매핑한다(성공 = exit 0).
fn run(cmd: &mut Command) -> Result<(), ServiceError> {
    match cmd.output() {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
            if stderr.contains("permission")
                || stderr.contains("denied")
                || stderr.contains("root")
                || stderr.contains("administrator")
            {
                Err(ServiceError::PermissionDenied)
            } else {
                Err(ServiceError::Io(format!(
                    "명령 실패(code={:?})",
                    out.status.code()
                )))
            }
        }
        Err(e) => Err(ServiceError::Io(e.to_string())),
    }
}

/// macOS launchd 어댑터 — 로그인 자동시작용 LaunchAgent plist(FR-21).
struct LaunchdController;

impl LaunchdController {
    fn plist_path() -> Option<PathBuf> {
        std::env::var_os("HOME").map(|home| {
            PathBuf::from(home)
                .join("Library/LaunchAgents")
                .join(format!("{SERVICE_LABEL}.plist"))
        })
    }

    fn plist_contents(spec: &ServiceSpec) -> String {
        let exec = spec.exec_path.display();
        let wd = spec.working_dir.display();
        let run_at_load = if spec.autostart { "true" } else { "false" };
        // config_path 가 있으면 `run <config>` 인자를 추가해 데몬을 그 config 에 바인딩한다(LIR-H1).
        // 각 인자는 개별 `<string>` 이라 경로 내 공백(예: macOS Application Support)도 안전하다.
        let program_args = match &spec.config_path {
            Some(config) => format!(
                "<string>{exec}</string><string>run</string><string>{}</string>",
                config.display()
            ),
            None => format!("<string>{exec}</string>"),
        };
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <plist version=\"1.0\"><dict>\n\
             <key>Label</key><string>{SERVICE_LABEL}</string>\n\
             <key>ProgramArguments</key><array>{program_args}</array>\n\
             <key>WorkingDirectory</key><string>{wd}</string>\n\
             <key>RunAtLoad</key><{run_at_load}/>\n\
             <key>KeepAlive</key><true/>\n\
             </dict></plist>\n"
        )
    }
}

impl ServiceController for LaunchdController {
    fn register(&self, spec: &ServiceSpec) -> Result<(), ServiceError> {
        let path = Self::plist_path().ok_or_else(|| ServiceError::Io("HOME 미설정".to_string()))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ServiceError::Io(e.to_string()))?;
        }
        std::fs::write(&path, Self::plist_contents(spec))
            .map_err(|e| ServiceError::Io(e.to_string()))?;
        run(Command::new("launchctl").arg("load").arg("-w").arg(&path))
    }

    fn deregister(&self) -> Result<(), ServiceError> {
        // idempotent: plist 부재/미로드도 성공으로 취급한다(best-effort unload + 파일 제거).
        if let Some(path) = Self::plist_path() {
            let _ = Command::new("launchctl")
                .arg("unload")
                .arg("-w")
                .arg(&path)
                .output();
            match std::fs::remove_file(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(ServiceError::Io(e.to_string())),
            }
        }
        Ok(())
    }

    fn enable_autostart(&self) -> Result<(), ServiceError> {
        // RunAtLoad 는 plist 에 기록되고 `load -w` 로 활성화됨 -> 추가 조치 불필요(no-op 성공).
        Ok(())
    }

    fn start(&self) -> Result<(), ServiceError> {
        run(Command::new("launchctl").arg("start").arg(SERVICE_LABEL))
    }

    fn stop(&self) -> Result<(), ServiceError> {
        run(Command::new("launchctl").arg("stop").arg(SERVICE_LABEL))
    }

    fn query(&self) -> Result<ServiceRegistration, ServiceError> {
        let registered = Self::plist_path().map(|p| p.exists()).unwrap_or(false);
        if !registered {
            return Err(ServiceError::NotInstalled);
        }
        // best-effort: `launchctl list <label>` 성공 시 등록/실행 추정(정밀 PID 파싱은 이연).
        let running = Command::new("launchctl")
            .arg("list")
            .arg(SERVICE_LABEL)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        Ok(ServiceRegistration {
            registered: true,
            running,
            pid: None,
        })
    }
}

/// Linux systemd 어댑터 — 로그인 자동시작용 user unit(FR-21, root 불요 경로).
struct SystemdController;

impl SystemdController {
    fn unit_name() -> String {
        format!("{SERVICE_LABEL}.service")
    }

    fn unit_path() -> Option<PathBuf> {
        std::env::var_os("HOME").map(|home| {
            PathBuf::from(home)
                .join(".config/systemd/user")
                .join(Self::unit_name())
        })
    }

    fn unit_contents(spec: &ServiceSpec) -> String {
        let wd = spec.working_dir.display();
        // config_path 가 있으면 `run <config>` 로 데몬을 그 config 에 바인딩한다(LIR-H1).
        let exec_start = match &spec.config_path {
            Some(config) => format!("{} run {}", spec.exec_path.display(), config.display()),
            None => spec.exec_path.display().to_string(),
        };
        format!(
            "[Unit]\nDescription=okc-hooks Watcher\n\n\
             [Service]\nExecStart={exec_start}\nWorkingDirectory={wd}\nRestart=on-failure\n\n\
             [Install]\nWantedBy=default.target\n"
        )
    }
}

impl ServiceController for SystemdController {
    fn register(&self, spec: &ServiceSpec) -> Result<(), ServiceError> {
        let path = Self::unit_path().ok_or_else(|| ServiceError::Io("HOME 미설정".to_string()))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ServiceError::Io(e.to_string()))?;
        }
        std::fs::write(&path, Self::unit_contents(spec))
            .map_err(|e| ServiceError::Io(e.to_string()))?;
        run(Command::new("systemctl").arg("--user").arg("daemon-reload"))
    }

    fn deregister(&self) -> Result<(), ServiceError> {
        // idempotent: 미등록 유닛의 disable/stop 실패는 무시하고 파일만 제거한다.
        let name = Self::unit_name();
        let _ = Command::new("systemctl")
            .arg("--user")
            .arg("disable")
            .arg("--now")
            .arg(&name)
            .output();
        if let Some(path) = Self::unit_path() {
            match std::fs::remove_file(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(ServiceError::Io(e.to_string())),
            }
        }
        let _ = Command::new("systemctl")
            .arg("--user")
            .arg("daemon-reload")
            .output();
        Ok(())
    }

    fn enable_autostart(&self) -> Result<(), ServiceError> {
        run(Command::new("systemctl")
            .arg("--user")
            .arg("enable")
            .arg(Self::unit_name()))
    }

    fn start(&self) -> Result<(), ServiceError> {
        run(Command::new("systemctl")
            .arg("--user")
            .arg("start")
            .arg(Self::unit_name()))
    }

    fn stop(&self) -> Result<(), ServiceError> {
        run(Command::new("systemctl")
            .arg("--user")
            .arg("stop")
            .arg(Self::unit_name()))
    }

    fn query(&self) -> Result<ServiceRegistration, ServiceError> {
        let registered = Self::unit_path().map(|p| p.exists()).unwrap_or(false);
        if !registered {
            return Err(ServiceError::NotInstalled);
        }
        let running = Command::new("systemctl")
            .arg("--user")
            .arg("is-active")
            .arg("--quiet")
            .arg(Self::unit_name())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        Ok(ServiceRegistration {
            registered: true,
            running,
            pid: None,
        })
    }
}

/// Windows Service Control Manager 어댑터 — `sc.exe`(관리자 특권 필요).
struct WindowsScmController;

impl ServiceController for WindowsScmController {
    fn register(&self, spec: &ServiceSpec) -> Result<(), ServiceError> {
        // config_path 가 있으면 `run <config>` 로 데몬을 그 config 에 바인딩한다(LIR-H1).
        let bin = match &spec.config_path {
            Some(config) => format!(
                "binPath= {} run {}",
                spec.exec_path.display(),
                config.display()
            ),
            None => format!("binPath= {}", spec.exec_path.display()),
        };
        run(Command::new("sc.exe")
            .arg("create")
            .arg(SERVICE_LABEL)
            .arg(bin))
    }

    fn deregister(&self) -> Result<(), ServiceError> {
        // idempotent: 미등록(존재하지 않는 서비스)의 delete 실패는 목표 상태로 취급해 성공 처리.
        let _ = Command::new("sc.exe")
            .arg("stop")
            .arg(SERVICE_LABEL)
            .output();
        let _ = Command::new("sc.exe")
            .arg("delete")
            .arg(SERVICE_LABEL)
            .output();
        Ok(())
    }

    fn enable_autostart(&self) -> Result<(), ServiceError> {
        run(Command::new("sc.exe")
            .arg("config")
            .arg(SERVICE_LABEL)
            .arg("start=")
            .arg("auto"))
    }

    fn start(&self) -> Result<(), ServiceError> {
        run(Command::new("sc.exe").arg("start").arg(SERVICE_LABEL))
    }

    fn stop(&self) -> Result<(), ServiceError> {
        run(Command::new("sc.exe").arg("stop").arg(SERVICE_LABEL))
    }

    fn query(&self) -> Result<ServiceRegistration, ServiceError> {
        let out = Command::new("sc.exe")
            .arg("query")
            .arg(SERVICE_LABEL)
            .output()
            .map_err(|e| ServiceError::Io(e.to_string()))?;
        if !out.status.success() {
            return Err(ServiceError::NotInstalled);
        }
        let text = String::from_utf8_lossy(&out.stdout);
        let running = text.contains("RUNNING");
        Ok(ServiceRegistration {
            registered: true,
            running,
            pid: None,
        })
    }
}

/// `Path` 절대성 헬퍼(외부에서 스펙 조립 시 사전 검증에 재사용 가능).
pub fn is_absolute_path(p: &Path) -> bool {
    p.is_absolute()
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
    use crate::testing::FakeController;

    fn abs(p: &str) -> PathBuf {
        // 테스트 결정성을 위해 OS 무관 절대 경로를 만든다.
        if cfg!(windows) {
            PathBuf::from(format!("C:\\okc{}", p.replace('/', "\\")))
        } else {
            PathBuf::from(format!("/okc{p}"))
        }
    }

    #[test]
    fn install_validates_absolute_paths() {
        let mgr = ServiceManager::new(Box::new(FakeController::empty()));
        let bad = ServiceSpec::new(PathBuf::from("relative/bin"), abs("/wd"));
        assert!(matches!(mgr.install(&bad), Err(ServiceError::Io(_))));
    }

    #[test]
    fn install_registers_and_enables_autostart() {
        let ctrl = FakeController::empty();
        let handle = ctrl.clone();
        let mgr = ServiceManager::new(Box::new(ctrl));
        let spec = ServiceSpec::new(abs("/bin/watcher"), abs("/data"));
        mgr.install(&spec).unwrap();
        let reg = mgr.status().unwrap();
        assert!(reg.registered);
        assert!(handle.autostart_enabled());
    }

    #[test]
    fn uninstall_is_idempotent_even_when_absent() {
        let mgr = ServiceManager::new(Box::new(FakeController::empty()));
        // 미등록 상태에서 uninstall 반복 -> 항상 Ok(R-SM-02).
        assert!(mgr.uninstall().is_ok());
        assert!(mgr.uninstall().is_ok());
        assert!(matches!(mgr.status(), Err(ServiceError::NotInstalled)));
    }

    #[test]
    fn start_stop_when_not_installed_is_not_installed() {
        let mgr = ServiceManager::new(Box::new(FakeController::empty()));
        assert!(matches!(mgr.start(), Err(ServiceError::NotInstalled)));
        assert!(matches!(mgr.stop(), Err(ServiceError::NotInstalled)));
    }

    #[test]
    fn restart_stops_then_starts() {
        let ctrl = FakeController::empty();
        let handle = ctrl.clone();
        let mgr = ServiceManager::new(Box::new(ctrl));
        mgr.install(&ServiceSpec::new(abs("/bin/w"), abs("/data")))
            .unwrap();
        mgr.start().unwrap();
        assert!(mgr.status().unwrap().running);
        mgr.restart().unwrap();
        assert!(mgr.status().unwrap().running);
        assert!(handle.restart_count() >= 1);
    }

    #[test]
    fn status_invariants_hold() {
        let ctrl = FakeController::running(4321);
        let mgr = ServiceManager::new(Box::new(ctrl));
        let reg = mgr.status().unwrap();
        // running -> registered, pid.is_some() -> running (R-SM-03).
        assert!(!reg.running || reg.registered);
        assert!(reg.pid.is_none() || reg.running);
    }

    #[test]
    fn plist_binds_config_when_set() {
        // config_path 지정 시 launchd plist 는 `run <config>` 인자로 데몬을 바인딩한다(LIR-H1).
        let cfg = abs("/cfg/config.json");
        let spec = ServiceSpec::new(abs("/bin/watcher"), abs("/data")).with_config_path(cfg.clone());
        let plist = LaunchdController::plist_contents(&spec);
        assert!(plist.contains("<string>run</string>"));
        assert!(plist.contains(&format!("<string>{}</string>", cfg.display())));
    }

    #[test]
    fn plist_omits_run_arg_when_config_absent() {
        // config_path 미지정 시 기존 install 동작(인자 없는 기동, 기본 발견)을 유지한다.
        let spec = ServiceSpec::new(abs("/bin/watcher"), abs("/data"));
        let plist = LaunchdController::plist_contents(&spec);
        assert!(!plist.contains("<string>run</string>"));
    }

    #[test]
    fn systemd_unit_binds_config_when_set() {
        let cfg = abs("/cfg/config.json");
        let spec = ServiceSpec::new(abs("/bin/watcher"), abs("/data")).with_config_path(cfg.clone());
        let unit = SystemdController::unit_contents(&spec);
        assert!(unit.contains(&format!("ExecStart={} run {}", abs("/bin/watcher").display(), cfg.display())));
    }

    #[test]
    fn systemd_unit_omits_run_arg_when_config_absent() {
        let spec = ServiceSpec::new(abs("/bin/watcher"), abs("/data"));
        let unit = SystemdController::unit_contents(&spec);
        assert!(unit.contains(&format!("ExecStart={}\n", abs("/bin/watcher").display())));
    }
}

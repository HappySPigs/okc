//! `watcher-bin` 바이너리 진입점 — 데몬 실행 또는 운영자 CLI 로 분기하는 얇은 디스패치(DEC-U8-09).
//!
//! 첫 인자가 `run`(또는 없음)이면 `WatcherDaemon::run` 으로 데몬을 전경 실행하고, 그 외
//! 서브커맨드(status/health/pause/.../install/uninstall)는 U7b `dispatch_args` 로 데몬 IPC/서비스
//! 조작에 라우팅한다. 조립 로직 외 도메인 로직은 담지 않는다(RESILIENCY-01 단일 배포 바이너리).

use std::path::PathBuf;
use std::process::ExitCode;

use ops_control::{IpcControlClient, NativeServiceOps, dispatch_args, native_connector};
use watcher_bin::config::load_runtime_config;
use watcher_bin::daemon::WatcherDaemon;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        None | Some("run") => run_daemon(args.get(1).cloned().map(PathBuf::from)),
        Some(_) => run_cli(&args),
    };
    ExitCode::from(code)
}

/// 데몬을 전경 실행하고 `DaemonError` 를 프로세스 종료코드로 사상한다(성공 0 / 실패 1).
fn run_daemon(config_path: Option<PathBuf>) -> u8 {
    match WatcherDaemon::run(config_path) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("watcher: {error}");
            1
        }
    }
}

/// 운영자 CLI 명령을 데몬 IPC/서비스 조작으로 라우팅한다(health -> 종료코드 매핑은 U7b 소유).
fn run_cli(args: &[String]) -> u8 {
    let loaded = match load_runtime_config(None) {
        Ok(loaded) => loaded,
        Err(error) => {
            eprintln!("watcher: {error}");
            return 2;
        }
    };
    let client = IpcControlClient::new(native_connector(), loaded.federated.ipc_endpoint.clone());
    let exec_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("watcher"));
    let service = NativeServiceOps::new(
        exec_path,
        loaded.federated.data_dir.clone(),
        PathBuf::from(loaded.core.vault_path.clone()),
    );
    let result = dispatch_args(args, &client, &service);
    println!("{}", result.output);
    result.exit_code.0
}

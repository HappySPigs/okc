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
use watcher_bin::setup::{run_setup, user_config_path};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        None | Some("run") => run_daemon(args.get(1).cloned().map(PathBuf::from)),
        Some("setup") => run_setup_cmd(&args),
        Some(_) => run_cli(&args),
    };
    ExitCode::from(code)
}

/// `setup --config <source>` 를 처리한다(config 로드 이전 분기 — 사전 config 부재에서도 동작).
///
/// 사용자 작성 config 를 검증·배치(0600)하고 자동시작 서비스를 그 config 에 바인딩한다(LIR-H1..H3).
/// 성공 0 / 오류 1. 토큰은 어떤 출력에도 노출되지 않는다(SECURITY-03).
fn run_setup_cmd(args: &[String]) -> u8 {
    let source = match parse_setup_source(args) {
        Ok(source) => source,
        Err(message) => {
            eprintln!("watcher: {message}");
            return 2;
        }
    };
    match run_setup(&source) {
        Ok(report) => {
            println!(
                "설정 완료: config 를 {} 에 기록(권한 0600)하고 자동시작 서비스를 등록했습니다. 엔드포인트: {}",
                report.dest_path.display(),
                report.server_endpoint
            );
            0
        }
        Err(error) => {
            eprintln!("watcher: setup 실패: {error}");
            1
        }
    }
}

/// `setup` 인자에서 `--config <source>` 소스 경로를 파싱한다(고정 최소 파서).
fn parse_setup_source(args: &[String]) -> Result<PathBuf, String> {
    let mut iter = args.iter().skip(1); // 서브커맨드("setup") 건너뜀
    let mut source: Option<PathBuf> = None;
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--config" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--config 는 소스 config 경로가 필요합니다".to_string())?;
                source = Some(PathBuf::from(value));
            }
            other => return Err(format!("알 수 없는 setup 인자: {other}\n사용법: watcher setup --config <소스-config-경로>")),
        }
    }
    source.ok_or_else(|| "사용법: watcher setup --config <소스-config-경로>".to_string())
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
///
/// 코어 발견 기본값(`/etc/...`)에서 config 로드가 실패하면 `setup` 이 기록한 사용자 레벨 경로로
/// 폴백한다 — `setup` 이후 `OKC_WATCHER_CONFIG` 없이도 uninstall/status 가 동작하게 한다(LIR-H4).
fn run_cli(args: &[String]) -> u8 {
    let loaded = match load_runtime_config(None) {
        Ok(loaded) => loaded,
        Err(primary) => match load_runtime_config(Some(&user_config_path())) {
            Ok(loaded) => loaded,
            Err(_) => {
                eprintln!("watcher: {primary}");
                return 2;
            }
        },
    };
    let client = IpcControlClient::new(native_connector(), loaded.federated.ipc_endpoint.clone());
    let exec_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("watcher"));
    let service = NativeServiceOps::new(
        exec_path,
        loaded.federated.data_dir.clone(),
        PathBuf::from(loaded.core.vault_path.clone()),
        loaded.config_path.clone(),
    );
    let result = dispatch_args(args, &client, &service);
    println!("{}", result.output);
    result.exit_code.0
}

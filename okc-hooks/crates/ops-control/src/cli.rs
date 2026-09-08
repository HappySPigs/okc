//! `OperatorCli` — 고정 서브커맨드 파싱 + 라우팅 이분법 + health 종료코드 매핑.
//!
//! 파싱된 `Command` 는 **정확히 두 경로 중 하나로** 디스패치된다(R-U7B-01, PROP-U7B-07):
//! - 데몬 대상(status/health/pause/resume/sync-now/stop/history/consent/reload) -> `ControlOp` 로
//!   사상해 `ControlClient`(IPC) 로 전송.
//! - 서비스 수명주기(install/uninstall) -> U7a `ServiceManager`/`Uninstaller` 를 in-process 로
//!   직접 호출(IPC 우회, 데몬 미기동에서도 동작).
//!
//! `health` 종료코드는 `Healthy -> 0`, `Unhealthy -> 1`, 데몬 미도달/전송 실패 -> `2` 로
//! 전수·배타 매핑된다(R-U7B-04, US-E5-02, PROP-U7B-06). CLI 파싱은 std 로 손수 구현하며 고정
//! 서브커맨드 집합만 다룬다(신규 외부 크레이트 없음).
//!
//! 프로세스/서비스 I/O 를 seam 뒤에서 수행하므로 순수 lint-gate 는 두지 않는다(단, 어떤 경로에서도
//! `unwrap`/`expect`/`panic` 을 쓰지 않는다). `gen` 은 예약어이므로 식별자로 쓰지 않는다.

use std::path::PathBuf;

use foundation::{Health, Timestamp};
use lifecycle_deploy::{
    Artifact, ArtifactKind, ArtifactSet, ServiceManager, ServiceSpec, StdFileSystem, Uninstaller,
    UninstallOptions, UnsupportedTokenPurge, native_controller,
};

use crate::control_plane::ControlClient;
use crate::ipc::IpcError;
use crate::protocol::{
    ConsentOp, ControlOp, ControlRequest, ControlResponse, ControlResult, HistoryFilter,
    PROTO_VERSION, StopMode,
};

/// 프로세스 종료코드(0 = healthy/성공, 1 = unhealthy/도메인 오류, 2 = 데몬 미도달/전송/파싱 오류).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitCode(pub u8);

/// 파싱된 서브커맨드(U7b 소유). `Status..Reload` 는 IPC 경로, `Install`/`Uninstall` 은 U7a 직접 경로.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// 상태 조회.
    Status,
    /// 건강 조회(종료코드 매핑).
    Health,
    /// 감시 일시중지.
    Pause,
    /// 감시 재개.
    Resume,
    /// 즉시 동기화.
    SyncNow,
    /// 종료 요청.
    Stop(StopMode),
    /// 히스토리 조회.
    History(HistoryFilter),
    /// 동의 제어.
    Consent(ConsentOp),
    /// config 리로드.
    Reload,
    /// 서비스 설치(U7a 직접 호출).
    Install,
    /// 서비스 제거(U7a 직접 호출).
    Uninstall(UninstallOptions),
}

/// 파싱 결과(명령 + `--json` 플래그).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliArgs {
    /// 파싱된 명령.
    pub command: Command,
    /// `--json` 기계판독 출력(status/health/history).
    pub json: bool,
}

/// 실행 결과(렌더 출력 + 종료코드).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliResult {
    /// 사람용 또는 `--json` 렌더 결과.
    pub output: String,
    /// 종료코드.
    pub exit_code: ExitCode,
}

/// 서비스 수명주기 조작 seam — install/uninstall 을 U7a 로 위임한다(테스트는 기록 fake 주입).
pub trait ServiceOps {
    /// 서비스를 설치한다. 사람이 읽는 결과 문자열 또는 오류 문자열.
    fn install(&self) -> Result<String, String>;
    /// 서비스를 제거한다. 사람이 읽는 결과 문자열 또는 오류 문자열.
    fn uninstall(&self, opts: &UninstallOptions) -> Result<String, String>;
}

/// 프로덕션 서비스 조작 — U7a `ServiceManager`/`Uninstaller` 를 in-process 로 구성·호출한다.
pub struct NativeServiceOps {
    /// 배포 바이너리 절대 경로(U8: `current_exe` 유래).
    exec_path: PathBuf,
    /// 데몬 작업/데이터 디렉터리(아티팩트 경로 관례 해소 기준).
    data_dir: PathBuf,
    /// 볼트 루트 — 삭제 이중 방어 기준(하위는 절대 삭제 안 함).
    vault_root: PathBuf,
}

impl NativeServiceOps {
    /// 조립 루트(U8)가 해소한 경로로 구성한다.
    pub fn new(exec_path: PathBuf, data_dir: PathBuf, vault_root: PathBuf) -> Self {
        NativeServiceOps {
            exec_path,
            data_dir,
            vault_root,
        }
    }
}

impl ServiceOps for NativeServiceOps {
    fn install(&self) -> Result<String, String> {
        let controller = native_controller().map_err(|err| err.to_string())?;
        let manager = ServiceManager::new(controller);
        let spec = ServiceSpec::new(self.exec_path.clone(), self.data_dir.clone());
        manager.install(&spec).map_err(|err| err.to_string())?;
        Ok("서비스가 설치되었습니다".to_string())
    }

    fn uninstall(&self, opts: &UninstallOptions) -> Result<String, String> {
        let controller = native_controller().map_err(|err| err.to_string())?;
        let manager = ServiceManager::new(controller);
        let uninstaller = Uninstaller::new(
            manager,
            Box::new(StdFileSystem),
            Box::new(UnsupportedTokenPurge),
        );
        let artifacts = assemble_artifact_set(&self.data_dir);
        match uninstaller.uninstall(opts, artifacts, &self.vault_root) {
            Ok(report) => Ok(format!(
                "제거 완료: {}개 삭제, {}개 skip",
                report.removed.len(),
                report.skipped.len()
            )),
            Err(err) => Err(err.to_string()),
        }
    }
}

/// data-dir 관례로 정리 대상 아티팩트 집합을 조립한다(vault_root 하위 배제는 U7a 가 재검사, R-U7B-02).
fn assemble_artifact_set(data_dir: &std::path::Path) -> ArtifactSet {
    ArtifactSet(vec![
        Artifact {
            kind: ArtifactKind::UploadHistory,
            path: data_dir.join("history.cbor"),
        },
        Artifact {
            kind: ArtifactKind::SyncState,
            path: data_dir.join("sync-state.cbor"),
        },
        Artifact {
            kind: ArtifactKind::Log,
            path: data_dir.join("watcher.log"),
        },
        Artifact {
            kind: ArtifactKind::ConfigToken,
            path: data_dir.join("config.json"),
        },
    ])
}

/// 데몬 대상 `Command` 를 `ControlOp` 로 사상한다. install/uninstall 은 대응이 없어 `None`.
pub fn to_control_op(command: &Command) -> Option<ControlOp> {
    Some(match command {
        Command::Status => ControlOp::Status,
        Command::Health => ControlOp::Health,
        Command::Pause => ControlOp::Pause,
        Command::Resume => ControlOp::Resume,
        Command::SyncNow => ControlOp::SyncNow,
        Command::Stop(mode) => ControlOp::Stop(*mode),
        Command::History(filter) => ControlOp::History(*filter),
        Command::Consent(op) => ControlOp::Consent(*op),
        Command::Reload => ControlOp::Reload,
        Command::Install | Command::Uninstall(_) => return None,
    })
}

/// 고정 서브커맨드 집합을 파싱한다(프로그램명 제외한 인자 슬라이스). 알 수 없는 입력은 사용법
/// 문자열을 `Err` 로 반환하며(종료코드 2 매핑) 파싱은 패닉하지 않는다.
pub fn parse(args: &[String]) -> Result<CliArgs, String> {
    let mut json = false;
    let mut immediate = false;
    let mut purge_token = false;
    let mut purge_logs = false;
    let mut keep_service = false;
    let mut since: Option<i64> = None;
    let mut only_failures: Option<bool> = None;
    let mut words: Vec<String> = Vec::new();

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--json" => json = true,
            "--immediate" => immediate = true,
            "--purge-token" => purge_token = true,
            "--purge-logs" => purge_logs = true,
            "--keep-service" => keep_service = true,
            "--since" => {
                let value = iter.next().ok_or_else(|| usage("--since 는 값(unix nanos)이 필요합니다"))?;
                let parsed = value
                    .parse::<i64>()
                    .map_err(|_| usage("--since 는 정수(unix nanos)여야 합니다"))?;
                since = Some(parsed);
            }
            "--status" => {
                let value = iter.next().ok_or_else(|| usage("--status 는 failed|success 가 필요합니다"))?;
                only_failures = Some(match value.as_str() {
                    "failed" => true,
                    "success" => false,
                    other => return Err(usage(&format!("--status 값이 잘못됨: {other}"))),
                });
            }
            other => words.push(other.to_string()),
        }
    }

    let subcommand = words.first().ok_or_else(|| usage("서브커맨드가 필요합니다"))?;
    let command = match subcommand.as_str() {
        "status" => Command::Status,
        "health" => Command::Health,
        "pause" => Command::Pause,
        "resume" => Command::Resume,
        "sync-now" => Command::SyncNow,
        "stop" => Command::Stop(if immediate {
            StopMode::Immediate
        } else {
            StopMode::Graceful
        }),
        "history" => Command::History(HistoryFilter {
            since: since.map(Timestamp::from_unix_nanos),
            only_failures,
        }),
        "consent" => {
            let action = words
                .get(1)
                .ok_or_else(|| usage("consent <view|grant|withdraw|acknowledge>"))?;
            let op = match action.as_str() {
                "view" => ConsentOp::View,
                "grant" => ConsentOp::Grant,
                "withdraw" => ConsentOp::Withdraw,
                "acknowledge" => ConsentOp::Acknowledge,
                other => return Err(usage(&format!("알 수 없는 consent 동작: {other}"))),
            };
            Command::Consent(op)
        }
        "reload" => Command::Reload,
        "install" => Command::Install,
        "uninstall" => Command::Uninstall(UninstallOptions {
            purge_token,
            purge_logs,
            deregister_service: !keep_service,
        }),
        other => return Err(usage(&format!("알 수 없는 서브커맨드: {other}"))),
    };

    Ok(CliArgs { command, json })
}

/// 사용법 문자열을 조립한다(오류 사유 prefix + 서브커맨드 목록).
fn usage(reason: &str) -> String {
    format!(
        "{reason}\n사용법: watcher <status|health|pause|resume|sync-now|stop|history|consent|reload|install|uninstall> [--json] [--immediate] [--since <nanos>] [--status <failed|success>] [--purge-token] [--purge-logs] [--keep-service]"
    )
}

/// 명령을 라우팅해 실행한다(라우팅 이분법, R-U7B-01). install/uninstall 은 IPC 를 건드리지 않는다.
pub fn run(args: &CliArgs, client: &dyn ControlClient, service: &dyn ServiceOps) -> CliResult {
    match &args.command {
        Command::Install => render_service(service.install()),
        Command::Uninstall(opts) => render_service(service.uninstall(opts)),
        command => {
            let op = match to_control_op(command) {
                Some(op) => op,
                // 방어 경로: 이 분기는 install/uninstall 을 제외하므로 도달하지 않는다.
                None => {
                    return CliResult {
                        output: "알 수 없는 명령".to_string(),
                        exit_code: ExitCode(2),
                    };
                }
            };
            let response = client.request(ControlRequest {
                proto_version: PROTO_VERSION,
                op,
            });
            match command {
                Command::Health => map_health(&response),
                _ => render_daemon(args.json, &response),
            }
        }
    }
}

/// 파싱 -> 라우팅을 한 번에 수행한다(파싱 오류는 사용법 + 종료코드 2). 출력/종료는 호출부(U8)가 담당.
pub fn dispatch_args(
    raw_args: &[String],
    client: &dyn ControlClient,
    service: &dyn ServiceOps,
) -> CliResult {
    match parse(raw_args) {
        Ok(args) => run(&args, client, service),
        Err(message) => CliResult {
            output: message,
            exit_code: ExitCode(2),
        },
    }
}

/// health 응답 -> 종료코드 매핑(R-U7B-04, US-E5-02). 전수·배타(PROP-U7B-06).
pub fn map_health(response: &Result<ControlResponse, IpcError>) -> CliResult {
    match response {
        Ok(resp) => match &resp.result {
            ControlResult::Health(Health::Healthy) => CliResult {
                output: "healthy".to_string(),
                exit_code: ExitCode(0),
            },
            ControlResult::Health(Health::Unhealthy { reasons }) => {
                let joined = reasons
                    .iter()
                    .map(|reason| reason.0.clone())
                    .collect::<Vec<String>>()
                    .join(", ");
                CliResult {
                    output: format!("unhealthy: {joined}"),
                    exit_code: ExitCode(1),
                }
            }
            ControlResult::Error(err) => CliResult {
                output: format!("오류: {err:?}"),
                exit_code: ExitCode(1),
            },
            other => CliResult {
                output: format!("예상치 못한 응답: {other:?}"),
                exit_code: ExitCode(1),
            },
        },
        Err(IpcError::Connect) => CliResult {
            output: "데몬에 도달할 수 없습니다".to_string(),
            exit_code: ExitCode(2),
        },
        Err(other) => CliResult {
            output: format!("전송 오류: {other}"),
            exit_code: ExitCode(2),
        },
    }
}

/// health 를 제외한 데몬 명령 응답을 렌더한다(성공 0 / 도메인 오류 1 / 전송 실패 2).
fn render_daemon(json: bool, response: &Result<ControlResponse, IpcError>) -> CliResult {
    match response {
        Ok(resp) => match &resp.result {
            ControlResult::Error(err) => CliResult {
                output: format!("오류: {err:?}"),
                exit_code: ExitCode(1),
            },
            ControlResult::Status(snapshot) => CliResult {
                output: render_value(snapshot, json, &format!("{snapshot:?}")),
                exit_code: ExitCode(0),
            },
            ControlResult::History(records) => CliResult {
                output: render_value(records, json, &format!("{records:?}")),
                exit_code: ExitCode(0),
            },
            ControlResult::Consent(view) => CliResult {
                output: render_value(view, json, &format!("{view:?}")),
                exit_code: ExitCode(0),
            },
            ControlResult::Health(health) => CliResult {
                output: format!("{health:?}"),
                exit_code: ExitCode(0),
            },
            ControlResult::Ack => CliResult {
                output: "OK".to_string(),
                exit_code: ExitCode(0),
            },
        },
        Err(IpcError::Connect) => CliResult {
            output: "데몬에 도달할 수 없습니다".to_string(),
            exit_code: ExitCode(2),
        },
        Err(other) => CliResult {
            output: format!("전송 오류: {other}"),
            exit_code: ExitCode(2),
        },
    }
}

/// `--json` 이면 serde_json, 아니면 사람용 텍스트로 렌더한다(R-U7B-03).
fn render_value<T: serde::Serialize>(value: &T, json: bool, human: &str) -> String {
    if json {
        serde_json::to_string(value).unwrap_or_else(|_| human.to_string())
    } else {
        human.to_string()
    }
}

/// 서비스 조작 결과를 렌더한다(성공 0 / 실패 1).
fn render_service(result: Result<String, String>) -> CliResult {
    match result {
        Ok(output) => CliResult {
            output,
            exit_code: ExitCode(0),
        },
        Err(output) => CliResult {
            output,
            exit_code: ExitCode(1),
        },
    }
}

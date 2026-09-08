#![deny(missing_docs)]
//! `ops-control` 크레이트 (U7b) — Watcher 운영 제어 단위.
//!
//! 세 개의 응집 컴포넌트를 노출한다:
//! - `OperatorCli`(모듈 `cli`): 고정 서브커맨드 집합(status/health/pause/resume/sync-now/stop/
//!   history/consent/reload/install/uninstall)을 파싱·라우팅한다. 데몬 대상 명령은
//!   `ControlPlane` IPC 클라이언트로 전송하고, install/uninstall 은 U7a `ServiceManager`/
//!   `Uninstaller` 를 in-process 로 직접 호출한다(IPC 우회). `health` 는 종료코드로 매핑된다.
//! - `ControlPlane`(모듈 `control_plane`): 전송-불변 프로토콜 로직(길이-프리픽스 프레이밍 + U0
//!   CBOR 코덱 + 엄격 버전 검사 + 디스패치)을 `IpcListener`/`IpcConnector`/`IpcStream` seam
//!   (모듈 `ipc`) 뒤에서 구동한다. 실 구현은 `std::os::unix::net`(mac/Linux) Unix 도메인 소켓이며
//!   Windows 명명 파이프는 이연(스텁)이다.
//! - `RunStateController`(모듈 `run_state`): 권위 있는 run-state(running|paused + sync/stop 신호)를
//!   보유한다. `paused` 만 crash-atomic 하게 재시작-지속하고(U4 파이프라인 미러), pause/resume 를
//!   주입된 `Arc<dyn StatusSink>`(U6 `StatusService` 구현)에 반영한다. U8 이 `current()` 로
//!   읽으며 이 컨트롤러는 U8 을 호출하지 않는다(no back-reference).
//!
//! OS/소켓/프로세스는 모두 seam 트레이트 뒤에 격리되어 테스트가 특권/네트워크 없이 프로토콜/
//! 전이/라우팅 순수 로직을 결정적으로 검증한다.
//!
//! 규약: 모든 doc 주석은 한국어, 코드 내 화살표는 ASCII `A -> B` 만 사용,
//! Rust 타입/식별자는 doc 주석에서 백틱으로 감싼다(`Arc<dyn StatusSink>`).

/// `OperatorCli` — CLI 파싱/라우팅 + health 종료코드 매핑 + install/uninstall 직접 호출.
pub mod cli;
/// `ControlPlane` — 서버 accept 루프 + 프레이밍/버전/디스패치 + 클라이언트 요청.
pub mod control_plane;
/// IPC 경계 seam — `IpcEndpoint`/`IpcError` + `IpcListener`/`IpcConnector`/`IpcStream` + UDS 구현.
pub mod ipc;
/// 프로토콜 메시지 타입 — `ControlRequest`/`ControlResponse`/`ControlOp`/`ControlResult` 등(순수).
pub mod protocol;
/// `RunStateController` — 권위 run-state 전이 + crash-atomic 지속 + `StatusSink` 반영.
pub mod run_state;

// ---------------------------------------------------------------------------
// 공개 API 표면(crate-root 재-export). 재-export 된 아이템은 원본 doc 을 승계한다.
// ---------------------------------------------------------------------------
pub use cli::{
    CliArgs, CliResult, Command, ExitCode, NativeServiceOps, ServiceOps, dispatch_args, map_health,
    parse, run, to_control_op,
};
pub use control_plane::{
    ControlClient, ControlHandlers, ControlPlane, IpcControlClient, WatcherHandlers, dispatch,
    handle_request,
};
pub use ipc::{
    IpcConnector, IpcEndpoint, IpcError, IpcListener, IpcStream, MAX_FRAME_BYTES, bind_native,
    native_connector, read_frame_from, write_frame_to,
};
pub use protocol::{
    ConsentOp, ConsentView, ControlError, ControlOp, ControlRequest, ControlResponse,
    ControlResult, HistoryFilter, PROTO_VERSION, StopMode,
};
pub use run_state::{PersistedRunState, RunMode, RunState, RunStateController, RunStateError};

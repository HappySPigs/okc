//! `ControlPlane` — 전송-불변 프로토콜 로직(프레이밍/버전/디스패치) + 서버 accept 루프 +
//! 클라이언트 요청.
//!
//! `dispatch`/`handle_request` 는 `ControlHandlers` seam 뒤에서 동작하는 순수 로직이라 실 소켓/
//! 데몬 없이 결정적으로 테스트된다(PROP-U7B-01/03). 수신 `proto_version` 이 `PROTO_VERSION` 과
//! 다르면 어떤 핸들러도 호출하지 않고 `Error(VersionMismatch)` 를 반환한다(R-U7B-06). 도메인
//! 오류는 응답 프레임 안 `ControlResult::Error` 로, 전송/프레이밍/버전 실패만 `IpcError` 로
//! 표면화된다(R-U7B-09).
//!
//! 실 배선(`WatcherHandlers`)은 주입된 `Arc<StatusService>`/`Arc<UploadHistoryStore>`/
//! `Arc<ConsentGate>`/`Arc<ConfigProvider>`/`Arc<RunStateController>` 의 **실제 공개 API** 에만
//! 위임한다(R-U7B-08). 어떤 핸들러도 동결 크레이트 API 를 재정의하거나 U8 을 역참조하지 않는다.
//!
//! 소켓 I/O(서버 루프/클라이언트)를 수행하므로 순수 lint-gate 는 두지 않는다(단, 어떤 경로에서도
//! `unwrap`/`expect`/`panic` 을 쓰지 않는다).

use std::sync::Arc;

use foundation::{
    ConfigProvider, Health, ReadJudgment, StatusSnapshot, UploadHistoryRecord, decode, encode,
};
use observability::{HistoryQuery, StatusService, UploadHistoryStore};
use auth_consent::ConsentGate;

use crate::ipc::{IpcConnector, IpcEndpoint, IpcError, IpcListener, IpcStream};
use crate::protocol::{
    ConsentOp, ConsentView, ControlError, ControlOp, ControlRequest, ControlResponse,
    ControlResult, HistoryFilter, PROTO_VERSION, StopMode,
};
use crate::run_state::RunStateController;

/// 디스패치 핸들러 seam — `ControlPlane` 이 op 별로 위임하는 계약(테스트는 기록 fake 주입).
///
/// 부수효과 있는 메서드(pause/resume/sync_now/stop/consent 변경/reload)는 버전 검사 통과 후에만
/// 호출되어야 한다(R-U7B-06, PROP-U7B-03).
pub trait ControlHandlers: Send + Sync {
    /// 상태 스냅샷.
    fn status(&self) -> StatusSnapshot;
    /// 건강 판정.
    fn health(&self) -> Health;
    /// 히스토리 조회.
    fn history(&self, filter: HistoryFilter) -> Result<Vec<UploadHistoryRecord>, ControlError>;
    /// 감시 일시중지.
    fn pause(&self) -> Result<(), ControlError>;
    /// 감시 재개.
    fn resume(&self) -> Result<(), ControlError>;
    /// 즉시 동기화 요청.
    fn sync_now(&self);
    /// 종료 요청.
    fn stop(&self, mode: StopMode);
    /// consent 상태 조회.
    fn consent_view(&self) -> ConsentView;
    /// 상시 동의 부여.
    fn consent_grant(&self) -> Result<(), ControlError>;
    /// 동의 철회.
    fn consent_withdraw(&self) -> Result<(), ControlError>;
    /// 고지 확인.
    fn consent_acknowledge(&self) -> Result<(), ControlError>;
    /// config 리로드.
    fn reload(&self) -> Result<(), ControlError>;
}

/// op 를 대응 핸들러로 위임해 `ControlResult` 를 만든다(R-U7B-08 디스패치 테이블, 순수 로직).
pub fn dispatch<H: ControlHandlers + ?Sized>(handlers: &H, op: ControlOp) -> ControlResult {
    match op {
        ControlOp::Status => ControlResult::Status(handlers.status()),
        ControlOp::Health => ControlResult::Health(handlers.health()),
        ControlOp::History(filter) => match handlers.history(filter) {
            Ok(records) => ControlResult::History(records),
            Err(err) => ControlResult::Error(err),
        },
        ControlOp::Pause => ack_or_err(handlers.pause()),
        ControlOp::Resume => ack_or_err(handlers.resume()),
        ControlOp::SyncNow => {
            handlers.sync_now();
            ControlResult::Ack
        }
        ControlOp::Stop(mode) => {
            handlers.stop(mode);
            ControlResult::Ack
        }
        ControlOp::Consent(ConsentOp::View) => ControlResult::Consent(handlers.consent_view()),
        ControlOp::Consent(ConsentOp::Grant) => ack_or_err(handlers.consent_grant()),
        ControlOp::Consent(ConsentOp::Withdraw) => ack_or_err(handlers.consent_withdraw()),
        ControlOp::Consent(ConsentOp::Acknowledge) => ack_or_err(handlers.consent_acknowledge()),
        ControlOp::Reload => ack_or_err(handlers.reload()),
    }
}

/// 부수효과 결과를 `Ack` 또는 `Error(ControlError)` 로 사상한다.
fn ack_or_err(result: Result<(), ControlError>) -> ControlResult {
    match result {
        Ok(()) => ControlResult::Ack,
        Err(err) => ControlResult::Error(err),
    }
}

/// 요청을 처리해 응답을 만든다 — 버전 검사(디스패치 전)를 포함한다(R-U7B-06).
pub fn handle_request<H: ControlHandlers + ?Sized>(
    handlers: &H,
    request: ControlRequest,
) -> ControlResponse {
    let result = if request.proto_version != PROTO_VERSION {
        ControlResult::Error(ControlError::VersionMismatch {
            server: PROTO_VERSION,
            client: request.proto_version,
        })
    } else {
        dispatch(handlers, request.op)
    };
    ControlResponse {
        proto_version: PROTO_VERSION,
        result,
    }
}

/// 서버 제어면 — 주입 핸들러로 accept 루프/프레임 처리를 구동한다.
pub struct ControlPlane<H> {
    handlers: Arc<H>,
}

impl<H: ControlHandlers> ControlPlane<H> {
    /// 주입 핸들러로 서버를 구성한다.
    pub fn new(handlers: Arc<H>) -> Self {
        ControlPlane { handlers }
    }

    /// 한 연결의 1 요청/응답을 처리한다(프레임 read -> decode -> handle -> encode -> write).
    ///
    /// 디코드/인코드 실패는 패닉 없이 `Protocol` 로 표면화된다(R-U7B-05). 도메인 오류는 정상
    /// 응답 프레임 안에서 전달된다(R-U7B-09).
    pub fn handle_conn(&self, stream: &mut dyn IpcStream) -> Result<(), IpcError> {
        let bytes = stream.read_frame()?;
        let request: ControlRequest = decode(&bytes).map_err(|_| IpcError::Protocol)?;
        let response = handle_request(self.handlers.as_ref(), request);
        let payload = encode(&response).map_err(|_| IpcError::Protocol)?;
        stream.write_frame(&payload)
    }

    /// accept 루프 — 개별 연결 처리 실패는 삼키고 다음 연결로 진행한다(1개 나쁜 연결이 서버를
    /// 죽이지 않음, R-U7B-09 회복력). accept 자체 실패도 계속 진행한다.
    pub fn serve_forever(&self, listener: &dyn IpcListener) {
        loop {
            match listener.accept() {
                Ok(mut stream) => {
                    let _ = self.handle_conn(stream.as_mut());
                }
                Err(_transient) => continue,
            }
        }
    }
}

/// 실 배선 핸들러 — 주입된 동결 크레이트 구체 타입의 실제 공개 API 에만 위임한다(R-U7B-08).
pub struct WatcherHandlers {
    status: Arc<StatusService>,
    history: Arc<UploadHistoryStore>,
    consent: Arc<ConsentGate>,
    config: Arc<ConfigProvider>,
    run_state: Arc<RunStateController>,
}

impl WatcherHandlers {
    /// 조립 루트(U8)가 주입하는 핸들로 배선한다.
    pub fn new(
        status: Arc<StatusService>,
        history: Arc<UploadHistoryStore>,
        consent: Arc<ConsentGate>,
        config: Arc<ConfigProvider>,
        run_state: Arc<RunStateController>,
    ) -> Self {
        WatcherHandlers {
            status,
            history,
            consent,
            config,
            run_state,
        }
    }
}

impl ControlHandlers for WatcherHandlers {
    fn status(&self) -> StatusSnapshot {
        self.status.snapshot()
    }

    fn health(&self) -> Health {
        // U6 `StatusService` 의 `ReadJudgment` 구현.
        self.status.health_check()
    }

    fn history(&self, filter: HistoryFilter) -> Result<Vec<UploadHistoryRecord>, ControlError> {
        // 와이어 `HistoryFilter` 를 U6 `HistoryQuery` 로 재조립한다(동일 두 필드).
        let query = HistoryQuery {
            since: filter.since,
            only_failures: filter.only_failures,
        };
        self.history
            .query(query)
            .map_err(|err| ControlError::HistoryUnavailable(err.to_string()))
    }

    fn pause(&self) -> Result<(), ControlError> {
        self.run_state
            .pause()
            .map_err(|err| ControlError::RunState(err.to_string()))
    }

    fn resume(&self) -> Result<(), ControlError> {
        self.run_state
            .resume()
            .map_err(|err| ControlError::RunState(err.to_string()))
    }

    fn sync_now(&self) {
        self.run_state.request_sync_now();
    }

    fn stop(&self, mode: StopMode) {
        self.run_state.request_stop(mode);
    }

    fn consent_view(&self) -> ConsentView {
        let status = self.consent.view();
        ConsentView {
            consent: self.consent.consent_state(),
            acknowledged: status.acknowledged,
        }
    }

    fn consent_grant(&self) -> Result<(), ControlError> {
        self.consent
            .grant()
            .map_err(|err| ControlError::ConsentRejected(err.to_string()))
    }

    fn consent_withdraw(&self) -> Result<(), ControlError> {
        self.consent
            .withdraw()
            .map_err(|err| ControlError::ConsentRejected(err.to_string()))
    }

    fn consent_acknowledge(&self) -> Result<(), ControlError> {
        self.consent
            .acknowledge()
            .map_err(|err| ControlError::ConsentRejected(err.to_string()))
    }

    fn reload(&self) -> Result<(), ControlError> {
        self.config
            .reload()
            .map_err(|err| ControlError::ReloadFailed(err.to_string()))
    }
}

/// CLI 클라이언트 요청 seam — `OperatorCli` 가 데몬 대상 명령을 전송할 때 사용한다.
pub trait ControlClient {
    /// 요청을 전송하고 응답을 수신한다(전송/프레이밍 실패 -> `IpcError`).
    fn request(&self, request: ControlRequest) -> Result<ControlResponse, IpcError>;
}

/// IPC 기반 실 클라이언트 — 주입 커넥터로 연결해 1 요청/응답을 왕복한다.
pub struct IpcControlClient {
    connector: Box<dyn IpcConnector>,
    endpoint: IpcEndpoint,
}

impl IpcControlClient {
    /// 커넥터 + 엔드포인트로 클라이언트를 구성한다(엔드포인트는 U8 이 해소해 주입).
    pub fn new(connector: Box<dyn IpcConnector>, endpoint: IpcEndpoint) -> Self {
        IpcControlClient {
            connector,
            endpoint,
        }
    }
}

impl ControlClient for IpcControlClient {
    fn request(&self, request: ControlRequest) -> Result<ControlResponse, IpcError> {
        let mut stream = self.connector.connect(&self.endpoint)?;
        let payload = encode(&request).map_err(|_| IpcError::Protocol)?;
        stream.write_frame(&payload)?;
        let bytes = stream.read_frame()?;
        decode(&bytes).map_err(|_| IpcError::Protocol)
    }
}

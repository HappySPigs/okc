//! 프로토콜 메시지 타입 — `ControlPlane` 와이어 표면(U7b 소유).
//!
//! 단일 요청 enum(`ControlRequest`/`ControlOp`)과 단일 응답 enum(`ControlResponse`/`ControlResult`)
//! 을 정의하며 모두 U0 CBOR 코덱(`encode`/`decode`)으로 무손실 직렬화된다(D-U7B-02/03,
//! R-U7B-05). 도메인 오류는 응답 프레임 안 `ControlResult::Error(ControlError)` 로 전달되고
//! (전송 성공), 전송/프레이밍/버전 실패만 `IpcError`(모듈 `ipc`)로 표면화된다(R-U7B-09).
//!
//! 소비 타입(`StatusSnapshot`/`Health`/`UploadHistoryRecord`/`ConsentState`/`Timestamp`)은 U0 가
//! 소유하며 여기서는 이름으로만 참조한다(재정의 금지). U6 `HistoryQuery` 는 serde 를 구현하지
//! 않으므로(동결 API) 와이어에는 동일 두 필드(`since`/`only_failures`)를 실어 나르는
//! `HistoryFilter`(U7b 소유)로 투영한다 — `ControlPlane` 이 디스패치 시 `HistoryQuery` 로 재조립한다.
//!
//! 순수 값 모듈로서 panic-free-total 을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use serde::{Deserialize, Serialize};

use foundation::{ConsentState, Health, StatusSnapshot, Timestamp, UploadHistoryRecord};

/// 프로토콜 버전 상수. 단일 배포 바이너리라 클라이언트=서버 동일 버전이며, 이 상수는 버전 스큐
/// (부분 업그레이드) 방어에 쓰인다(D-U7B-04, R-U7B-06).
pub const PROTO_VERSION: u16 = 1;

/// 셧다운 모드 — `stop` 신호가 실어 나르는 유예 정책 힌트(U7b 소유). 실제 종료 실행은 U8 소관.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopMode {
    /// 진행 중 작업을 마치고 종료(유예).
    Graceful,
    /// 즉시 종료.
    Immediate,
}

/// consent 서브 동작(D-U7B-13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsentOp {
    /// 부수효과 없는 상태 조회.
    View,
    /// 상시 동의 부여.
    Grant,
    /// 동의 철회(forward-only).
    Withdraw,
    /// RISK-01 고지 확인.
    Acknowledge,
}

/// `history` 필터 와이어 투영 — U6 `HistoryQuery{since, only_failures}` 의 두 필드를 그대로 실어
/// 나른다(U6 `HistoryQuery` 가 serde 미구현이라 동일 형상의 U7b 소유 타입으로 투영).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HistoryFilter {
    /// `record.timestamp >= since` 만 통과(`None` = 무제약).
    pub since: Option<Timestamp>,
    /// `Some(true)` = 실패만, `Some(false)` = 성공만, `None` = 무제약.
    pub only_failures: Option<bool>,
}

/// 요청 op — 데몬 대상 동작 집합(install/uninstall 은 IPC 우회이므로 여기에 없다, D-U7B-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlOp {
    /// 현재 상태 스냅샷 조회 -> `StatusService.snapshot()`.
    Status,
    /// 운영 건강 판정 -> `ReadJudgment.health_check()`.
    Health,
    /// 감시 일시중지 -> `RunStateController.pause()`.
    Pause,
    /// 감시 재개 -> `RunStateController.resume()`.
    Resume,
    /// 즉시 동기화 요청 -> `RunStateController.request_sync_now()`.
    SyncNow,
    /// 종료 요청 -> `RunStateController.request_stop(mode)`.
    Stop(StopMode),
    /// 업로드 히스토리 조회 -> `UploadHistoryStore.query(filter)`.
    History(HistoryFilter),
    /// 동의 제어 -> `ConsentGate`.
    Consent(ConsentOp),
    /// config 리로드 -> `ConfigProvider.reload()`.
    Reload,
}

/// 단일 요청 프레임(D-U7B-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlRequest {
    /// 발신 시 `PROTO_VERSION`. 수신 시 엄격 동일성 검사(R-U7B-06).
    pub proto_version: u16,
    /// 수행할 동작.
    pub op: ControlOp,
}

/// consent 와이어 응답 — U5 내부 타입(`ConsentLifecycle`/`ConsentGrant`)과의 결합을 피하려
/// U0 `ConsentState` + `acknowledged` 두 필드로 투영한다(D-U7B-13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentView {
    /// `ConsentGate.consent_state()` projection.
    pub consent: ConsentState,
    /// `ConsentGate.view().acknowledged`.
    pub acknowledged: bool,
}

/// 핸들러 도메인 오류(전송 오류 `IpcError` 와 분리, 응답 프레임 안에서 전달).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlError {
    /// 버전 불일치 — 디스패치 전 거부(R-U7B-06).
    VersionMismatch {
        /// 서버 프로토콜 버전.
        server: u16,
        /// 수신한 클라이언트 프로토콜 버전.
        client: u16,
    },
    /// `ConsentError` 매핑(NotAcknowledged/AlreadyGranted/NoGrant/PersistFailed).
    ConsentRejected(String),
    /// `ConfigError` 매핑(검증 실패 등 keep-last-good).
    ReloadFailed(String),
    /// `RunStateError` 매핑(Persist/Io).
    RunState(String),
    /// `HistoryError` 매핑.
    HistoryUnavailable(String),
}

/// 응답 result — 성공 페이로드 또는 도메인 오류.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlResult {
    /// 상태 스냅샷(U0 타입).
    Status(StatusSnapshot),
    /// 건강 판정(U0 타입).
    Health(Health),
    /// 히스토리 레코드 벡터(U0 타입).
    History(Vec<UploadHistoryRecord>),
    /// consent 와이어 뷰(U7b 타입).
    Consent(ConsentView),
    /// 부수효과 완료 확인(pause/resume/sync-now/stop/reload/consent 변경 성공).
    Ack,
    /// 핸들러 도메인 오류.
    Error(ControlError),
}

/// 단일 응답 프레임.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlResponse {
    /// 서버 `PROTO_VERSION`.
    pub proto_version: u16,
    /// 처리 결과.
    pub result: ControlResult,
}

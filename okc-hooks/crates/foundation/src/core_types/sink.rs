//! SinkContracts — U0 가 계약으로 소유하고 U6 가 구현하는 push-only 관측 싱크 트레이트 계열.
//!
//! 모든 싱크는 하위 단위가 주입받아 호출(push)만 하는 단방향 계약이다 — U0 는 U6 를 역참조하지
//! 않으며(back-reference 없음, 빌드 순서 역전 해소), 조립 루트 U8 이 U6 구현체를 하위 단위에
//! 하향 주입한다. 관측 push 는 best-effort·infallible·비치명적이다.
//!
//! `LogRecord`/`UploadHistoryRecord` 등 도메인 레코드 스키마와 `CycleOutcome`/`LimitReport`/
//! `Version`/`RollbackReason` 등은 U5/U6/U7 소유 참조 타입의 **최소 자리표시**(placeholder)이며,
//! 트레이트 시그니처 컴파일을 위해 U0 에 최소 정의만 둔다. 최종 스키마는 하류가 확정한다.
//!
//! 순수 계약 모듈로서 panic-free-total 을 컴파일타임으로 강제한다.
//!
//! 모든 싱크/관찰자/조회 트레이트는 `Send + Sync` 상위트레이트를 요구한다 — 데몬은 CLI 트리거
//! reload(R-TRIGGER-01)와 백그라운드 스레드가 공존하므로 `ConfigProvider` 및 싱크가
//! `Arc<dyn _>` 로 스레드 간 공유되어야 한다. 지금 요구하지 않으면 U8 조립 시 이 공개
//! 트레이트에 후행 파괴적 변경(breaking change)이 필요해진다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use serde::{Deserialize, Serialize};

use super::error::{ClassifiedError, TransportError};
use super::primitives::{ByteCount, Timestamp};
use super::status::{ActiveCondition, Health, Liveness, LivenessSignal, OperationalState};

/// 로그 최소 심각도 레벨. `WatcherConfig.log_level` 과 `Logger.event` 가 공유하는 U0 값 타입.
///
/// `Ord` 는 `Trace < Debug < Info < Warn < Error` 순으로 레벨 필터링에 사용할 수 있다.
/// serde 표현은 소문자(`"info"` 등)로 config JSON 과 정합한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// 가장 상세한 추적 레벨.
    Trace,
    /// 디버그 레벨.
    Debug,
    /// 일반 정보 레벨(기본값).
    Info,
    /// 경고 레벨.
    Warn,
    /// 오류 레벨.
    Error,
}

/// 사이클 식별자 — **U6 소유 참조 타입의 최소 자리표시**(placeholder).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CycleId(pub u64);

/// 구조화 로그 필드맵 — **U6 소유 참조 타입의 최소 자리표시**(placeholder).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LogFields(pub Vec<(String, String)>);

/// 구조화 로그 레코드 — **U6 `StructuredLogger` 소유 참조 타입의 최소 자리표시**(placeholder).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogRecord {
    /// 로그 레벨.
    pub level: LogLevel,
    /// 이벤트 식별 문자열.
    pub event: String,
    /// 연관 사이클 식별자(있으면).
    pub cycle_id: Option<CycleId>,
    /// 사람이 읽는 메시지.
    pub message: String,
    /// 구조화 필드맵.
    pub fields: LogFields,
}

/// 업로드 히스토리 append 레코드 — **U6 `UploadHistoryStore` 소유 참조 타입의 최소 자리표시**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadHistoryRecord {
    /// 레코드 시각.
    pub timestamp: Timestamp,
    /// 전송 바이트 수.
    pub bytes_transferred: ByteCount,
    /// 오류 상세(실패 시).
    pub error_detail: Option<String>,
}

/// 사이클 결과 — **U6 소유 참조 타입의 최소 자리표시**(placeholder).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CycleOutcome {
    /// 사이클 성공.
    Success,
    /// 사이클 실패 — 분류된 오류 동반.
    Failure(ClassifiedError),
}

/// 프리플라이트 한도 초과 리포트 — **U6 소유 참조 타입의 최소 자리표시**(placeholder).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LimitReport(pub String);

/// 버전 식별자 — **U7 소유 참조 타입의 최소 자리표시**(placeholder).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Version(pub String);

/// 롤백 사유 — **U7 소유 참조 타입의 최소 자리표시**(placeholder).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackReason(pub String);

/// 구조화 로그 파사드 계약. 구현은 U6 `StructuredLogger`.
///
/// SEC-02 계약: 구현은 config 유래 필드를 redacted `Debug`/`Display` 또는 명시적 per-field
/// 프로젝션으로만 방출해야 한다(토큰 원문 로그 유출 금지).
pub trait Logger: Send + Sync {
    /// 조립된 로그 레코드 1건을 push 한다(best-effort, infallible).
    fn log(&self, record: LogRecord);

    /// 레벨/이벤트/사이클 식별자/필드로부터 편의 로그를 push 한다.
    fn event(&self, level: LogLevel, event: &str, cycle_id: Option<CycleId>, fields: LogFields);
}

/// 상태 push 뮤테이터 계약(in-memory, infallible). 구현은 U6 `StatusService`.
pub trait StatusSink: Send + Sync {
    /// 운영 라이프사이클(축1)을 갱신한다.
    fn set_operational(&self, state: OperationalState);

    /// 활성 조건(축2)을 켠다.
    fn raise_condition(&self, cond: ActiveCondition);

    /// 활성 조건(축2)을 끈다.
    fn clear_condition(&self, cond: ActiveCondition);

    /// 동기화 성공 시각을 기록한다.
    fn record_sync_success(&self, at: Timestamp);

    /// 미커밋 변경 대기 표시(dirty)를 갱신한다.
    fn set_dirty(&self, dirty: bool);

    /// 진행 중 전송의 (전송량, 전체) 진행도를 갱신한다.
    fn set_resume_progress(&self, transferred: ByteCount, total: ByteCount);

    /// startup liveness 신호를 push 한다.
    fn set_liveness(&self, signal: LivenessSignal);
}

/// 업로드 히스토리 append 계약(append-only). 구현은 U6 `UploadHistoryStore`.
pub trait HistorySink: Send + Sync {
    /// 히스토리 레코드 1건을 append 한다(best-effort, infallible).
    fn append(&self, record: UploadHistoryRecord);
}

/// 중대 이벤트 보고 계약(best-effort, infallible). 구현은 U6 `CriticalErrorNotifier`.
pub trait CriticalEventSink: Send + Sync {
    /// 케이스1: 401/토큰 거부 등 인증 실패 보고.
    fn report_auth_failure(&self, detail: TransportError);

    /// 케이스2: 사이클 결과 보고(연속 실패 카운트, 성공 시 리셋).
    fn report_cycle_result(&self, outcome: CycleOutcome);

    /// 케이스3: 프리플라이트 한도 초과 보고.
    fn report_preflight_exceeded(&self, report: LimitReport);

    /// 케이스4: 자동 업데이트 롤백 보고.
    fn report_update_rollback(&self, from: Version, to: Version, reason: RollbackReason);
}

/// config 리로드 관찰자 계약. 성공 스왑 직후 push-only 로 통지된다(back-reference 없음).
///
/// 통지는 페이로드 없이 이루어지며(component-methods 정합), 관찰자는 필요 시
/// `ConfigProvider::current()` 로 새 스냅샷을 재조회해 재해석한다. 구현은 U5/U6.
pub trait ConfigReloadObserver: Send + Sync {
    /// config 가 성공적으로 리로드되었음을 통지받는다(재해석 트리거).
    fn on_config_reload(&self);
}

/// read-judgment intent 계약 — 운영 건강(`health_check`)과 순수 liveness(`update_probe`)를
/// **별개** 판정으로 노출하는 조회(read) 트레이트. 구현은 U6 `StatusService`.
///
/// U0 는 반환 형상(return-shape) 계약만 소유하며 판정 임계값 로직은 U6 소관이다. 두 판정을
/// 분리하는 이유는, 일시적 운영 조건(AuthFailed/OverLimit)이 좋은 새 버전을 오판 롤백시키는
/// 롤백 루프를 방지하기 위함이다(update_probe 는 U7a AutoUpdater 만 소비, 노트3·Q9=B).
pub trait ReadJudgment: Send + Sync {
    /// 운영 건강을 판정한다 — CLI `status`/`health` 종료코드 매핑의 근거(US-E5-02).
    fn health_check(&self) -> Health;

    /// 순수 liveness 를 판정한다 — startup + IdleReached + CredentialReadable 충족 여부.
    fn update_probe(&self) -> Liveness;
}

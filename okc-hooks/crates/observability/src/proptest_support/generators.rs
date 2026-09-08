//! U6 고유 타입 도메인 제너레이터 — U0 제너레이터를 재사용하고 U6 확정 스키마만 새로 노출한다.

use proptest::prelude::*;

use foundation::proptest_support::generators as fg;
use foundation::{CycleId, LogFields, LogRecord, Timestamp, UploadHistoryRecord};

use crate::history::HistoryQuery;

/// `error_detail` 후보: `None`(성공) + 빈/유니코드/개행/임의(실패)를 포괄한다.
pub fn arb_error_detail() -> impl Strategy<Value = Option<String>> {
    prop_oneof![
        Just(None),
        Just(Some(String::new())),
        Just(Some("줄1\n줄2\r\n줄3".to_string())),
        Just(Some("emoji 🚀 유니코드 混在".to_string())),
        "[A-Za-z0-9 가-힣]{0,24}".prop_map(Some),
    ]
}

/// `UploadHistoryRecord`(U0 FROZEN 3필드) 제너레이터 — 성공/실패·경계 바이트·임의 시각.
pub fn arb_upload_history_record() -> impl Strategy<Value = UploadHistoryRecord> {
    (fg::arb_timestamp(), fg::arb_byte_count(), arb_error_detail()).prop_map(
        |(timestamp, bytes_transferred, error_detail)| UploadHistoryRecord {
            timestamp,
            bytes_transferred,
            error_detail,
        },
    )
}

/// `CycleId` 유/무 제너레이터.
pub fn arb_cycle_id() -> impl Strategy<Value = Option<CycleId>> {
    prop::option::of(any::<u64>().prop_map(CycleId))
}

/// 무손실 round-trip 경계(빈/유니코드/개행)를 포괄하는 자유형 문자열.
fn arb_free_text() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::new()),
        Just("줄1\n줄2".to_string()),
        Just("메시지 🚀 混在".to_string()),
        "[A-Za-z0-9 가-힣]{0,24}".prop_map(|s| s),
    ]
}

/// `LogFields`(다수/빈 필드, 유니코드 key/value) 제너레이터.
pub fn arb_log_fields() -> impl Strategy<Value = LogFields> {
    prop::collection::vec((arb_free_text(), arb_free_text()), 0..4).prop_map(LogFields)
}

/// 방출 `LogRecord` 제너레이터 — 전 `LogLevel`·`cycle_id` 유무·유니코드/개행/빈 `message`·필드.
pub fn arb_log_record() -> impl Strategy<Value = LogRecord> {
    (
        fg::arb_log_level(),
        arb_free_text(),
        arb_cycle_id(),
        arb_free_text(),
        arb_log_fields(),
    )
        .prop_map(|(level, event, cycle_id, message, fields)| LogRecord {
            level,
            event,
            cycle_id,
            message,
            fields,
        })
}

/// `HistoryQuery`(since/only_failures 조합, `None` 포함) 제너레이터.
pub fn arb_history_query() -> impl Strategy<Value = HistoryQuery> {
    (
        prop::option::of(fg::arb_timestamp()),
        prop::option::of(any::<bool>()),
    )
        .prop_map(|(since, only_failures)| HistoryQuery {
            since,
            only_failures,
        })
}

/// 상태 명령 시퀀스의 단일 명령(중복 raise/clear·2축 인터리빙 포함).
#[derive(Debug, Clone)]
pub enum StatusCommand {
    /// `set_operational` 축1 갱신.
    SetOperational(foundation::OperationalState),
    /// `raise_condition` 축2 add.
    Raise(foundation::ActiveCondition),
    /// `clear_condition` 축2 remove.
    Clear(foundation::ActiveCondition),
    /// `set_liveness` 신호 단조 누적.
    Liveness(foundation::LivenessSignal),
    /// escalation 플래그 push(U6-내부).
    Escalation(bool),
}

/// 상태 명령 1건 제너레이터.
pub fn arb_status_command() -> impl Strategy<Value = StatusCommand> {
    prop_oneof![
        fg::arb_operational_state().prop_map(StatusCommand::SetOperational),
        fg::arb_active_condition().prop_map(StatusCommand::Raise),
        fg::arb_active_condition().prop_map(StatusCommand::Clear),
        fg::arb_liveness_signal().prop_map(StatusCommand::Liveness),
        any::<bool>().prop_map(StatusCommand::Escalation),
    ]
}

/// 상태 명령 시퀀스(0..16 길이) 제너레이터.
pub fn arb_status_commands() -> impl Strategy<Value = Vec<StatusCommand>> {
    prop::collection::vec(arb_status_command(), 0..16)
}

/// 임의 `Timestamp`(round-trip 경계 포함) — U0 재노출 편의.
pub fn arb_timestamp() -> impl Strategy<Value = Timestamp> {
    fg::arb_timestamp()
}

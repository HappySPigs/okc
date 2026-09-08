//! U6 Property-Based Tests (`proptest-support` feature 게이트).
//!
//! FD Testable Properties(PROP-U6-DE-*/BR-*)를 구현한다: 히스토리 레코드/로그 라인 round-trip,
//! 레벨 필터, 2축 독립성, health 오라클(닫힌 4조건), update_probe 격리, 연속실패 상태머신,
//! append-only + query 오라클, truncated-tail 관용. feature 가 꺼진 기본 빌드에서는 빈 테스트
//! 크레이트로 컴파일된다(프로덕션 빌드 그래프에 proptest 미유입, PBT-07).
#![cfg(feature = "proptest-support")]

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use foundation::proptest_support::generators as fg;
use foundation::{
    ActiveCondition, ByteCount, ClassifiedError, ConfigProvider, CriticalEventSink, CycleOutcome,
    ErrorClass, FOUNDATION_CONFIG_KEYS, Health, HistorySink, LivenessSignal, LogFields, LogLevel,
    LogRecord, Logger, OperationalState, ReadJudgment, StatusSink, Timestamp, UploadHistoryRecord,
    decode, encode,
};
use observability::proptest_support::generators as g;
use observability::proptest_support::generators::StatusCommand;
use observability::{
    Clock, CriticalErrorNotifier, EmittedLogLine, HistoryQuery, LoggerConfig, StatusService,
    StructuredLogger, TrayIndicator, UploadHistoryStore,
};
use proptest::prelude::*;

/// 고정 시각 테스트 clock.
struct FixedClock;
impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_unix_nanos(123)
    }
}

fn unique_path(tag: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut path = std::env::temp_dir();
    path.push(format!("okc_u6_prop_{}_{}_{}", tag, std::process::id(), n));
    path
}

fn make_logger(path: PathBuf, level: LogLevel) -> StructuredLogger {
    let cfg = LoggerConfig {
        log_file: path,
        log_max_size_bytes: ByteCount::new(u64::MAX),
        log_max_retained: 3,
        log_level: level,
    };
    StructuredLogger::new(
        cfg,
        Arc::new(FixedClock),
        Arc::new(ConfigProvider::new(FOUNDATION_CONFIG_KEYS)),
    )
}

proptest! {
    /// PROP-U6-DE-01 — `decode(encode(r)) == r`(히스토리 레코드 무손실 round-trip).
    #[test]
    fn de01_history_record_roundtrip(record in g::arb_upload_history_record()) {
        let bytes = encode(&record).expect("encode");
        let decoded: UploadHistoryRecord = decode(&bytes).expect("decode");
        prop_assert_eq!(decoded, record);
    }

    /// PROP-U6-DE-02 — 방출 로그 라인 JSON round-trip(유니코드/개행/`cycle_id None` 포함).
    #[test]
    fn de02_log_line_roundtrip(record in g::arb_log_record()) {
        let ts = Timestamp::from_unix_nanos(999);
        let line = EmittedLogLine::new(ts, &record);
        let json = line.to_json().expect("to_json");
        prop_assert!(!json.contains('\n')); // JSON-line 1건(개행 없음)
        let parsed: EmittedLogLine = serde_json::from_str(&json).expect("parse");
        prop_assert_eq!(parsed, line);
    }

    /// PROP-U6-BR-01 — 레벨 필터: 방출됨 iff `record.level >= active_level`.
    #[test]
    fn br01_level_filter(active in fg::arb_log_level(), record in g::arb_log_record()) {
        let path = unique_path("br01");
        let logger = make_logger(path.clone(), active);
        logger.log(record.clone());
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let emitted = !content.trim().is_empty();
        prop_assert_eq!(emitted, record.level >= active);
        let _ = std::fs::remove_file(&path);
    }

    /// PROP-U6-BR-02 — 2축 독립성: operational = 마지막 set, conditions = raise - clear.
    #[test]
    fn br02_two_axis_independence(commands in g::arb_status_commands()) {
        let svc = StatusService::new();
        let mut ref_operational = OperationalState::Idle;
        let mut ref_conditions: BTreeSet<ActiveCondition> = BTreeSet::new();
        for cmd in &commands {
            match cmd {
                StatusCommand::SetOperational(s) => { svc.set_operational(*s); ref_operational = *s; }
                StatusCommand::Raise(c) => { svc.raise_condition(*c); ref_conditions.insert(*c); }
                StatusCommand::Clear(c) => { svc.clear_condition(*c); ref_conditions.remove(c); }
                StatusCommand::Liveness(sig) => svc.set_liveness(*sig),
                StatusCommand::Escalation(b) => svc.set_failure_escalation(*b),
            }
        }
        let snap = svc.snapshot();
        prop_assert_eq!(snap.operational, ref_operational);
        let expected: Vec<ActiveCondition> = ref_conditions.into_iter().collect();
        prop_assert_eq!(snap.conditions, expected);
    }

    /// PROP-U6-BR-04 — health 오라클: Unhealthy iff 조건 ∩ 닫힌3집합 ≠ ∅ 또는 escalation.
    #[test]
    fn br04_health_oracle(
        conditions in prop::collection::vec(fg::arb_active_condition(), 0..6),
        escalation in any::<bool>(),
    ) {
        let svc = StatusService::new();
        for c in &conditions { svc.raise_condition(*c); }
        svc.set_failure_escalation(escalation);
        let critical = [
            ActiveCondition::AuthFailed,
            ActiveCondition::OverLimit,
            ActiveCondition::UpdateRolledBack,
        ];
        let expected_unhealthy =
            escalation || conditions.iter().any(|c| critical.contains(c));
        let is_unhealthy = matches!(svc.health_check(), Health::Unhealthy { .. });
        prop_assert_eq!(is_unhealthy, expected_unhealthy);
    }

    /// PROP-U6-BR-05 — update_probe 격리: 오직 두 liveness 신호에만 의존(조건/상태 무관).
    #[test]
    fn br05_update_probe_isolation(
        idle in any::<bool>(),
        cred in any::<bool>(),
        conditions in prop::collection::vec(fg::arb_active_condition(), 0..6),
        operational in fg::arb_operational_state(),
    ) {
        let svc = StatusService::new();
        // 운영 조건/상태를 임의로 세팅해도 probe 결과에 영향 없어야 한다.
        svc.set_operational(operational);
        for c in &conditions { svc.raise_condition(*c); }
        if idle { svc.set_liveness(LivenessSignal::IdleReached); }
        if cred { svc.set_liveness(LivenessSignal::CredentialReadable); }
        let alive = matches!(svc.update_probe(), foundation::Liveness::Alive);
        prop_assert_eq!(alive, idle && cred);
    }

    /// PROP-U6-BR-07 — 연속실패 임계 상태머신: 임계 도달 순간 정확히 1회 발화, Success 시 리셋.
    #[test]
    fn br07_escalation_state_machine(
        seq in prop::collection::vec(any::<bool>(), 0..24),
        threshold in 1u32..8,
    ) {
        // 참조 모델(단순 카운터).
        let mut consecutive = 0u32;
        let mut escalated = false;
        let mut ref_fires = 0usize;
        for &is_failure in &seq {
            if is_failure {
                consecutive += 1;
                if consecutive >= threshold && !escalated { escalated = true; ref_fires += 1; }
            } else {
                consecutive = 0;
                escalated = false;
            }
        }

        let logger = Arc::new(RecordingLogger::default());
        let status = Arc::new(StatusService::new());
        let notifier = CriticalErrorNotifier::new(
            logger.clone(),
            status.clone(),
            Arc::new(TrayIndicator::new()),
            threshold,
        );
        for &is_failure in &seq {
            if is_failure {
                notifier.report_cycle_result(CycleOutcome::Failure(ClassifiedError {
                    class: ErrorClass::Retryable,
                    code: None,
                    detail: "x".to_string(),
                }));
            } else {
                notifier.report_cycle_result(CycleOutcome::Success);
            }
        }
        let fires = logger.count("critical.consecutive_failures");
        prop_assert_eq!(fires, ref_fires);
        // 최종 escalation 상태가 health 에 반영(다른 조건 없음).
        let is_unhealthy = matches!(status.health_check(), Health::Unhealthy { .. });
        prop_assert_eq!(is_unhealthy, escalated);
    }

    /// PROP-U6-BR-08 — append-only 불변 + query AND 오라클.
    #[test]
    fn br08_append_only_query_oracle(
        records in prop::collection::vec(g::arb_upload_history_record(), 0..12),
        query in g::arb_history_query(),
    ) {
        let path = unique_path("br08");
        let store = UploadHistoryStore::new(path.clone(), None);
        for r in &records { store.append(r.clone()); }

        // query(all): append 순서로 누락/수정 없이 전부 반환.
        let all = store.query(HistoryQuery::all()).expect("query all");
        prop_assert_eq!(&all, &records);

        // query(filter): in-memory 참조 필터와 정확히 일치.
        let expected: Vec<UploadHistoryRecord> =
            records.iter().filter(|r| query.matches(r)).cloned().collect();
        let got = store.query(query).expect("query filter");
        prop_assert_eq!(got, expected);
        let _ = std::fs::remove_file(&path);
    }

    /// PROP-U6-BR-09 — truncated-tail 관용: 트레일링 절단 시 정상 prefix 반환.
    #[test]
    fn br09_truncated_tail_prefix(
        records in prop::collection::vec(g::arb_upload_history_record(), 1..10),
        drop_frac in 1u64..64,
    ) {
        let path = unique_path("br09");
        let store = UploadHistoryStore::new(path.clone(), None);
        for r in &records { store.append(r.clone()); }
        let raw = std::fs::read(&path).expect("read");
        let total = raw.len() as u64;
        // 제거할 트레일링 바이트 수(최소 1, total 미만).
        let drop_bytes = (drop_frac % total.max(1)).max(1);
        let keep = total.saturating_sub(drop_bytes) as usize;

        // 오라클: end_offset <= keep 인 레코드만 온전히 남는다.
        let mut expected: Vec<UploadHistoryRecord> = Vec::new();
        let mut offset: usize = 0;
        for r in &records {
            let frame = 8 + encode(r).expect("encode").len();
            offset += frame;
            if offset <= keep { expected.push(r.clone()); } else { break; }
        }

        std::fs::write(&path, &raw[..keep]).expect("truncate");
        let got = store.query(HistoryQuery::all()).expect("tolerant query");
        prop_assert_eq!(got, expected);
        let _ = std::fs::remove_file(&path);
    }
}

/// 방출 이벤트명을 기록하는 테스트 로거.
#[derive(Default)]
struct RecordingLogger {
    events: std::sync::Mutex<Vec<String>>,
}
impl RecordingLogger {
    fn count(&self, event: &str) -> usize {
        self.events
            .lock()
            .map(|v| v.iter().filter(|e| e.as_str() == event).count())
            .unwrap_or(0)
    }
}
impl Logger for RecordingLogger {
    fn log(&self, record: LogRecord) {
        if let Ok(mut v) = self.events.lock() {
            v.push(record.event);
        }
    }
    fn event(
        &self,
        _level: LogLevel,
        event: &str,
        _cycle_id: Option<foundation::CycleId>,
        _fields: LogFields,
    ) {
        if let Ok(mut v) = self.events.lock() {
            v.push(event.to_string());
        }
    }
}

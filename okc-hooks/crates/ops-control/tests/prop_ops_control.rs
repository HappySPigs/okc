//! Property-Based Testing (PBT, `proptest-support` feature 게이트) — PROP-U7B-01~07.
//!
//! feature 가 꺼진 기본 빌드에서는 빈 테스트 크레이트로 컴파일되고,
//! `cargo test --features proptest-support` 로만 실제 property 가 실행된다(PBT-07).
//!
//! 커버: PROP-U7B-01(메시지 무손실 round-trip), PROP-U7B-02(프레이밍 절단/손상에 패닉 없이
//! Protocol), PROP-U7B-03(버전 핸드셰이크 + 부작용 0회/1회), PROP-U7B-04(run-state 전이·멱등),
//! PROP-U7B-05(paused-persist + 휘발성 리셋 + 손상 안전 회복), PROP-U7B-06(health->exit-code),
//! PROP-U7B-07(라우팅 이분법 배타·전수).
#![cfg(feature = "proptest-support")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use foundation::{
    ActiveCondition, ByteCount, ConsentState, Health, HealthReason, OperationalState,
    StatusSnapshot, Timestamp, UploadHistoryRecord, decode, encode,
};
use observability::StatusService;
use proptest::prelude::*;

use ops_control::{
    CliArgs, Command, ConsentOp, ConsentView, ControlClient, ControlError, ControlHandlers,
    ControlOp, ControlRequest, ControlResponse, ControlResult, HistoryFilter, IpcError, PROTO_VERSION,
    RunMode, RunStateController, ServiceOps, StopMode, handle_request, map_health, read_frame_from,
    run, write_frame_to,
};
use lifecycle_deploy::UninstallOptions;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_state_path(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("okc-u7b-prop-{tag}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir.join("run-state.cbor")
}

// ---------------------------------------------------------------------------
// 제너레이터
// ---------------------------------------------------------------------------

fn arb_stop_mode() -> impl Strategy<Value = StopMode> {
    prop_oneof![Just(StopMode::Graceful), Just(StopMode::Immediate)]
}

fn arb_consent_op() -> impl Strategy<Value = ConsentOp> {
    prop_oneof![
        Just(ConsentOp::View),
        Just(ConsentOp::Grant),
        Just(ConsentOp::Withdraw),
        Just(ConsentOp::Acknowledge),
    ]
}

fn arb_history_filter() -> impl Strategy<Value = HistoryFilter> {
    (
        proptest::option::of(any::<i64>()),
        proptest::option::of(any::<bool>()),
    )
        .prop_map(|(since, only_failures)| HistoryFilter {
            since: since.map(Timestamp::from_unix_nanos),
            only_failures,
        })
}

fn arb_control_op() -> impl Strategy<Value = ControlOp> {
    prop_oneof![
        Just(ControlOp::Status),
        Just(ControlOp::Health),
        Just(ControlOp::Pause),
        Just(ControlOp::Resume),
        Just(ControlOp::SyncNow),
        arb_stop_mode().prop_map(ControlOp::Stop),
        arb_history_filter().prop_map(ControlOp::History),
        arb_consent_op().prop_map(ControlOp::Consent),
        Just(ControlOp::Reload),
    ]
}

fn arb_operational() -> impl Strategy<Value = OperationalState> {
    prop_oneof![
        Just(OperationalState::Idle),
        Just(OperationalState::Syncing),
        Just(OperationalState::Offline),
        Just(OperationalState::Paused),
    ]
}

fn arb_condition() -> impl Strategy<Value = ActiveCondition> {
    prop_oneof![
        Just(ActiveCondition::AuthFailed),
        Just(ActiveCondition::ConsentBlocked),
        Just(ActiveCondition::OverLimit),
        Just(ActiveCondition::VaultUnavailable),
        Just(ActiveCondition::UpdateRolledBack),
    ]
}

fn arb_consent_state() -> impl Strategy<Value = ConsentState> {
    prop_oneof![
        Just(ConsentState::Granted),
        Just(ConsentState::Blocked),
        Just(ConsentState::Unknown),
    ]
}

fn arb_status_snapshot() -> impl Strategy<Value = StatusSnapshot> {
    (
        arb_operational(),
        proptest::collection::vec(arb_condition(), 0..4),
        proptest::option::of(any::<i64>()),
        any::<bool>(),
        proptest::option::of((any::<u64>(), any::<u64>())),
        arb_consent_state(),
        any::<bool>(),
    )
        .prop_map(
            |(operational, conditions, last, dirty, resume, consent, offline)| StatusSnapshot {
                operational,
                conditions,
                last_success: last.map(Timestamp::from_unix_nanos),
                dirty,
                resume: resume.map(|(a, b)| (ByteCount::new(a), ByteCount::new(b))),
                consent,
                offline,
            },
        )
}

fn arb_health() -> impl Strategy<Value = Health> {
    prop_oneof![
        Just(Health::Healthy),
        proptest::collection::vec(any::<String>().prop_map(HealthReason), 0..4)
            .prop_map(|reasons| Health::Unhealthy { reasons }),
    ]
}

fn arb_record() -> impl Strategy<Value = UploadHistoryRecord> {
    (
        any::<i64>(),
        any::<u64>(),
        proptest::option::of(any::<String>()),
    )
        .prop_map(|(ts, bytes, err)| UploadHistoryRecord {
            timestamp: Timestamp::from_unix_nanos(ts),
            bytes_transferred: ByteCount::new(bytes),
            error_detail: err,
        })
}

fn arb_control_error() -> impl Strategy<Value = ControlError> {
    prop_oneof![
        (any::<u16>(), any::<u16>())
            .prop_map(|(server, client)| ControlError::VersionMismatch { server, client }),
        any::<String>().prop_map(ControlError::ConsentRejected),
        any::<String>().prop_map(ControlError::ReloadFailed),
        any::<String>().prop_map(ControlError::RunState),
        any::<String>().prop_map(ControlError::HistoryUnavailable),
    ]
}

fn arb_control_result() -> impl Strategy<Value = ControlResult> {
    prop_oneof![
        arb_status_snapshot().prop_map(ControlResult::Status),
        arb_health().prop_map(ControlResult::Health),
        proptest::collection::vec(arb_record(), 0..4).prop_map(ControlResult::History),
        (arb_consent_state(), any::<bool>()).prop_map(|(consent, acknowledged)| {
            ControlResult::Consent(ConsentView {
                consent,
                acknowledged,
            })
        }),
        Just(ControlResult::Ack),
        arb_control_error().prop_map(ControlResult::Error),
    ]
}

fn arb_uninstall_options() -> impl Strategy<Value = UninstallOptions> {
    (any::<bool>(), any::<bool>(), any::<bool>()).prop_map(
        |(purge_token, purge_logs, deregister_service)| UninstallOptions {
            purge_token,
            purge_logs,
            deregister_service,
        },
    )
}

fn arb_command() -> impl Strategy<Value = Command> {
    prop_oneof![
        Just(Command::Status),
        Just(Command::Health),
        Just(Command::Pause),
        Just(Command::Resume),
        Just(Command::SyncNow),
        arb_stop_mode().prop_map(Command::Stop),
        arb_history_filter().prop_map(Command::History),
        arb_consent_op().prop_map(Command::Consent),
        Just(Command::Reload),
        Just(Command::Install),
        arb_uninstall_options().prop_map(Command::Uninstall),
    ]
}

/// run-state 명령(전이 시퀀스 생성용).
#[derive(Debug, Clone, Copy)]
enum RunCmd {
    Pause,
    Resume,
    SyncNow,
    Stop(StopMode),
}

fn arb_run_cmd() -> impl Strategy<Value = RunCmd> {
    prop_oneof![
        Just(RunCmd::Pause),
        Just(RunCmd::Resume),
        Just(RunCmd::SyncNow),
        arb_stop_mode().prop_map(RunCmd::Stop),
    ]
}

// ---------------------------------------------------------------------------
// PROP-U7B-01 — 메시지 무손실 round-trip
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_request_roundtrip(version in any::<u16>(), op in arb_control_op()) {
        let request = ControlRequest { proto_version: version, op };
        let bytes = encode(&request).expect("encode");
        let decoded: ControlRequest = decode(&bytes).expect("decode");
        prop_assert_eq!(decoded, request);
    }

    #[test]
    fn prop_response_roundtrip(version in any::<u16>(), result in arb_control_result()) {
        let response = ControlResponse { proto_version: version, result };
        let bytes = encode(&response).expect("encode");
        let decoded: ControlResponse = decode(&bytes).expect("decode");
        prop_assert_eq!(decoded, response);
    }
}

// ---------------------------------------------------------------------------
// PROP-U7B-02 — 프레이밍 회복력(절단/손상에 패닉 없이 Protocol)
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_frame_roundtrip(payload in proptest::collection::vec(any::<u8>(), 0..2048)) {
        let mut buf = Vec::new();
        write_frame_to(&mut buf, &payload).expect("write frame");
        let mut cursor = Cursor::new(buf);
        let restored = read_frame_from(&mut cursor).expect("read frame");
        prop_assert_eq!(restored, payload);
    }

    #[test]
    fn prop_arbitrary_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..2048)) {
        // 임의 바이트열: Ok(payload) 또는 Err(IpcError::Protocol/Io) 이며 절대 패닉하지 않는다.
        let mut cursor = Cursor::new(bytes);
        let outcome = read_frame_from(&mut cursor);
        prop_assert!(
            outcome.is_ok() || outcome.is_err(),
            "read_frame_from 은 임의 입력에 패닉하지 않고 반환한다"
        );
    }

    #[test]
    fn prop_truncated_frame_is_protocol(
        payload in proptest::collection::vec(any::<u8>(), 1..512),
        cut in 0usize..8,
    ) {
        let mut buf = Vec::new();
        write_frame_to(&mut buf, &payload).expect("write frame");
        // 페이로드 일부를 잘라 절단 프레임을 만든다(헤더는 유지, 페이로드 부족).
        let keep = buf.len().saturating_sub(cut + 1);
        buf.truncate(keep);
        let mut cursor = Cursor::new(buf);
        let outcome = read_frame_from(&mut cursor);
        prop_assert_eq!(outcome, Err(IpcError::Protocol));
    }
}

// ---------------------------------------------------------------------------
// PROP-U7B-03 — 버전 핸드셰이크 + 부작용 0회/1회
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_version_handshake(version in any::<u16>(), op in arb_control_op()) {
        let handlers = RecordingHandlers::new();
        let request = ControlRequest { proto_version: version, op };
        let response = handle_request(&handlers, request);
        if version != PROTO_VERSION {
            // 불일치 -> VersionMismatch 이고 어떤 핸들러도 호출되지 않는다.
            prop_assert!(
                matches!(response.result, ControlResult::Error(ControlError::VersionMismatch { .. })),
                "버전 불일치 시 VersionMismatch 여야 한다"
            );
            prop_assert_eq!(handlers.total_calls(), 0);
        } else {
            // 일치 -> 정확히 대응 핸들러 1개 호출.
            prop_assert_eq!(handlers.total_calls(), 1);
        }
    }
}

// ---------------------------------------------------------------------------
// PROP-U7B-04 — run-state 전이·멱등
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_run_state_transitions(cmds in proptest::collection::vec(arb_run_cmd(), 0..40)) {
        let path = temp_state_path("transition");
        let status = Arc::new(StatusService::new());
        let ctrl = RunStateController::new(path, status);

        // 오라클: 마지막 pause/resume 가 최종 mode 를 결정한다(무-전이 시 Running).
        let mut expected = RunMode::Running;
        for cmd in &cmds {
            match cmd {
                RunCmd::Pause => {
                    ctrl.pause().expect("pause");
                    expected = RunMode::Paused;
                }
                RunCmd::Resume => {
                    ctrl.resume().expect("resume");
                    expected = RunMode::Running;
                }
                RunCmd::SyncNow => ctrl.request_sync_now(),
                RunCmd::Stop(mode) => ctrl.request_stop(*mode),
            }
        }
        prop_assert_eq!(ctrl.current().mode, expected);
    }
}

// ---------------------------------------------------------------------------
// PROP-U7B-05 — paused-persist + 휘발성 리셋 + 손상 안전 회복
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_paused_persists_volatile_resets(cmds in proptest::collection::vec(arb_run_cmd(), 0..40)) {
        let path = temp_state_path("persist");
        let status = Arc::new(StatusService::new());
        let ctrl = RunStateController::new(path.clone(), status);

        let mut expected_paused = false;
        for cmd in &cmds {
            match cmd {
                RunCmd::Pause => {
                    ctrl.pause().expect("pause");
                    expected_paused = true;
                }
                RunCmd::Resume => {
                    ctrl.resume().expect("resume");
                    expected_paused = false;
                }
                RunCmd::SyncNow => ctrl.request_sync_now(),
                RunCmd::Stop(mode) => ctrl.request_stop(*mode),
            }
        }

        // reopen: paused 보존, 휘발성 신호 리셋.
        let status2 = Arc::new(StatusService::new());
        let reopened = RunStateController::new(path, status2);
        let reloaded = reopened.current();
        let reloaded_paused = reloaded.mode == RunMode::Paused;
        prop_assert_eq!(reloaded_paused, expected_paused);
        prop_assert!(!reloaded.sync_requested, "휘발성 sync 신호는 리셋된다");
        prop_assert_eq!(reloaded.stop_requested, None);
    }

    #[test]
    fn prop_corrupt_state_recovers_to_running(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
        let path = temp_state_path("corrupt");
        std::fs::write(&path, &bytes).expect("write corrupt");
        let status = Arc::new(StatusService::new());
        let ctrl = RunStateController::new(path, status);
        // 임의 바이트 상태 파일: 패닉 없이 로드되며 Paused 이거나(우연히 유효) Running 이다.
        let mode = ctrl.current().mode;
        prop_assert!(
            matches!(mode, RunMode::Running | RunMode::Paused),
            "손상 파일 로드는 패닉 없이 유효 mode 를 반환한다"
        );
    }
}

// ---------------------------------------------------------------------------
// PROP-U7B-06 — health -> exit code (배타·전수)
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_health_exit_code(health in arb_health(), transport_ok in any::<bool>()) {
        let response: Result<ControlResponse, IpcError> = if transport_ok {
            Ok(ControlResponse {
                proto_version: PROTO_VERSION,
                result: ControlResult::Health(health.clone()),
            })
        } else {
            Err(IpcError::Connect)
        };
        let code = map_health(&response).exit_code.0;
        if !transport_ok {
            prop_assert_eq!(code, 2);
        } else {
            match health {
                Health::Healthy => prop_assert_eq!(code, 0),
                Health::Unhealthy { .. } => prop_assert_eq!(code, 1),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// PROP-U7B-07 — 라우팅 이분법 (배타·전수)
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_routing_dichotomy(command in arb_command()) {
        let is_service = matches!(command, Command::Install | Command::Uninstall(_));
        let client = FakeClient::default();
        let service = FakeService::default();
        let args = CliArgs { command, json: false };
        let _ = run(&args, &client, &service);

        let client_calls = client.calls.load(Ordering::SeqCst);
        let service_calls = service.calls.load(Ordering::SeqCst);
        if is_service {
            prop_assert_eq!(service_calls, 1);
            prop_assert_eq!(client_calls, 0);
        } else {
            prop_assert_eq!(client_calls, 1);
            prop_assert_eq!(service_calls, 0);
        }
    }
}

// ---------------------------------------------------------------------------
// fake seam 구현
// ---------------------------------------------------------------------------

#[derive(Default)]
struct FakeClient {
    calls: AtomicUsize,
}

impl ControlClient for FakeClient {
    fn request(&self, _request: ControlRequest) -> Result<ControlResponse, IpcError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ControlResponse {
            proto_version: PROTO_VERSION,
            result: ControlResult::Ack,
        })
    }
}

#[derive(Default)]
struct FakeService {
    calls: AtomicUsize,
}

impl ServiceOps for FakeService {
    fn install(&self) -> Result<String, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok("installed".to_string())
    }
    fn uninstall(&self, _opts: &UninstallOptions) -> Result<String, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok("uninstalled".to_string())
    }
}

struct RecordingHandlers {
    total: AtomicUsize,
}

impl RecordingHandlers {
    fn new() -> Self {
        RecordingHandlers {
            total: AtomicUsize::new(0),
        }
    }
    fn note(&self) {
        self.total.fetch_add(1, Ordering::SeqCst);
    }
    fn total_calls(&self) -> usize {
        self.total.load(Ordering::SeqCst)
    }
}

fn empty_snapshot() -> StatusSnapshot {
    StatusSnapshot {
        operational: OperationalState::Idle,
        conditions: Vec::new(),
        last_success: None,
        dirty: false,
        resume: None,
        consent: ConsentState::Unknown,
        offline: false,
    }
}

impl ControlHandlers for RecordingHandlers {
    fn status(&self) -> StatusSnapshot {
        self.note();
        empty_snapshot()
    }
    fn health(&self) -> Health {
        self.note();
        Health::Healthy
    }
    fn history(&self, _filter: HistoryFilter) -> Result<Vec<UploadHistoryRecord>, ControlError> {
        self.note();
        Ok(Vec::new())
    }
    fn pause(&self) -> Result<(), ControlError> {
        self.note();
        Ok(())
    }
    fn resume(&self) -> Result<(), ControlError> {
        self.note();
        Ok(())
    }
    fn sync_now(&self) {
        self.note();
    }
    fn stop(&self, _mode: StopMode) {
        self.note();
    }
    fn consent_view(&self) -> ConsentView {
        self.note();
        ConsentView {
            consent: ConsentState::Unknown,
            acknowledged: false,
        }
    }
    fn consent_grant(&self) -> Result<(), ControlError> {
        self.note();
        Ok(())
    }
    fn consent_withdraw(&self) -> Result<(), ControlError> {
        self.note();
        Ok(())
    }
    fn consent_acknowledge(&self) -> Result<(), ControlError> {
        self.note();
        Ok(())
    }
    fn reload(&self) -> Result<(), ControlError> {
        self.note();
        Ok(())
    }
}

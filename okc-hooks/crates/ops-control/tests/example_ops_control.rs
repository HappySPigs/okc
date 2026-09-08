//! 예제 단위/통합 테스트 — U7b 핵심 규칙을 결정적으로 검증한다(소켓/데몬/특권 불요).
//!
//! 커버: run-state 전이 + paused-persist + paused->set_operational 반영(R-U7B-10/11/12/13),
//! IPC 프레이밍 round-trip + 버전 핸드셰이크(R-U7B-05/06), 디스패치 테이블(R-U7B-08),
//! health->exit-code(R-U7B-04), 라우팅 이분법(R-U7B-01, fake ServiceController 경로 포함).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use foundation::{
    ByteCount, ConfigProvider, ConsentState, Health, HealthReason, HistorySink, OperationalState,
    StatusSnapshot, Timestamp, UploadHistoryRecord,
};
use observability::{StatusService, UploadHistoryStore};
use auth_consent::ConsentGate;

use ops_control::{
    CliArgs, Command, ConsentView, ControlClient, ControlError, ControlHandlers, ControlOp,
    ControlPlane, ControlRequest, ControlResponse, ControlResult, HistoryFilter, IpcError,
    IpcStream, PROTO_VERSION, RunMode, RunStateController, ServiceOps, StopMode, WatcherHandlers,
    dispatch, handle_request, map_health, read_frame_from, run, write_frame_to,
};
use lifecycle_deploy::UninstallOptions;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_state_path(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("okc-u7b-ex-{tag}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir.join("run-state.cbor")
}

// ---------------------------------------------------------------------------
// RunStateController — 전이 + 지속 + StatusSink 반영
// ---------------------------------------------------------------------------

#[test]
fn pause_resume_transition_persist_and_reflect() {
    let path = temp_state_path("transition");
    let status = Arc::new(StatusService::new());
    let ctrl = RunStateController::new(path.clone(), status.clone());

    assert_eq!(ctrl.current().mode, RunMode::Running);

    ctrl.pause().unwrap();
    assert_eq!(ctrl.current().mode, RunMode::Paused);
    // R-U7B-12: pause 는 StatusService 에 Paused 를 반영한다.
    assert_eq!(status.snapshot().operational, OperationalState::Paused);
    // 멱등 pause 는 no-op 성공.
    ctrl.pause().unwrap();
    assert_eq!(ctrl.current().mode, RunMode::Paused);

    // paused 는 재시작(reopen)에도 보존된다(R-U7B-11).
    let status2 = Arc::new(StatusService::new());
    let reopened = RunStateController::new(path.clone(), status2);
    assert_eq!(reopened.current().mode, RunMode::Paused);

    ctrl.resume().unwrap();
    assert_eq!(ctrl.current().mode, RunMode::Running);
    // R-U7B-13: resume 은 Idle 로 되돌린다.
    assert_eq!(status.snapshot().operational, OperationalState::Idle);
}

#[test]
fn volatile_signals_are_not_persisted() {
    let path = temp_state_path("volatile");
    let status = Arc::new(StatusService::new());
    let ctrl = RunStateController::new(path.clone(), status);
    ctrl.request_sync_now();
    ctrl.request_stop(StopMode::Graceful);
    let st = ctrl.current();
    assert!(st.sync_requested);
    assert_eq!(st.stop_requested, Some(StopMode::Graceful));

    // 재시작 후 휘발성 신호는 기본값으로 리셋된다(D-U7B-07).
    let status2 = Arc::new(StatusService::new());
    let reopened = RunStateController::new(path, status2);
    let st2 = reopened.current();
    assert!(!st2.sync_requested);
    assert_eq!(st2.stop_requested, None);
}

#[test]
fn corrupt_state_file_recovers_to_running() {
    let path = temp_state_path("corrupt");
    std::fs::write(&path, b"not valid cbor at all").unwrap();
    let status = Arc::new(StatusService::new());
    let ctrl = RunStateController::new(path, status);
    // 손상 파일은 패닉 없이 Running 으로 안전 회복(R-U7B-11).
    assert_eq!(ctrl.current().mode, RunMode::Running);
}

// ---------------------------------------------------------------------------
// IPC 프레이밍 + 버전 핸드셰이크 + 서버 라운드트립
// ---------------------------------------------------------------------------

/// 인메모리 fake 스트림 — 읽을 요청 프레임 + 기록된 응답 버퍼(소켓 없이 프로토콜 검증).
struct FakeStream {
    to_read: Cursor<Vec<u8>>,
    written: Vec<u8>,
}

impl IpcStream for FakeStream {
    fn read_frame(&mut self) -> Result<Vec<u8>, IpcError> {
        read_frame_from(&mut self.to_read)
    }
    fn write_frame(&mut self, payload: &[u8]) -> Result<(), IpcError> {
        write_frame_to(&mut self.written, payload)
    }
}

fn frame_of(request: &ControlRequest) -> Vec<u8> {
    let mut buf = Vec::new();
    let payload = foundation::encode(request).unwrap();
    write_frame_to(&mut buf, &payload).unwrap();
    buf
}

fn decode_response(written: &[u8]) -> ControlResponse {
    let mut cursor = Cursor::new(written.to_vec());
    let payload = read_frame_from(&mut cursor).unwrap();
    foundation::decode(&payload).unwrap()
}

#[test]
fn frame_roundtrip_preserves_payload() {
    let payload = b"hello-frame-payload".to_vec();
    let mut buf = Vec::new();
    write_frame_to(&mut buf, &payload).unwrap();
    let mut cursor = Cursor::new(buf);
    let restored = read_frame_from(&mut cursor).unwrap();
    assert_eq!(restored, payload);
}

#[test]
fn truncated_frame_is_protocol_error_not_panic() {
    // 길이 프리픽스만 있고 페이로드 절단.
    let mut buf = Vec::new();
    write_frame_to(&mut buf, b"payload").unwrap();
    buf.truncate(10); // 8바이트 헤더 + 2바이트 페이로드만 남김
    let mut cursor = Cursor::new(buf);
    assert_eq!(read_frame_from(&mut cursor), Err(IpcError::Protocol));
}

#[test]
fn server_dispatch_roundtrip_with_recording_handlers() {
    let handlers = Arc::new(RecordingHandlers::new());
    let plane = ControlPlane::new(handlers.clone());

    let request = ControlRequest {
        proto_version: PROTO_VERSION,
        op: ControlOp::Pause,
    };
    let mut stream = FakeStream {
        to_read: Cursor::new(frame_of(&request)),
        written: Vec::new(),
    };
    plane.handle_conn(&mut stream).unwrap();

    let response = decode_response(&stream.written);
    assert_eq!(response.proto_version, PROTO_VERSION);
    assert_eq!(response.result, ControlResult::Ack);
    assert_eq!(handlers.total_calls(), 1);
    assert!(handlers.was_called("pause"));
}

#[test]
fn version_mismatch_blocks_dispatch_before_side_effects() {
    let handlers = RecordingHandlers::new();
    let request = ControlRequest {
        proto_version: PROTO_VERSION + 7,
        op: ControlOp::Pause,
    };
    let response = handle_request(&handlers, request);
    match response.result {
        ControlResult::Error(ControlError::VersionMismatch { server, client }) => {
            assert_eq!(server, PROTO_VERSION);
            assert_eq!(client, PROTO_VERSION + 7);
        }
        other => panic!("VersionMismatch 예상, 실제: {other:?}"),
    }
    // 어떤 핸들러도 호출되지 않아야 한다(R-U7B-06).
    assert_eq!(handlers.total_calls(), 0);
}

#[test]
fn dispatch_history_maps_error_into_response_frame() {
    // 손상된 히스토리 파일 -> query 실패 -> HistoryUnavailable(응답 프레임 안, 전송 성공).
    let dir = std::env::temp_dir().join(format!("okc-u7b-hist-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let hist_path = dir.join("history.cbor");
    // 중간 프레임 손상: 유효 프레임 하나 뒤에 손상 프레임을 붙인다.
    let store = UploadHistoryStore::new(hist_path.clone(), None);
    store.append(UploadHistoryRecord {
        timestamp: Timestamp::from_unix_nanos(1),
        bytes_transferred: ByteCount::new(10),
        error_detail: None,
    });
    // 손상 바이트 추가(중간 프레임처럼 보이는 길이 헤더 + 잘못된 CBOR).
    let mut raw = std::fs::read(&hist_path).unwrap();
    raw.extend_from_slice(&3u64.to_le_bytes());
    raw.extend_from_slice(&[0xff, 0xff, 0xff]);
    std::fs::write(&hist_path, &raw).unwrap();

    let handlers = real_handlers(store);
    let result = dispatch(&handlers, ControlOp::History(HistoryFilter::default()));
    assert!(matches!(
        result,
        ControlResult::Error(ControlError::HistoryUnavailable(_))
    ));
}

// ---------------------------------------------------------------------------
// health -> exit code
// ---------------------------------------------------------------------------

#[test]
fn health_exit_code_mapping_is_exhaustive() {
    let healthy = Ok(ControlResponse {
        proto_version: PROTO_VERSION,
        result: ControlResult::Health(Health::Healthy),
    });
    assert_eq!(map_health(&healthy).exit_code.0, 0);

    let unhealthy = Ok(ControlResponse {
        proto_version: PROTO_VERSION,
        result: ControlResult::Health(Health::Unhealthy {
            reasons: vec![HealthReason("auth".to_string())],
        }),
    });
    assert_eq!(map_health(&unhealthy).exit_code.0, 1);

    let unreachable: Result<ControlResponse, IpcError> = Err(IpcError::Connect);
    assert_eq!(map_health(&unreachable).exit_code.0, 2);
}

// ---------------------------------------------------------------------------
// 라우팅 이분법 (fake client / service)
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
    installed: AtomicBool,
    uninstalled: AtomicBool,
}

impl ServiceOps for FakeService {
    fn install(&self) -> Result<String, String> {
        self.installed.store(true, Ordering::SeqCst);
        Ok("installed".to_string())
    }
    fn uninstall(&self, _opts: &UninstallOptions) -> Result<String, String> {
        self.uninstalled.store(true, Ordering::SeqCst);
        Ok("uninstalled".to_string())
    }
}

#[test]
fn install_routes_to_service_not_ipc() {
    let client = FakeClient::default();
    let service = FakeService::default();
    let args = CliArgs {
        command: Command::Install,
        json: false,
    };
    let result = run(&args, &client, &service);
    assert_eq!(result.exit_code.0, 0);
    assert!(service.installed.load(Ordering::SeqCst));
    assert_eq!(client.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn status_routes_to_ipc_not_service() {
    let client = FakeClient::default();
    let service = FakeService::default();
    let args = CliArgs {
        command: Command::Status,
        json: false,
    };
    let result = run(&args, &client, &service);
    assert_eq!(result.exit_code.0, 0);
    assert_eq!(client.calls.load(Ordering::SeqCst), 1);
    assert!(!service.installed.load(Ordering::SeqCst));
    assert!(!service.uninstalled.load(Ordering::SeqCst));
}

#[test]
fn reload_without_load_surfaces_domain_error() {
    // 최초 load 미완료 상태 reload -> ConfigError -> ReloadFailed(응답 프레임 안).
    let handlers = real_handlers(UploadHistoryStore::new(
        std::env::temp_dir().join(format!("okc-u7b-reload-{}.cbor", std::process::id())),
        None,
    ));
    let result = dispatch(&handlers, ControlOp::Reload);
    assert!(matches!(
        result,
        ControlResult::Error(ControlError::ReloadFailed(_))
    ));
}

// ---------------------------------------------------------------------------
// 헬퍼 — 실 WatcherHandlers 조립 + 기록 fake 핸들러
// ---------------------------------------------------------------------------

fn real_handlers(history: UploadHistoryStore) -> WatcherHandlers {
    let dir = std::env::temp_dir().join(format!(
        "okc-u7b-real-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let status = Arc::new(StatusService::new());
    let consent = Arc::new(ConsentGate::open_with_system_clock(
        dir.join("consent.cbor"),
        None,
    ));
    let config = Arc::new(ConfigProvider::new(&[]));
    let run_state = Arc::new(RunStateController::new(
        dir.join("run-state.cbor"),
        status.clone(),
    ));
    WatcherHandlers::new(status, Arc::new(history), consent, config, run_state)
}

struct RecordingHandlers {
    total: AtomicUsize,
    names: std::sync::Mutex<Vec<String>>,
}

impl RecordingHandlers {
    fn new() -> Self {
        RecordingHandlers {
            total: AtomicUsize::new(0),
            names: std::sync::Mutex::new(Vec::new()),
        }
    }
    fn note(&self, name: &str) {
        self.total.fetch_add(1, Ordering::SeqCst);
        self.names
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(name.to_string());
    }
    fn total_calls(&self) -> usize {
        self.total.load(Ordering::SeqCst)
    }
    fn was_called(&self, name: &str) -> bool {
        self.names
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .iter()
            .any(|n| n == name)
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
        self.note("status");
        empty_snapshot()
    }
    fn health(&self) -> Health {
        self.note("health");
        Health::Healthy
    }
    fn history(&self, _filter: HistoryFilter) -> Result<Vec<UploadHistoryRecord>, ControlError> {
        self.note("history");
        Ok(Vec::new())
    }
    fn pause(&self) -> Result<(), ControlError> {
        self.note("pause");
        Ok(())
    }
    fn resume(&self) -> Result<(), ControlError> {
        self.note("resume");
        Ok(())
    }
    fn sync_now(&self) {
        self.note("sync_now");
    }
    fn stop(&self, _mode: StopMode) {
        self.note("stop");
    }
    fn consent_view(&self) -> ConsentView {
        self.note("consent_view");
        ConsentView {
            consent: ConsentState::Unknown,
            acknowledged: false,
        }
    }
    fn consent_grant(&self) -> Result<(), ControlError> {
        self.note("consent_grant");
        Ok(())
    }
    fn consent_withdraw(&self) -> Result<(), ControlError> {
        self.note("consent_withdraw");
        Ok(())
    }
    fn consent_acknowledge(&self) -> Result<(), ControlError> {
        self.note("consent_acknowledge");
        Ok(())
    }
    fn reload(&self) -> Result<(), ControlError> {
        self.note("reload");
        Ok(())
    }
}

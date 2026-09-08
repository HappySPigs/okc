//! `RunStateController` — 권위 있는 run-state(running|paused + sync/stop 신호) 보유 +
//! crash-atomic 지속 + `StatusSink` 반영.
//!
//! `mode`(running|paused)만 재시작-지속하며(D-U7B-07), 지속 파이프라인은 U4 `SyncStateStore`
//! (`encode(CBOR) -> 같은 디렉터리 temp -> fsync -> atomic rename -> 부모 dir fsync`, R-STATE-01)를
//! 미러한다(R-U7B-11). `sync_requested`/`stop_requested` 는 휘발성 신호라 지속하지 않고 기동 시
//! 기본값으로 리셋된다. 손상/절단/부재/잔존-temp 상태 파일은 패닉 없이 `Running` 으로 안전
//! 회복한다.
//!
//! pause/resume 성공 시 주입된 `Arc<dyn StatusSink>`(U0 트레이트, U6 `StatusService` 구현)에
//! `set_operational(Paused|Idle)` 을 호출해 `watcher status` 가 paused 를 표면화하게 한다
//! (R-U7B-12/13). U8 이 `current()` 로 상태를 읽고 신호를 consume-and-clear 하며 이 컨트롤러는
//! U8 을 호출하지 않는다(no back-reference, US-E6-03).
//!
//! `std::fs` I/O 와 `Mutex` 를 보유하는 상태 모듈이므로 순수 lint-gate 는 두지 않는다(단, 어떤
//! 경로에서도 `unwrap`/`expect`/`panic` 을 쓰지 않는다).

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use serde::{Deserialize, Serialize};

use foundation::{OperationalState, StatusSink, decode, encode};

use crate::protocol::StopMode;

/// 실행 모드(축1 지속 대상, D-U7B-07).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    /// 정상 실행(사이클 허용).
    Running,
    /// 운영자 pause(감시 상주, 사이클 보류).
    Paused,
}

/// 권위 있는 실행 상태(인메모리 보유). `mode` 만 지속되고 신호 두 개는 휘발성이다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunState {
    /// running | paused (지속 대상).
    pub mode: RunMode,
    /// sync-now 신호(휘발성, U8 이 consume-and-clear).
    pub sync_requested: bool,
    /// stop 신호(휘발성).
    pub stop_requested: Option<StopMode>,
}

/// 재시작-지속 최소 부분집합(D-U7B-07).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedRunState {
    /// `RunState.mode == Paused` 여부만 지속.
    pub paused: bool,
}

/// run-state 오류 taxonomy(U7b 소유, D-U7B-12).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RunStateError {
    /// 상태 파일 원자 쓰기(인코딩/rename) 실패 — 인메모리 전이는 이미 반영됨.
    #[error("run-state 원자 지속 실패")]
    Persist,
    /// 하위 파일 I/O 실패.
    #[error("run-state I/O 실패: {0}")]
    Io(String),
}

/// 권위 run-state 컨트롤러. 전이+지속+반영을 임계구역 안에서 직렬화한다.
pub struct RunStateController {
    state: Mutex<RunState>,
    path: PathBuf,
    tmp_path: PathBuf,
    status_sink: Arc<dyn StatusSink>,
}

impl RunStateController {
    /// 상태 파일을 읽어 `paused` 를 복원하고 컨트롤러를 구성한다(휘발성 신호는 기본값).
    ///
    /// 손상/절단/부재/잔존-temp 는 `Running` 으로 안전 회복한다(패닉 없음, R-U7B-11).
    pub fn new(state_path: PathBuf, status_sink: Arc<dyn StatusSink>) -> Self {
        let tmp_path = tmp_path_for(&state_path);
        let mode = match load_persisted(&state_path) {
            Some(true) => RunMode::Paused,
            _ => RunMode::Running,
        };
        RunStateController {
            state: Mutex::new(RunState {
                mode,
                sync_requested: false,
                stop_requested: None,
            }),
            path: state_path,
            tmp_path,
            status_sink,
        }
    }

    /// 현재 run-state 스냅샷을 반환한다(U8 이 read + consume-and-clear).
    pub fn current(&self) -> RunState {
        self.lock().clone()
    }

    /// 감시를 일시중지한다(멱등). `Running -> Paused` 시 지속 + `set_operational(Paused)` 반영.
    ///
    /// 이미 `Paused` 면 no-op 성공(재지속·재반영 불필요, R-U7B-10). 지속 실패 시 인메모리 전이는
    /// 반영된 채 `Err(Persist)` 를 반환한다(인메모리 권위).
    pub fn pause(&self) -> Result<(), RunStateError> {
        let mut state = self.lock();
        if state.mode == RunMode::Paused {
            return Ok(());
        }
        state.mode = RunMode::Paused;
        self.persist(true)?;
        self.status_sink.set_operational(OperationalState::Paused);
        Ok(())
    }

    /// 감시를 재개한다(멱등). `Paused -> Running` 시 지속 + `set_operational(Idle)` 반영.
    ///
    /// `OperationalState` 에 `Running` 변형이 없어 재개 정지 상태는 `Idle` 로 매핑한다(R-U7B-13).
    pub fn resume(&self) -> Result<(), RunStateError> {
        let mut state = self.lock();
        if state.mode == RunMode::Running {
            return Ok(());
        }
        state.mode = RunMode::Running;
        self.persist(false)?;
        self.status_sink.set_operational(OperationalState::Idle);
        Ok(())
    }

    /// 즉시 동기화를 요청한다(휘발성, 미지속). 이미 세팅되어 있으면 no-op.
    pub fn request_sync_now(&self) {
        self.lock().sync_requested = true;
    }

    /// 종료를 요청한다(휘발성, 미지속). 마지막 값이 유지된다.
    pub fn request_stop(&self, mode: StopMode) {
        self.lock().stop_requested = Some(mode);
    }

    /// crash-atomic 지속(R-U7B-11) — U4 파이프라인 미러(`encode -> temp fsync -> rename -> dir fsync`).
    fn persist(&self, paused: bool) -> Result<(), RunStateError> {
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| RunStateError::Io(e.to_string()))?;
            }
        let bytes = encode(&PersistedRunState { paused }).map_err(|_| RunStateError::Persist)?;
        {
            let mut file =
                File::create(&self.tmp_path).map_err(|e| RunStateError::Io(e.to_string()))?;
            file.write_all(&bytes)
                .map_err(|e| RunStateError::Io(e.to_string()))?;
            file.sync_all().map_err(|e| RunStateError::Io(e.to_string()))?;
        }
        std::fs::rename(&self.tmp_path, &self.path).map_err(|_| RunStateError::Persist)?;
        fsync_parent_dir(&self.path);
        Ok(())
    }

    /// 뮤텍스를 취득한다. poison 시 패닉 대신 내부 상태를 회수한다(회복력, U0 관례).
    fn lock(&self) -> MutexGuard<'_, RunState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// 상태 파일을 읽어 `paused` 를 복원한다. 부재/절단/손상은 `None`(= Running 안전 회복).
fn load_persisted(path: &Path) -> Option<bool> {
    let bytes = std::fs::read(path).ok()?;
    let persisted: PersistedRunState = decode(&bytes).ok()?;
    Some(persisted.paused)
}

/// `<state_path>.tmp` 임시 파일 경로(cross-device rename 회피, 같은 디렉터리).
fn tmp_path_for(path: &Path) -> PathBuf {
    let mut os = path.as_os_str().to_os_string();
    os.push(".tmp");
    PathBuf::from(os)
}

/// 부모 디렉터리 fsync(best-effort) — rename 메타데이터 내구성 배리어(Unix).
#[cfg(unix)]
fn fsync_parent_dir(path: &Path) {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
            && let Ok(dir) = File::open(parent) {
                let _ = dir.sync_all();
            }
}

/// non-Unix: 디렉터리 핸들 fsync 의미가 달라 no-op(내구성은 rename atomicity + temp fsync 에 의존).
#[cfg(not(unix))]
fn fsync_parent_dir(_path: &Path) {}

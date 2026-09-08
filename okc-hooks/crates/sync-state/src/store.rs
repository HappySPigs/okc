//! SyncStateStore — crash-atomic 상태 지속/복구(유일 I/O·상태 보유 컴포넌트).
//!
//! 최신-상태-대체 모델(FQ-2=A)의 지속 애그리게이트 `PersistedState`(마지막 커밋 `Manifest`
//! 1개 + dirty boolean + 재개 오프셋 맵)를 **하나의 원자 쓰기 단위**로 디스크에 지속한다.
//! 모든 지속 쓰기는 `bytes -> 같은 디렉터리 temp 기록 -> fsync -> atomic rename -> 부모 dir
//! fsync(best-effort)` 파이프라인을 경유한다(R-STATE-01). 적재 시 손상/절단/잔존-temp 입력은
//! 패닉 없이 last-good/빈 상태로 복구한다(R-STATE-04).
//!
//! 이 모듈은 `std::fs` I/O 를 수행하므로 U0 의 `store.rs`/`loader.rs` 처럼 순수 리프
//! clippy lint-gate 를 두지 않는다(단, 프로덕션 경로에서 `unwrap`/`expect`/`panic` 은 쓰지 않는다).

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use foundation::{ByteCount, CodecError, Manifest, Sha256Digest, decode, encode};
use serde::{Deserialize, Serialize};

/// 진행 중 업로드의 blob 별 마지막 ack 재개 오프셋 맵.
///
/// 키 = blob 의 `raw_sha256`(`Sha256Digest`), 값 = 마지막 ack 바이트 오프셋(`ByteCount`).
/// `BTreeMap` 의 결정적 정렬이 무손실 round-trip(PROP-U4-03)의 안정성을 보장한다.
pub type ResumeOffsetMap = BTreeMap<Sha256Digest, ByteCount>;

/// 디스크에 원자적으로 저장/복구되는 단일 결합 지속 문서(최신-상태-대체 스냅샷).
///
/// 세 필드는 개별 저장되지 않고 함께 인코딩·교체된다(R-STATE-02, 중간 상태 없음).
/// 무손실 CBOR round-trip 대상이다(NFR-13, PROP-U4-03): `decode(encode(s)) == s`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedState {
    /// 마지막으로 커밋된 볼트 매니페스트(커밋 이력 없으면 `None`).
    pub last_committed: Option<Manifest>,
    /// 미커밋 변경 존재를 나타내는 단일 신호(누적 이벤트 큐가 아님, R-STATE-05).
    pub dirty: bool,
    /// 진행 중 업로드의 blob 별 재개 오프셋(커밋 성공 시 비워짐, R-RESUME-03).
    pub resume_offsets: ResumeOffsetMap,
}

/// `SyncStateStore.open_and_recover` 의 입력 — 상태 파일 위치 config 투영(U4 소유 키).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StateConfig {
    /// 상태 파일의 절대 경로. `None` 이면 플랫폼 기본 경로를 사용한다.
    pub state_path: Option<PathBuf>,
}

/// 손상 복구가 도달한 상태 목적지(빈 초기 상태 vs 마지막 정상 상태).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveredState {
    /// 빈 초기 상태로 복구됨(최종 파일 부재 또는 디코드 불가).
    Empty,
    /// 마지막 정상(last-good) 상태로 복구됨.
    LastGood,
}

/// `SyncStateStore` 연산의 실패 taxonomy(운영 오류, `thiserror` 파생).
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    /// 손상 감지 -> last-good/빈 상태로 복구됨(오류라기보다 **비치명 복구 신호**).
    ///
    /// `recovered_to` 로 복구된 목적지를 보고한다. 호출부(U8)는 이를 재조정(FR-04) 유발
    /// 신호로 받아 계속 진행하며, 재-`open_and_recover` 시 dirty=true 인 빈 상태를 얻는다.
    #[error("상태 파일 손상 감지 -> 복구됨(recovered_to={recovered_to:?})")]
    CorruptRecovered {
        /// 복구가 도달한 상태 목적지.
        recovered_to: RecoveredState,
    },
    /// 파일 I/O 실패(쓰기/읽기/rename).
    #[error("상태 파일 I/O 실패: {0}")]
    Io(#[from] std::io::Error),
    /// 인코딩/디코딩 실패(U0 `CodecError`, Fatal 계열).
    #[error("상태 직렬화/역직렬화 실패: {0}")]
    Serde(#[from] CodecError),
}

/// crash-atomic 상태 지속/복구를 소유하는 스토어. 프로세스 내 단일 writer 를 전제한다(R-STATE-03).
#[derive(Debug)]
pub struct SyncStateStore {
    /// 최종 상태 파일 경로.
    path: PathBuf,
    /// 같은 디렉터리의 임시 파일 경로(`<state_path>.tmp`) — cross-device rename 회피.
    tmp_path: PathBuf,
    /// 인메모리 권위 상태(디스크와 항상 정합, 모든 mutator 가 원자 쓰기로 반영).
    state: PersistedState,
}

impl SyncStateStore {
    /// 상태 파일을 열어 복구한다(R-STATE-04, total·panic-free).
    ///
    /// - 최종 파일 **부재** -> 빈 초기 상태(`Ok`, 디스크 미기록).
    /// - 정상 **디코드** -> 잔존 임시 파일 정리 후 last-good 채택(`Ok`).
    /// - 디코드 **실패**(손상) -> 빈 상태 + `dirty=true` 를 원자 기록하고
    ///   `Err(StateError::CorruptRecovered{recovered_to: Empty})` 반환(비치명 신호).
    pub fn open_and_recover(cfg: &StateConfig) -> Result<Self, StateError> {
        let path = resolve_state_path(cfg);
        let tmp_path = tmp_path_for(&path);

        match std::fs::read(&path) {
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(Self {
                path,
                tmp_path,
                state: PersistedState::default(),
            }),
            Err(e) => Err(StateError::Io(e)),
            Ok(bytes) => match decode::<PersistedState>(&bytes) {
                Ok(state) => {
                    // 잔존 임시 파일(부분 쓰기 흔적)은 무해하게 정리한다.
                    let _ = std::fs::remove_file(&tmp_path);
                    Ok(Self {
                        path,
                        tmp_path,
                        state,
                    })
                }
                Err(_corrupt) => {
                    // 방어 경로: 최종 파일 손상 -> 빈 상태 + dirty 강제, 완결 상태로 재기록.
                    let recovered = Self {
                        path,
                        tmp_path,
                        state: PersistedState {
                            last_committed: None,
                            dirty: true,
                            resume_offsets: ResumeOffsetMap::new(),
                        },
                    };
                    recovered.persist()?;
                    Err(StateError::CorruptRecovered {
                        recovered_to: RecoveredState::Empty,
                    })
                }
            },
        }
    }

    /// 마지막으로 커밋된 매니페스트를 반환한다(사이클 시작 시 diff 기준선, 없으면 `None`).
    pub fn last_committed_manifest(&self) -> Option<&Manifest> {
        self.state.last_committed.as_ref()
    }

    /// 미커밋 변경 존재 여부(단일 dirty 신호)를 반환한다.
    pub fn is_dirty(&self) -> bool {
        self.state.dirty
    }

    /// `dirty = true` 로 표시하고 원자적으로 지속한다(T1/T5/T6/T8).
    pub fn mark_dirty(&mut self) -> Result<(), StateError> {
        self.state.dirty = true;
        self.persist()
    }

    /// 새 매니페스트를 커밋한다 — `{ last_committed=m, dirty=false, resume_offsets={} }` 를
    /// **한 번의 원자 쓰기**로 반영한다(R-STATE-06, R-RESUME-03: stale 오프셋 clear 흡수).
    pub fn commit_manifest(&mut self, m: Manifest) -> Result<(), StateError> {
        self.state.last_committed = Some(m);
        self.state.dirty = false;
        self.state.resume_offsets.clear();
        self.persist()
    }

    /// 지정 blob 의 진행 중 재개 오프셋을 반환한다(없으면 `None` -> 처음부터 전송).
    pub fn resume_offset(&self, blob: &Sha256Digest) -> Option<ByteCount> {
        self.state.resume_offsets.get(blob).copied()
    }

    /// blob 의 재개 오프셋을 기록하고 원자적으로 지속한다(청크 ack 마다 U3 가 호출).
    pub fn persist_resume_offset(
        &mut self,
        blob: Sha256Digest,
        off: ByteCount,
    ) -> Result<(), StateError> {
        self.state.resume_offsets.insert(blob, off);
        self.persist()
    }

    /// 모든 재개 오프셋을 비우고 원자적으로 지속한다(커밋 없이 진행 중 전송 폐기 시).
    pub fn clear_resume_offsets(&mut self) -> Result<(), StateError> {
        self.state.resume_offsets.clear();
        self.persist()
    }

    /// crash-atomic 원자 쓰기 파이프라인(R-STATE-01) — 모든 지속 mutator 의 단일 경유점.
    ///
    /// `encode -> 같은 디렉터리 temp 기록 -> temp fsync -> atomic rename -> 부모 dir fsync`.
    /// 어느 단계에서 크래시해도 최종 파일은 완결된 last-good 이거나 완결된 새 상태다.
    fn persist(&self) -> Result<(), StateError> {
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }

        let bytes = encode(&self.state)?;

        {
            let mut f = File::create(&self.tmp_path)?;
            f.write_all(&bytes)?;
            f.sync_all()?;
        }

        std::fs::rename(&self.tmp_path, &self.path)?;
        fsync_parent_dir(&self.path);
        Ok(())
    }
}

/// `StateConfig.state_path` 를 해소한다(지정 시 그대로, 부재 시 플랫폼 기본 경로).
fn resolve_state_path(cfg: &StateConfig) -> PathBuf {
    match &cfg.state_path {
        Some(p) => p.clone(),
        None => default_state_path(),
    }
}

/// 플랫폼 기본 상태 파일 경로(`<state-base-dir>/okc-watcher/sync-state.cbor`).
fn default_state_path() -> PathBuf {
    state_base_dir().join("okc-watcher").join("sync-state.cbor")
}

/// Windows 상태 기본 디렉터리(`%LOCALAPPDATA%`, 부재 시 시스템 temp).
#[cfg(windows)]
fn state_base_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

/// non-Windows 상태 기본 디렉터리(`$XDG_STATE_HOME` -> `$HOME/.local/state` -> 시스템 temp).
#[cfg(not(windows))]
fn state_base_dir() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_STATE_HOME") {
        return PathBuf::from(xdg);
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".local").join("state");
    }
    std::env::temp_dir()
}

/// 최종 파일과 **같은 디렉터리**의 임시 파일 경로(`<state_path>.tmp`)를 구성한다.
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

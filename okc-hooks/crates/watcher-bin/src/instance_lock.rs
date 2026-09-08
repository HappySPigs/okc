//! OS advisory 파일 잠금 기반 단일 인스턴스 보호.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use foundation::Timestamp;
use serde::{Deserialize, Serialize};

/// 단일 인스턴스 락 파일 위치.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockConfig {
    /// data-dir 하위 락 파일.
    pub path: PathBuf,
}

/// 락파일에 기록되는 활성 소유자 정보.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockRecord {
    /// 프로세스 식별자.
    pub pid: u32,
    /// OS 사용자/소유자 라벨.
    pub owner: String,
    /// 획득 시각.
    pub started_at: Timestamp,
}

/// 이미 실행 중인 인스턴스 보고용 최소 정보.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceInfo {
    /// 활성 프로세스 식별자. 레코드가 손상되면 0.
    pub pid: u32,
    /// 활성 소유자. 레코드가 손상되면 `unknown`.
    pub owner: String,
}

/// 단일 인스턴스 획득 오류.
#[derive(Debug, thiserror::Error)]
pub enum LockError {
    /// 다른 live holder가 advisory lock을 보유한다.
    #[error("Watcher가 이미 실행 중입니다(pid={pid}, owner={owner})")]
    AlreadyRunning {
        /// 활성 PID.
        pid: u32,
        /// 활성 소유자.
        owner: String,
    },
    /// 락파일 열기/잠금/기록 오류.
    #[error("단일 인스턴스 락 I/O 실패: {0}")]
    Io(#[from] std::io::Error),
    /// 락 레코드 직렬화 오류.
    #[error("단일 인스턴스 락 레코드 직렬화 실패: {0}")]
    Record(String),
}

/// 획득한 advisory lock의 수명 가드. drop 시 열린 파일과 OS lock이 함께 해제된다.
pub struct LockGuard {
    _file: File,
    record: LockRecord,
}

impl LockGuard {
    /// 현재 holder 레코드를 반환한다.
    pub fn record(&self) -> &LockRecord {
        &self.record
    }
}

/// 단일 인스턴스 잠금 연산의 네임스페이스.
pub struct SingleInstanceLock;

impl SingleInstanceLock {
    /// 락을 비차단 획득한다. free/stale 파일은 새 레코드로 덮고, live holder는 보고 후 거부한다.
    pub fn acquire(config: &LockConfig) -> Result<LockGuard, LockError> {
        ensure_parent(&config.path)?;
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&config.path)?;

        match file.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                let info = read_instance_info(&mut file).unwrap_or_else(unknown_instance);
                return Err(LockError::AlreadyRunning {
                    pid: info.pid,
                    owner: info.owner,
                });
            }
            Err(TryLockError::Error(source)) => return Err(LockError::Io(source)),
        }

        let record = LockRecord {
            pid: std::process::id(),
            owner: current_owner(),
            started_at: now_timestamp(),
        };
        let bytes = serde_json::to_vec(&record).map_err(|error| LockError::Record(error.to_string()))?;
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        Ok(LockGuard {
            _file: file,
            record,
        })
    }

    /// 락파일 레코드를 best-effort로 조회한다. 파일 부재/손상은 `None`이다.
    pub fn holder_info(config: &LockConfig) -> Option<InstanceInfo> {
        let mut file = OpenOptions::new().read(true).open(&config.path).ok()?;
        read_instance_info(&mut file)
    }
}

fn ensure_parent(path: &Path) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    Ok(())
}

fn read_instance_info(file: &mut File) -> Option<InstanceInfo> {
    let _ = file.seek(SeekFrom::Start(0));
    let mut text = String::new();
    file.read_to_string(&mut text).ok()?;
    let record: LockRecord = serde_json::from_str(&text).ok()?;
    Some(InstanceInfo {
        pid: record.pid,
        owner: record.owner,
    })
}

fn unknown_instance() -> InstanceInfo {
    InstanceInfo {
        pid: 0,
        owner: "unknown".to_string(),
    }
}

fn current_owner() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn now_timestamp() -> Timestamp {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_nanos()).unwrap_or(i64::MAX))
        .unwrap_or(0);
    Timestamp::from_unix_nanos(nanos)
}

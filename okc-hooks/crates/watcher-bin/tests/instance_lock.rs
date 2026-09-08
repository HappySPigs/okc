//! `SingleInstanceLock` 예제 테스트(always-compiled) — free/stale 획득 + live holder 상호배제.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use watcher_bin::instance_lock::{LockConfig, LockError, SingleInstanceLock};

/// 프로세스 내 유일한 임시 락파일 경로를 만든다.
fn unique_lock_path(tag: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut path = std::env::temp_dir();
    path.push(format!("okc_u8_lock_{}_{}_{}", tag, std::process::id(), n));
    let _ = std::fs::remove_file(&path);
    path
}

/// free(파일 부재) -> 획득 성공 + 현재 pid 로 레코드 기록.
#[test]
fn free_acquires_and_records_pid() {
    let path = unique_lock_path("free");
    let cfg = LockConfig { path: path.clone() };
    let guard = SingleInstanceLock::acquire(&cfg).expect("free acquire");
    assert_eq!(guard.record().pid, std::process::id());
    drop(guard);
    let _ = std::fs::remove_file(&path);
}

/// live holder 점유 중 두 번째 획득은 `AlreadyRunning`, drop 후 재획득 성공(정리 보장).
#[test]
fn live_holder_excludes_second_then_reacquires_after_drop() {
    let path = unique_lock_path("live");
    let cfg = LockConfig { path: path.clone() };
    let guard = SingleInstanceLock::acquire(&cfg).expect("first acquire");

    let second = SingleInstanceLock::acquire(&cfg);
    assert!(matches!(second, Err(LockError::AlreadyRunning { .. })));
    drop(second);
    drop(guard);

    let reacquired = SingleInstanceLock::acquire(&cfg).expect("reacquire after drop");
    drop(reacquired);
    let _ = std::fs::remove_file(&path);
}

/// stale(레코드는 있으나 아무도 점유 안 함) -> 회수 후 획득 성공.
#[test]
fn stale_lockfile_is_reclaimed() {
    let path = unique_lock_path("stale");
    // 잔존 파일(내용 무관): 아무도 advisory lock 을 보유하지 않으므로 try_lock 이 성공한다.
    std::fs::write(&path, b"stale leftover content").expect("seed stale file");
    let cfg = LockConfig { path: path.clone() };
    let guard = SingleInstanceLock::acquire(&cfg).expect("stale reclaim");
    assert_eq!(guard.record().pid, std::process::id());
    drop(guard);
    let _ = std::fs::remove_file(&path);
}

/// `holder_info` 는 활성 레코드를 best-effort 로 조회한다.
#[test]
fn holder_info_reports_active_record() {
    let path = unique_lock_path("info");
    let cfg = LockConfig { path: path.clone() };
    let guard = SingleInstanceLock::acquire(&cfg).expect("acquire");
    let info = SingleInstanceLock::holder_info(&cfg).expect("holder info");
    assert_eq!(info.pid, std::process::id());
    drop(guard);
    let _ = std::fs::remove_file(&path);
}

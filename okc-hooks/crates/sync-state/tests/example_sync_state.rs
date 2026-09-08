//! 예제 기반 앵커 테스트(PBT-10) — business-critical 경로를 PBT 와 병행 검증한다.
//!
//! 이 파일은 feature 게이트 없이 기본 `cargo test` 로 실행된다: crash-atomic 지속/복구,
//! latest-state-wins, 손상 복구, classify total 매핑, 백오프 단조·유계, 에스컬레이션 게이트.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use foundation::{
    ByteCount, Manifest, ManifestDigest, ManifestEntry, RelativePath, Sha256Digest, TransportError,
    TransportErrorClass,
};
use sync_state::{
    BackoffConfig, JitterMode, RecoveredState, RetryBackoffController, RetryClass, StateConfig,
    StateError, SyncStateStore,
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// 프로세스 내 유일한 임시 상태 파일 경로를 만든다(부모 디렉터리 생성 포함).
fn unique_state_path(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "okc-u4-{tag}-{pid}-{n}",
        pid = std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("temp 디렉터리 생성");
    dir.join("sync-state.cbor")
}

/// `<state_path>.tmp` 임시 파일 경로(스토어 내부 규약과 동일).
fn tmp_path_of(path: &Path) -> PathBuf {
    let mut os = path.as_os_str().to_os_string();
    os.push(".tmp");
    PathBuf::from(os)
}

/// 지정 SHA 바이트를 채운 단일 엔트리 매니페스트를 만든다(테스트 픽스처).
fn make_manifest(seed: u8) -> Manifest {
    Manifest {
        entries: vec![ManifestEntry {
            relative_path: RelativePath::normalize("dir/file.md").expect("정규 경로"),
            raw_sha256: Sha256Digest::from_bytes([seed; 32]),
            size: ByteCount::new(u64::from(seed) * 100),
        }],
        manifest_digest: ManifestDigest::from_bytes([seed; 32]),
    }
}

fn open(path: &Path) -> Result<SyncStateStore, StateError> {
    SyncStateStore::open_and_recover(&StateConfig {
        state_path: Some(path.to_path_buf()),
    })
}

// ---------------------------------------------------------------------------
// SyncStateStore
// ---------------------------------------------------------------------------

#[test]
fn open_missing_yields_empty_state() {
    let path = unique_state_path("missing");
    let store = open(&path).expect("최초 실행은 빈 상태 Ok");
    assert!(!store.is_dirty());
    assert!(store.last_committed_manifest().is_none());
}

#[test]
fn commit_and_resume_survive_reopen() {
    let path = unique_state_path("persist");
    let m = make_manifest(7);
    let blob = Sha256Digest::from_bytes([9; 32]);
    {
        let mut store = open(&path).expect("open");
        store.mark_dirty().expect("mark_dirty");
        store
            .persist_resume_offset(blob, ByteCount::new(42))
            .expect("persist offset");
        assert!(store.is_dirty());
        store.commit_manifest(m.clone()).expect("commit");
        // 커밋은 dirty 를 clear 하고 오프셋을 비운다(원자 전이).
        assert!(!store.is_dirty());
        assert!(store.resume_offset(&blob).is_none());
    }
    // 재-open: 커밋된 상태가 크래시/재시작을 넘어 보존된다.
    let reopened = open(&path).expect("reopen");
    assert_eq!(reopened.last_committed_manifest(), Some(&m));
    assert!(!reopened.is_dirty());
}

#[test]
fn latest_state_wins_across_commits() {
    let path = unique_state_path("latest");
    let mut store = open(&path).expect("open");
    for seed in 1u8..=4 {
        store.commit_manifest(make_manifest(seed)).expect("commit");
    }
    let last = make_manifest(4);
    assert_eq!(store.last_committed_manifest(), Some(&last));
    // 재-open 후에도 마지막 커밋이 권위 상태다.
    let reopened = open(&path).expect("reopen");
    assert_eq!(reopened.last_committed_manifest(), Some(&last));
}

#[test]
fn residual_temp_is_cleaned_and_last_good_adopted() {
    let path = unique_state_path("residual");
    let m = make_manifest(3);
    {
        let mut store = open(&path).expect("open");
        store.commit_manifest(m.clone()).expect("commit");
    }
    // rename 직전 크래시 모사: 최종 파일과 같은 디렉터리에 잔존 부분 temp.
    let tmp = tmp_path_of(&path);
    std::fs::write(&tmp, b"partial garbage bytes").expect("write residual temp");

    let reopened = open(&path).expect("reopen는 last-good 을 채택");
    assert_eq!(reopened.last_committed_manifest(), Some(&m));
    assert!(!tmp.exists(), "잔존 temp 는 정리되어야 한다");
}

#[test]
fn corrupt_final_file_recovers_empty_dirty() {
    let path = unique_state_path("corrupt");
    {
        let mut store = open(&path).expect("open");
        store.commit_manifest(make_manifest(5)).expect("commit");
    }
    // 최종 파일 손상 주입(방어 경로).
    std::fs::write(&path, b"\xff\xff not-cbor \x00\x01").expect("corrupt");

    match open(&path) {
        Err(StateError::CorruptRecovered { recovered_to }) => {
            assert_eq!(recovered_to, RecoveredState::Empty);
        }
        other => panic!("CorruptRecovered 를 기대했으나: {other:?}"),
    }

    // 재-open: 손상은 빈 상태 + dirty=true 로 원자 재기록되었다(재조정 유발).
    let recovered = open(&path).expect("재-open 은 Ok");
    assert!(recovered.is_dirty());
    assert!(recovered.last_committed_manifest().is_none());
}

// ---------------------------------------------------------------------------
// RetryBackoffController
// ---------------------------------------------------------------------------

#[test]
fn classify_total_mapping_no_permanent() {
    let cases = [
        (TransportErrorClass::AuthFailed, RetryClass::AuthFailed),
        (TransportErrorClass::ServerError, RetryClass::Transient),
        (TransportErrorClass::Backpressure, RetryClass::Backpressure),
        (TransportErrorClass::Timeout, RetryClass::Offline),
        (TransportErrorClass::Network, RetryClass::Offline),
    ];
    for (class, expected) in cases {
        let err = TransportError {
            class,
            http_status: None,
            detail: String::new(),
        };
        let got = RetryBackoffController::classify(&err);
        assert_eq!(got, expected);
        assert_ne!(got, RetryClass::Permanent, "전송 경로는 Permanent 미산출");
    }
}

fn err(class: TransportErrorClass) -> TransportError {
    TransportError {
        class,
        http_status: None,
        detail: String::new(),
    }
}

#[test]
fn backoff_monotonic_and_capped() {
    let cfg = BackoffConfig {
        initial_delay: Duration::from_secs(1),
        multiplier: 2.0,
        cap: Duration::from_secs(8),
        jitter: JitterMode::None, // 단조/유계 검증을 위해 지터 제거
    };
    let mut ctrl = RetryBackoffController::new(&cfg, 3, 0);
    let now = Instant::now();
    let expected = [1u64, 2, 4, 8, 8, 8];
    let mut prev = Duration::ZERO;
    for want_secs in expected {
        let d = ctrl
            .on_failure(&err(TransportErrorClass::ServerError), now)
            .retry_after
            .expect("Transient 은 Some");
        assert_eq!(d, Duration::from_secs(want_secs));
        assert!(d >= prev, "base_delay 는 비감소");
        assert!(d <= cfg.cap, "상한 유계");
        prev = d;
    }
    // on_success 는 백오프를 initial 로 리셋한다.
    ctrl.on_success();
    let after = ctrl
        .on_failure(&err(TransportErrorClass::ServerError), now)
        .retry_after
        .expect("Some");
    assert_eq!(after, Duration::from_secs(1));
    assert_eq!(ctrl.consecutive_failures(), 1);
}

#[test]
fn escalation_only_transient_at_threshold() {
    let cfg = BackoffConfig::default();
    let mut ctrl = RetryBackoffController::new(&cfg, 3, 42);
    let now = Instant::now();

    // Transient 3회 -> 3번째에 escalate.
    let d1 = ctrl.on_failure(&err(TransportErrorClass::ServerError), now);
    assert!(!d1.escalate);
    let d2 = ctrl.on_failure(&err(TransportErrorClass::ServerError), now);
    assert!(!d2.escalate);
    let d3 = ctrl.on_failure(&err(TransportErrorClass::ServerError), now);
    assert!(d3.escalate, "N=3 도달 시 escalate");
    assert_eq!(ctrl.consecutive_failures(), 3);
}

#[test]
fn offline_and_backpressure_never_escalate() {
    let cfg = BackoffConfig::default();
    let mut ctrl = RetryBackoffController::new(&cfg, 1, 7);
    let now = Instant::now();

    for _ in 0..5 {
        let d = ctrl.on_failure(&err(TransportErrorClass::Timeout), now);
        assert!(d.is_offline);
        assert!(!d.escalate, "오프라인은 에스컬레이션 제외");
        assert!(d.retry_after.is_some());
    }
    let bp = ctrl.on_failure(&err(TransportErrorClass::Backpressure), now);
    assert!(!bp.escalate, "백프레셔는 에스컬레이션 제외");
    assert!(!bp.is_offline);
}

#[test]
fn auth_failed_leaves_counter_unchanged_and_no_retry() {
    let cfg = BackoffConfig::default();
    let mut ctrl = RetryBackoffController::new(&cfg, 3, 1);
    let now = Instant::now();

    // 재시도 대상으로 카운터를 2 로 올린다.
    ctrl.on_failure(&err(TransportErrorClass::ServerError), now);
    ctrl.on_failure(&err(TransportErrorClass::ServerError), now);
    assert_eq!(ctrl.consecutive_failures(), 2);

    // AuthFailed: retry_after None + 카운터 불변(증가·리셋 안 함).
    let auth = ctrl.on_failure(&err(TransportErrorClass::AuthFailed), now);
    assert_eq!(auth.retry_after, None);
    assert!(!auth.is_offline);
    assert!(!auth.escalate);
    assert_eq!(ctrl.consecutive_failures(), 2);
}

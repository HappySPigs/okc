//! Property-Based Testing (PBT, `proptest-support` feature 게이트) — PROP-U4-01~07.
//!
//! feature 가 꺼진 기본 빌드에서는 빈 테스트 크레이트로 컴파일되고,
//! `cargo test --features proptest-support` 로만 실제 property 가 실행된다(PBT-07).
//!
//! 커버: PROP-U4-03(무손실 round-trip), PROP-U4-05(백오프 상태머신 불변식),
//! PROP-U4-06(classify 전역성 + 형상 불변식), PROP-U4-07(에스컬레이션 게이트),
//! PROP-U4-01/02/04(상태머신 명령 시퀀스 + 크래시-복구 삽입 + latest-state-wins).
#![cfg(feature = "proptest-support")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use foundation::proptest_support::generators as fg;
use foundation::{
    ByteCount, Manifest, Sha256Digest, TransportError, TransportErrorClass, decode, encode,
};
use proptest::prelude::*;
use sync_state::proptest_support::generators as g;
use sync_state::{
    BackoffConfig, JitterMode, PersistedState, RetryBackoffController, RetryClass, StateConfig,
    StateError, SyncStateStore,
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_state_path(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "okc-u4-prop-{tag}-{pid}-{n}",
        pid = std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("temp 디렉터리 생성");
    dir.join("sync-state.cbor")
}

fn tmp_path_of(path: &Path) -> PathBuf {
    let mut os = path.as_os_str().to_os_string();
    os.push(".tmp");
    PathBuf::from(os)
}

fn open(path: &Path) -> Result<SyncStateStore, StateError> {
    SyncStateStore::open_and_recover(&StateConfig {
        state_path: Some(path.to_path_buf()),
    })
}

// ---------------------------------------------------------------------------
// PROP-U4-03 — PersistedState 무손실 CBOR round-trip
// ---------------------------------------------------------------------------

proptest! {
    /// `decode(encode(s)) == s` — None/빈/다수 엔트리·경계 오프셋·dirty 양값 모두 보존.
    #[test]
    fn prop_persisted_state_roundtrip(s in g::arb_persisted_state()) {
        let bytes = encode(&s).expect("encode 는 실패하지 않는다");
        let decoded: PersistedState = decode(&bytes).expect("정상 인코딩의 decode 는 성공");
        prop_assert_eq!(decoded, s);
    }
}

// ---------------------------------------------------------------------------
// PROP-U4-06 — classify 전역성 + RetryDecision 형상 불변식
// ---------------------------------------------------------------------------

proptest! {
    /// 임의 `TransportError` 는 정확히 하나의 non-Permanent `RetryClass` 로 매핑된다.
    #[test]
    fn prop_classify_total_no_permanent(err in fg::arb_transport_error()) {
        let class = RetryBackoffController::classify(&err);
        prop_assert_ne!(class, RetryClass::Permanent);
        // 오라클: U0 mapping 과 정합(R-RETRY-01).
        let expected = match err.class {
            TransportErrorClass::AuthFailed => RetryClass::AuthFailed,
            TransportErrorClass::ServerError => RetryClass::Transient,
            TransportErrorClass::Backpressure => RetryClass::Backpressure,
            TransportErrorClass::Timeout => RetryClass::Offline,
            TransportErrorClass::Network => RetryClass::Offline,
        };
        prop_assert_eq!(class, expected);
    }
}

// ---------------------------------------------------------------------------
// PROP-U4-05 / PROP-U4-07 — 백오프 상태머신 + 에스컬레이션 게이트
// ---------------------------------------------------------------------------

/// `on_failure`/`on_success` 시퀀스 원소(성공 섞임).
fn arb_event() -> impl Strategy<Value = Option<TransportErrorClass>> {
    prop_oneof![
        Just(Option::<TransportErrorClass>::None), // on_success
        prop::sample::select(vec![
            TransportErrorClass::AuthFailed,
            TransportErrorClass::ServerError,
            TransportErrorClass::Backpressure,
            TransportErrorClass::Timeout,
            TransportErrorClass::Network,
        ])
        .prop_map(Some),
    ]
}

proptest! {
    /// 임의 이벤트 시퀀스에 대해 백오프·에스컬레이션 불변식이 항상 성립한다.
    #[test]
    fn prop_backoff_state_machine(
        cfg in g::arb_backoff_config(),
        n in 1u32..6,
        seed in any::<u64>(),
        events in prop::collection::vec(arb_event(), 0..40),
    ) {
        let mut ctrl = RetryBackoffController::new(&cfg, n, seed);
        let now = Instant::now();
        let mut consecutive: u32 = 0;

        for ev in events {
            match ev {
                None => {
                    ctrl.on_success();
                    consecutive = 0;
                    prop_assert_eq!(ctrl.consecutive_failures(), 0);
                    prop_assert_eq!(ctrl.next_retry_at(), None);
                }
                Some(class) => {
                    let err = TransportError { class, http_status: None, detail: String::new() };
                    let decision = ctrl.on_failure(&err, now);

                    match RetryBackoffController::classify(&err) {
                        RetryClass::AuthFailed | RetryClass::Permanent => {
                            // 카운터 불변, 재시도 없음.
                            prop_assert_eq!(decision.retry_after, None);
                            prop_assert!(!decision.is_offline);
                            prop_assert!(!decision.escalate);
                            prop_assert_eq!(ctrl.consecutive_failures(), consecutive);
                        }
                        retry_class => {
                            consecutive += 1;
                            // retry_after 는 항상 [0, cap] 유계.
                            let d = decision.retry_after.expect("재시도 대상은 Some");
                            prop_assert!(d <= cfg.cap);
                            prop_assert_eq!(ctrl.consecutive_failures(), consecutive);

                            // is_offline => !escalate 불변식.
                            if decision.is_offline {
                                prop_assert!(!decision.escalate);
                                prop_assert_eq!(retry_class, RetryClass::Offline);
                            }
                            // escalate 는 Transient && consecutive >= N 에서만 true.
                            let want_escalate = retry_class == RetryClass::Transient
                                && consecutive >= n;
                            prop_assert_eq!(decision.escalate, want_escalate);
                        }
                    }
                }
            }
        }
    }
}

proptest! {
    /// 지터 없는 설정에서 연속 재시도 대상 실패의 `base_delay` 는 `cap` 까지 비감소한다.
    #[test]
    fn prop_backoff_non_decreasing_no_jitter(
        seed in any::<u64>(),
        count in 1usize..12,
    ) {
        let cfg = BackoffConfig {
            initial_delay: Duration::from_millis(10),
            multiplier: 2.0,
            cap: Duration::from_secs(30),
            jitter: JitterMode::None,
        };
        let mut ctrl = RetryBackoffController::new(&cfg, 100, seed);
        let now = Instant::now();
        let mut prev = Duration::ZERO;
        for _ in 0..count {
            let err = TransportError {
                class: TransportErrorClass::ServerError,
                http_status: None,
                detail: String::new(),
            };
            let d = ctrl.on_failure(&err, now).retry_after.expect("Some");
            prop_assert!(d >= prev, "비감소 위반: {:?} < {:?}", d, prev);
            prop_assert!(d <= cfg.cap);
            prev = d;
        }
    }
}

// ---------------------------------------------------------------------------
// PROP-U4-01 / PROP-U4-02 / PROP-U4-04 — 상태머신 명령 시퀀스 + 크래시-복구 + latest-wins
// ---------------------------------------------------------------------------

/// 지속 명령. `Recover` 는 persist->drop->open(크래시 모사)을 나타낸다.
#[derive(Debug, Clone)]
enum Cmd {
    MarkDirty,
    Commit(Manifest),
    Persist(Sha256Digest, ByteCount),
    Clear,
    Recover,
    /// 잔존 부분 temp 주입 후 recover(rename 직전 크래시 모사).
    RecoverWithResidualTemp,
}

fn arb_cmd() -> impl Strategy<Value = Cmd> {
    prop_oneof![
        Just(Cmd::MarkDirty),
        fg::arb_manifest().prop_map(Cmd::Commit),
        (fg::arb_sha256_digest(), fg::arb_byte_count())
            .prop_map(|(b, o)| Cmd::Persist(b, o)),
        Just(Cmd::Clear),
        Just(Cmd::Recover),
        Just(Cmd::RecoverWithResidualTemp),
    ]
}

/// 참조 모델에 명령을 반영한다(U0 전이 의미 + PersistedState 규칙).
fn apply_model(model: &mut PersistedState, cmd: &Cmd) {
    match cmd {
        Cmd::MarkDirty => model.dirty = true,
        Cmd::Commit(m) => {
            model.last_committed = Some(m.clone());
            model.dirty = false;
            model.resume_offsets.clear();
        }
        Cmd::Persist(b, o) => {
            model.resume_offsets.insert(*b, *o);
        }
        Cmd::Clear => model.resume_offsets.clear(),
        Cmd::Recover | Cmd::RecoverWithResidualTemp => {} // 지속 상태는 recover 를 넘어 불변
    }
}

proptest! {
    /// 임의 명령 시퀀스(크래시-복구 삽입 포함)에서 관측 상태가 참조 모델과 항상 일치한다.
    ///
    /// - 모든 mutator 는 원자 쓰기로 지속되므로 recover 후에도 상태 불변(PROP-U4-01/02).
    /// - `commit_manifest` 는 last-committed 를 최신으로 대체한다(PROP-U4-04).
    #[test]
    fn prop_state_machine_matches_model(cmds in prop::collection::vec(arb_cmd(), 0..24)) {
        let path = unique_state_path("sm");
        let mut model = PersistedState::default();
        let mut store = open(&path).expect("초기 open");

        for cmd in &cmds {
            apply_model(&mut model, cmd);
            match cmd {
                Cmd::MarkDirty => store.mark_dirty().expect("mark_dirty"),
                Cmd::Commit(m) => store.commit_manifest(m.clone()).expect("commit"),
                Cmd::Persist(b, o) => store.persist_resume_offset(*b, *o).expect("persist offset"),
                Cmd::Clear => store.clear_resume_offsets().expect("clear"),
                Cmd::Recover => {
                    drop(store);
                    store = open(&path).expect("recover open");
                }
                Cmd::RecoverWithResidualTemp => {
                    drop(store);
                    // rename 직전 크래시 모사: 잔존 부분 temp.
                    std::fs::write(tmp_path_of(&path), b"crash-residual").expect("residual");
                    store = open(&path).expect("recover open (residual)");
                }
            }

            // 관측 상태 == 모델.
            prop_assert_eq!(store.last_committed_manifest(), model.last_committed.as_ref());
            prop_assert_eq!(store.is_dirty(), model.dirty);
            for (blob, off) in &model.resume_offsets {
                prop_assert_eq!(store.resume_offset(blob), Some(*off));
            }
        }

        // 최종 상태를 완전 재구성해 대조(round-trip 지속 정합).
        drop(store);
        let final_store = open(&path);
        match final_store {
            Ok(s) => {
                prop_assert_eq!(s.last_committed_manifest(), model.last_committed.as_ref());
                prop_assert_eq!(s.is_dirty(), model.dirty);
                let mut got: BTreeMap<Sha256Digest, ByteCount> = BTreeMap::new();
                for blob in model.resume_offsets.keys() {
                    if let Some(v) = s.resume_offset(blob) {
                        got.insert(*blob, v);
                    }
                }
                prop_assert_eq!(got, model.resume_offsets.clone());
            }
            Err(e) => prop_assert!(false, "예상치 못한 open 실패: {:?}", e),
        }
    }
}

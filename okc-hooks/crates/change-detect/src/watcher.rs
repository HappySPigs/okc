//! `FilesystemWatcher` 애그리게이트 — OS 네이티브 감시 어댑터(`WatchBackend`) + 볼트-전체
//! 단일 디바운스 타이머 스레드 -> 편집 버스트당 1 트리거(FR-01 / NFR-01 / NFR-05).
//!
//! OS I/O 경계(어댑터, `notify` 뒤 캡슐화)와 순수 로직(`debounce::Debouncer`)이 분리된다.
//! `WatchBackend` 트레이트가 OS 차이를 흡수하므로 상위 디바운스 로직은 백엔드 종류를 모른다
//! (behavioral equivalence, PROP-U2-05). rename 은 delete+create 로(D2), 백엔드 오버플로는
//! `Overflow` 로(D6) 정규화한다.
//!
//! 이 모듈은 스레드/파일시스템 I/O 를 다루므로(U0 `store.rs`/`loader.rs` 와 동일) 순수 리프
//! clippy lint-gate 를 부착하지 않는다. `TriggerStream`/조회 표면은 부수효과·push 가 없다
//! (R-PURE-01/R-DEB-06).

use std::any::Any;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use foundation::RelativePath;

use crate::debounce::Debouncer;
use crate::error::{BackendCause, WatchError};
use crate::trigger::{TriggerSignal, TriggerStream};

/// 정규화된 파일시스템 이벤트 어휘(백엔드 무관 공통). rename -> delete+create, 오버플로 ->
/// `Overflow` 로 정규화된다(`domain-entities.md` §2.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawFsEvent {
    /// 파일 생성.
    Created(RelativePath),
    /// 파일 수정.
    Modified(RelativePath),
    /// 파일 삭제.
    Deleted(RelativePath),
    /// 백엔드 이벤트 손실/rescan(inotify `IN_Q_OVERFLOW` / FSEvents 병합 / 무이벤트 마운트).
    /// 최소 1개의 합성 change 로 디바운스에 투입된다(R-DEB-04).
    Overflow,
}

/// 감시자의 조회 가능한 런타임 상태(coordinator/CLI 진단용). 4-변이 폐쇄.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchState {
    /// 백엔드 구독 활성, 이벤트 대기 중.
    Watching,
    /// 디바운스 만료로 트리거 발행 직후(전이 상태).
    Triggered,
    /// 활성이나 최근 이벤트/디바운스 없음.
    Idle,
    /// pause 요청으로 감시 억제(이벤트 폐기, D13) — 데몬은 상주.
    Paused,
}

/// 활성 구독 — 정규화된 이벤트 수신단 + 백엔드 핸들(watcher 를 살려 두는 keepalive).
///
/// `keepalive` 가 drop 되면 백엔드가 해제되고 송신단이 끊겨 `events` 가 disconnect 된다 —
/// 이것이 `FilesystemWatcher` 정지 경로다.
pub struct Subscription {
    /// 정규화된 `RawFsEvent` 수신단.
    pub events: Receiver<RawFsEvent>,
    /// 백엔드 watcher/송신단을 살려 두는 불투명 핸들.
    pub keepalive: Box<dyn Any + Send>,
}

/// OS별 네이티브 감시 API 를 정규화된 이벤트 소스로 추상화하는 트레이트(NFR-05 이식성 경계).
///
/// 모든 백엔드는 동일한 `RawFsEvent` 어휘로 정규화한다 — `notify` 타입은 트레이트 뒤에
/// 캡슐화되어 공개 API 에 새지 않는다.
pub trait WatchBackend: Send {
    /// 볼트 루트를 구독해 정규화된 이벤트 수신단과 활성 구독 핸들을 반환한다.
    fn subscribe(&self, root: &Path) -> Result<Subscription, WatchError>;
}

/// 무이벤트 폴백 백엔드(D3/T7) — 이벤트를 전혀 방출하지 않는다.
///
/// 네이티브 어댑터가 MVP 초과이거나 `Unsupported` 일 때 사용한다. 디바운스 트리거가 0 이므로
/// 트리거는 재조정 소스에서만 나오며, 정확성 바닥은 재조정 백스톱(`<= T_recon`)이 보장한다.
#[derive(Debug, Default, Clone, Copy)]
pub struct DegradedBackend;

impl WatchBackend for DegradedBackend {
    fn subscribe(&self, _root: &Path) -> Result<Subscription, WatchError> {
        let (tx, rx) = std::sync::mpsc::channel::<RawFsEvent>();
        // 송신단을 keepalive 로 보관해 수신단이 disconnect 되지 않게 한다(무이벤트 유지).
        Ok(Subscription {
            events: rx,
            keepalive: Box::new(tx),
        })
    }
}

/// `notify`(FSEvents/inotify/ReadDirectoryChangesW) 기반 네이티브 백엔드.
///
/// 볼트 루트 하위 create/modify/delete/rename 을 재귀 구독하고 `RawFsEvent` 로 정규화한다.
#[derive(Debug, Default, Clone, Copy)]
pub struct NotifyBackend;

impl WatchBackend for NotifyBackend {
    fn subscribe(&self, root: &Path) -> Result<Subscription, WatchError> {
        use notify::{RecursiveMode, Watcher};

        let (tx, rx) = std::sync::mpsc::channel::<RawFsEvent>();
        let root_owned: PathBuf = root.to_path_buf();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                for normalized in normalize_event(&event, &root_owned) {
                    // 수신단이 사라졌으면 조용히 무시(정지 경로).
                    let _ = tx.send(normalized);
                }
            }
        })
        .map_err(|e| WatchError::OsWatchInit(BackendCause(e.to_string())))?;

        watcher
            .watch(root, RecursiveMode::Recursive)
            .map_err(|e| WatchError::Backend(BackendCause(e.to_string())))?;

        Ok(Subscription {
            events: rx,
            keepalive: Box::new(watcher),
        })
    }
}

/// `notify::Event` 를 정규화된 `RawFsEvent` 시퀀스로 변환한다(EventNormalizer, 백엔드 무관 규칙).
///
/// rescan/이벤트 손실은 `Overflow` 로, rename(2-경로)은 delete+create 로 정규화한다. 경로 정규화
/// 실패(볼트 경계 밖 등)는 패닉이 아니라 스킵으로 흡수한다. access 이벤트는 무시한다.
fn normalize_event(event: &notify::Event, root: &Path) -> Vec<RawFsEvent> {
    if event.need_rescan() {
        return vec![RawFsEvent::Overflow];
    }

    let kind = &event.kind;
    if kind.is_access() {
        return Vec::new();
    }

    let mut out: Vec<RawFsEvent> = Vec::new();

    if kind.is_remove() {
        for p in &event.paths {
            if let Some(rel) = to_relative(p, root) {
                out.push(RawFsEvent::Deleted(rel));
            }
        }
    } else if kind.is_create() {
        for p in &event.paths {
            if let Some(rel) = to_relative(p, root) {
                out.push(RawFsEvent::Created(rel));
            }
        }
    } else if kind.is_modify() {
        // rename 은 delete + create 로 정규화(D2). notify 는 rename 시 (from, to) 2 경로를 준다.
        if event.paths.len() == 2 {
            if let Some(rel) = event.paths.first().and_then(|p| to_relative(p, root)) {
                out.push(RawFsEvent::Deleted(rel));
            }
            if let Some(rel) = event.paths.get(1).and_then(|p| to_relative(p, root)) {
                out.push(RawFsEvent::Created(rel));
            }
        } else {
            for p in &event.paths {
                if let Some(rel) = to_relative(p, root) {
                    out.push(RawFsEvent::Modified(rel));
                }
            }
        }
    } else {
        // Any / Other -> 보수적으로 수정으로 취급(코디네이터 재스냅샷이 실제 diff 를 드러냄, FQ-2).
        for p in &event.paths {
            if let Some(rel) = to_relative(p, root) {
                out.push(RawFsEvent::Modified(rel));
            }
        }
    }

    out
}

/// 절대 경로를 볼트 루트 기준 `RelativePath` 로 변환한다(정규화 실패는 `None` 으로 흡수).
fn to_relative(path: &Path, root: &Path) -> Option<RelativePath> {
    let stripped: &Path = path.strip_prefix(root).unwrap_or(path);
    stripped
        .to_str()
        .and_then(|s| RelativePath::normalize(s).ok())
}

/// 스레드 간 공유되는 조회 상태(watch 상태 + 마지막 관측 이벤트 시점 + pause 플래그).
#[derive(Debug)]
struct Shared {
    state: Mutex<WatchState>,
    last_event_at: Mutex<Option<Instant>>,
    paused: AtomicBool,
}

impl Shared {
    fn new() -> Self {
        Shared {
            state: Mutex::new(WatchState::Watching),
            last_event_at: Mutex::new(None),
            paused: AtomicBool::new(false),
        }
    }

    fn set_state(&self, s: WatchState) {
        if let Ok(mut guard) = self.state.lock() {
            *guard = s;
        }
    }

    fn state(&self) -> WatchState {
        match self.state.lock() {
            Ok(guard) => *guard,
            Err(_) => WatchState::Idle,
        }
    }

    fn set_last_event_at(&self, at: Instant) {
        if let Ok(mut guard) = self.last_event_at.lock() {
            *guard = Some(at);
        }
    }

    fn last_event_at(&self) -> Option<Instant> {
        match self.last_event_at.lock() {
            Ok(guard) => *guard,
            Err(_) => None,
        }
    }
}

/// 볼트 루트 이벤트를 감지해 버스트당 1 트리거로 변환하는 애그리게이트.
///
/// `start` 가 감시 스레드를 띄우고 `(FilesystemWatcher, TriggerStream)` 을 반환한다. drop/`stop`
/// 시 백엔드 keepalive 를 해제해 스레드를 종료한다.
pub struct FilesystemWatcher {
    shared: Arc<Shared>,
    keepalive: Option<Box<dyn Any + Send>>,
    handle: Option<JoinHandle<()>>,
}

impl FilesystemWatcher {
    /// 백엔드를 구독해 감시 스레드를 시작하고 트리거 수신단을 반환한다.
    ///
    /// `root` 가 존재하지 않으면 `WatchError::RootMissing` 으로 감시를 미개시한다. `t_debounce`
    /// 는 U8 조립 루트가 주입한 검증된 duration 이다(T8).
    pub fn start(
        root: &Path,
        backend: &dyn WatchBackend,
        t_debounce: Duration,
    ) -> Result<(FilesystemWatcher, TriggerStream), WatchError> {
        if !root.exists() {
            return Err(WatchError::RootMissing);
        }

        let subscription = backend.subscribe(root)?;
        let Subscription { events, keepalive } = subscription;

        let (trig_tx, trig_rx) = std::sync::mpsc::channel::<TriggerSignal>();
        let shared = Arc::new(Shared::new());
        let shared_for_thread = Arc::clone(&shared);

        let handle = std::thread::spawn(move || {
            run_debounce_loop(events, trig_tx, shared_for_thread, t_debounce);
        });

        Ok((
            FilesystemWatcher {
                shared,
                keepalive: Some(keepalive),
                handle: Some(handle),
            },
            trig_rx,
        ))
    }

    /// 현재 런타임 상태 라벨(순수 read-only 조회, push 아님 — R-PURE-01).
    pub fn watch_state(&self) -> WatchState {
        self.shared.state()
    }

    /// 마지막으로 관측한 `RawFsEvent` 의 단조 시점(관측 이벤트가 없으면 `None`).
    ///
    /// U8 이 `elapsed = now - last_event_at` 로 "마지막 이벤트 이후 경과"(US-E1-01 AC)를 계산한다.
    /// `Paused` 중 폐기된 이벤트는 갱신하지 않으며, 단조 시점이라 벽시계 점프에 영향받지 않는다.
    pub fn last_event_at(&self) -> Option<Instant> {
        self.shared.last_event_at()
    }

    /// 감시를 억제한다(이후 이벤트 폐기, D13). 데몬은 상주하며 정확성은 recon 백스톱이 보장.
    pub fn pause(&self) {
        self.shared.paused.store(true, Ordering::SeqCst);
        self.shared.set_state(WatchState::Paused);
    }

    /// 감시를 재개한다(새 버스트부터 다시 감시).
    pub fn resume(&self) {
        self.shared.paused.store(false, Ordering::SeqCst);
        self.shared.set_state(WatchState::Watching);
    }

    /// 감시를 정지하고 스레드가 종료될 때까지 대기한다(구독 해제).
    pub fn stop(self) {
        // self drop -> keepalive 해제 -> 백엔드 disconnect -> 루프 종료 -> join.
    }
}

impl Drop for FilesystemWatcher {
    fn drop(&mut self) {
        // keepalive 를 먼저 해제해 백엔드 송신단을 끊는다(수신단 disconnect -> 루프 break).
        self.keepalive.take();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// 감시 스레드 본체 — `recv_timeout` 정적-구간 루프. `Debouncer` 와 동일 로직(`simulate_debounce`).
fn run_debounce_loop(
    events: Receiver<RawFsEvent>,
    trig_tx: Sender<TriggerSignal>,
    shared: Arc<Shared>,
    t_debounce: Duration,
) {
    let mut debouncer = Debouncer::new(t_debounce);

    loop {
        let recv_result = match debouncer.deadline() {
            Some(deadline) => {
                let now = Instant::now();
                let timeout = deadline.saturating_duration_since(now);
                events.recv_timeout(timeout)
            }
            None => match events.recv() {
                Ok(ev) => Ok(ev),
                Err(_) => Err(RecvTimeoutError::Disconnected),
            },
        };

        match recv_result {
            Ok(_ev) if shared.paused.load(Ordering::SeqCst) => {
                // Paused: 이벤트 폐기(버퍼링 없음, 타이머 미가동, last_event_at 미갱신 — R-DEB-05).
            }
            Ok(_ev) => {
                let now = Instant::now();
                shared.set_last_event_at(now);
                debouncer.on_event(now);
                shared.set_state(WatchState::Watching);
            }
            Err(RecvTimeoutError::Timeout) => {
                let now = Instant::now();
                let signal = debouncer.fire(now);
                shared.set_state(WatchState::Triggered);
                if trig_tx.send(signal).is_err() {
                    break; // 소비자(U8) 사라짐 -> 종료.
                }
                shared.set_state(WatchState::Idle);
            }
            Err(RecvTimeoutError::Disconnected) => break, // 백엔드 해제 -> 종료.
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )]
    use super::*;

    /// 예정된 이벤트를 즉시 방출한 뒤 송신단을 keepalive 로 보관하는 테스트 백엔드.
    struct ScriptedBackend {
        events: Vec<RawFsEvent>,
    }

    impl WatchBackend for ScriptedBackend {
        fn subscribe(&self, _root: &Path) -> Result<Subscription, WatchError> {
            let (tx, rx) = std::sync::mpsc::channel::<RawFsEvent>();
            for ev in &self.events {
                let _ = tx.send(ev.clone());
            }
            Ok(Subscription {
                events: rx,
                keepalive: Box::new(tx),
            })
        }
    }

    #[test]
    fn burst_coalesces_to_single_trigger_over_thread() {
        let root = std::env::temp_dir();
        let rel = RelativePath::normalize("a.txt").expect("정규화");
        let backend = ScriptedBackend {
            events: vec![
                RawFsEvent::Created(rel.clone()),
                RawFsEvent::Modified(rel.clone()),
                RawFsEvent::Modified(rel),
            ],
        };
        let (watcher, stream) =
            FilesystemWatcher::start(&root, &backend, Duration::from_millis(80)).expect("start");

        // 정적 구간(80ms) 경과 후 정확히 1 트리거.
        let first = stream
            .recv_timeout(Duration::from_secs(2))
            .expect("트리거 수신");
        assert_eq!(first.kind, crate::trigger::TriggerKind::Debounced);
        // 추가 트리거 없음(버스트 합치기).
        assert!(stream.recv_timeout(Duration::from_millis(300)).is_err());

        assert!(watcher.last_event_at().is_some());
        watcher.stop();
    }

    #[test]
    fn missing_root_reports_root_missing() {
        let mut missing = std::env::temp_dir();
        missing.push("okc_u2_definitely_missing_dir_zzz");
        let backend = DegradedBackend;
        let result = FilesystemWatcher::start(&missing, &backend, Duration::from_millis(50));
        assert!(matches!(result, Err(WatchError::RootMissing)));
    }

    #[test]
    fn degraded_backend_emits_no_trigger() {
        let root = std::env::temp_dir();
        let backend = DegradedBackend;
        let (watcher, stream) =
            FilesystemWatcher::start(&root, &backend, Duration::from_millis(50)).expect("start");
        // 무이벤트 -> 트리거 없음.
        assert!(stream.recv_timeout(Duration::from_millis(200)).is_err());
        assert_eq!(watcher.last_event_at(), None);
        watcher.stop();
    }
}

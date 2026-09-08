//! U8 고유 제어 핸들러 — U7b `WatcherHandlers` 위임 + sync-now 엣지 push + reload 체이닝.
//!
//! `ControlPlane<H: ControlHandlers>` 가 제네릭이므로(control_plane.rs `new(Arc<H>)`) 동결
//! `WatcherHandlers` 대신 U8 고유 `DaemonControlHandlers` 를 그대로 주입한다(DEC-U8-12). 대부분의
//! op 는 `inner`(WatcherHandlers)로 위임하되 두 op 만 U8 이 특화한다:
//! - `sync_now`(FIX1): `RunStateController.sync_requested` 폴링 대신 `CycleTrigger::SyncNow` 를
//!   합류 채널에 직접 push 하는 엣지 이벤트. 동결 `RunStateController` 에 신호 clear/take API 가
//!   없어(run_state.rs `request_sync_now` 만) 폴링은 사이클 폭주/fire-once 가 되기 때문.
//! - `reload`(FIX2): `ConfigProvider::reload()` 성공 후 `StructuredLogger::reload()` 를 체이닝해
//!   캐시된 로그레벨을 재적용한다. 로거를 `ConfigProvider::subscribe` 로 관찰자 등록하면 참조
//!   순환이라 컴파일 불가이므로(R-U8-04) 명시 체이닝으로 R-LOG-06 을 충족한다.
//!
//! I/O(로그 reload)·채널 push 를 수행하나 어떤 경로에서도 `unwrap`/`expect`/`panic`/인덱싱을 쓰지
//! 않는다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard};

use foundation::{ConfigProvider, Health, StatusSnapshot, UploadHistoryRecord};
use observability::StructuredLogger;
use ops_control::{
    ControlError, ControlHandlers, ConsentView, HistoryFilter, StopMode, WatcherHandlers,
};

use crate::coordinator::CycleTrigger;

/// `ControlHandlers: Send + Sync` 를 만족시키기 위한 `Sync` 한 트리거 송신단 래퍼.
///
/// std `mpsc::Sender` 는 `Send` 이나 `Sync` 가 아니므로 `Mutex` 로 감싼다. 단일 직렬 사이클
/// 설계라 sync-now 엣지 push 는 경합을 유발하지 않는다(짧은 임계구역).
pub struct TriggerSender {
    inner: Mutex<Sender<CycleTrigger>>,
}

impl TriggerSender {
    /// 합류 채널의 송신단으로 래퍼를 구성한다.
    pub fn new(sender: Sender<CycleTrigger>) -> Self {
        TriggerSender {
            inner: Mutex::new(sender),
        }
    }

    /// 트리거를 best-effort 로 push 한다(소비자 부재 시 조용히 무시 — 종료 경로).
    pub fn send(&self, trigger: CycleTrigger) {
        let _ = self.lock().send(trigger);
    }

    /// 뮤텍스를 취득한다. poison 시 패닉 대신 내부 상태를 회수한다(회복력).
    fn lock(&self) -> MutexGuard<'_, Sender<CycleTrigger>> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// U8 고유 제어 핸들러 — 동결 `WatcherHandlers` 위임 + sync-now/reload 특화(domain-entities §5.3).
pub struct DaemonControlHandlers {
    /// 동결 배선(status/health/history/pause/resume/stop/consent 위임).
    inner: WatcherHandlers,
    /// sync-now 엣지 push 대상 합류 채널.
    trigger_tx: Arc<TriggerSender>,
    /// reload 시 로그레벨 재적용 체이닝 대상.
    logger: Arc<StructuredLogger>,
    /// reload 시 `reload()` 를 호출하는 config 프로바이더.
    config: Arc<ConfigProvider>,
}

impl DaemonControlHandlers {
    /// 동결 핸들러 + U8 트리거 채널/로거/config 로 핸들러를 구성한다.
    pub fn new(
        inner: WatcherHandlers,
        trigger_tx: Arc<TriggerSender>,
        logger: Arc<StructuredLogger>,
        config: Arc<ConfigProvider>,
    ) -> Self {
        DaemonControlHandlers {
            inner,
            trigger_tx,
            logger,
            config,
        }
    }
}

impl ControlHandlers for DaemonControlHandlers {
    fn status(&self) -> StatusSnapshot {
        self.inner.status()
    }

    fn health(&self) -> Health {
        self.inner.health()
    }

    fn history(&self, filter: HistoryFilter) -> Result<Vec<UploadHistoryRecord>, ControlError> {
        self.inner.history(filter)
    }

    fn pause(&self) -> Result<(), ControlError> {
        self.inner.pause()
    }

    fn resume(&self) -> Result<(), ControlError> {
        self.inner.resume()
    }

    fn sync_now(&self) {
        // FIX1: 엣지 이벤트로 즉시 동기화 사이클을 요청한다(run_state 폴링 아님).
        self.trigger_tx.send(CycleTrigger::SyncNow);
    }

    fn stop(&self, mode: StopMode) {
        // 종료 신호는 동결 `RunStateController.request_stop` 에 위임한다(데몬이 `current()` 로 관측).
        self.inner.stop(mode);
    }

    fn consent_view(&self) -> ConsentView {
        self.inner.consent_view()
    }

    fn consent_grant(&self) -> Result<(), ControlError> {
        self.inner.consent_grant()
    }

    fn consent_withdraw(&self) -> Result<(), ControlError> {
        self.inner.consent_withdraw()
    }

    fn consent_acknowledge(&self) -> Result<(), ControlError> {
        self.inner.consent_acknowledge()
    }

    fn reload(&self) -> Result<(), ControlError> {
        // FIX2: config 리로드 성공 후 로거 캐시 레벨을 명시 재적용(체이닝).
        self.config
            .reload()
            .map_err(|err| ControlError::ReloadFailed(err.to_string()))?;
        let _ = self.logger.reload();
        Ok(())
    }
}

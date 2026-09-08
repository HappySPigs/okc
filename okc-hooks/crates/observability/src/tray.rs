//! TrayIndicator — MVP no-op 트레이 어댑터(D1, R-TRAY-01/02).
//!
//! 실제 데스크톱 트레이 백엔드(3-OS)와 트레이 크레이트는 도입하지 않는다(GUI-less 데몬 모델).
//! 모든 동작은 무연산이며 비치명이다 — 트레이 부재/no-op 는 로그·status·health·history 표면화를
//! 막지 않는다(R-TRAY-02). 향후 confirm 시 이 no-op 을 실체 구현으로 교체한다(타 컴포넌트 불변).
//!
//! 순수(무동작) 리프 모듈로서 panic-free-total 을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

/// 트레이 config 뷰. no-op 은 값을 무시한다(호환 유지).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrayConfig {
    /// 트레이 활성 플래그(파싱만, 동작 무영향 — D1).
    pub tray_enabled: bool,
}

/// 불투명 트레이 핸들. no-op 은 항상 `None` 을 반환하므로 실체 값을 갖지 않는다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrayHandle;

/// 트레이 알림 메시지. no-op 은 무시한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationMessage {
    /// 알림 제목.
    pub title: String,
    /// 알림 본문.
    pub body: String,
}

/// 트레이 오류(모두 비치명, best-effort).
#[derive(Debug, thiserror::Error)]
pub enum TrayError {
    /// 플랫폼이 트레이를 지원하지 않음.
    #[error("트레이 미지원")]
    Unsupported,
    /// 트레이 초기화 실패.
    #[error("트레이 초기화 실패")]
    InitFailed,
}

/// nullable no-op 트레이 싱크(D1).
#[derive(Debug, Clone, Copy, Default)]
pub struct TrayIndicator;

impl TrayIndicator {
    /// no-op 트레이 인디케이터를 생성한다.
    pub fn new() -> Self {
        TrayIndicator
    }

    /// 트레이를 기동한다 — no-op 은 실체 트레이를 만들지 않고 `Ok(None)` 을 반환한다.
    pub fn start(&self, _cfg: &TrayConfig) -> Result<Option<TrayHandle>, TrayError> {
        Ok(None)
    }

    /// 현재 상태를 렌더한다 — no-op(무연산).
    pub fn render(&self, _snapshot: &foundation::StatusSnapshot) {}

    /// 알림을 표시한다 — no-op(무연산). 트레이 비의존 표면(로그+status)이 항상 성립한다.
    pub fn notify(&self, _message: NotificationMessage) {}

    /// 트레이를 정지한다 — no-op(무연산).
    pub fn stop(&self) {}
}

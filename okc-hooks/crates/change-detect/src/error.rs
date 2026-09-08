//! 감시 계층 오류 taxonomy — `WatchError`(감시 시작/백엔드 초기화 실패 분류).
//!
//! `WatchError` 는 vault-availability 판정(`Availability`/`GuardVerdict`)과 구분되는 **운영 오류**
//! 이며 `thiserror` 로 파생한다(U0 Q4=A 관례 상속). `source`(진단 상세)는 자유형이며 지속하지
//! 않는다(로그 전용, round-trip 대상 아님).

use std::fmt;

/// 백엔드 초기화/런타임 실패의 진단 상세(자유형, 로그 전용 — 지속하지 않음).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendCause(pub String);

impl fmt::Display for BackendCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 감시 시작/백엔드 초기화 실패의 분류. `FilesystemWatcher::start` 가 반환한다.
///
/// `RootMissing` 은 여기서 감시 실패로 표면화되지만, "빈-커밋 방지" 판정은
/// `VaultAvailabilityGuard` 가 소유한다(관심사 분리).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WatchError {
    /// 현재 OS 에 네이티브 백엔드가 없음 — degraded 폴백(무이벤트)으로 전환 가능(정확성은 recon).
    #[error("현재 플랫폼에 네이티브 감시 백엔드가 없다(degraded 폴백 대상)")]
    Unsupported,
    /// 감시 대상 볼트 루트 부재 — 감시 미개시.
    #[error("감시 대상 볼트 루트가 존재하지 않는다")]
    RootMissing,
    /// OS 감시 API 초기화 실패(진단 상세 동반).
    #[error("OS 감시 API 초기화 실패: {0}")]
    OsWatchInit(BackendCause),
    /// 백엔드 런타임 오류(진단 상세 동반).
    #[error("감시 백엔드 런타임 오류: {0}")]
    Backend(BackendCause),
}

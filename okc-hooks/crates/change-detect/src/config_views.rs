//! U2 config 뷰 — U0 `WatcherConfig` 의 U2 소비 섹션에 대한 **read-only 투영** + 검증.
//!
//! U2 는 `ConfigProvider`/`ConfigSnapshot` 을 직접 읽지 않고 U8 조립 루트가 해소한 타입드 값
//! (`Duration`/`bool`)을 생성자 주입받는다(T8). 이 뷰 타입은 그 값의 **개념 형상**과 검증 규칙을
//! 소유한다(PROP-DE-U2-03). 세 값은 고정 duration/플래그이며 적응형 튜닝이 없다(D4).
//!
//! 순수 리프 모듈로서 panic-free 를 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::time::Duration;

/// config 뷰 검증 실패 사유(시작 시 명확한 오류로 거부, US-E1-01 AC).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigViewError {
    /// `debounce_ms` 가 0 (양의 정수 > 0 위반).
    #[error("debounce_ms 는 양의 정수(> 0)여야 한다")]
    NonPositiveDebounce,
    /// `reconciliation_interval_s` 가 0 (양의 정수 >= 1초 위반).
    #[error("reconciliation_interval_s 는 양의 정수(>= 1초)여야 한다")]
    NonPositiveReconInterval,
}

/// `FilesystemWatcher` config 뷰 — 볼트 경로 + 디바운스 정적 구간(ms).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchConfig {
    /// 볼트 루트 경로(U0 공통 필드).
    pub vault_path: String,
    /// 디바운스 정적 구간(ms). 양의 정수(> 0). 기본값 2000ms(T5).
    pub debounce_ms: u64,
}

impl WatchConfig {
    /// 검증된 `T_debounce` `Duration` 을 도출한다. `debounce_ms == 0` 이면 거부.
    pub fn debounce(&self) -> Result<Duration, ConfigViewError> {
        if self.debounce_ms == 0 {
            return Err(ConfigViewError::NonPositiveDebounce);
        }
        Ok(Duration::from_millis(self.debounce_ms))
    }
}

/// `ReconciliationScheduler` config 뷰 — 재조정 백스톱 간격(초).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconConfig {
    /// 재조정 백스톱 간격(초, `T_recon`). 양의 정수(>= 1). 기본값 900s(T6).
    pub reconciliation_interval_s: u64,
}

impl ReconConfig {
    /// 검증된 `T_recon` `Duration` 을 도출한다. `reconciliation_interval_s == 0` 이면 거부.
    pub fn interval(&self) -> Result<Duration, ConfigViewError> {
        if self.reconciliation_interval_s == 0 {
            return Err(ConfigViewError::NonPositiveReconInterval);
        }
        Ok(Duration::from_secs(self.reconciliation_interval_s))
    }
}

/// `VaultAvailabilityGuard` config 뷰 — 볼트 경로 + confirm-empty 영속 플래그.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultConfig {
    /// 볼트 루트 경로(U0 공통 필드).
    pub vault_path: String,
    /// 진짜 빈 볼트 수용 영속 플래그(기본 `false` = 파괴적-빈-커밋 가드 활성, D10).
    pub confirm_empty: bool,
}

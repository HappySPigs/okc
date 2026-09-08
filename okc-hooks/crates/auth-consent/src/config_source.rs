//! ConfigSource — `ConfigSnapshot` 라이브 조회 seam.
//!
//! U0 `ConfigProvider` 는 트레이트가 아니라 구체 struct 이므로, `AuthTransport`/
//! `CredentialProvider` 가 스냅샷을 라이브 조회(캐시 없음)하면서도 테스트에서 config 를
//! 결정적으로 주입할 수 있도록 최소 조회 트레이트를 둔다. 실 배선은 `foundation::ConfigProvider`
//! 에 대한 blanket impl 로 제공된다(U8 조립 시). `Send + Sync` — 데몬 멀티스레드 공유(U0 싱크
//! 근거 계승).
//!
//! 순수 계약 표면으로서 panic-free-total 을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use foundation::{ConfigProvider, ConfigSnapshot};

/// 현재 활성 `ConfigSnapshot` 을 lock-free 로 조회하는 seam.
///
/// `CredentialProvider`(`token`/`secure_store_enabled`)와 `AuthTransport`(`server_endpoint`)가
/// 매 연산마다 라이브 조회해 config 리로드(토큰 회전)를 구조적으로 반영한다(R-CP-04, §4 동시성).
pub trait ConfigSource: Send + Sync {
    /// 현재 활성 config 스냅샷을 반환한다(`ConfigProvider::current()` 위임).
    fn current(&self) -> ConfigSnapshot;
}

impl ConfigSource for ConfigProvider {
    fn current(&self) -> ConfigSnapshot {
        ConfigProvider::current(self)
    }
}

//! SecureStore seam + null-object 기본 어댑터(DEC-U5-07).
//!
//! OS 보안 저장소 조회를 포트 트레이트로 감싸고, MVP 는 실 keyring 백엔드 없이
//! `UnavailableSecureStore`(항상 `Err(Unavailable)`)만 주입한다. secure-store 실패/불가는
//! **비치명**이며 반드시 config `token` -> env 로 안전 폴백한다(실효 순서 = config -> env).
//!
//! 순수 계약 표면으로서 panic-free-total 을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use foundation::TokenSecret;

/// 보안 저장소 조회 실패 사유(둘 다 config/env 폴백 신호).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SecureStoreError {
    /// 세션 불가(헤드리스/데몬 등) — 폴백.
    #[error("보안 저장소 세션 불가")]
    Unavailable,
    /// 백엔드 오류 — 폴백. 진단용 상세 동반(토큰 원문 미포함).
    #[error("보안 저장소 백엔드 오류: {0}")]
    Backend(String),
}

/// OS 보안 저장소 조회 포트(US-E4-02). `Ok(None)` = 세션 가용하나 미저장.
///
/// 실 백엔드는 이 경계 내에서 교체 가능하다(seam). `Send + Sync` — 데몬 멀티스레드 공유.
pub trait SecureStore: Send + Sync {
    /// 보안 저장소에서 토큰을 조회한다.
    fn read_token(&self) -> Result<Option<TokenSecret>, SecureStoreError>;
}

/// `SecureStore` 의 null-object 기본 어댑터 — 항상 `Err(Unavailable)`.
///
/// MVP 기본 주입체로 config `token` -> env 안전 폴백을 성립시킨다. 실 keyring 백엔드는
/// NFR-05 부담·헤드리스 데몬 사유로 미채택(Code Generation 이월). 외부 keyring 크레이트 미유입.
#[derive(Debug, Default, Clone)]
pub struct UnavailableSecureStore;

impl SecureStore for UnavailableSecureStore {
    fn read_token(&self) -> Result<Option<TokenSecret>, SecureStoreError> {
        Err(SecureStoreError::Unavailable)
    }
}

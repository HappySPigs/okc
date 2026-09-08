//! `CredentialProvider` 애그리게이트 — 토큰 출처 라이브 해소(캐시 없음).
//!
//! 논리 컴포넌트: TokenResolver(우선순위 실행) · SecureStorePort(seam) · UnavailableSecureStore
//! (null-object). 매 `resolve_token()` 호출이 U0 `ConfigSnapshot`(core 필드 `token`/
//! `secure_store_enabled`) + env 를 그때그때 재해소하므로 config 리로드(토큰 회전)가 구조적으로
//! 즉시 반영된다(R-CP-04). 토큰 원문은 `TokenSecret` 로만 다루며 `token_status()` 는 존재/소스만
//! 노출한다(R-CP-03, SEC-02).
//!
//! 순수 로직 표면(IO 는 seam 위임)으로서 panic-free-total 을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

pub mod secure_store;

use std::sync::Arc;

use foundation::{ConfigReloadObserver, TokenSecret};

use crate::config_source::ConfigSource;

pub use secure_store::{SecureStore, SecureStoreError, UnavailableSecureStore};

/// 토큰 env 폴백 환경변수 이름(DEC-U5-08). config 키가 아니라 환경변수다.
pub const ENV_TOKEN_KEY: &str = "OKC_WATCHER_TOKEN";

/// 해소된 토큰의 실제 출처(우선순위 결과).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenSource {
    /// config `token` 필드.
    Config,
    /// 환경변수 `OKC_WATCHER_TOKEN`.
    Env,
    /// OS 보안 저장소.
    SecureStore,
}

/// 토큰 존재 여부 + (존재 시) 소스 — CLI `status` 관측 표면(원문 미노출).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenStatus {
    /// 해소 가능 여부.
    pub present: bool,
    /// 존재 시 승리 소스.
    pub source: Option<TokenSource>,
}

/// 토큰 해소 오류. `Missing`/`Empty` 는 데몬 정지가 아니라 **인증 시점 표면화 대상**이다.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CredentialError {
    /// 세 소스 모두 토큰 부재.
    #[error("자격증명 없음: config token/env 미설정")]
    Missing,
    /// 소스에 존재하나 값이 공백(빈 문자열).
    #[error("자격증명 공백: 빈 토큰")]
    Empty,
    /// 보안 저장소 세션 불가 — 내부 폴백 신호(최종 오류로 승격하지 않음).
    #[error("보안 저장소 세션 불가(폴백 신호)")]
    SecureStoreUnavailable,
}

/// 환경변수 조회 seam.
///
/// Edition 2024 에서 `std::env::set_var` 는 `unsafe` 이므로, env 를 seam 으로 추상화해
/// 테스트가 프로세스 전역 env 를 변경하지 않고 결정적으로 우선순위를 검증할 수 있게 한다.
/// 프로덕션 구현은 `ProcessEnv`(실제 `std::env` 읽기). `Send + Sync`.
pub trait EnvReader: Send + Sync {
    /// 환경변수 값을 조회한다(없으면 `None`).
    fn read(&self, key: &str) -> Option<String>;
}

/// 실제 프로세스 환경변수를 읽는 `EnvReader` 구현체.
#[derive(Debug, Default, Clone)]
pub struct ProcessEnv;

impl EnvReader for ProcessEnv {
    fn read(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

/// 토큰 출처(secure-store -> config -> env)를 캐시 없이 라이브 해소하는 프로바이더.
pub struct CredentialProvider {
    config: Arc<dyn ConfigSource>,
    secure_store: Arc<dyn SecureStore>,
    env: Arc<dyn EnvReader>,
}

impl CredentialProvider {
    /// 주입 의존으로 프로바이더를 구성한다.
    pub fn new(
        config: Arc<dyn ConfigSource>,
        secure_store: Arc<dyn SecureStore>,
        env: Arc<dyn EnvReader>,
    ) -> Self {
        CredentialProvider {
            config,
            secure_store,
            env,
        }
    }

    /// MVP 기본 배선 — `UnavailableSecureStore` + `ProcessEnv` 를 주입한다.
    pub fn with_defaults(config: Arc<dyn ConfigSource>) -> Self {
        CredentialProvider::new(
            config,
            Arc::new(UnavailableSecureStore),
            Arc::new(ProcessEnv),
        )
    }

    /// 우선순위(R-CP-01)를 실행해 토큰을 해소한다.
    ///
    /// secure-store 실패/불가/미저장은 비치명 폴백(config `token` -> env). 셋 다 없으면 `Missing`,
    /// 소스에 존재하나 공백이면 `Empty`.
    pub fn resolve_token(&self) -> Result<TokenSecret, CredentialError> {
        self.resolve_with_source().map(|(token, _)| token)
    }

    /// 부수효과 없이 현재 해소 결과의 존재/소스를 반환한다(R-CP-03, 원문 미노출).
    pub fn token_status(&self) -> TokenStatus {
        match self.resolve_with_source() {
            Ok((_, source)) => TokenStatus {
                present: true,
                source: Some(source),
            },
            Err(_) => TokenStatus {
                present: false,
                source: None,
            },
        }
    }

    /// 우선순위 실행 + 승리 소스 판정(R-CP-01, business-logic-model §2.2 알고리즘).
    fn resolve_with_source(&self) -> Result<(TokenSecret, TokenSource), CredentialError> {
        let snapshot = self.config.current();

        // 1) secure-store(활성 시 최우선). 실패/불가/미저장은 비치명 폴백.
        if snapshot.secure_store_enabled {
            match self.secure_store.read_token() {
                Ok(Some(token)) => return Ok((token, TokenSource::SecureStore)),
                Ok(None) => {}
                Err(_) => {}
            }
        }

        // 2) config token(평문 1차·기본). 존재하나 공백이면 Empty.
        match &snapshot.token {
            Some(token) if !token.expose().is_empty() => {
                return Ok((token.clone(), TokenSource::Config));
            }
            Some(_) => return Err(CredentialError::Empty),
            None => {}
        }

        // 3) env 폴백(최후). 존재하나 공백이면 Empty, 부재면 Missing.
        match self.env.read(ENV_TOKEN_KEY) {
            Some(value) if !value.is_empty() => {
                Ok((TokenSecret::new(value), TokenSource::Env))
            }
            Some(_) => Err(CredentialError::Empty),
            None => Err(CredentialError::Missing),
        }
    }
}

/// config 리로드 관찰자 — 라이브 스냅샷이라 별도 무효화가 불필요한 no-op(R-CP-04, DEC-U5-17).
///
/// 캐시가 없어 회전 반영이 구조적으로 자동이므로 훅은 U0 팬아웃 배선 계약 유지를 위해 등록된
/// no-op 으로 남긴다.
impl ConfigReloadObserver for CredentialProvider {
    fn on_config_reload(&self) {}
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]
    use super::*;
    use crate::testing::{StaticConfig, StaticEnv, StaticSecureStore};

    fn provider(
        token: Option<&str>,
        secure_store_enabled: bool,
        secure: Result<Option<TokenSecret>, SecureStoreError>,
        env: Option<&str>,
    ) -> CredentialProvider {
        let config = Arc::new(StaticConfig::build(
            "https://ex.com",
            token.map(|t| TokenSecret::new(t.to_string())),
            secure_store_enabled,
        ));
        CredentialProvider::new(
            config,
            Arc::new(StaticSecureStore::new(secure)),
            Arc::new(StaticEnv::new(env.map(|s| s.to_string()))),
        )
    }

    /// PROP-U5-04 예제: config token 이 env 보다 우선(secure-store off).
    #[test]
    fn config_wins_over_env() {
        let p = provider(Some("cfg"), false, Err(SecureStoreError::Unavailable), Some("env"));
        assert_eq!(p.resolve_token().unwrap().expose(), "cfg");
        assert_eq!(p.token_status().source, Some(TokenSource::Config));
    }

    /// R-CP-01 예제: config 부재 -> env 폴백.
    #[test]
    fn env_fallback_when_no_config() {
        let p = provider(None, false, Err(SecureStoreError::Unavailable), Some("envtok"));
        assert_eq!(p.resolve_token().unwrap().expose(), "envtok");
        assert_eq!(p.token_status().source, Some(TokenSource::Env));
    }

    /// R-CP-02 예제: config 공백 토큰 -> Empty(env 로 폴백하지 않음, FD 알고리즘).
    #[test]
    fn empty_config_token_is_empty_error() {
        let p = provider(Some(""), false, Err(SecureStoreError::Unavailable), Some("env"));
        assert_eq!(p.resolve_token(), Err(CredentialError::Empty));
    }

    /// R-CP-02 예제: 세 소스 모두 없음 -> Missing.
    #[test]
    fn all_absent_is_missing() {
        let p = provider(None, false, Err(SecureStoreError::Unavailable), None);
        assert_eq!(p.resolve_token(), Err(CredentialError::Missing));
        assert!(!p.token_status().present);
    }

    /// R-CP-01 예제: secure-store 활성 + 저장됨 -> 최우선.
    #[test]
    fn secure_store_wins_when_available() {
        let p = provider(
            Some("cfg"),
            true,
            Ok(Some(TokenSecret::new("stored".to_string()))),
            Some("env"),
        );
        assert_eq!(p.resolve_token().unwrap().expose(), "stored");
        assert_eq!(p.token_status().source, Some(TokenSource::SecureStore));
    }

    /// R-CP-01 예제: secure-store 활성이나 불가 -> config 폴백(비치명).
    #[test]
    fn secure_store_unavailable_falls_back_to_config() {
        let p = provider(Some("cfg"), true, Err(SecureStoreError::Unavailable), Some("env"));
        assert_eq!(p.resolve_token().unwrap().expose(), "cfg");
        assert_eq!(p.token_status().source, Some(TokenSource::Config));
    }

    /// MVP 실효 순서: UnavailableSecureStore 기본 -> config -> env.
    #[test]
    fn default_secure_store_is_unavailable() {
        assert_eq!(
            UnavailableSecureStore.read_token(),
            Err(SecureStoreError::Unavailable)
        );
    }
}

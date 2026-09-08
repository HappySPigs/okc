#![deny(missing_docs)]
//! `auth-consent` 크레이트 (U5) — 인증 전송 + 자격증명 해소 + 동의 게이트.
//!
//! 세 개의 공개 애그리게이트를 노출한다:
//! - `AuthTransport`: Watcher -> 서버 유일 아웃바운드 HTTP 경로(토큰 첨부 + TLS 강제 + 타임아웃
//!   부착 + U0 taxonomy 분류). 무상태 `Send + Sync` 파이프라인이며 재시도/프로토콜 해석은 하지
//!   않는다(재시도 -> U4, 프로토콜 -> U3).
//! - `CredentialProvider`: 토큰 출처(secure-store -> config -> env)를 캐시 없이 라이브 해소한다.
//! - `ConsentGate`: 동의 4-상태 라이프사이클 권위 소스 + 원자적 지속 + 업로드 게이트.
//!
//! 이 크레이트는 `foundation`(U0)에만 의존한다(비순환 DAG). U0 소유 타입(`TokenSecret`,
//! `TransportError`, `TransportErrorClass`, `Timestamp`, `ConsentState`, `ActiveCondition`,
//! `StatusSink`, `encode`/`decode`, `validate_https_url`, `ConfigProvider` 등)은 재정의하지 않고
//! 소비한다.
//!
//! 규약: 모든 doc 주석은 한국어, 코드 내 화살표는 ASCII `A -> B` 만 사용,
//! Rust 타입/식별자는 doc 주석에서 백틱으로 감싼다.

use std::time::Duration;

/// `ConfigSnapshot` 조회 seam — U0 `ConfigProvider`(구체 struct)를 트레이트 뒤로 추상화.
pub mod config_source;
/// 동의 상태머신 + 원자 지속 + 업로드 게이트(`ConsentGate`).
pub mod consent;
/// 토큰 해소(`CredentialProvider`) + `SecureStore` seam.
pub mod credential;
/// 인증 전송 파이프라인(`AuthTransport`) + `HttpTransport` seam.
pub mod transport;

/// 테스트 더블(mock/stub) — 비-default `proptest-support` feature 또는 `test` 에서만 빌드.
#[cfg(any(test, feature = "proptest-support"))]
pub mod testing;

/// 도메인 제너레이터(`proptest`) — 런타임 그래프 밖 test-support 논리 단위(PBT-07).
#[cfg(feature = "proptest-support")]
pub mod generators;

pub use config_source::ConfigSource;
pub use consent::{
    BlockReason, ConsentDecision, ConsentError, ConsentGate, ConsentGrant, ConsentLifecycle,
    ConsentRecord, ConsentStatus, GrantId, disclosure_text,
};
pub use credential::{
    CredentialError, CredentialProvider, EnvReader, ProcessEnv, SecureStore, SecureStoreError,
    TokenSource, TokenStatus, UnavailableSecureStore,
};
pub use transport::{
    AuthTransport, Body, Classification, Headers, HttpError, HttpMethod, HttpTransport, OkcRequest,
    OkcResponse, RawHttpRequest, RawHttpResponse, UreqAdapter, classify, classify_error,
    classify_with_code,
};

/// U5 가 소유·확정하는 federated known-key 목록(DEC-FEDERATED-KEYS, Q6=A).
///
/// `watcher-bin`(U8)이 `foundation::FOUNDATION_CONFIG_KEYS` + 이 상수 + 타 단위 키를 union 으로
/// 집계해 `ConfigProvider::new(known_keys)` 에 주입한다. **미지-키 수용 전용**이며 그 자체로 값을
/// 읽을 수 있게 하지 않는다 — `request_timeout_s` 값은 U8 조립루트가 파싱해 `AuthTransport` 에
/// `Duration` 으로 하향 주입한다(U0 `ConfigSnapshot` 은 core 6필드만 노출).
pub const AUTH_CONSENT_CONFIG_KEYS: &[&str] = &["request_timeout_s"];

/// `request_timeout_s` 의 기본값(초, NFR-04 / DEC-U5-04).
pub const DEFAULT_REQUEST_TIMEOUT_S: u64 = 30;

/// `request_timeout_s` 검증 실패 사유(U5 소유 검증 규칙).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TimeoutConfigError {
    /// 0/음수 등 양의 정수(`>= 1`)가 아님.
    #[error("request_timeout_s 는 양의 정수(>= 1)여야 합니다: {0}")]
    NonPositive(i64),
}

/// `request_timeout_s`(federated) 를 검증해 요청 데드라인 `Duration` 으로 해소한다.
///
/// U8 조립루트가 원본 config 에서 읽은 정수를 이 함수로 검증·변환해 `AuthTransport` 에 주입한다.
/// 검증 규칙(양의 정수 `>= 1`, 0/음수 = 실패)은 U5 소유(DEC-U5-04). U5 자신은 config 에서 이
/// 값을 읽지 않는다(값 읽기 경로 부재).
pub fn validate_request_timeout_s(value: i64) -> Result<Duration, TimeoutConfigError> {
    if value >= 1 {
        Ok(Duration::from_secs(value as u64))
    } else {
        Err(TimeoutConfigError::NonPositive(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DEC-U5-04 예제: request_timeout_s 검증(양의 정수 >= 1).
    #[test]
    fn validate_timeout_rules() {
        assert_eq!(validate_request_timeout_s(30), Ok(Duration::from_secs(30)));
        assert_eq!(validate_request_timeout_s(1), Ok(Duration::from_secs(1)));
        assert_eq!(
            validate_request_timeout_s(0),
            Err(TimeoutConfigError::NonPositive(0))
        );
        assert_eq!(
            validate_request_timeout_s(-5),
            Err(TimeoutConfigError::NonPositive(-5))
        );
    }

    /// federated known-key 상수 확인.
    #[test]
    fn config_keys() {
        assert_eq!(AUTH_CONSENT_CONFIG_KEYS, &["request_timeout_s"]);
    }
}

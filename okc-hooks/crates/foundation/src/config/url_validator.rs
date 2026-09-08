//! UrlValidator — `server_endpoint` 의 URL 파싱 + https-only 스킴 강제.
//!
//! `url::Url::parse` 로 파싱한 뒤 `scheme() == "https"` 를 강제한다(`starts_with` 아님).
//! `http`/무스킴/malformed authority+port 는 모두 거부된다. 파싱된 `url::Url` 은 이후 U5
//! `AuthTransport` 의 base-URL join 에 재사용될 수 있어(하류 재파싱 없음) `Ok` 시 반환한다.
//! 이는 U0-NFR-SEC-01(TLS/https 유일 잔여 통제, NFR-06)과 business-rules §4.1 을 실현한다.
//!
//! 순수 표면으로서 panic-free-total(U0-NFR-REL-02)을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use url::Url;

/// `server_endpoint` URL 검증 실패 사유.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UrlValidationError {
    /// URL 문법 자체가 유효하지 않음(무스킴/파싱 실패 포함).
    #[error("URL 파싱 실패: {0}")]
    Malformed(String),
    /// 파싱은 되었으나 스킴이 `https` 가 아님.
    #[error("허용되지 않는 스킴 `{0}`(https 만 허용)")]
    NotHttps(String),
}

/// `server_endpoint` 문자열을 파싱하고 https 스킴을 강제한다.
///
/// 성공 시 파싱된 `url::Url` 을 반환하고, 실패는 `UrlValidationError` 로 표면화한다
/// (호출측 `ConfigValidator` 가 이를 `ConfigError` field 위반으로 승격한다).
pub fn validate_https_url(raw: &str) -> Result<Url, UrlValidationError> {
    let parsed = Url::parse(raw).map_err(|e| UrlValidationError::Malformed(e.to_string()))?;
    if parsed.scheme() == "https" {
        Ok(parsed)
    } else {
        Err(UrlValidationError::NotHttps(parsed.scheme().to_string()))
    }
}

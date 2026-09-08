//! RequestAssembler — 논리 `OkcRequest` -> 저수준 `RawHttpRequest` 확정.
//!
//! `server_endpoint` base + `path` -> 절대 URL, TLS 가드(https 아니면 seam 호출 전 거부),
//! 전송 시점 토큰 헤더 주입(`OkcRequest.headers` 아님), 주입 데드라인을 `timeout` 필드에 부착한다.
//! 데드라인 없는 요청과 토큰 원문 유출을 구조적으로 차단한다(R-AT-TLS/TOKEN/TO).
//!
//! 순수 표면(파일 IO 없음)으로서 panic-free-total 을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::time::Duration;

use foundation::{TokenSecret, TransportError, TransportErrorClass, validate_https_url};

use super::types::{OkcRequest, RawHttpRequest};

/// 토큰을 주입하는 요청 헤더 이름(전송 시점 주입, R-AT-TOKEN).
pub const AUTHORIZATION_HEADER: &str = "Authorization";

/// 논리 요청을 저수준 요청으로 조립한다(TLS 가드 + 토큰 헤더 + 데드라인 부착).
///
/// base URL 이 https 가 아니거나 무효이면 seam 호출 전 `TransportError{Network}` 로 거부한다
/// (회귀 방어; 정상 경로는 U0 `validate_https_url` config 검증으로 이미 https). 오류 `detail` 에는
/// 토큰 원문을 넣지 않는다(SEC-02).
pub fn assemble(
    req: &OkcRequest,
    token: &TokenSecret,
    base_endpoint: &str,
    deadline: Duration,
) -> Result<RawHttpRequest, TransportError> {
    let base_trimmed = base_endpoint.trim_end_matches('/');
    let full = if req.path.starts_with('/') {
        format!("{base_trimmed}{}", req.path)
    } else {
        format!("{base_trimmed}/{}", req.path)
    };

    // TLS 가드(R-AT-TLS): 확정 절대 URL 이 https 가 아니면 거부. 정상 경로는 이미 https.
    let url = validate_https_url(&full).map_err(|e| TransportError {
        class: TransportErrorClass::Network,
        http_status: None,
        detail: format!("TLS 강제 위반 또는 URL 무효: {e}"),
    })?;

    // 전송 시점 토큰 헤더 주입(정확히 1회). 호출자(U3)는 토큰을 다루지 않는다.
    let mut headers = req.headers.clone();
    headers.push(AUTHORIZATION_HEADER, format!("Bearer {}", token.expose()));

    Ok(RawHttpRequest {
        url: url.to_string(),
        method: req.method,
        headers,
        body: req.body.clone(),
        timeout: deadline,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]
    use super::*;
    use crate::transport::types::{HttpMethod, OkcRequest};

    fn token() -> TokenSecret {
        TokenSecret::new("secret-abc".to_string())
    }

    /// PROP-U5-03 예제: https URL + 토큰 헤더 1회 + 데드라인 부착.
    #[test]
    fn assemble_attaches_token_tls_and_timeout() {
        let req = OkcRequest::new(HttpMethod::Post, "/v1/upload");
        let raw = assemble(&req, &token(), "https://example.com/api", Duration::from_secs(30))
            .expect("정상 https base 는 성공");

        assert!(raw.url.starts_with("https://"));
        assert_eq!(raw.timeout, Duration::from_secs(30));

        let auth: Vec<&(String, String)> = raw
            .headers
            .iter()
            .filter(|(name, _)| name.eq_ignore_ascii_case(AUTHORIZATION_HEADER))
            .collect();
        assert_eq!(auth.len(), 1, "토큰 헤더는 정확히 1회");
        assert_eq!(auth[0].1, "Bearer secret-abc");
    }

    /// R-AT-TLS 예제: 비-https base 는 seam 도달 전 거부.
    #[test]
    fn assemble_rejects_non_https() {
        let req = OkcRequest::new(HttpMethod::Get, "/x");
        let err = assemble(&req, &token(), "http://insecure.example.com", Duration::from_secs(5))
            .expect_err("비-https 는 거부");
        assert_eq!(err.class, TransportErrorClass::Network);
        // 오류 detail 에 토큰 원문이 유출되지 않는다(SEC-02).
        assert!(!err.detail.contains("secret-abc"));
    }
}

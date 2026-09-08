//! UreqAdapter — `HttpTransport` 포트의 프로덕션 어댑터(`ureq` blocking + rustls TLS).
//!
//! blocking 클라이언트가 seam 동기 시그니처에 정합하고(Q4=A), rustls 순수 Rust TLS 로
//! native-tls/OpenSSL 시스템 C 의존을 피한다(NFR-05). 평문 http fallback 없음 —
//! 비-https URL 은 `AuthTransport` 가 seam 호출 전 거부한다(R-AT-TLS).
//!
//! 이 모듈은 실제 IO 를 수행하므로 순수 lint-gate 를 적용하지 않는다(U0 `store.rs`/`loader.rs`
//! 관례). `ureq`/rustls 의존은 이 어댑터에만 유입되고 목 경로에는 유입되지 않는다.
//!
//! **어댑터 배선 주석**: `ureq`(major 3) 의 정확한 crypto provider(ring vs aws-lc-rs) 확정과
//! feature 조합은 Build-and-Test 이월(TS-U5-03). 비-2xx 상태코드는 `ureq` 기본 동작상 오류로
//! 표면화되므로 `Error::StatusCode(code)` 를 다시 `RawHttpResponse` 로 되돌려 분류를
//! `AuthTransport` 가 소유하도록 한다(분류 단일 소유).

use super::types::{
    Body, Headers, HttpError, HttpMethod, HttpTransport, RawHttpRequest, RawHttpResponse,
};

/// `ureq` 기반 프로덕션 `HttpTransport` 구현체(rustls TLS).
#[derive(Debug, Default, Clone)]
pub struct UreqAdapter;

impl UreqAdapter {
    /// 새 어댑터를 만든다(무상태).
    pub fn new() -> Self {
        UreqAdapter
    }
}

impl HttpTransport for UreqAdapter {
    fn execute(&self, req: RawHttpRequest) -> Result<RawHttpResponse, HttpError> {
        // 요청당 전체 데드라인을 부착한 agent(R-AT-TO). 얇은 전송이라 요청당 구성 비용은 무시.
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(req.timeout))
            .build()
            .into();

        let url = req.url.as_str();
        let result = match req.method {
            HttpMethod::Get => {
                let mut builder = agent.get(url);
                for (name, value) in req.headers.iter() {
                    builder = builder.header(name.as_str(), value.as_str());
                }
                builder.call()
            }
            HttpMethod::Delete => {
                let mut builder = agent.delete(url);
                for (name, value) in req.headers.iter() {
                    builder = builder.header(name.as_str(), value.as_str());
                }
                builder.call()
            }
            HttpMethod::Post => {
                let mut builder = agent.post(url);
                for (name, value) in req.headers.iter() {
                    builder = builder.header(name.as_str(), value.as_str());
                }
                builder.send(req.body.as_bytes())
            }
            HttpMethod::Put => {
                let mut builder = agent.put(url);
                for (name, value) in req.headers.iter() {
                    builder = builder.header(name.as_str(), value.as_str());
                }
                builder.send(req.body.as_bytes())
            }
            HttpMethod::Patch => {
                let mut builder = agent.patch(url);
                for (name, value) in req.headers.iter() {
                    builder = builder.header(name.as_str(), value.as_str());
                }
                builder.send(req.body.as_bytes())
            }
        };

        match result {
            Ok(mut response) => {
                let status = response.status().as_u16();
                let mut headers = Headers::new();
                for (name, value) in response.headers().iter() {
                    if let Ok(value) = value.to_str() {
                        headers.push(name.as_str(), value);
                    }
                }
                let body_bytes = response
                    .body_mut()
                    .read_to_vec()
                    .map_err(map_ureq_error)?;
                Ok(RawHttpResponse {
                    status,
                    headers,
                    body: Body::from_bytes(body_bytes),
                })
            }
            // 비-2xx 는 ureq 가 오류로 표면화 -> 상태코드를 되살려 분류를 AuthTransport 로 위임.
            Err(ureq::Error::StatusCode(code)) => Ok(RawHttpResponse {
                status: code,
                headers: Headers::new(),
                body: Body::empty(),
            }),
            Err(error) => Err(map_ureq_error(error)),
        }
    }
}

/// `ureq::Error` 를 저수준 `HttpError` 로 사상한다(`Timeout -> Timeout`, 그 외 -> `Io`).
///
/// `Connect`/`Tls` 도 최종적으로 `Network` 로 흡수되므로(R-AT-CLASS-01) 세분 없이 `Io` 로 사상해도
/// 분류 결과가 동일하다.
fn map_ureq_error(error: ureq::Error) -> HttpError {
    match error {
        ureq::Error::Timeout(_) => HttpError::Timeout,
        other => HttpError::Io(other.to_string()),
    }
}

//! `AuthTransport` 애그리게이트 — Watcher -> 서버 유일 아웃바운드 HTTP 경로.
//!
//! 논리 컴포넌트 전개(추적성): TransportPipeline(오케스트레이션) · RequestAssembler(조립+TLS
//! 가드+토큰 주입) · ResponseClassifier(순수 분류) · HttpTransportPort(seam) · UreqAdapter(프로덕션
//! 어댑터). 무상태 `Send + Sync` 파이프라인이며 자체 재시도/백오프·프로토콜 해석은 하지 않는다
//! (재시도 -> U4, 2xx body 해석 -> U3).
//!
//! `ureq` 어댑터가 실제 IO 를 수행하므로 이 모듈에는 순수 lint-gate 를 걸지 않는다(파이프라인은
//! seam 을 통해 IO 를 위임하지만 assembler/classifier 는 개별 lint-gate 를 갖는다).

pub mod assembler;
pub mod classifier;
pub mod types;
pub mod ureq_adapter;

use std::sync::Arc;
use std::time::Duration;

use foundation::{TransportError, TransportErrorClass};

use crate::config_source::ConfigSource;
use crate::credential::{CredentialError, CredentialProvider};

pub use assembler::AUTHORIZATION_HEADER;
pub use classifier::{BACKPRESSURE_CODES, Classification, classify, classify_error, classify_with_code};
pub use types::{
    Body, Headers, HttpError, HttpMethod, HttpTransport, OkcRequest, OkcResponse, RawHttpRequest,
    RawHttpResponse,
};
pub use ureq_adapter::UreqAdapter;

/// 서버가 backpressure 코드를 실을 수 있는 응답 헤더 이름(R-AT-CLASS-03, 잠정 — `[blocked-on-server]`).
pub const OKC_STATUS_CODE_HEADER: &str = "x-okc-code";

/// 인증 전송 파이프라인 — 토큰 첨부 + TLS 강제 + 타임아웃 부착 + U0 taxonomy 분류.
///
/// 무상태 `Send + Sync`: 주입 의존만 보관하며 자체 가변 상태가 없어 재진입 안전하고 락이 불필요.
/// 요청 데드라인(`Duration`)은 U8 조립루트가 `request_timeout_s` 를 파싱해 하향 주입한 값이다
/// (U0 `ConfigSnapshot` 에서 읽지 않음, DEC-U5-04).
pub struct AuthTransport {
    credential: Arc<CredentialProvider>,
    transport: Arc<dyn HttpTransport>,
    config: Arc<dyn ConfigSource>,
    deadline: Duration,
}

impl AuthTransport {
    /// 주입 의존으로 전송 파이프라인을 구성한다.
    ///
    /// `deadline` 은 U8 이 `request_timeout_s`(federated) 를 검증·해소해 하향 주입한 요청 데드라인.
    pub fn new(
        credential: Arc<CredentialProvider>,
        transport: Arc<dyn HttpTransport>,
        config: Arc<dyn ConfigSource>,
        deadline: Duration,
    ) -> Self {
        AuthTransport {
            credential,
            transport,
            config,
            deadline,
        }
    }

    /// 논리 요청을 전송하고 2xx body 또는 `TransportError` 를 반환한다(business-logic-model §1.2).
    ///
    /// 흐름: 토큰 해소 -> URL 확정 -> TLS 가드 -> `RawHttpRequest` 조립(토큰 헤더 + 데드라인) ->
    /// `HttpTransport.execute`(1회) -> 응답/실패 분류. 재시도 스케줄링·프로토콜 해석은 하지 않는다.
    pub fn send(&self, req: OkcRequest) -> Result<OkcResponse, TransportError> {
        // 1) 토큰 해소. Missing/Empty -> 요청 미발송, 사전 실패 표면화(R-AT-TOKEN).
        let token = self.credential.resolve_token().map_err(credential_to_transport)?;

        // 2) server_endpoint 라이브 조회(U0 core 필드).
        let snapshot = self.config.current();
        let base = snapshot.server_endpoint.clone();

        // 3) TLS 가드 + 토큰 헤더 + 데드라인 부착.
        let raw = assembler::assemble(&req, &token, &base, self.deadline)?;

        // 4) seam 전송(1회 시도, 재시도 없음).
        match self.transport.execute(raw) {
            Ok(response) => classify_response(response),
            Err(http_error) => Err(transport_failure(&http_error)),
        }
    }
}

/// 2xx 응답 -> `OkcResponse`, 비-2xx -> `TransportError`(상태코드 + 알려진 코드 힌트 분류).
fn classify_response(response: RawHttpResponse) -> Result<OkcResponse, TransportError> {
    let code = header_value(&response.headers, OKC_STATUS_CODE_HEADER);
    match classify_with_code(response.status, code.as_deref()) {
        Classification::Success => Ok(OkcResponse {
            status: response.status,
            headers: response.headers,
            body: response.body,
        }),
        Classification::Failure(class) => Err(TransportError {
            class,
            http_status: Some(response.status),
            detail: format!("서버 응답 상태코드 {} -> {:?}", response.status, class),
        }),
    }
}

/// 저수준 전송 실패 -> `TransportError`(HTTP 상태코드 없음).
fn transport_failure(error: &HttpError) -> TransportError {
    let class = classify_error(error);
    let detail = match error {
        HttpError::Timeout => "요청 데드라인 초과".to_string(),
        HttpError::Connect => "연결 수립 실패".to_string(),
        HttpError::Tls => "TLS 핸드셰이크 실패".to_string(),
        HttpError::Io(detail) => format!("전송 IO 오류: {detail}"),
    };
    TransportError {
        class,
        http_status: None,
        detail,
    }
}

/// 자격증명 오류를 사전 실패 `TransportError` 로 사상한다(요청 미발송, US-E4-01 AC3).
///
/// `Missing`/`Empty` 는 `AuthFailed`(401 이전 사전 판정)로 표면화한다. 오류 `detail` 에 토큰
/// 원문은 포함되지 않는다(SEC-02). `SecureStoreUnavailable` 은 내부 폴백 신호이므로 최종 오류로
/// 승격하지 않으나(정상 경로에서 `resolve_token` 이 반환하지 않음), total 성을 위해 사상해 둔다.
fn credential_to_transport(error: CredentialError) -> TransportError {
    let detail = match error {
        CredentialError::Missing => "자격증명 없음(config token/env 미설정)".to_string(),
        CredentialError::Empty => "자격증명 공백(빈 토큰)".to_string(),
        CredentialError::SecureStoreUnavailable => "보안 저장소 세션 불가".to_string(),
    };
    TransportError {
        class: TransportErrorClass::AuthFailed,
        http_status: None,
        detail,
    }
}

/// 헤더 목록에서 이름(대소문자 무시)에 해당하는 첫 값을 반환한다.
fn header_value(headers: &Headers, name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.clone())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]
    use super::*;
    use crate::credential::CredentialProvider;
    use crate::testing::{MockHttpTransport, StaticConfig, StaticEnv, StaticSecureStore};
    use foundation::TokenSecret;

    fn credential(token: Option<&str>) -> Arc<CredentialProvider> {
        let config = Arc::new(StaticConfig::build(
            "https://ex.com/api",
            token.map(|t| TokenSecret::new(t.to_string())),
            false,
        ));
        Arc::new(CredentialProvider::new(
            config,
            Arc::new(StaticSecureStore::new(Err(
                crate::credential::SecureStoreError::Unavailable,
            ))),
            Arc::new(StaticEnv::new(None)),
        ))
    }

    fn config() -> Arc<dyn ConfigSource> {
        Arc::new(StaticConfig::build("https://ex.com/api", None, false))
    }

    /// PROP-BL-U5-01 예제: 2xx -> OkcResponse, 토큰 헤더 + 데드라인이 execute 에 도달.
    #[test]
    fn send_success_attaches_token_and_timeout() {
        let mock = Arc::new(MockHttpTransport::with_status(200));
        let transport: Arc<dyn HttpTransport> = mock.clone();
        let auth = AuthTransport::new(
            credential(Some("tok")),
            transport,
            config(),
            Duration::from_secs(30),
        );

        let resp = auth.send(OkcRequest::new(HttpMethod::Get, "/ping")).unwrap();
        assert_eq!(resp.status, 200);

        let captured = mock.last().expect("execute 호출됨");
        assert_eq!(captured.timeout, Duration::from_secs(30));
        assert!(captured.url.starts_with("https://"));
        let auth_headers = captured
            .headers
            .iter()
            .filter(|(n, _)| n.eq_ignore_ascii_case(AUTHORIZATION_HEADER))
            .count();
        assert_eq!(auth_headers, 1);
    }

    /// R-AT-TOKEN 예제: 토큰 Missing -> 요청 미발송(execute 미호출) + AuthFailed.
    #[test]
    fn send_without_token_does_not_execute() {
        let mock = Arc::new(MockHttpTransport::with_status(200));
        let transport: Arc<dyn HttpTransport> = mock.clone();
        let auth = AuthTransport::new(credential(None), transport, config(), Duration::from_secs(5));

        let err = auth
            .send(OkcRequest::new(HttpMethod::Get, "/ping"))
            .expect_err("토큰 없으면 실패");
        assert_eq!(err.class, foundation::TransportErrorClass::AuthFailed);
        assert!(err.http_status.is_none());
        assert!(mock.captured().is_empty(), "요청 미발송");
    }

    /// PROP-U5-01 흐름 예제: 5xx -> ServerError, 401 -> AuthFailed.
    #[test]
    fn send_classifies_error_statuses() {
        for (status, class) in [
            (500u16, foundation::TransportErrorClass::ServerError),
            (401, foundation::TransportErrorClass::AuthFailed),
            (429, foundation::TransportErrorClass::Backpressure),
        ] {
            let mock = Arc::new(MockHttpTransport::with_status(status));
            let transport: Arc<dyn HttpTransport> = mock;
            let auth =
                AuthTransport::new(credential(Some("t")), transport, config(), Duration::from_secs(5));
            let err = auth.send(OkcRequest::new(HttpMethod::Get, "/x")).expect_err("비-2xx");
            assert_eq!(err.class, class);
            assert_eq!(err.http_status, Some(status));
        }
    }

    /// R-AT-CLASS-01 예제: 전송 실패 -> Timeout/Network.
    #[test]
    fn send_maps_transport_failures() {
        let mock = Arc::new(MockHttpTransport::with_error(HttpError::Timeout));
        let transport: Arc<dyn HttpTransport> = mock;
        let auth =
            AuthTransport::new(credential(Some("t")), transport, config(), Duration::from_secs(5));
        let err = auth.send(OkcRequest::new(HttpMethod::Get, "/x")).expect_err("타임아웃");
        assert_eq!(err.class, foundation::TransportErrorClass::Timeout);
    }
}

//! 전송 봉투(envelope) 값 타입 + `HttpTransport` seam.
//!
//! 논리 요청/응답(`OkcRequest`/`OkcResponse`)과 저수준 요청/응답(`RawHttpRequest`/
//! `RawHttpResponse`) + 전송 seam 트레이트(`HttpTransport`) + 저수준 실패(`HttpError`)를
//! 정의한다. 봉투는 U5 소유이며, 오류/전송 결과 분류 타입은 U0 소유를 소비한다(재정의 없음).
//!
//! 순수 값/계약 표면으로서 panic-free-total 을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use core::fmt;
use std::time::Duration;

/// HTTP 메서드(전송 봉투용, U5 소유 어휘).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    /// GET(본문 없음).
    Get,
    /// POST(본문 있음).
    Post,
    /// PUT(본문 있음).
    Put,
    /// PATCH(본문 있음).
    Patch,
    /// DELETE(본문 없음).
    Delete,
}

impl HttpMethod {
    /// 표준 대문자 메서드 문자열을 반환한다.
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
        }
    }

    /// 이 메서드가 요청 본문을 동반하는지 여부(`ureq` builder 분기용).
    pub fn has_body(&self) -> bool {
        matches!(self, HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch)
    }
}

/// 요청/응답 헤더 목록(순서 보존, 중복 허용 — 전송 계층 결정).
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Headers(pub Vec<(String, String)>);

impl Headers {
    /// 빈 헤더 목록을 만든다.
    pub fn new() -> Self {
        Headers(Vec::new())
    }

    /// `(name, value)` 쌍으로부터 헤더 목록을 만든다.
    pub fn from_pairs(pairs: Vec<(String, String)>) -> Self {
        Headers(pairs)
    }

    /// 헤더 1건을 추가한다.
    pub fn push(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.0.push((name.into(), value.into()));
    }

    /// `(name, value)` 쌍 이터레이터.
    pub fn iter(&self) -> impl Iterator<Item = &(String, String)> {
        self.0.iter()
    }
}

/// `Headers` 는 헤더 값(토큰 Bearer 포함 가능)을 리댁션해 로그 유출을 방지한다(SEC-02).
impl fmt::Debug for Headers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();
        for (name, _value) in &self.0 {
            // 값은 리댁션한다 — Authorization 등 민감 헤더 원문 유출 금지.
            list.entry(&(name.as_str(), "***"));
        }
        list.finish()
    }
}

/// 요청/응답 본문 바이트(U3 가 프로토콜 의미로 해석; U5 는 봉투만).
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Body(pub Vec<u8>);

impl Body {
    /// 빈 본문을 만든다.
    pub fn empty() -> Self {
        Body(Vec::new())
    }

    /// 바이트 벡터로부터 본문을 만든다.
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Body(bytes)
    }

    /// 본문 바이트 슬라이스.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// 본문 바이트는 잠재적 시크릿을 담을 수 있으므로 길이만 노출한다(로그 위생).
impl fmt::Debug for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Body({} bytes)", self.0.len())
    }
}

/// U3 `UploadProtocolDriver` 가 조립하는 논리 요청.
///
/// `headers` 에는 토큰을 넣지 않는다 — 토큰은 전송 시점 `RawHttpRequest` 조립에서 주입된다
/// (R-AT-TOKEN, 유출면 축소).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OkcRequest {
    /// HTTP 메서드.
    pub method: HttpMethod,
    /// `server_endpoint` base 에 이어붙일 상대 경로.
    pub path: String,
    /// 요청 헤더(토큰 제외).
    pub headers: Headers,
    /// 요청 본문.
    pub body: Body,
}

impl OkcRequest {
    /// 최소 필드로 요청을 만든다(헤더 없음, 빈 본문).
    pub fn new(method: HttpMethod, path: impl Into<String>) -> Self {
        OkcRequest {
            method,
            path: path.into(),
            headers: Headers::new(),
            body: Body::empty(),
        }
    }
}

/// `AuthTransport.send` 의 성공 반환 — 반환 시 `status` 는 항상 2xx 다(비-2xx 는 `TransportError`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OkcResponse {
    /// HTTP 상태코드(항상 2xx).
    pub status: u16,
    /// 응답 헤더.
    pub headers: Headers,
    /// 응답 본문(U3 해석).
    pub body: Body,
}

/// 절대 URL 로 확정된 저수준 요청(토큰 헤더 포함, 타임아웃 부착됨).
///
/// `Debug` 는 헤더 값(Authorization Bearer 토큰)을 리댁션한다(SEC-02, R-AT-TOKEN).
#[derive(Clone, PartialEq, Eq)]
pub struct RawHttpRequest {
    /// 확정된 절대 URL(https 만 도달 — TLS 가드 통과).
    pub url: String,
    /// HTTP 메서드.
    pub method: HttpMethod,
    /// 토큰 헤더가 주입된 요청 헤더.
    pub headers: Headers,
    /// 요청 본문.
    pub body: Body,
    /// 요청당 전체 데드라인(항상 부착됨, R-AT-TO).
    pub timeout: Duration,
}

impl fmt::Debug for RawHttpRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawHttpRequest")
            .field("url", &self.url)
            .field("method", &self.method)
            .field("headers", &self.headers)
            .field("body", &self.body)
            .field("timeout", &self.timeout)
            .finish()
    }
}

/// 저수준 응답(상태코드 무관 — 분류는 `AuthTransport` 가 소유).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawHttpResponse {
    /// HTTP 상태코드(2xx/비-2xx 무관).
    pub status: u16,
    /// 응답 헤더.
    pub headers: Headers,
    /// 응답 본문.
    pub body: Body,
}

/// 저수준 전송 실패(분류 이전 원인). `AuthTransport` 가 `TransportError` 로 사상한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpError {
    /// 요청 데드라인 초과(-> `TransportErrorClass::Timeout`).
    Timeout,
    /// 연결 수립 실패(-> `Network`).
    Connect,
    /// TLS 핸드셰이크 실패(-> `Network`).
    Tls,
    /// 그 외 IO 오류(-> `Network`). 진단용 상세 동반(토큰 원문 미포함).
    Io(String),
}

/// TLS 위에서만 전송하는 최소 전송 seam(DEC-U5-01).
///
/// 비-https URL 은 `AuthTransport` 가 호출 전 거부하므로 이 seam 에 도달하지 않는다(R-AT-TLS).
/// 분류 이전 원시 결과(`RawHttpResponse` 또는 `HttpError`)만 반환하며, 상태코드 -> 클래스 분류는
/// `AuthTransport` 가 소유한다. `Send + Sync` — 데몬 멀티스레드 공유.
pub trait HttpTransport: Send + Sync {
    /// 저수준 요청을 실행한다(1회 시도, 재시도 없음 — DEC-U5-15).
    fn execute(&self, req: RawHttpRequest) -> Result<RawHttpResponse, HttpError>;
}

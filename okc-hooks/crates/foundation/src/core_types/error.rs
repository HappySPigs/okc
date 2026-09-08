//! ErrorTaxonomy — 오류 분류(classify) 값 계층과 운영 오류 타입.
//!
//! 재시도 게이트(U4)가 소비하는 `ErrorClass` 와 그 total 판정 `is_retryable`,
//! 전송 계층 분류 `TransportErrorClass`/`TransportError`, 일반 분류 `ClassifiedError`,
//! 전송 결과 `TransferResult`, 그리고 코덱 실패 `CodecError`(thiserror)를 정의한다.
//! 백오프·재시도 스케줄은 U4 소관이며 U0 는 분류만 한다.
//!
//! 순수 리프 모듈로서 panic-free-total(U0-NFR-REL-02)을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use serde::{Deserialize, Serialize};

use super::primitives::ByteCount;

/// 재시도 게이트(U4)가 소비하는 오류 의미 분류.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorClass {
    /// 일시 오류 — 백오프 후 재시도 대상.
    Retryable,
    /// 인증 실패(예: 401) — 재시도 제외, U5 인증 흐름 위임.
    AuthAborted,
    /// 서버 혼잡(PROJECT_BUSY/queue-full) — 오류 아닌 정상 지연으로 백오프.
    Backpressure,
    /// 영구/치명(예: 코덱 실패, 프로토콜 위반) — 재시도 무의미.
    Fatal,
}

impl ErrorClass {
    /// 이 분류가 재시도 가능한지 판정하는 **total 함수**(모든 변형 소진, 패닉 경로 없음).
    ///
    /// R-CLASS-02/03: `Retryable`·`Backpressure` -> yes, `AuthAborted`·`Fatal` -> no.
    pub fn is_retryable(&self) -> bool {
        match self {
            ErrorClass::Retryable => true,
            ErrorClass::Backpressure => true,
            ErrorClass::AuthAborted => false,
            ErrorClass::Fatal => false,
        }
    }
}

/// `AuthTransport`(U5)가 산출하는 전송 계층 오류의 의미 분류.
///
/// component-methods 의 상위 `ErrorClass` 와의 이름 충돌을 해소하기 위해 별개 타입으로 명명된다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransportErrorClass {
    /// 인증 실패(HTTP 401).
    AuthFailed,
    /// 서버측 일시 오류(HTTP 5xx).
    ServerError,
    /// 서버 혼잡(PROJECT_BUSY/queue-full/429).
    Backpressure,
    /// 요청 타임아웃.
    Timeout,
    /// 연결 실패(네트워크 단절).
    Network,
}

impl TransportErrorClass {
    /// R-CLASS-01 매핑: 각 전송 오류 클래스를 정확히 하나의 재시도 분류로 사상한다
    /// (누락·중복 없음, total).
    ///
    /// `AuthFailed -> AuthAborted`(no), `ServerError -> Retryable`(yes),
    /// `Backpressure -> Backpressure`(yes), `Timeout -> Retryable`(yes),
    /// `Network -> Retryable`(yes).
    pub fn to_error_class(&self) -> ErrorClass {
        match self {
            TransportErrorClass::AuthFailed => ErrorClass::AuthAborted,
            TransportErrorClass::ServerError => ErrorClass::Retryable,
            TransportErrorClass::Backpressure => ErrorClass::Backpressure,
            TransportErrorClass::Timeout => ErrorClass::Retryable,
            TransportErrorClass::Network => ErrorClass::Retryable,
        }
    }
}

/// 전송 계층 오류의 세부 형상. `detail` 은 무손실 round-trip 대상(유니코드·개행 보존).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportError {
    /// 전송 실패의 의미 분류.
    pub class: TransportErrorClass,
    /// HTTP 상태코드(있으면). Network/Timeout 시 부재(`None`).
    pub http_status: Option<u16>,
    /// 진단용 자유형 상세(유니코드 포함 가능).
    pub detail: String,
}

/// 전송 무관하게 분류된 오류의 일반 형상. 상위 계층(U4 재시도, U6 히스토리)이 공통 소비한다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassifiedError {
    /// 재시도 판정용 의미 분류.
    pub class: ErrorClass,
    /// 도메인/프로토콜 코드(있으면). 무손실 round-trip 대상.
    pub code: Option<String>,
    /// 진단용 자유형 상세(유니코드 포함). 무손실 round-trip 대상.
    pub detail: String,
}

/// 한 blob/사이클 전송의 결과 요약.
///
/// 개념 불변식: `Partial.resume_offset <= Partial.bytes`(ack된 오프셋은 전송량 이내).
/// 재개 오프셋의 지속은 U4 소관이며 U0 는 결과 형상만 정의한다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferResult {
    /// 전송 완료 — 전송 바이트 수.
    Success {
        /// 전송된 총 바이트 수.
        bytes: ByteCount,
    },
    /// 부분 전송 — 마지막 ack 오프셋에서 재개 가능.
    Partial {
        /// 전송된 바이트 수.
        bytes: ByteCount,
        /// 재개 오프셋(마지막 ack 위치).
        resume_offset: ByteCount,
    },
    /// 실패 — 분류된 오류 동반.
    Failed {
        /// 분류된 실패 오류.
        error: ClassifiedError,
    },
}

/// 코덱(encode/decode) 실패 — 운영 오류. `ErrorClass::Fatal` 에 대응하며 재시도 대상이 아니다.
///
/// 순수 serde 분류 값 타입과 달리 `thiserror` 파생 운영 오류이므로 round-trip 대상이 아니다.
#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    /// 값을 CBOR 로 인코딩하는 중 실패.
    #[error("CBOR 인코딩 실패: {0}")]
    Encode(String),
    /// 바이트열을 대상 타입으로 디코딩하는 중 실패(잘림/손상/적대적 입력 포함).
    #[error("CBOR 디코딩 실패: {0}")]
    Decode(String),
}

impl CodecError {
    /// 이 오류의 재시도 분류를 반환한다 — 코덱 실패는 항상 `ErrorClass::Fatal`
    /// (`is_retryable() == false`).
    pub fn error_class(&self) -> ErrorClass {
        ErrorClass::Fatal
    }
}

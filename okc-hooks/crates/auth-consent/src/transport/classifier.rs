//! ResponseClassifier — 응답/전송실패 -> U0 `TransportErrorClass` 순수 total 분류(R-AT-CLASS).
//!
//! 상태코드 -> 클래스 사상은 범위 기반 total 매칭이며 미정의·미래 상태코드까지 흡수한다
//! (누락·패닉 경로 없음). `TransportErrorClass -> ErrorClass` 매핑과 재시도 판정은 U0/U4 소유이며
//! 여기서 관여하지 않는다(U5 는 산출만).
//!
//! 순수 매칭 모듈로서 panic-free-total(U5-NFR-REL-01)을 컴파일타임으로 강제한다 —
//! 런타임 방어 래퍼(`catch_unwind`)는 두지 않는다(비용 0).
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use foundation::TransportErrorClass;

use super::types::HttpError;

/// 서버가 backpressure 를 상태코드 대신 코드 문자열로 표현할 때 인식하는 알려진 값(R-AT-CLASS-03).
///
/// **MVP 트림**: 최소 힌트(상태코드 + 알려진 코드 문자열)만으로 Backpressure 로 승격한다.
/// 응답 body 스키마 깊은 파싱은 U3 이월(프로토콜 의미 비소유). 코드 문자열의 소스 헤더 이름은
/// 서버 계약 확정 시까지 잠정적이며 `[blocked-on-server]` 목 계약으로 검증한다.
pub const BACKPRESSURE_CODES: [&str; 2] = ["PROJECT_BUSY", "queue-full"];

/// 응답 분류 결과 — 2xx 성공 또는 단일 `TransportErrorClass`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    /// 2xx — 정상 응답(`AuthTransport` 가 `OkcResponse` 로 반환).
    Success,
    /// 비-2xx — 단일 전송 오류 클래스.
    Failure(TransportErrorClass),
}

/// HTTP 상태코드를 `TransportErrorClass` 로 분류한다(R-AT-CLASS-01/02, total function).
///
/// 매핑: 2xx -> `Success` / 401·403 -> `AuthFailed` / 429 -> `Backpressure` /
/// 5xx·기타 4xx·3xx -> `ServerError` / 그 외(1xx·>=600 등) -> `Network`.
pub fn classify(status: u16) -> Classification {
    match status {
        200..=299 => Classification::Success,
        401 | 403 => Classification::Failure(TransportErrorClass::AuthFailed),
        429 => Classification::Failure(TransportErrorClass::Backpressure),
        500..=599 => Classification::Failure(TransportErrorClass::ServerError),
        // 기타 4xx / 3xx -> ServerError (MVP 트림: permanent-client 클래스 부재, U4 가 상한).
        300..=499 => Classification::Failure(TransportErrorClass::ServerError),
        // 1xx / >=600 등 비정상 상태코드 -> Network(범위 규칙 흡수).
        _ => Classification::Failure(TransportErrorClass::Network),
    }
}

/// 상태코드 + 알려진 코드 문자열 힌트로 분류한다(R-AT-CLASS-03, total function).
///
/// `code` 가 `BACKPRESSURE_CODES` 중 하나이면 상태코드와 무관하게 `Backpressure` 로 승격하고,
/// 그 외에는 `classify(status)` 로 위임한다.
pub fn classify_with_code(status: u16, code: Option<&str>) -> Classification {
    if let Some(code) = code
        && BACKPRESSURE_CODES.contains(&code) {
            return Classification::Failure(TransportErrorClass::Backpressure);
        }
    classify(status)
}

/// 저수준 전송 실패를 `TransportErrorClass` 로 분류한다(R-AT-CLASS-01, total function).
///
/// `Timeout -> Timeout`, `Connect`/`Tls`/`Io -> Network`(오프라인/재시도 -> U4 graceful degrade).
pub fn classify_error(error: &HttpError) -> TransportErrorClass {
    match error {
        HttpError::Timeout => TransportErrorClass::Timeout,
        HttpError::Connect => TransportErrorClass::Network,
        HttpError::Tls => TransportErrorClass::Network,
        HttpError::Io(_) => TransportErrorClass::Network,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]
    use super::*;
    use foundation::TransportErrorClass::*;

    /// PROP-U5-01 예제: 매핑 표 오라클과 일치.
    #[test]
    fn classify_oracle_examples() {
        assert_eq!(classify(200), Classification::Success);
        assert_eq!(classify(299), Classification::Success);
        assert_eq!(classify(401), Classification::Failure(AuthFailed));
        assert_eq!(classify(403), Classification::Failure(AuthFailed));
        assert_eq!(classify(429), Classification::Failure(Backpressure));
        assert_eq!(classify(500), Classification::Failure(ServerError));
        assert_eq!(classify(503), Classification::Failure(ServerError));
        assert_eq!(classify(404), Classification::Failure(ServerError));
        assert_eq!(classify(400), Classification::Failure(ServerError));
        assert_eq!(classify(302), Classification::Failure(ServerError));
        // 비정상 범위 -> Network(범위 규칙 흡수).
        assert_eq!(classify(100), Classification::Failure(Network));
        assert_eq!(classify(199), Classification::Failure(Network));
        assert_eq!(classify(600), Classification::Failure(Network));
        assert_eq!(classify(0), Classification::Failure(Network));
    }

    /// PROP-U5-01/02 예제: 전 상태코드 total(패닉 없음) + 결과는 U0 5변이 폐쇄.
    #[test]
    fn classify_total_no_panic_over_all_codes() {
        for status in 0u16..=1000 {
            let _ = classify(status);
        }
    }

    /// R-AT-CLASS-03 예제: 알려진 코드 문자열은 상태코드 무관 Backpressure 승격.
    #[test]
    fn classify_with_code_promotes_backpressure() {
        assert_eq!(
            classify_with_code(503, Some("PROJECT_BUSY")),
            Classification::Failure(Backpressure)
        );
        assert_eq!(
            classify_with_code(200, Some("queue-full")),
            Classification::Failure(Backpressure)
        );
        // 미지 코드는 상태코드 분류로 위임.
        assert_eq!(classify_with_code(500, Some("weird")), Classification::Failure(ServerError));
        assert_eq!(classify_with_code(200, None), Classification::Success);
    }

    /// R-AT-CLASS-01 예제: 전송 실패 분류.
    #[test]
    fn classify_error_mapping() {
        assert_eq!(classify_error(&HttpError::Timeout), Timeout);
        assert_eq!(classify_error(&HttpError::Connect), Network);
        assert_eq!(classify_error(&HttpError::Tls), Network);
        assert_eq!(classify_error(&HttpError::Io("x".into())), Network);
    }
}

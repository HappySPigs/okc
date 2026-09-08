//! TokenSecret — API 토큰을 감싸는 redacting newtype.
//!
//! `Debug`/`Display` 표면에서는 실제 값 대신 `***` 를 노출하고, 실제 값은 `.expose()` 로만
//! 해소한다. `Serialize`/`Deserialize` 는 **값-보존**(round-trip/지속 경로 전용, PROP-BR-02)이며,
//! Q7=A no-Serialize-to-log 계약에 따라 로그 방출 경로에서는 사용되지 않는다.
//! 외부 시크릿 크레이트에 의존하지 않는 자체 구현이다(Security Baseline OFF, RISK-01 수용).
//!
//! 순수 리프 모듈로서 panic-free-total(U0-NFR-REL-02)을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use core::fmt;

use serde::{Deserialize, Serialize};

/// API 토큰을 감싸 로그 유출을 방지하는 redacting newtype.
///
/// `Debug` -> `TokenSecret(***)`, `Display` -> `***`. 실제 값은 `expose()` 로만 접근한다.
/// `PartialEq`/`Eq` 는 내부 값을 비교하므로 round-trip 검증에 사용할 수 있다.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TokenSecret(String);

impl TokenSecret {
    /// 평문 토큰 문자열로부터 구성한다.
    pub fn new(value: String) -> Self {
        TokenSecret(value)
    }

    /// 실제 토큰 값을 노출한다 — 인증 시점(U5)에서만 사용해야 한다.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for TokenSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TokenSecret(***)")
    }
}

impl fmt::Display for TokenSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

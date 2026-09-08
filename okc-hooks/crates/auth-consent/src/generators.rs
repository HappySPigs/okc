//! ProptestGenerators — 도메인 제약 준수 `proptest` 제너레이터(PBT-07).
//!
//! 런타임 의존 그래프 밖 test-support 논리 단위이며 비-default `proptest-support` feature 뒤에
//! 게이트된다(U0 패턴 mirror). U0 제너레이터(`Timestamp`/`TokenSecret`)는
//! `foundation::proptest_support` 를 재사용한다(단일 출처 -> 드리프트 방지).

use foundation::proptest_support::generators as fg;
use proptest::prelude::*;

use crate::consent::{ConsentGrant, ConsentLifecycle, ConsentRecord, GrantId};
use crate::transport::{Body, Headers, HttpError, HttpMethod, OkcRequest, RawHttpResponse};

/// 임의 HTTP 메서드.
pub fn arb_http_method() -> impl Strategy<Value = HttpMethod> {
    prop_oneof![
        Just(HttpMethod::Get),
        Just(HttpMethod::Post),
        Just(HttpMethod::Put),
        Just(HttpMethod::Patch),
        Just(HttpMethod::Delete),
    ]
}

/// 임의 헤더 목록(0..4 쌍, 토큰 헤더는 넣지 않음).
pub fn arb_headers() -> impl Strategy<Value = Headers> {
    prop::collection::vec(("[a-z-]{1,8}", "[A-Za-z0-9 ]{0,12}"), 0..4)
        .prop_map(|pairs| Headers::from_pairs(pairs.into_iter().collect()))
}

/// 임의 본문(0..32 바이트).
pub fn arb_body() -> impl Strategy<Value = Body> {
    prop::collection::vec(any::<u8>(), 0..32).prop_map(Body::from_bytes)
}

/// 임의 논리 요청(메서드/경로/헤더/본문 변형).
pub fn arb_okc_request() -> impl Strategy<Value = OkcRequest> {
    (
        arb_http_method(),
        "[a-z0-9/_-]{0,16}",
        arb_headers(),
        arb_body(),
    )
        .prop_map(|(method, path, headers, body)| OkcRequest {
            method,
            path,
            headers,
            body,
        })
}

/// 전 상태코드 열거(100..=599 전수 대상) + 경계.
pub fn arb_status_code() -> impl Strategy<Value = u16> {
    prop_oneof![
        100u16..600u16,
        prop_oneof![Just(200u16), Just(401), Just(403), Just(429), Just(500), Just(302), Just(404)],
        // 범위 밖 비정상 상태코드도 total 분류 대상.
        prop_oneof![Just(0u16), Just(99), Just(600), Just(u16::MAX)],
    ]
}

/// 임의 저수준 전송 실패(전 변이).
pub fn arb_http_error() -> impl Strategy<Value = HttpError> {
    prop_oneof![
        Just(HttpError::Timeout),
        Just(HttpError::Connect),
        Just(HttpError::Tls),
        "[a-z ]{0,16}".prop_map(HttpError::Io),
    ]
}

/// 목 전송 결과(성공 응답 또는 전송 실패) — 상태코드/HttpError 조합.
pub fn arb_mock_result() -> impl Strategy<Value = Result<RawHttpResponse, HttpError>> {
    prop_oneof![
        (arb_status_code(), arb_headers(), arb_body())
            .prop_map(|(status, headers, body)| Ok(RawHttpResponse { status, headers, body })),
        arb_http_error().prop_map(Err),
    ]
}

/// 임의 라이프사이클 상태(4변이 전수).
pub fn arb_consent_lifecycle() -> impl Strategy<Value = ConsentLifecycle> {
    prop_oneof![
        Just(ConsentLifecycle::NotAcknowledged),
        Just(ConsentLifecycle::AcknowledgedNotGranted),
        Just(ConsentLifecycle::Granted),
        Just(ConsentLifecycle::Withdrawn),
    ]
}

/// 임의 부여 참조(경계 `GrantId` 포함).
pub fn arb_consent_grant() -> impl Strategy<Value = ConsentGrant> {
    (grant_id_string(), fg::arb_timestamp()).prop_map(|(id, granted_at)| ConsentGrant {
        grant_id: GrantId(id),
        granted_at,
    })
}

/// 경계 `GrantId` 문자열(빈/유니코드 포함).
fn grant_id_string() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::new()),
        Just("부여-식별자".to_string()),
        "[A-Za-z0-9-]{0,24}".prop_map(|s| s),
    ]
}

/// 일관성 불변식(§3.5)을 만족하는 `ConsentRecord`.
pub fn arb_consistent_record() -> impl Strategy<Value = ConsentRecord> {
    prop_oneof![
        Just(ConsentRecord::initial()),
        Just(ConsentRecord {
            acknowledged: true,
            state: ConsentLifecycle::AcknowledgedNotGranted,
            grant: None,
        }),
        arb_consent_grant().prop_map(|grant| ConsentRecord {
            acknowledged: true,
            state: ConsentLifecycle::Granted,
            grant: Some(grant),
        }),
        proptest::option::of(arb_consent_grant()).prop_map(|grant| ConsentRecord {
            acknowledged: true,
            state: ConsentLifecycle::Withdrawn,
            grant,
        }),
    ]
}

/// 임의(일관/비일관 혼재) `ConsentRecord` — sanitize/fail-safe 강등 검증용.
pub fn arb_any_record() -> impl Strategy<Value = ConsentRecord> {
    (
        any::<bool>(),
        arb_consent_lifecycle(),
        proptest::option::of(arb_consent_grant()),
    )
        .prop_map(|(acknowledged, state, grant)| ConsentRecord {
            acknowledged,
            state,
            grant,
        })
}

/// 동의 게이트 연산(상태머신 시퀀스 테스트용).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// `acknowledge()`.
    Acknowledge,
    /// `grant()`.
    Grant,
    /// `withdraw()`.
    Withdraw,
}

/// 임의 연산 시퀀스(0..16).
pub fn arb_op_sequence() -> impl Strategy<Value = Vec<Op>> {
    let op = prop_oneof![Just(Op::Acknowledge), Just(Op::Grant), Just(Op::Withdraw)];
    prop::collection::vec(op, 0..16)
}

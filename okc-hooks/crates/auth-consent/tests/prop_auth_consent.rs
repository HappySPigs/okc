//! U5 Property-Based Testing (PBT, `proptest-support` feature 게이트).
//!
//! feature 가 꺼진 기본 빌드에서는 빈 테스트 크레이트로 컴파일되고,
//! `cargo test --features proptest-support` 로만 실제 property 가 실행된다(프로덕션 빌드
//! 그래프에 proptest 미유입, PBT-07). FD 확정 속성(PROP-U5-01..09, PROP-DE-U5-01/02,
//! PROP-BL-U5-01)을 구현한다.
#![cfg(feature = "proptest-support")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use auth_consent::consent::store::ConsentStore;
use auth_consent::consent::types::{decide, project};
use auth_consent::consent::{ConsentDecision, ConsentGate, ConsentLifecycle, ConsentRecord};
use auth_consent::credential::{CredentialError, CredentialProvider, SecureStoreError, TokenSource};
use auth_consent::generators as g;
use auth_consent::testing::{
    FixedClock, MockHttpTransport, StaticConfig, StaticEnv, StaticSecureStore,
};
use auth_consent::transport::{AuthTransport, HttpTransport};
use auth_consent::{ConfigSource, transport};
use foundation::{TokenSecret, TransportErrorClass, decode, encode};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// PROP-U5-01 / PROP-U5-02 — 분류 total + 오라클 + taxonomy 폐쇄성.
// ---------------------------------------------------------------------------

/// 참조 오라클(§1 매핑 표) — status -> Option<TransportErrorClass>(None = 2xx 성공).
fn classify_oracle(status: u16) -> Option<TransportErrorClass> {
    match status {
        200..=299 => None,
        401 | 403 => Some(TransportErrorClass::AuthFailed),
        429 => Some(TransportErrorClass::Backpressure),
        300..=599 => Some(TransportErrorClass::ServerError),
        _ => Some(TransportErrorClass::Network),
    }
}

proptest! {
    /// PROP-U5-01: 전 상태코드 분류가 오라클과 일치 + total(패닉 없음).
    #[test]
    fn prop_classify_matches_oracle(status in 0u16..=1200) {
        let actual = match transport::classify(status) {
            transport::Classification::Success => None,
            transport::Classification::Failure(class) => Some(class),
        };
        prop_assert_eq!(actual, classify_oracle(status));
    }

    /// PROP-U5-02: 분류 결과 클래스는 항상 U0 5변이 중 하나이며 to_error_class 로 total 이어진다.
    #[test]
    fn prop_classify_error_total(error in g::arb_http_error()) {
        let class = transport::classify_error(&error);
        // to_error_class 가 패닉 없이 total 하게 이어진다.
        let _ = class.to_error_class();
    }
}

// ---------------------------------------------------------------------------
// PROP-U5-03 / PROP-BL-U5-01 — TLS + 토큰 1회 + 타임아웃 부착 파이프라인 불변식(목 관측).
// ---------------------------------------------------------------------------

/// 토큰 축.
#[derive(Debug, Clone)]
enum TokenAxis {
    Present,
    Empty,
    Absent,
}

fn arb_token_axis() -> impl Strategy<Value = TokenAxis> {
    prop_oneof![Just(TokenAxis::Present), Just(TokenAxis::Empty), Just(TokenAxis::Absent)]
}

fn config_token(axis: &TokenAxis) -> Option<TokenSecret> {
    match axis {
        TokenAxis::Present => Some(TokenSecret::new("live-token".to_string())),
        TokenAxis::Empty => Some(TokenSecret::new(String::new())),
        TokenAxis::Absent => None,
    }
}

proptest! {
    /// PROP-U5-03 / PROP-BL-U5-01: execute 에 도달하는 요청은 https + 토큰 1회 + 데드라인;
    /// 토큰 Missing/Empty 이면 execute 미호출.
    #[test]
    fn prop_pipeline_invariants(
        req in g::arb_okc_request(),
        token_axis in arb_token_axis(),
        timeout_s in 1u64..3600,
    ) {
        let deadline = Duration::from_secs(timeout_s);
        let cfg = Arc::new(StaticConfig::build(
            "https://server.example.com/api",
            config_token(&token_axis),
            false,
        ));
        let credential = Arc::new(CredentialProvider::new(
            cfg,
            Arc::new(StaticSecureStore::new(Err(SecureStoreError::Unavailable))),
            Arc::new(StaticEnv::new(None)),
        ));
        let mock = Arc::new(MockHttpTransport::with_status(200));
        let transport_dyn: Arc<dyn HttpTransport> = mock.clone();
        let config_dyn: Arc<dyn ConfigSource> =
            Arc::new(StaticConfig::build("https://server.example.com/api", None, false));

        let auth = AuthTransport::new(credential, transport_dyn, config_dyn, deadline);
        let _ = auth.send(req);

        match token_axis {
            TokenAxis::Present => {
                let captured = mock.last().expect("토큰 존재 시 execute 호출");
                prop_assert!(captured.url.starts_with("https://"));
                prop_assert_eq!(captured.timeout, deadline);
                let auth_count = captured
                    .headers
                    .iter()
                    .filter(|(n, _)| n.eq_ignore_ascii_case(transport::AUTHORIZATION_HEADER))
                    .count();
                prop_assert_eq!(auth_count, 1);
            }
            TokenAxis::Empty | TokenAxis::Absent => {
                prop_assert!(mock.captured().is_empty(), "토큰 부재/공백 -> 요청 미발송");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// PROP-U5-04 — 토큰 우선순위 오라클(4축 조합).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum SecureAxis {
    SomeToken,
    NoneStored,
    Unavailable,
    Backend,
}

fn arb_secure_axis() -> impl Strategy<Value = SecureAxis> {
    prop_oneof![
        Just(SecureAxis::SomeToken),
        Just(SecureAxis::NoneStored),
        Just(SecureAxis::Unavailable),
        Just(SecureAxis::Backend),
    ]
}

fn secure_result(axis: &SecureAxis) -> Result<Option<TokenSecret>, SecureStoreError> {
    match axis {
        SecureAxis::SomeToken => Ok(Some(TokenSecret::new("secure-token".to_string()))),
        SecureAxis::NoneStored => Ok(None),
        SecureAxis::Unavailable => Err(SecureStoreError::Unavailable),
        SecureAxis::Backend => Err(SecureStoreError::Backend("x".to_string())),
    }
}

fn env_value(axis: &TokenAxis) -> Option<String> {
    match axis {
        TokenAxis::Present => Some("env-token".to_string()),
        TokenAxis::Empty => Some(String::new()),
        TokenAxis::Absent => None,
    }
}

/// 참조 오라클(§2 우선순위 표) — 예상 승리 소스 또는 오류.
fn priority_oracle(
    secure_enabled: bool,
    secure: &SecureAxis,
    cfg: &TokenAxis,
    env: &TokenAxis,
) -> Result<TokenSource, CredentialError> {
    if secure_enabled
        && let SecureAxis::SomeToken = secure {
            return Ok(TokenSource::SecureStore);
        }
    match cfg {
        TokenAxis::Present => return Ok(TokenSource::Config),
        TokenAxis::Empty => return Err(CredentialError::Empty),
        TokenAxis::Absent => {}
    }
    match env {
        TokenAxis::Present => Ok(TokenSource::Env),
        TokenAxis::Empty => Err(CredentialError::Empty),
        TokenAxis::Absent => Err(CredentialError::Missing),
    }
}

proptest! {
    /// PROP-U5-04: 임의 4축 조합에서 해소 결과가 우선순위 오라클과 일치.
    #[test]
    fn prop_priority_oracle(
        secure_enabled in any::<bool>(),
        secure in arb_secure_axis(),
        cfg in arb_token_axis(),
        env in arb_token_axis(),
    ) {
        let config = Arc::new(StaticConfig::build("https://ex.com", config_token(&cfg), secure_enabled));
        let provider = CredentialProvider::new(
            config,
            Arc::new(StaticSecureStore::new(secure_result(&secure))),
            Arc::new(StaticEnv::new(env_value(&env))),
        );

        let expected = priority_oracle(secure_enabled, &secure, &cfg, &env);
        match expected {
            Ok(source) => {
                prop_assert_eq!(provider.token_status().source, Some(source));
                prop_assert!(provider.resolve_token().is_ok());
            }
            Err(err) => {
                prop_assert_eq!(provider.resolve_token(), Err(err));
                prop_assert!(!provider.token_status().present);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// PROP-U5-05 / 06 / 07 — 동의 상태머신 Induction + 게이트 불변식 + forward-only.
// ---------------------------------------------------------------------------

fn temp_consent_path() -> std::path::PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut path = std::env::temp_dir();
    path.push(format!("okc_consent_prop_{}_{}.cbor", std::process::id(), n));
    let _ = std::fs::remove_file(&path);
    path
}

/// 순수 참조 모델 상태(state, acknowledged).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Model {
    state: ConsentLifecycle,
    acknowledged: bool,
}

fn model_apply(model: &mut Model, op: g::Op) -> bool {
    match op {
        g::Op::Acknowledge => {
            if model.state == ConsentLifecycle::NotAcknowledged {
                model.state = ConsentLifecycle::AcknowledgedNotGranted;
                model.acknowledged = true;
            }
            true
        }
        g::Op::Grant => match model.state {
            ConsentLifecycle::NotAcknowledged | ConsentLifecycle::Granted => false,
            _ => {
                model.state = ConsentLifecycle::Granted;
                model.acknowledged = true;
                true
            }
        },
        g::Op::Withdraw => {
            if model.state == ConsentLifecycle::Granted {
                model.state = ConsentLifecycle::Withdrawn;
                true
            } else {
                false
            }
        }
    }
}

fn open_gate(path: std::path::PathBuf) -> ConsentGate {
    ConsentGate::open(ConsentStore::new(path), Arc::new(FixedClock::new(7)), None)
}

proptest! {
    /// PROP-U5-05/06/07: 임의 연산 시퀀스(+중간 reload)에서 게이트가 참조 모델과 관측적 동일;
    /// 게이트는 state==Granted 일 때만 Permitted; withdraw 후 재부여 전까지 항상 Blocked.
    #[test]
    fn prop_state_machine(steps in prop::collection::vec((
        prop_oneof![Just(g::Op::Acknowledge), Just(g::Op::Grant), Just(g::Op::Withdraw)],
        any::<bool>(),
    ), 0..24)) {
        let path = temp_consent_path();
        let mut gate = open_gate(path.clone());
        let mut model = Model {
            state: ConsentLifecycle::NotAcknowledged,
            acknowledged: false,
        };

        for (op, reload) in steps {
            let gate_ok = match op {
                g::Op::Acknowledge => gate.acknowledge().is_ok(),
                g::Op::Grant => gate.grant().is_ok(),
                g::Op::Withdraw => gate.withdraw().is_ok(),
            };
            let model_ok = model_apply(&mut model, op);
            prop_assert_eq!(gate_ok, model_ok, "연산 성공/실패 일치");

            // 재시작(reload) 후에도 상태 보존(R-CG-PERSIST 결합).
            if reload {
                gate = open_gate(path.clone());
            }

            let view = gate.view();
            prop_assert_eq!(view.state, model.state);
            prop_assert_eq!(view.acknowledged, model.acknowledged);

            // PROP-U5-06: Permitted iff Granted.
            let permitted = matches!(gate.is_upload_permitted(), ConsentDecision::Permitted);
            prop_assert_eq!(permitted, model.state == ConsentLifecycle::Granted);
        }
    }
}

// ---------------------------------------------------------------------------
// PROP-U5-08 / PROP-DE-U5-01 — ConsentRecord/ConsentGrant round-trip.
// PROP-DE-U5-02 — projection total.
// ---------------------------------------------------------------------------

proptest! {
    /// PROP-DE-U5-01 / PROP-U5-08: 일관 레코드 round-trip 무손실.
    #[test]
    fn prop_record_roundtrip(record in g::arb_consistent_record()) {
        let bytes = encode(&record).expect("encode 성공");
        let decoded: ConsentRecord = decode(&bytes).expect("decode 성공");
        prop_assert_eq!(decoded, record);
    }

    /// PROP-DE-U5-01: ConsentGrant round-trip.
    #[test]
    fn prop_grant_roundtrip(grant in g::arb_consent_grant()) {
        let bytes = encode(&grant).expect("encode 성공");
        let decoded: auth_consent::ConsentGrant = decode(&bytes).expect("decode 성공");
        prop_assert_eq!(decoded, grant);
    }

    /// PROP-DE-U5-02: projection 은 모든 변이를 정확히 하나의 ConsentState 로(패닉 없음).
    #[test]
    fn prop_projection_total(state in g::arb_consent_lifecycle()) {
        let _ = project(state);
        // decide 도 total.
        let _ = decide(state);
    }
}

// ---------------------------------------------------------------------------
// PROP-U5-09 — 고지 4대 필수 내용(회귀 방지).
// ---------------------------------------------------------------------------

#[test]
fn disclosure_four_elements() {
    let text = auth_consent::disclosure_text();
    assert!(text.contains("연속성"));
    assert!(text.contains("forward-only"));
    assert!(text.contains("필터"));
    assert!(text.contains("평문"));
}

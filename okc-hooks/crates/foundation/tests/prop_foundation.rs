//! STEP 17 — Property-Based Testing (PBT, `proptest-support` feature 게이트).
//!
//! 이 테스트 크레이트 전체는 `proptest-support` feature 뒤에 게이트된다: feature 가 꺼진
//! 기본 빌드에서는 빈 테스트 크레이트로 컴파일되고, `cargo test --features proptest-support`
//! 로만 실제 property 가 실행된다(프로덕션 빌드 그래프에 proptest 미유입, MNT-02/PBT-07).
//!
//! 위임(생성/실행 하류): PROP-DE-02 / PROP-BL-02 / PROP-BL-03(digest shuffle-invariance +
//! diff apply oracle) -> U1; PROP-DE-04 / PROP-BR-06 / PROP-BL-05(SyncState stateful) -> U4.
//! U0 는 여기서 stateful 실행 property 를 구현하지 않는다(모델/제너레이터 계약만 제공).
#![cfg(feature = "proptest-support")]

use std::sync::atomic::{AtomicU64, Ordering};

use foundation::proptest_support::generators as g;
use foundation::{
    ByteCount, ChangeSet, ClassifiedError, ConfigIssue, ConfigProvider, FOUNDATION_CONFIG_KEYS,
    Manifest, ManifestDigest, ManifestEntry, RelativePath, Sha256Digest, StatusSnapshot, SyncState,
    Timestamp, TokenSecret, TransferResult, TransportError, decode, encode, validate,
};
use proptest::prelude::*;

/// PROP-DE-01 / PROP-BR-01 / PROP-BL-01 — CBOR round-trip 불변식 `decode(encode(v)) == v`.
///
/// 이 codec round-trip property 군이 스토리 **US-E7-06**(직렬화 무손실 round-trip, NFR-13)의
/// 수용 증거다. 아래 매크로가 각 CBOR 타입에 대해 동일 불변식을 인스턴스화한다.
macro_rules! roundtrip {
    ($name:ident, $strategy:expr, $ty:ty) => {
        proptest! {
            #[test]
            fn $name(value in $strategy) {
                let bytes = encode(&value).expect("encode 는 실패하지 않는다");
                let decoded: $ty = decode(&bytes).expect("정상 인코딩의 decode 는 성공한다");
                prop_assert_eq!(decoded, value);
            }
        }
    };
}

roundtrip!(rt_relative_path, g::arb_relative_path(), RelativePath);
roundtrip!(rt_sha256_digest, g::arb_sha256_digest(), Sha256Digest);
roundtrip!(rt_manifest_digest, g::arb_manifest_digest(), ManifestDigest);
roundtrip!(rt_timestamp, g::arb_timestamp(), Timestamp);
roundtrip!(rt_byte_count, g::arb_byte_count(), ByteCount);
roundtrip!(rt_manifest_entry, g::arb_manifest_entry(), ManifestEntry);
roundtrip!(rt_manifest, g::arb_manifest(), Manifest);
roundtrip!(rt_change_set, g::arb_change_set(), ChangeSet);
roundtrip!(rt_sync_state, g::arb_sync_state(), SyncState);
roundtrip!(rt_transport_error, g::arb_transport_error(), TransportError);
roundtrip!(rt_classified_error, g::arb_classified_error(), ClassifiedError);
roundtrip!(rt_transfer_result, g::arb_transfer_result(), TransferResult);
roundtrip!(rt_status_snapshot, g::arb_status_snapshot(), StatusSnapshot);
roundtrip!(rt_token_secret, g::arb_token_secret(), TokenSecret);

proptest! {
    /// PROP-DE-03 — `RelativePath::normalize` 멱등성 + 정규 형상 불변식.
    #[test]
    fn prop_normalize_idempotent(raw in ".*") {
        if let Ok(first) = RelativePath::normalize(&raw) {
            let second = RelativePath::normalize(first.as_str())
                .expect("이미 정규화된 값의 재정규화는 성공한다");
            prop_assert_eq!(&first, &second);
            // 정규 형상: 상대 경로 + `.`/`..` 세그먼트 없음.
            prop_assert!(!first.as_str().starts_with('/'));
            prop_assert!(!first.as_str().split('/').any(|s| s == "." || s == ".."));
        }
    }

    /// no-panic 불변식(U0-NFR-REL-02) — 임의 바이트열 `decode` 는 패닉하지 않는다.
    #[test]
    fn prop_decode_arbitrary_no_panic(bytes in prop::collection::vec(any::<u8>(), 0..96)) {
        let _m: Result<Manifest, _> = decode(&bytes);
        let _s: Result<SyncState, _> = decode(&bytes);
        let _t: Result<TransferResult, _> = decode(&bytes);
        // 여기 도달했다는 것 자체가 패닉 없음의 증거.
    }

    /// no-panic 불변식 — 임의 문자열 `normalize` 는 패닉하지 않는다.
    #[test]
    fn prop_normalize_arbitrary_no_panic(raw in ".*") {
        let _ = RelativePath::normalize(&raw);
    }

    /// PROP-BR-04 — `ErrorClass::is_retryable` total(모든 변형에서 패닉 없이 bool 반환).
    #[test]
    fn prop_is_retryable_total(class in g::arb_error_class()) {
        let _ = class.is_retryable();
    }

    /// PROP-BR-05 — 주입된 모든 미지/오타 키를 전수 나열하며 REJECT.
    #[test]
    fn prop_unknown_key_reject((json, unknowns) in g::arb_config_with_unknown_keys()) {
        let err = validate(&json, FOUNDATION_CONFIG_KEYS)
            .expect_err("미지 키가 있으면 반드시 거부된다");
        for expected in &unknowns {
            prop_assert!(
                err.issues().iter().any(|issue| matches!(
                    issue,
                    ConfigIssue::UnknownKey { key, .. } if key == expected
                )),
                "미지 키 {expected} 가 이슈 목록에 포함되어야 한다"
            );
        }
    }

    /// PROP-BR-02 — config parse round-trip `parse(serialize(cfg)) == cfg`(토큰 값-보존 포함).
    #[test]
    fn prop_config_roundtrip((json, config) in g::arb_valid_config_json()) {
        let parsed = validate(&json, FOUNDATION_CONFIG_KEYS)
            .expect("valid config 는 파싱된다");
        prop_assert_eq!(parsed, config);
    }

    /// invalid config JSON 은 반드시 `ConfigError` 로 거부된다.
    #[test]
    fn prop_invalid_config_rejected(json in g::arb_invalid_config_json()) {
        prop_assert!(validate(&json, FOUNDATION_CONFIG_KEYS).is_err());
    }

    /// PROP-BR-03 / PROP-BL-04 — reload 멱등 + keep-last-good.
    ///
    /// 동일 유효 config 를 2회 reload 해도 `current()` 는 동일하고(멱등), 이후 invalid 로 덮어
    /// reload 하면 `Err` 를 반환하며 이전 스냅샷을 보존한다(keep-last-good, 부분 적용 없음).
    #[test]
    fn prop_reload_idempotent_keep_last_good((json, config) in g::arb_valid_config_json()) {
        let path = unique_temp_path();
        std::fs::write(&path, &json).expect("temp config 쓰기");

        let provider = ConfigProvider::new(FOUNDATION_CONFIG_KEYS);
        let snapshot = provider.load(Some(&path)).expect("최초 load 성공");
        prop_assert_eq!(snapshot.config().clone(), config.clone());

        provider.reload().expect("동일 유효 config 재적용(1회)");
        let after_first = provider.current().config().clone();
        provider.reload().expect("동일 유효 config 재적용(2회)");
        let after_second = provider.current().config().clone();
        prop_assert_eq!(after_first.clone(), config.clone());
        prop_assert_eq!(after_first, after_second);

        // keep-last-good: invalid 로 덮어 reload -> Err + 이전 스냅샷 보존.
        std::fs::write(&path, "{ this is not valid json").expect("invalid 덮어쓰기");
        prop_assert!(provider.reload().is_err());
        prop_assert_eq!(provider.current().config().clone(), config);

        let _ = std::fs::remove_file(&path);
    }
}

/// 프로세스 유일 임시 파일 경로를 생성한다(테스트 격리용).
fn unique_temp_path() -> std::path::PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut path = std::env::temp_dir();
    path.push(format!(
        "okc_foundation_prop_{}_{}.json",
        std::process::id(),
        n
    ));
    path
}

//! 도메인 제약 준수 `proptest` 제너레이터.
//!
//! 각 제너레이터는 대응 도메인 타입의 불변식을 위반하지 않는 값을 생성한다
//! (예: `RelativePath` 는 정규화 가능한 입력만, `Manifest` 는 canonical 정렬·경로 유일,
//! `TransferResult::Partial` 은 `resume_offset <= bytes`, valid `WatcherConfig` 는 절대 경로 +
//! https 엔드포인트). valid/invalid `WatcherConfig` JSON 제너레이터는 config 검증 경로의
//! property 테스트(PROP-BR-02/BR-05)를 지원한다.

use proptest::prelude::*;

use crate::config::WatcherConfig;
use crate::core_types::{
    ActiveCondition, ByteCount, ChangeSet, ClassifiedError, ConsentState, ErrorClass,
    LivenessSignal, LogLevel, Manifest, ManifestDigest, ManifestEntry, OperationalState,
    RelativePath, Sha256Digest, StatusSnapshot, SyncState, Timestamp, TokenSecret, TransferResult,
    TransportError, TransportErrorClass,
};

/// 정규화 가능한 단일 경로 세그먼트(슬래시·드라이브 접두 없음) 후보를 생성한다.
fn arb_segment() -> impl Strategy<Value = String> {
    prop_oneof![
        "[a-z0-9]{1,8}".prop_map(|s| s),
        "[A-Za-z0-9_.-]{1,8}".prop_map(|s| s),
        Just("파일".to_string()),
        Just("café".to_string()),
        Just("데이터".to_string()),
    ]
}

/// 유효한 정규화 결과를 갖는 `RelativePath` 를 생성한다(정규화 실패 케이스는 필터로 제거).
///
/// 혼합 세그먼트를 `/` 로 결합한 뒤 `RelativePath::normalize` 를 통과한 값만 취한다.
pub fn arb_relative_path() -> impl Strategy<Value = RelativePath> {
    prop::collection::vec(arb_segment(), 1..5)
        .prop_map(|segments| segments.join("/"))
        .prop_filter_map("정규화 가능한 경로만", |raw| {
            RelativePath::normalize(&raw).ok()
        })
}

/// 임의 32바이트 SHA-256 다이제스트를 생성한다.
pub fn arb_sha256_digest() -> impl Strategy<Value = Sha256Digest> {
    any::<[u8; 32]>().prop_map(Sha256Digest::from_bytes)
}

/// 임의 32바이트 매니페스트 다이제스트를 생성한다.
pub fn arb_manifest_digest() -> impl Strategy<Value = ManifestDigest> {
    any::<[u8; 32]>().prop_map(ManifestDigest::from_bytes)
}

/// 경계값(음/양 극단)을 포함한 `Timestamp` 를 생성한다.
pub fn arb_timestamp() -> impl Strategy<Value = Timestamp> {
    prop_oneof![
        Just(Timestamp::from_unix_nanos(0)),
        Just(Timestamp::from_unix_nanos(i64::MIN)),
        Just(Timestamp::from_unix_nanos(i64::MAX)),
        any::<i64>().prop_map(Timestamp::from_unix_nanos),
    ]
}

/// 경계값(0/1/최대)을 포함한 `ByteCount` 를 생성한다.
pub fn arb_byte_count() -> impl Strategy<Value = ByteCount> {
    prop_oneof![
        Just(ByteCount::new(0)),
        Just(ByteCount::new(1)),
        Just(ByteCount::new(u64::MAX)),
        any::<u64>().prop_map(ByteCount::new),
    ]
}

/// 무손실 round-trip 경계(빈/유니코드/개행 포함)를 포괄하는 detail 문자열을 생성한다.
pub fn arb_detail() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::new()),
        Just("줄1\n줄2\r\n줄3".to_string()),
        Just("emoji 🚀 유니코드 混在".to_string()),
        "[A-Za-z0-9 가-힣]{0,24}".prop_map(|s| s),
    ]
}

/// `ManifestEntry` 를 생성한다(정규 경로 + 임의 해시 + 임의 크기).
pub fn arb_manifest_entry() -> impl Strategy<Value = ManifestEntry> {
    (arb_relative_path(), arb_sha256_digest(), arb_byte_count()).prop_map(
        |(relative_path, raw_sha256, size)| ManifestEntry {
            relative_path,
            raw_sha256,
            size,
        },
    )
}

/// empty/single/many 를 포괄하는 `Manifest` 를 생성한다(canonical 정렬 + 경로 유일 보정).
pub fn arb_manifest() -> impl Strategy<Value = Manifest> {
    (
        prop::collection::vec(arb_manifest_entry(), 0..6),
        arb_manifest_digest(),
    )
        .prop_map(|(mut entries, manifest_digest)| {
            entries.sort();
            entries.dedup_by(|a, b| a.relative_path == b.relative_path);
            Manifest {
                entries,
                manifest_digest,
            }
        })
}

/// added/modified/deleted 세 목록을 갖는 `ChangeSet` 을 생성한다.
pub fn arb_change_set() -> impl Strategy<Value = ChangeSet> {
    (
        prop::collection::vec(arb_manifest_entry(), 0..4),
        prop::collection::vec(arb_manifest_entry(), 0..4),
        prop::collection::vec(arb_relative_path(), 0..4),
    )
        .prop_map(|(added, modified, deleted)| ChangeSet {
            added,
            modified,
            deleted,
        })
}

/// 전 변형(4종)을 열거하는 `SyncState` 를 생성한다.
pub fn arb_sync_state() -> impl Strategy<Value = SyncState> {
    prop::sample::select(vec![
        SyncState::Idle,
        SyncState::Dirty,
        SyncState::Uploading,
        SyncState::Committed,
    ])
}

/// 전 변형(4종)을 열거하는 `ErrorClass` 를 생성한다.
pub fn arb_error_class() -> impl Strategy<Value = ErrorClass> {
    prop::sample::select(vec![
        ErrorClass::Retryable,
        ErrorClass::AuthAborted,
        ErrorClass::Backpressure,
        ErrorClass::Fatal,
    ])
}

/// 전 변형(5종)을 열거하는 `TransportErrorClass` 를 생성한다.
pub fn arb_transport_error_class() -> impl Strategy<Value = TransportErrorClass> {
    prop::sample::select(vec![
        TransportErrorClass::AuthFailed,
        TransportErrorClass::ServerError,
        TransportErrorClass::Backpressure,
        TransportErrorClass::Timeout,
        TransportErrorClass::Network,
    ])
}

/// `TransportError` 를 생성한다(http_status 유/무, 무손실 detail 포함).
pub fn arb_transport_error() -> impl Strategy<Value = TransportError> {
    (
        arb_transport_error_class(),
        prop::option::of(any::<u16>()),
        arb_detail(),
    )
        .prop_map(|(class, http_status, detail)| TransportError {
            class,
            http_status,
            detail,
        })
}

/// `ClassifiedError` 를 생성한다(code 유/무, 무손실 detail 포함).
pub fn arb_classified_error() -> impl Strategy<Value = ClassifiedError> {
    (
        arb_error_class(),
        prop::option::of(arb_detail()),
        arb_detail(),
    )
        .prop_map(|(class, code, detail)| ClassifiedError {
            class,
            code,
            detail,
        })
}

/// `TransferResult` 를 생성한다(`Partial` 은 `resume_offset <= bytes` 불변식 보장).
pub fn arb_transfer_result() -> impl Strategy<Value = TransferResult> {
    let success = arb_byte_count().prop_map(|bytes| TransferResult::Success { bytes });
    let partial = (arb_byte_count(), any::<u64>()).prop_map(|(bytes, raw_offset)| {
        let total = bytes.get();
        // `resume` 를 `[0, total]`(끝값 포함)에 균등 사상한다. `total + 1` 로 끝값을 포함시키되
        // `total == u64::MAX` 경계에서는 덧셈이 오버플로하므로, 이 경우 `raw_offset <= total`
        // 이 항상 성립함을 이용해 `raw_offset` 을 그대로 쓴다(불변식 `resume <= bytes` 보존).
        let resume = match total.checked_add(1) {
            Some(modulus) => raw_offset % modulus,
            None => raw_offset,
        };
        TransferResult::Partial {
            bytes: ByteCount::new(total),
            resume_offset: ByteCount::new(resume),
        }
    });
    let failed = arb_classified_error().prop_map(|error| TransferResult::Failed { error });
    prop_oneof![success, partial, failed]
}

/// 값-보존 round-trip 검증용 비-빈 `TokenSecret` 을 생성한다.
pub fn arb_token_secret() -> impl Strategy<Value = TokenSecret> {
    "[A-Za-z0-9_-]{1,32}".prop_map(TokenSecret::new)
}

/// 전 변형을 열거하는 `OperationalState` 를 생성한다.
pub fn arb_operational_state() -> impl Strategy<Value = OperationalState> {
    prop::sample::select(vec![
        OperationalState::Idle,
        OperationalState::Syncing,
        OperationalState::Offline,
        OperationalState::Paused,
    ])
}

/// 전 변형을 열거하는 `ActiveCondition` 을 생성한다.
pub fn arb_active_condition() -> impl Strategy<Value = ActiveCondition> {
    prop::sample::select(vec![
        ActiveCondition::AuthFailed,
        ActiveCondition::ConsentBlocked,
        ActiveCondition::OverLimit,
        ActiveCondition::VaultUnavailable,
        ActiveCondition::UpdateRolledBack,
    ])
}

/// 전 변형을 열거하는 `LivenessSignal` 을 생성한다.
pub fn arb_liveness_signal() -> impl Strategy<Value = LivenessSignal> {
    prop::sample::select(vec![
        LivenessSignal::IdleReached,
        LivenessSignal::CredentialReadable,
    ])
}

/// 전 변형을 열거하는 `ConsentState`(U5 참조 자리표시)를 생성한다.
pub fn arb_consent_state() -> impl Strategy<Value = ConsentState> {
    prop::sample::select(vec![
        ConsentState::Granted,
        ConsentState::Blocked,
        ConsentState::Unknown,
    ])
}

/// `StatusSnapshot` 을 생성한다(조건 집합·resume 쌍·consent 포함).
pub fn arb_status_snapshot() -> impl Strategy<Value = StatusSnapshot> {
    (
        arb_operational_state(),
        prop::collection::vec(arb_active_condition(), 0..4),
        prop::option::of(arb_timestamp()),
        any::<bool>(),
        prop::option::of((arb_byte_count(), arb_byte_count())),
        arb_consent_state(),
        any::<bool>(),
    )
        .prop_map(
            |(operational, conditions, last_success, dirty, resume, consent, offline)| {
                StatusSnapshot {
                    operational,
                    conditions,
                    last_success,
                    dirty,
                    resume,
                    consent,
                    offline,
                }
            },
        )
}

/// 전 변형(5종)을 열거하는 `LogLevel` 을 생성한다.
pub fn arb_log_level() -> impl Strategy<Value = LogLevel> {
    prop::sample::select(vec![
        LogLevel::Trace,
        LogLevel::Debug,
        LogLevel::Info,
        LogLevel::Warn,
        LogLevel::Error,
    ])
}

/// 절대 파일시스템 경로(POSIX 선행 `/`)를 생성한다.
fn arb_abs_path() -> impl Strategy<Value = String> {
    "(/[a-z0-9]{1,8}){1,4}".prop_map(|s| s)
}

/// 유효한 https 엔드포인트 URL 을 생성한다.
fn arb_https_endpoint() -> impl Strategy<Value = String> {
    "https://[a-z]{2,8}\\.[a-z]{2,4}(/[a-z]{0,6})*".prop_map(|s| s)
}

/// 존재 시 비-빈인 선택적 토큰을 생성한다.
fn arb_opt_token() -> impl Strategy<Value = Option<TokenSecret>> {
    prop_oneof![
        Just(Option::<TokenSecret>::None),
        arb_token_secret().prop_map(Some),
    ]
}

/// 검증을 통과하는 valid `WatcherConfig` 를 생성한다(절대 vault + https 엔드포인트 + notify>=1).
pub fn arb_valid_watcher_config() -> impl Strategy<Value = WatcherConfig> {
    (
        arb_abs_path(),
        arb_https_endpoint(),
        arb_opt_token(),
        any::<bool>(),
        arb_log_level(),
        1u32..1000,
    )
        .prop_map(
            |(vault_path, server_endpoint, token, secure_store_enabled, log_level, notify)| {
                WatcherConfig {
                    vault_path,
                    server_endpoint,
                    token,
                    secure_store_enabled,
                    log_level,
                    notify_consecutive_failures: notify,
                }
            },
        )
}

/// valid `WatcherConfig` 와 그 JSON 직렬화를 함께 생성한다(config round-trip PROP-BR-02).
pub fn arb_valid_config_json() -> impl Strategy<Value = (String, WatcherConfig)> {
    arb_valid_watcher_config().prop_map(|config| {
        let json = serde_json::to_string(&config).unwrap_or_default();
        (json, config)
    })
}

/// known-key 집합 밖의 미지/오타 키 목록을 생성한다(중복 제거).
pub fn arb_unknown_keys() -> impl Strategy<Value = Vec<String>> {
    let pool = prop::sample::select(vec![
        "vault_paht",
        "serverendpoint",
        "tokens",
        "loglevel",
        "notify_failures",
        "debounce_ms",
        "exclude_patterns",
        "unknown_field",
        "xyz",
    ])
    .prop_map(|s| s.to_string());
    prop::collection::vec(pool, 1..4).prop_map(|mut keys| {
        keys.sort();
        keys.dedup();
        keys
    })
}

/// valid 기반에 미지 키를 주입한 config JSON 과 주입된 미지 키 목록을 생성한다(PROP-BR-05).
pub fn arb_config_with_unknown_keys() -> impl Strategy<Value = (String, Vec<String>)> {
    (arb_valid_watcher_config(), arb_unknown_keys()).prop_map(|(config, unknowns)| {
        let mut value = serde_json::to_value(&config).unwrap_or(serde_json::Value::Null);
        if let serde_json::Value::Object(map) = &mut value {
            for key in &unknowns {
                map.insert(key.clone(), serde_json::Value::from(1));
            }
        }
        let json = serde_json::to_string(&value).unwrap_or_default();
        (json, unknowns)
    })
}

/// 검증에 실패하는 invalid config JSON 을 생성한다(bad scheme / 상대 vault_path / 필수 누락).
pub fn arb_invalid_config_json() -> impl Strategy<Value = String> {
    let bad_scheme = arb_valid_watcher_config().prop_map(|mut config| {
        config.server_endpoint = "http://insecure.example/api".to_string();
        serde_json::to_string(&config).unwrap_or_default()
    });
    let relative_vault = arb_valid_watcher_config().prop_map(|mut config| {
        config.vault_path = "relative/dir".to_string();
        serde_json::to_string(&config).unwrap_or_default()
    });
    let missing_required =
        arb_https_endpoint().prop_map(|endpoint| format!("{{\"server_endpoint\":\"{endpoint}\"}}"));
    prop_oneof![bad_scheme, relative_vault, missing_required]
}

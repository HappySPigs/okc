//! federated config 예제 테스트(always-compiled) — known-key union + typed 해소 + round-trip.

use std::sync::Arc;

use auth_consent::AUTH_CONSENT_CONFIG_KEYS;
use foundation::{ConfigSnapshot, FOUNDATION_CONFIG_KEYS, LogLevel, WatcherConfig};
use observability::OBSERVABILITY_CONFIG_KEYS;

use watcher_bin::config::{RawFederatedConfig, U8_CONFIG_KEYS, known_config_keys};

/// 테스트용 core 스냅샷을 만든다.
fn core_snapshot() -> ConfigSnapshot {
    ConfigSnapshot::new(Arc::new(WatcherConfig {
        vault_path: "/vault".to_string(),
        server_endpoint: "https://example.com/api".to_string(),
        token: None,
        secure_store_enabled: false,
        log_level: LogLevel::Info,
        notify_consecutive_failures: 3,
    }))
}

/// known_keys 는 U0/U5/U6/U8 네 소스의 정확한 합집합(중복 없음)이다.
#[test]
fn known_keys_union_covers_four_sources_without_duplicates() {
    let keys = known_config_keys();
    for key in FOUNDATION_CONFIG_KEYS {
        assert!(keys.contains(key), "missing foundation key: {key}");
    }
    for key in AUTH_CONSENT_CONFIG_KEYS {
        assert!(keys.contains(key), "missing auth-consent key: {key}");
    }
    for key in OBSERVABILITY_CONFIG_KEYS {
        assert!(keys.contains(key), "missing observability key: {key}");
    }
    for key in U8_CONFIG_KEYS {
        assert!(keys.contains(key), "missing u8 key: {key}");
    }
    let mut deduped = keys.clone();
    deduped.sort_unstable();
    deduped.dedup();
    assert_eq!(deduped.len(), keys.len(), "union must be duplicate-free");
}

/// 기본 raw -> typed 해소는 data-dir 관례로 하위 아티팩트 경로를 파생한다.
#[test]
fn resolve_derives_default_artifact_paths() {
    let core = core_snapshot();
    let data_dir = std::path::PathBuf::from("/data");
    let federated = RawFederatedConfig::default()
        .resolve_with_default_data_dir(&core, data_dir.clone())
        .expect("resolve defaults");
    assert_eq!(federated.data_dir, data_dir);
    assert_eq!(federated.lock_file, data_dir.join("watcher.lock"));
    assert_eq!(federated.consent_file, data_dir.join("consent.cbor"));
    assert_eq!(federated.run_state_file, data_dir.join("run-state.cbor"));
}

/// 비양수 `request_timeout_s` 는 U5 규칙대로 거부된다.
#[test]
fn non_positive_timeout_rejected() {
    let core = core_snapshot();
    let raw = RawFederatedConfig {
        request_timeout_s: Some(0),
        ..Default::default()
    };
    assert!(
        raw.resolve_with_default_data_dir(&core, std::path::PathBuf::from("/data"))
            .is_err()
    );
}

/// raw federated 투영은 JSON round-trip 에서 값이 보존된다(PROP-U8-07 보조).
#[test]
fn raw_federated_json_round_trip() {
    let raw = RawFederatedConfig {
        request_timeout_s: Some(45),
        debounce_ms: Some(1500),
        data_dir: Some("/custom/data".to_string()),
        ..Default::default()
    };
    let value = raw.to_json_value().expect("to json");
    let restored = RawFederatedConfig::from_json_value(value).expect("from json");
    assert_eq!(raw, restored);
}

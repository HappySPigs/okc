//! State identity and fail-closed config reload regressions.

use foundation::{ConfigProvider, TokenSecret, WatcherConfig};
use watcher_bin::target_binding::{ensure_target_binding, target_fingerprint, validate_target};

fn config(root: &std::path::Path, endpoint: &str, token: &str) -> WatcherConfig {
    serde_json::from_value(serde_json::json!({
        "vault_path": root, "server_endpoint": endpoint, "token": token,
    }))
    .unwrap()
}

#[test]
fn committed_state_cannot_move_between_vaults_endpoints_or_token_selectors() {
    let temp = tempfile::tempdir().unwrap();
    let first = temp.path().join("vault-one");
    let second = temp.path().join("vault-two");
    std::fs::create_dir(&first).unwrap();
    std::fs::create_dir(&second).unwrap();
    let original = config(&first, "https://okc.example/api/sync", "one.secret");
    let fingerprint = target_fingerprint(&original).unwrap();
    let path = temp.path().join("state.target");
    ensure_target_binding(&path, &fingerprint, false).unwrap();
    ensure_target_binding(&path, &fingerprint, true).unwrap();
    for changed in [
        config(&second, "https://okc.example/api/sync", "one.secret"),
        config(&first, "https://other.example/api/sync", "one.secret"),
        config(&first, "https://okc.example/api/sync", "two.secret"),
    ] {
        let different = target_fingerprint(&changed).unwrap();
        assert!(ensure_target_binding(&path, &different, true).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), fingerprint);
    }
    let saved = std::fs::read_to_string(path).unwrap();
    assert!(!saved.contains("secret"));
    assert!(!saved.contains("okc.example"));
}

#[test]
fn legacy_committed_state_is_not_silently_claimed() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("state.target");
    assert!(ensure_target_binding(&path, "v1:fingerprint\n", true).is_err());
    assert!(!path.exists());
}

#[test]
fn changed_target_reload_keeps_previous_snapshot_but_verifier_rotation_is_allowed() {
    let temp = tempfile::tempdir().unwrap();
    let cfg_path = temp.path().join("config.json");
    let mut original = config(temp.path(), "https://okc.example/api/sync", "one.before");
    std::fs::write(&cfg_path, serde_json::to_vec(&original).unwrap()).unwrap();
    let provider = ConfigProvider::new(foundation::FOUNDATION_CONFIG_KEYS);
    provider.load(Some(&cfg_path)).unwrap();
    let fingerprint = target_fingerprint(&original).unwrap();
    original.token = Some(TokenSecret::new("two.unrelated".into()));
    std::fs::write(&cfg_path, serde_json::to_vec(&original).unwrap()).unwrap();
    assert!(
        provider
            .reload_guarded(|candidate| validate_target(candidate, &fingerprint))
            .is_err()
    );
    assert_eq!(
        provider.current().token.as_ref().unwrap().expose(),
        "one.before"
    );
    original.token = Some(TokenSecret::new("one.after".into()));
    std::fs::write(&cfg_path, serde_json::to_vec(&original).unwrap()).unwrap();
    provider
        .reload_guarded(|candidate| validate_target(candidate, &fingerprint))
        .unwrap();
    assert_eq!(
        provider.current().token.as_ref().unwrap().expose(),
        "one.after"
    );
}

#[cfg(feature = "proptest-support")]
mod properties {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn binding_is_idempotent_and_verifier_independent(selector in "[a-z]{8,24}", first in "[A-Za-z0-9]{16,64}", second in "[A-Za-z0-9]{16,64}") {
            let temp = tempfile::tempdir().unwrap();
            let left = config(temp.path(), "https://EXAMPLE.com:443/api/sync/", &format!("{selector}.{first}"));
            let right = config(temp.path(), "https://example.com/api/sync", &format!("{selector}.{second}"));
            let fingerprint = target_fingerprint(&left).unwrap();
            prop_assert_eq!(&fingerprint, &target_fingerprint(&right).unwrap());
            let path = temp.path().join("state.target");
            ensure_target_binding(&path, &fingerprint, false).unwrap();
            ensure_target_binding(&path, &fingerprint, true).unwrap();
            prop_assert_eq!(std::fs::read_to_string(path).unwrap(), fingerprint);
        }
    }
}

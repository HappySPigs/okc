//! A committed manifest belongs to one local vault and one upload destination.

use std::io::Write;
use std::path::Path;

use content_core::ContentAddressing;
use foundation::{ConfigError, ConfigIssue, WatcherConfig, validate_https_url};

/// 오류에 자격증명을 포함하지 않는 target 변경 거부 메시지.
pub const TARGET_CHANGED: &str = "sync target differs from the saved state; keep the original vault/endpoint/token selector or choose a fresh data_dir and state_path (existing state was preserved)";

fn binding_error(detail: &str) -> ConfigError {
    ConfigError::single(ConfigIssue::Discovery(detail.to_string()))
}

/// canonical vault, normalized HTTPS endpoint, token selector를 비밀 없는 지문으로 만든다.
pub fn target_fingerprint(config: &WatcherConfig) -> Result<String, ConfigError> {
    let root = std::fs::canonicalize(&config.vault_path)
        .map_err(|_| binding_error("cannot canonicalize vault_path for sync target binding"))?;
    let endpoint = validate_https_url(&config.server_endpoint)
        .map_err(|_| binding_error("invalid HTTPS sync endpoint"))?;
    if !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
    {
        return Err(binding_error(
            "sync endpoint must not contain userinfo, query, or fragment",
        ));
    }
    let token = config
        .token
        .as_ref()
        .map(|token| token.expose().to_string())
        .or_else(|| std::env::var(auth_consent::credential::ENV_TOKEN_KEY).ok())
        .unwrap_or_default();
    // The server's selector.verifier token scopes a stable source. Never store either component.
    let selector = token.split('.').next().unwrap_or_default();
    let framed = serde_json::to_vec(&(
        "okc-hooks-target-v1",
        root,
        endpoint.as_str().trim_end_matches('/'),
        selector,
    ))
    .map_err(|_| binding_error("cannot encode sync target identity"))?;
    ContentAddressing::hash_stream(framed.as_slice())
        .map(|digest| format!("v1:{digest}\n"))
        .map_err(|_| binding_error("cannot hash sync target identity"))
}

/// 새 스냅샷의 목적지가 기존 지문과 다르면 활성 config 교체 전에 거부한다.
pub fn validate_target(config: &WatcherConfig, expected: &str) -> Result<(), ConfigError> {
    if target_fingerprint(config)? == expected {
        Ok(())
    } else {
        Err(binding_error(TARGET_CHANGED))
    }
}

/// 기존 바인딩을 확인하거나 커밋 이력이 없는 새 상태에 한 번만 바인딩을 만든다.
pub fn ensure_target_binding(
    path: &Path,
    fingerprint: &str,
    has_committed: bool,
) -> Result<(), ConfigError> {
    match std::fs::read_to_string(path) {
        Ok(existing) if existing == fingerprint => return Ok(()),
        Ok(_) => return Err(binding_error(TARGET_CHANGED)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(binding_error(
                "cannot read sync target binding; state preserved",
            ));
        }
    }
    if has_committed {
        return Err(binding_error(
            "committed legacy state has no target binding; choose a fresh data_dir and state_path, preserving the legacy state",
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| binding_error("target binding parent missing"))?;
    std::fs::create_dir_all(parent)
        .map_err(|_| binding_error("cannot create target binding directory"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|_| binding_error("cannot stage target binding"))?;
    temporary
        .write_all(fingerprint.as_bytes())
        .map_err(|_| binding_error("cannot write target binding"))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|_| binding_error("cannot persist target binding"))?;
    temporary
        .persist_noclobber(path)
        .map_err(|_| binding_error("cannot create target binding; state preserved"))?;
    Ok(())
}

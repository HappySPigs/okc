//! STEP 18 — Non-Property Unit Tests (example anchors).
//!
//! MNT-03(Q13=A): 비즈니스-크리티컬 경로는 property + example 을 **둘 다** 보유한다. 이 파일은
//! proptest feature 없이 컴파일/실행되는 결정적 example 테스트로, 2-pass config 검증(Q5=C),
//! keep-last-good, observer `catch_unwind` 격리, https-only, 토큰 redaction, `is_retryable`
//! 매핑, `RelativePath` 거부 케이스, Manifest 정규 정렬/`ChangeSet::is_empty` 를 고정한다.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use foundation::{
    ByteCount, ChangeSet, ConfigIssue, ConfigProvider, ConfigReloadObserver, ErrorClass,
    FOUNDATION_CONFIG_KEYS, ManifestEntry, PathError, RelativePath, Sha256Digest, TokenSecret,
    TransportErrorClass, UrlValidationError, decode, encode, validate, validate_https_url,
};

/// 유효 config JSON(절대 vault + https 엔드포인트, 나머지 필드는 기본값).
const VALID_JSON: &str =
    r#"{"vault_path":"/tmp/okc_vault","server_endpoint":"https://example.com/api"}"#;

/// 프로세스 유일 임시 파일 경로를 만든 뒤 내용을 기록한다.
fn write_temp(contents: &str) -> std::path::PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut path = std::env::temp_dir();
    path.push(format!(
        "okc_foundation_unit_{}_{}.json",
        std::process::id(),
        n
    ));
    std::fs::write(&path, contents).expect("temp 파일 쓰기");
    path
}

// --- config 2-pass (Q5=C) ---------------------------------------------------

#[test]
fn two_pass_lists_all_unknown_keys() {
    // 미지 키가 존재하면 1st-pass 에서 전수 수집해 즉시 거부하며 typed pass 로 넘어가지 않는다.
    let json = r#"{"vault_path":"/tmp/v","server_endpoint":"https://ex.com","foo":1,"vault_paht":2}"#;
    let err = validate(json, FOUNDATION_CONFIG_KEYS).expect_err("미지 키는 거부된다");

    let unknown_keys: Vec<&str> = err
        .issues()
        .iter()
        .filter_map(|issue| match issue {
            ConfigIssue::UnknownKey { key, .. } => Some(key.as_str()),
            _ => None,
        })
        .collect();
    assert!(unknown_keys.contains(&"foo"));
    assert!(unknown_keys.contains(&"vault_paht"));
    // gate 특성: 미지 키가 있으면 field 위반은 수집되지 않는다(2-pass 순서).
    assert_eq!(err.issues().len(), 2);
}

#[test]
fn two_pass_first_field_violation_after_clean() {
    // 미지 키가 clean 인 경우에만 typed + per-field 검증이 수행되고 첫 위반만 보고된다.
    let relative = r#"{"vault_path":"relative","server_endpoint":"https://ex.com"}"#;
    let err = validate(relative, FOUNDATION_CONFIG_KEYS).expect_err("상대 vault_path 거부");
    assert_eq!(err.issues().len(), 1);
    assert!(matches!(
        &err.issues()[0],
        ConfigIssue::FieldViolation { field, .. } if field == "vault_path"
    ));

    let bad_notify =
        r#"{"vault_path":"/tmp/v","server_endpoint":"https://ex.com","notify_consecutive_failures":0}"#;
    let err = validate(bad_notify, FOUNDATION_CONFIG_KEYS).expect_err("notify 0 거부");
    assert!(matches!(
        &err.issues()[0],
        ConfigIssue::FieldViolation { field, .. } if field == "notify_consecutive_failures"
    ));
}

// --- keep-last-good ----------------------------------------------------------

#[test]
fn reload_keeps_last_good_on_invalid() {
    let path = write_temp(VALID_JSON);
    let provider = ConfigProvider::new(FOUNDATION_CONFIG_KEYS);
    provider.load(Some(&path)).expect("최초 load 성공");
    assert_eq!(provider.current().config().vault_path, "/tmp/okc_vault");

    std::fs::write(&path, "{ invalid json").expect("invalid 덮어쓰기");
    assert!(provider.reload().is_err());
    // 검증 실패 -> swap 미도달 -> 이전 스냅샷 유지.
    assert_eq!(provider.current().config().vault_path, "/tmp/okc_vault");

    let _ = std::fs::remove_file(&path);
}

// --- observer catch_unwind 격리 ---------------------------------------------

struct PanicObserver;
impl ConfigReloadObserver for PanicObserver {
    fn on_config_reload(&self) {
        panic!("observer 패닉(격리되어야 함)");
    }
}

struct CountingObserver(Arc<AtomicUsize>);
impl ConfigReloadObserver for CountingObserver {
    fn on_config_reload(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn observer_panic_is_isolated() {
    let path = write_temp(VALID_JSON);
    let counter = Arc::new(AtomicUsize::new(0));

    let mut provider = ConfigProvider::new(FOUNDATION_CONFIG_KEYS);
    provider.subscribe(Arc::new(PanicObserver));
    provider.subscribe(Arc::new(CountingObserver(counter.clone())));
    provider.load(Some(&path)).expect("최초 load 성공");

    // 성공 reload -> fan-out. 첫 observer 가 패닉해도 catch_unwind 로 격리되어
    // 나머지 observer 는 통지되고 current() 는 손상되지 않는다.
    provider.reload().expect("유효 config reload 성공");
    assert_eq!(counter.load(Ordering::SeqCst), 1);
    assert_eq!(provider.current().config().vault_path, "/tmp/okc_vault");

    let _ = std::fs::remove_file(&path);
}

// --- https-only 거부 ---------------------------------------------------------

#[test]
fn https_only_enforced() {
    assert!(validate_https_url("https://example.com/api").is_ok());
    assert!(matches!(
        validate_https_url("http://example.com"),
        Err(UrlValidationError::NotHttps(_))
    ));
    assert!(matches!(
        validate_https_url("example.com/path"),
        Err(UrlValidationError::Malformed(_))
    ));

    // config 경로에서도 http 엔드포인트는 field 위반으로 거부된다.
    let http_json = r#"{"vault_path":"/tmp/v","server_endpoint":"http://ex.com"}"#;
    let err = validate(http_json, FOUNDATION_CONFIG_KEYS).expect_err("http 엔드포인트 거부");
    assert!(matches!(
        &err.issues()[0],
        ConfigIssue::FieldViolation { field, .. } if field == "server_endpoint"
    ));
}

// --- 토큰 redaction ----------------------------------------------------------

#[test]
fn token_is_redacted_but_serialize_preserves_value() {
    let token = TokenSecret::new("super-secret-123".to_string());
    // Debug/Display 는 실제 값을 노출하지 않는다.
    assert_eq!(format!("{token:?}"), "TokenSecret(***)");
    assert_eq!(format!("{token}"), "***");
    assert!(!format!("{token:?}").contains("super-secret-123"));

    // Serialize/Deserialize 는 값-보존(round-trip).
    let bytes = encode(&token).expect("encode");
    let restored: TokenSecret = decode(&bytes).expect("decode");
    assert_eq!(restored.expose(), "super-secret-123");
}

// --- is_retryable 매핑 -------------------------------------------------------

#[test]
fn is_retryable_mapping_is_fixed() {
    assert!(ErrorClass::Retryable.is_retryable());
    assert!(ErrorClass::Backpressure.is_retryable());
    assert!(!ErrorClass::AuthAborted.is_retryable());
    assert!(!ErrorClass::Fatal.is_retryable());

    // R-CLASS-01: 전송 오류 클래스 -> 재시도 분류 매핑.
    assert!(!TransportErrorClass::AuthFailed.to_error_class().is_retryable());
    assert!(TransportErrorClass::ServerError.to_error_class().is_retryable());
    assert!(TransportErrorClass::Backpressure.to_error_class().is_retryable());
    assert!(TransportErrorClass::Timeout.to_error_class().is_retryable());
    assert!(TransportErrorClass::Network.to_error_class().is_retryable());
}

// --- RelativePath 거부 케이스 ------------------------------------------------

#[test]
fn relative_path_rejects_invalid_inputs() {
    assert_eq!(RelativePath::normalize("/etc/passwd"), Err(PathError::Absolute));
    assert_eq!(RelativePath::normalize("C:/win"), Err(PathError::Absolute));
    assert_eq!(RelativePath::normalize("a/../../b"), Err(PathError::Escape));
    assert_eq!(RelativePath::normalize("."), Err(PathError::Empty));
    assert_eq!(RelativePath::normalize(""), Err(PathError::Empty));

    // 정상 입력은 정규화되고 백슬래시/중복 슬래시/`.` 세그먼트가 제거된다.
    let ok = RelativePath::normalize("a\\b//./c").expect("정규화 성공");
    assert_eq!(ok.as_str(), "a/b/c");
}

// --- Manifest 정규 정렬 / ChangeSet::is_empty --------------------------------

fn entry(path: &str) -> ManifestEntry {
    ManifestEntry {
        relative_path: RelativePath::normalize(path).expect("valid path"),
        raw_sha256: Sha256Digest::from_bytes([0u8; 32]),
        size: ByteCount::new(1),
    }
}

#[test]
fn manifest_entries_sort_canonically_by_path() {
    let mut entries = [entry("b/file"), entry("a/file"), entry("a/aaa")];
    entries.sort();
    let paths: Vec<&str> = entries.iter().map(|e| e.relative_path.as_str()).collect();
    // relative_path 오름차순(byte-lex) 1차 정렬.
    assert_eq!(paths, vec!["a/aaa", "a/file", "b/file"]);
}

#[test]
fn change_set_is_empty_semantics() {
    let empty = ChangeSet {
        added: Vec::new(),
        modified: Vec::new(),
        deleted: Vec::new(),
    };
    assert!(empty.is_empty());

    let non_empty = ChangeSet {
        added: vec![entry("a/x")],
        modified: Vec::new(),
        deleted: Vec::new(),
    };
    assert!(!non_empty.is_empty());
}

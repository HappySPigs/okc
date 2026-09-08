//! `setup` 코어 예제 테스트(always-compiled) — 검증·0600 기록·서비스 바인딩·멱등·토큰 무유출.
//!
//! 서비스 등록은 `ServiceRegistrar` seam 을 기록 fake 로 주입해 특권/OS 없이 검증하고, config 쓰기는
//! 임시 디렉터리를 대상으로 실제 수행해 Unix 권한(0600)을 관측한다. 토큰은 어떤 오류 출력에도
//! 노출되지 않음을 단언한다(SECURITY-03). PBT config round-trip 은 `proptest-support` feature 뒤에 있다.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use watcher_bin::setup::{ServiceRegistrar, perform_setup};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// 유니크한 임시 작업 디렉터리를 만든다(프로세스 id + 카운터).
fn temp_dir(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("okc-setup-{tag}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// 등록 호출을 기록하는 fake `ServiceRegistrar`(특권/OS 불요).
#[derive(Default)]
struct FakeRegistrar {
    calls: Mutex<Vec<PathBuf>>,
    fail: bool,
}

impl FakeRegistrar {
    fn recording() -> Self {
        FakeRegistrar::default()
    }
    fn failing() -> Self {
        FakeRegistrar {
            calls: Mutex::new(Vec::new()),
            fail: true,
        }
    }
    fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
    fn last_bound(&self) -> Option<PathBuf> {
        self.calls.lock().unwrap().last().cloned()
    }
}

impl ServiceRegistrar for FakeRegistrar {
    fn register(&self, config_path: &Path) -> Result<(), String> {
        self.calls.lock().unwrap().push(config_path.to_path_buf());
        if self.fail {
            Err("서비스 등록기 실패(주입)".to_string())
        } else {
            Ok(())
        }
    }
}

/// 유효한 config 원본을 소스 파일로 기록하고 경로를 반환한다.
fn write_source(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("source-config.json");
    std::fs::write(&path, body).expect("write source");
    path
}

const VALID_CONFIG: &str = r#"{
  "vault_path": "/absolute/vault",
  "server_endpoint": "https://okc.example.com/api/sync",
  "token": "super-secret-upload-token-value",
  "data_dir": "/absolute/data"
}"#;

#[test]
fn happy_path_writes_config_and_registers_bound_service() {
    let dir = temp_dir("happy");
    let source = write_source(&dir, VALID_CONFIG);
    let dest = dir.join("dest").join("config.json");
    let registrar = FakeRegistrar::recording();

    let report = perform_setup(&source, &dest, &registrar).expect("setup succeeds");

    // config 가 사용자 레벨 경로에 기록되고 서버 엔드포인트가 리포트된다.
    assert_eq!(report.dest_path, dest);
    assert_eq!(report.server_endpoint, "https://okc.example.com/api/sync");
    assert!(dest.exists());
    // 원본 내용이 그대로 보존된다(재직렬화로 federated 필드가 유실되지 않음).
    let written = std::fs::read_to_string(&dest).unwrap();
    assert!(written.contains("\"data_dir\": \"/absolute/data\""));

    // 서비스는 정확히 이 config 경로에 바인딩되어 한 번 등록된다.
    assert_eq!(registrar.call_count(), 1);
    assert_eq!(registrar.last_bound().as_deref(), Some(dest.as_path()));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(&dest).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "config 는 0600 이어야 한다");
    }
}

#[test]
fn invalid_config_unknown_key_rejected_without_write_or_register() {
    let dir = temp_dir("unknown-key");
    let source = write_source(
        &dir,
        r#"{"vault_path":"/v","server_endpoint":"https://h/api","token":"t","not_a_real_key":1}"#,
    );
    let dest = dir.join("dest").join("config.json");
    let registrar = FakeRegistrar::recording();

    let err = perform_setup(&source, &dest, &registrar).expect_err("unknown key rejected");
    assert!(matches!(err, watcher_bin::setup::SetupError::Invalid(_)));
    // hard-gate 실패 시 어떤 쓰기/등록도 일어나지 않는다(fail-closed).
    assert!(!dest.exists());
    assert_eq!(registrar.call_count(), 0);
}

#[test]
fn non_https_endpoint_rejected() {
    let dir = temp_dir("http-scheme");
    let source = write_source(
        &dir,
        r#"{"vault_path":"/v","server_endpoint":"http://insecure/api","token":"t"}"#,
    );
    let dest = dir.join("dest").join("config.json");
    let registrar = FakeRegistrar::recording();

    let err = perform_setup(&source, &dest, &registrar).expect_err("http scheme rejected");
    assert!(matches!(err, watcher_bin::setup::SetupError::Invalid(_)));
    assert!(!dest.exists());
    assert_eq!(registrar.call_count(), 0);
}

#[test]
fn blank_token_rejected() {
    let dir = temp_dir("blank-token");
    let source = write_source(
        &dir,
        r#"{"vault_path":"/v","server_endpoint":"https://h/api","token":"   "}"#,
    );
    let dest = dir.join("dest").join("config.json");
    let registrar = FakeRegistrar::recording();

    let err = perform_setup(&source, &dest, &registrar).expect_err("blank token rejected");
    assert!(matches!(err, watcher_bin::setup::SetupError::Invalid(_)));
    assert!(!dest.exists());
    assert_eq!(registrar.call_count(), 0);
}

#[test]
fn error_output_never_leaks_token() {
    // 유효한(비밀스러운) 토큰이 있으나 다른 사유(비-https)로 검증 실패 -> 오류 문자열에 토큰이
    // 절대 나타나지 않아야 한다(SECURITY-03).
    let dir = temp_dir("redaction");
    let secret = "TOP-SECRET-TOKEN-DO-NOT-LEAK-1234567890";
    let source = write_source(
        &dir,
        &format!(
            r#"{{"vault_path":"/v","server_endpoint":"http://insecure/api","token":"{secret}"}}"#
        ),
    );
    let dest = dir.join("dest").join("config.json");
    let registrar = FakeRegistrar::recording();

    let err = perform_setup(&source, &dest, &registrar).expect_err("rejected");
    let rendered = format!("{err}");
    assert!(
        !rendered.contains(secret),
        "오류 출력에 토큰이 노출되면 안 된다: {rendered}"
    );
    // Debug 표면도 토큰을 노출하지 않아야 한다.
    let debug = format!("{err:?}");
    assert!(!debug.contains(secret), "Debug 출력에 토큰이 노출되면 안 된다");
}

#[test]
fn setup_is_idempotent_on_rerun() {
    let dir = temp_dir("idempotent");
    let source = write_source(&dir, VALID_CONFIG);
    let dest = dir.join("dest").join("config.json");
    let registrar = FakeRegistrar::recording();

    perform_setup(&source, &dest, &registrar).expect("first run");
    // 재실행: 덮어쓰기 + 재등록(upsert)으로 오류 없이 성공한다(NFR-3).
    perform_setup(&source, &dest, &registrar).expect("second run");

    assert!(dest.exists());
    assert_eq!(registrar.call_count(), 2);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(&dest).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "재실행 후에도 0600 이 유지되어야 한다");
    }
}

#[test]
fn registration_failure_surfaces_after_write() {
    // config 는 검증·기록되지만 서비스 등록기 실패는 오류로 표면화된다(fail-closed 표면).
    let dir = temp_dir("reg-fail");
    let source = write_source(&dir, VALID_CONFIG);
    let dest = dir.join("dest").join("config.json");
    let registrar = FakeRegistrar::failing();

    let err = perform_setup(&source, &dest, &registrar).expect_err("registration fails");
    assert!(matches!(
        err,
        watcher_bin::setup::SetupError::Registration(_)
    ));
    assert_eq!(registrar.call_count(), 1);
}

// ---------------------------------------------------------------------------
// PBT — config serde round-trip (proptest-support feature 뒤).
// ---------------------------------------------------------------------------

#[cfg(feature = "proptest-support")]
mod pbt {
    use foundation::proptest_support::generators::arb_valid_watcher_config;
    use proptest::prelude::*;
    use watcher_bin::config::known_config_keys;

    proptest! {
        /// NFR-4 — `setup` 이 기록하는 config 모델의 serde round-trip: 직렬화 후 재검증하면 원본과 같다.
        #[test]
        fn setup_config_serialize_roundtrip(config in arb_valid_watcher_config()) {
            let serialized = serde_json::to_string(&config).expect("serialize");
            let reparsed = foundation::validate(&serialized, &known_config_keys())
                .expect("serialized config re-validates");
            prop_assert_eq!(reparsed, config);
        }
    }
}

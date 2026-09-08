//! `ConfigProvider` 애그리게이트 — JSON 로드·검증 + 활성 스냅샷 + 관찰자 팬아웃.
//!
//! 서브모듈 분해(추적성 맵, 물리 증식 강제 아님):
//! - `model`: `WatcherConfig`(6개 core 필드) + `ConfigSnapshot` + R-LIMIT-01 상수 + core-key 상수.
//! - `url_validator`: `server_endpoint` 의 https-only URL 검증.
//! - `validator`: `serde_json` 2-pass 검증(미지 키 전수 + FIRST-field 위반) + `ConfigError`.
//! - `loader`: R-DISCOVER-01 경로 우선순위 해소 + 파일 읽기 + 검증 오케스트레이션.
//! - `observer`: 불변 순서 관찰자 레지스트리 + per-observer `catch_unwind` fan-out.
//! - `store`: `ArcSwap` lock-free 읽기 + reload_mutex 직렬화 쓰기(`ConfigProvider`).
//!
//! 공개 표면: `ConfigProvider::new(known_keys)`, `load(path)`, `current() -> ConfigSnapshot`,
//! `reload() -> Result<(), ConfigError>`, `subscribe(observer)`.

/// config 파일 읽기 + R-DISCOVER-01 경로 우선순위 해소 오케스트레이션.
pub mod loader;
/// `WatcherConfig` / `ConfigSnapshot` 값 모델 + R-LIMIT-01 상수 + core-key 상수.
pub mod model;
/// 불변 순서 관찰자 레지스트리 + per-observer 패닉 격리 fan-out.
pub mod observer;
/// `ArcSwap` lock-free 읽기 + reload_mutex 직렬화 쓰기(`ConfigProvider`).
pub mod store;
/// `server_endpoint` 의 https-only URL 검증.
pub mod url_validator;
/// `serde_json` 2-pass 검증 + `ConfigError`(thiserror, 이슈 `Vec`).
pub mod validator;

pub use loader::{read_and_validate, resolve_config_path};
pub use model::{
    ConfigSnapshot, FOUNDATION_CONFIG_KEYS, MAX_FILE_BYTES, MAX_FILE_COUNT, MAX_VAULT_TOTAL_BYTES,
    WatcherConfig,
};
pub use observer::ObserverRegistry;
pub use store::ConfigProvider;
pub use url_validator::{UrlValidationError, validate_https_url};
pub use validator::{ConfigError, ConfigIssue, validate};

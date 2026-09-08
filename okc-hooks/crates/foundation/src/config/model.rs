//! WatcherConfig 값 모델 — U0 가 소유하는 6개 core config 필드와 활성 스냅샷 래퍼.
//!
//! `WatcherConfig` 는 사람이 편집하는 단일 JSON(nginx식)에서 파싱·검증되는 타입드 모델이며,
//! `ConfigSnapshot` 은 `Arc<WatcherConfig>` 를 감싼 lock-free 읽기용 스냅샷이다.
//! `SafetyLimits` 는 config 필드가 아니라 컴파일타임 상수(R-LIMIT-01)이며, 대응 config 키는
//! 미지 키로 거부된다. U0 가 소유하는 core 키 목록은 `FOUNDATION_CONFIG_KEYS` 로 노출된다.
//!
//! 순수 값 모델 표면으로서 panic-free-total(U0-NFR-REL-02)을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::core_types::{LogLevel, TokenSecret};

/// U0(`foundation`)가 소유하는 6개 core config 키 목록(DEC-FEDERATED-KEYS, Q6=A).
///
/// `watcher-bin` 이 이 상수를 하위 단위들의 키와 함께 federated union 으로 집계해
/// `ConfigProvider::new(known_keys)` 로 주입한다. 이 상수가 union 에서 누락되면
/// R-CFG-STRICT-01(strict unknown-key reject)이 U0 자신의 키를 unknown 으로 거부하므로
/// 필수다. U0 는 하위 단위 키를 import 하지 않고 자기 키만 노출해 DAG 루트 비순환을 유지한다.
pub const FOUNDATION_CONFIG_KEYS: &[&str] = &[
    "vault_path",
    "server_endpoint",
    "token",
    "secure_store_enabled",
    "log_level",
    "notify_consecutive_failures",
];

/// R-LIMIT-01: 총 볼트 크기 상한(20 GiB). config 로 변경 불가한 컴파일타임 프리플라이트 가드.
pub const MAX_VAULT_TOTAL_BYTES: u64 = 20 * 1024 * 1024 * 1024;

/// R-LIMIT-01: 파일당 크기 상한(2 GiB). 실제 경계·단조성 검사는 U1 `SafetyLimitsValidator` 소유.
pub const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// R-LIMIT-01: 총 파일 수 상한(100,000). 대응 config 키가 존재하면 미지 키로 거부된다.
pub const MAX_FILE_COUNT: u64 = 100_000;

/// 사람이 편집하는 단일 JSON 에서 파싱·검증되는 U0 core config 모델.
///
/// U0 가 소유하는 6개 필드만 담으며, 다른 단위가 소비하는 섹션(debounce_ms 등)은 federated
/// known-key 집합의 일부이나 U0 타입드 모델의 필드가 아니다(typed deserialize 시 무시된다 —
/// `deny_unknown_fields` 미채택, 미지 키 판정은 2-pass Value 스캔이 담당).
/// `token` 필드는 redacting `TokenSecret` 으로 감싸 로그 유출을 방지한다(SEC-02).
/// `Serialize`/`Deserialize` 는 무손실 round-trip(PROP-BR-02) 대상이다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatcherConfig {
    /// 볼트 루트 경로. 필수·비어있지 않은 **절대 파일시스템 경로**(존재 검사는 U2 관심사).
    pub vault_path: String,
    /// 서버 엔드포인트. 필수·유효 URL·**https 스킴만 허용**(TLS 강제, NFR-06).
    pub server_endpoint: String,
    /// API 토큰(선택). 부재 시 env/secure-store 폴백. 존재하되 공백이면 검증 실패.
    #[serde(default)]
    pub token: Option<TokenSecret>,
    /// secure-store 사용 여부(선택, 기본 `false`).
    #[serde(default)]
    pub secure_store_enabled: bool,
    /// 최소 로그 레벨(선택, 기본 `Info`). `trace`/`debug`/`info`/`warn`/`error` 중 하나.
    #[serde(default = "default_log_level")]
    pub log_level: LogLevel,
    /// 연속 실패 통지 임계값(선택, 기본 `3`). `>= 1` 인 양의 정수여야 한다.
    #[serde(default = "default_notify_consecutive_failures")]
    pub notify_consecutive_failures: u32,
}

/// `log_level` 의 serde 기본값(`Info`)을 제공한다.
fn default_log_level() -> LogLevel {
    LogLevel::Info
}

/// `notify_consecutive_failures` 의 serde 기본값(`3`)을 제공한다.
fn default_notify_consecutive_failures() -> u32 {
    3
}

/// 활성 `WatcherConfig` 의 lock-free 읽기용 스냅샷(`Arc<WatcherConfig>` 래퍼).
///
/// `ConfigProvider::current()` 가 반환하며, 동시 reader 는 항상 완결된 하나의 스냅샷만 관측한다
/// (중간 상태 없음). `Deref` 로 내부 `WatcherConfig` 필드에 직접 접근할 수 있다.
#[derive(Debug, Clone)]
pub struct ConfigSnapshot(Arc<WatcherConfig>);

impl ConfigSnapshot {
    /// `Arc<WatcherConfig>` 로부터 스냅샷을 구성한다.
    pub fn new(config: Arc<WatcherConfig>) -> Self {
        ConfigSnapshot(config)
    }

    /// 내부 `WatcherConfig` 참조를 반환한다.
    pub fn config(&self) -> &WatcherConfig {
        &self.0
    }

    /// 내부 `Arc<WatcherConfig>` 를 소유권째 반환한다.
    pub fn into_arc(self) -> Arc<WatcherConfig> {
        self.0
    }
}

impl std::ops::Deref for ConfigSnapshot {
    type Target = WatcherConfig;

    fn deref(&self) -> &WatcherConfig {
        &self.0
    }
}

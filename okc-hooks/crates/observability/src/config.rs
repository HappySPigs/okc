//! ObservabilityConfig — U8 조립 루트가 원시 config 를 파싱·해소해 U6 컴포넌트 생성자에
//! 주입하는 RESOLVED TYPED 값 뷰 + federated known-key 선언(R-CFG-U6-01).
//!
//! U6 는 비코어 config(`log_file`/`log_max_size_bytes`/`log_max_retained`/`history_file`/
//! `tray_enabled`)를 `ConfigProvider` 로 읽지 않고 이 주입 뷰로 소비한다(U0 FROZEN). U0 코어
//! 필드(`log_level`/`notify_consecutive_failures`)만 `ConfigProvider` 경유로 소비한다.
//!
//! 순수 값 뷰 모듈로서 panic-free-total 을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::path::PathBuf;

use foundation::{ByteCount, LogLevel};

/// U6 가 소유하는 federated known-key 목록(R-CFG-U6-01, D14).
///
/// `watcher-bin`(U8)이 전 단위 키를 federated union 으로 집계해 `ConfigProvider::new(known_keys)`
/// 로 주입한다. 이 목록이 union 에서 누락되면 U0 strict unknown-key reject 가 U6 키를 오탐
/// 거부한다. 코어 키(`log_level`/`notify_consecutive_failures`)는 U0 소유라 포함하지 않는다.
pub const OBSERVABILITY_CONFIG_KEYS: &[&str] = &[
    "log_file",
    "log_max_size_bytes",
    "log_max_retained",
    "history_file",
    "tray_enabled",
];

/// U8 이 해소해 U6 컴포넌트에 주입하는 RESOLVED TYPED config 뷰.
///
/// 파일 경로는 부재 시 U8 이 플랫폼 기본 경로로 해소한 뒤 `PathBuf` 로 주입한다(§0 federated 주입).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservabilityConfig {
    /// JSON-line 로그 방출 대상 파일 경로.
    pub log_file: PathBuf,
    /// rotation 트리거 크기 임계(`>= 1`).
    pub log_max_size_bytes: ByteCount,
    /// 보존할 회전 파일 수(`>= 0`, 개수 근사 — D2 트림).
    pub log_max_retained: usize,
    /// append-only 히스토리 파일 경로.
    pub history_file: PathBuf,
    /// 트레이 활성 플래그(파싱만, no-op — D1).
    pub tray_enabled: bool,
}

/// `StructuredLogger` 생성자에 주입되는 로그 설정 뷰(로그 파일 + rotation 파라미터 + 초기 레벨).
///
/// `log_level` 은 U0 코어 필드(초기값은 `ConfigProvider` 에서 해소해 주입); rotation 파라미터는
/// federated 주입 값으로 live-reload 대상이 아니다(재시작 반영, §0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggerConfig {
    /// JSON-line 방출 대상 파일 경로.
    pub log_file: PathBuf,
    /// rotation 트리거 크기 임계.
    pub log_max_size_bytes: ByteCount,
    /// 보존할 회전 파일 수(개수 근사).
    pub log_max_retained: usize,
    /// 최소 방출 레벨(초기값, reload 로 재적용 가능).
    pub log_level: LogLevel,
}

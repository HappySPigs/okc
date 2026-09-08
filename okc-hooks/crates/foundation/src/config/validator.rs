//! ConfigValidator — `serde_json` 2-pass 검증(Q5=C)과 이슈 `Vec` 을 나르는 `ConfigError`.
//!
//! 2-pass(확정):
//! - 1st-pass: `serde_json::Value` 에서 주입된 federated known-key 집합 밖의 **모든** 미지 키를
//!   전수 수집한다(첫 키에서 중단하지 않음, R-CFG-STRICT-01). 미지 키가 하나라도 있으면 즉시
//!   실패로 표면화하고 typed deserialize 를 수행하지 않는다.
//! - 2nd-pass: 미지 키가 clean 판정된 후에만 typed deserialize 를 수행하고, 그 뒤 per-field
//!   의미 검증(business-rules §4.1)을 필드 순서대로 수행해 **첫** 위반만 보고한다
//!   (bespoke full-field 누적 없음).
//!
//! `ConfigError` 이슈 `Vec` = (다수) 미지-키 이슈 **또는** 단일 typed/first-field 위반이다.
//! `SafetyLimits` 대응 키는 known-key 집합 밖이므로 미지 키로 거부된다(R-LIMIT-01).
//!
//! 순수 검증 표면으로서 panic-free-total(U0-NFR-REL-02)을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use super::model::WatcherConfig;
use super::url_validator;

/// 단일 config 검증 이슈. `ConfigError` 가 이들의 `Vec` 을 나른다.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigIssue {
    /// JSON 문서 파싱 실패(문법 오류·빈 파일 등).
    #[error("JSON 파싱 실패: {0}")]
    Parse(String),
    /// 스키마에 없는 미지/여분 키(R-CFG-STRICT-01).
    #[error("알 수 없는 키 `{key}`(위치 {pointer})")]
    UnknownKey {
        /// 미지 키 이름.
        key: String,
        /// JSON pointer 위치(예: `/vault_paht`).
        pointer: String,
    },
    /// typed deserialize 실패(타입 불일치·필수 필드 누락·열거값 위반 등).
    #[error("타입/필수 필드 위반: {0}")]
    TypeError(String),
    /// per-field 의미 검증 위반(business-rules §4.1).
    #[error("필드 `{field}` 위반 — 기대: {expected}")]
    FieldViolation {
        /// 위반 필드 이름.
        field: String,
        /// 기대하는 형식/범위 설명.
        expected: String,
    },
    /// config 경로 해소 실패(예: reload 이전 최초 load 미완료).
    #[error("config 경로 해소 실패: {0}")]
    Discovery(String),
    /// config 파일 읽기(IO) 실패.
    #[error("config 파일 읽기 실패 `{path}`: {detail}")]
    Io {
        /// 읽기를 시도한 경로.
        path: String,
        /// IO 오류 상세.
        detail: String,
    },
}

/// config 로드·검증 실패를 표면화하는 운영 오류. 하나 이상의 `ConfigIssue` 를 나른다.
///
/// `thiserror` 파생 운영 오류이며(분류 값 타입 `ErrorClass` 와 별개 부류, U0-NFR-MNT-01),
/// 미지 키 다수 또는 단일 first-field 위반을 이슈 `Vec` 으로 운반한다.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("config 검증 실패: {} 개 이슈", .issues.len())]
pub struct ConfigError {
    /// 수집된 검증 이슈 목록(미지 키 전수 또는 단일 위반).
    pub issues: Vec<ConfigIssue>,
}

impl ConfigError {
    /// 이슈 목록으로부터 `ConfigError` 를 구성한다.
    pub fn new(issues: Vec<ConfigIssue>) -> Self {
        ConfigError { issues }
    }

    /// 단일 이슈로부터 `ConfigError` 를 구성한다.
    pub fn single(issue: ConfigIssue) -> Self {
        ConfigError {
            issues: vec![issue],
        }
    }

    /// 수집된 이슈 슬라이스를 반환한다.
    pub fn issues(&self) -> &[ConfigIssue] {
        &self.issues
    }

    /// 모든 이슈를 사람이 읽는 단일 문자열로 결합한다(구조화 메시지, U0-NFR-USE-01).
    pub fn report(&self) -> String {
        self.issues
            .iter()
            .map(|issue| issue.to_string())
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// 2-pass 로 raw JSON 문자열을 검증하고 타입드 `WatcherConfig` 를 산출한다.
///
/// `known_keys` 는 `watcher-bin` 이 집계한 federated union(§5)이며 미지-키 판정 기준이다.
/// 실패 시 이슈 `Vec` 을 담은 `ConfigError` 를 반환한다.
pub fn validate(raw_json: &str, known_keys: &[&str]) -> Result<WatcherConfig, ConfigError> {
    // 1st-pass: Value 파싱 후 미지 키 전수 수집.
    let value: serde_json::Value = serde_json::from_str(raw_json)
        .map_err(|e| ConfigError::single(ConfigIssue::Parse(e.to_string())))?;

    if let Some(object) = value.as_object() {
        let unknown: Vec<ConfigIssue> = object
            .keys()
            .filter(|key| !known_keys.contains(&key.as_str()))
            .map(|key| ConfigIssue::UnknownKey {
                key: key.clone(),
                pointer: format!("/{key}"),
            })
            .collect();
        if !unknown.is_empty() {
            return Err(ConfigError::new(unknown));
        }
    }

    // 2nd-pass: 미지 키가 clean 인 경우에만 typed deserialize.
    let config: WatcherConfig = serde_json::from_value(value)
        .map_err(|e| ConfigError::single(ConfigIssue::TypeError(e.to_string())))?;

    // per-field 의미 검증 — 필드 순서대로 첫 위반만 보고.
    if let Some(issue) = first_field_violation(&config) {
        return Err(ConfigError::single(issue));
    }

    Ok(config)
}

/// business-rules §4.1 per-field 규칙을 필드 순서대로 평가해 첫 위반을 반환한다.
fn first_field_violation(config: &WatcherConfig) -> Option<ConfigIssue> {
    // vault_path: 비어있지 않은 절대 파일시스템 경로(존재 검사는 U2).
    if config.vault_path.is_empty() || !is_absolute_path(&config.vault_path) {
        return Some(ConfigIssue::FieldViolation {
            field: "vault_path".to_string(),
            expected: "비어있지 않은 절대 파일시스템 경로".to_string(),
        });
    }

    // server_endpoint: 유효 URL + https-only(UrlValidator 위임).
    if let Err(err) = url_validator::validate_https_url(&config.server_endpoint) {
        return Some(ConfigIssue::FieldViolation {
            field: "server_endpoint".to_string(),
            expected: format!("https 스킴의 유효한 URL({err})"),
        });
    }

    // token: 존재 시 비어있지 않은 문자열(공백-only 거부).
    if let Some(token) = &config.token
        && token.expose().trim().is_empty()
    {
        return Some(ConfigIssue::FieldViolation {
            field: "token".to_string(),
            expected: "존재할 경우 비어있지 않은 문자열".to_string(),
        });
    }

    // notify_consecutive_failures: >= 1(0/음수/비정수는 typed pass 에서 이미 걸러짐).
    if config.notify_consecutive_failures < 1 {
        return Some(ConfigIssue::FieldViolation {
            field: "notify_consecutive_failures".to_string(),
            expected: ">= 1 인 양의 정수".to_string(),
        });
    }

    None
}

/// 절대 파일시스템 경로인지 판정한다(POSIX 선행 `/`, UNC/백슬래시, Windows 드라이브 접두).
fn is_absolute_path(path: &str) -> bool {
    path.starts_with('/') || path.starts_with('\\') || is_windows_drive_absolute(path)
}

/// `C:\` 또는 `C:/` 형태의 Windows 드라이브 절대경로인지 판정한다(인덱싱 없이 char 순회).
fn is_windows_drive_absolute(path: &str) -> bool {
    let mut chars = path.chars();
    matches!(
        (chars.next(), chars.next(), chars.next()),
        (Some(drive), Some(':'), Some(sep))
            if drive.is_ascii_alphabetic() && (sep == '/' || sep == '\\')
    )
}

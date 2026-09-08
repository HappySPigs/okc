//! ConfigLoader — R-DISCOVER-01 경로 우선순위 해소 + 파일 읽기 + 2-pass 검증 오케스트레이션.
//!
//! 경로 우선순위(로드타임 관심사, config 내용 아님): `--config` CLI 플래그 > `OKC_WATCHER_CONFIG`
//! 환경변수 > 플랫폼 기본 경로. 파일 읽기 실패/파싱 실패/스키마 위반은 `ConfigError` 로 표면화한다.
//! 최초 로드 실패 = abort 의미(호출측 `watcher-bin` 이 non-zero exit 을 수행), 런타임 로드 실패 =
//! keep-last-good 경로(`ConfigStore::reload` 가 이전 스냅샷을 유지)로 이어진다.
//!
//! 파일 IO·환경변수 접근을 수행하므로 순수 표면이 아니다(clippy 순수 lint-gate 미적용).
//! 주입된 federated known-key 집합은 CONSUME 만 하며 하위 단위를 import 하지 않는다(DAG 루트).

use std::path::{Path, PathBuf};

use super::model::WatcherConfig;
use super::validator::{self, ConfigError, ConfigIssue};

/// R-DISCOVER-01: config 파일 경로를 우선순위대로 해소한다.
///
/// `--config` 플래그(`cli_path`)가 있으면 최우선으로 채택하고, 없으면 `OKC_WATCHER_CONFIG`
/// 환경변수(비어있지 않을 때), 그마저 없으면 플랫폼 기본 경로를 사용한다.
pub fn resolve_config_path(cli_path: Option<&Path>) -> PathBuf {
    if let Some(path) = cli_path {
        return path.to_path_buf();
    }
    if let Ok(env_path) = std::env::var("OKC_WATCHER_CONFIG")
        && !env_path.is_empty()
    {
        return PathBuf::from(env_path);
    }
    platform_default_path()
}

/// 플랫폼별 표준 기본 config 경로를 반환한다(최후순위).
fn platform_default_path() -> PathBuf {
    if cfg!(windows) {
        match std::env::var("PROGRAMDATA") {
            Ok(program_data) if !program_data.is_empty() => {
                let mut path = PathBuf::from(program_data);
                path.push("okc-watcher");
                path.push("config.json");
                path
            }
            _ => PathBuf::from("C:\\ProgramData\\okc-watcher\\config.json"),
        }
    } else {
        PathBuf::from("/etc/okc-watcher/config.json")
    }
}

/// 해소된 경로의 파일을 읽어 2-pass 검증(`validator::validate`)을 수행한다.
///
/// 파일 읽기 실패는 `ConfigIssue::Io` 로, 파싱/스키마 위반은 하위 검증기의 이슈로 표면화한다.
/// `known_keys` 는 주입된 federated union 이다.
pub fn read_and_validate(path: &Path, known_keys: &[&str]) -> Result<WatcherConfig, ConfigError> {
    let raw = std::fs::read_to_string(path).map_err(|e| {
        ConfigError::single(ConfigIssue::Io {
            path: path.display().to_string(),
            detail: e.to_string(),
        })
    })?;
    validator::validate(&raw, known_keys)
}

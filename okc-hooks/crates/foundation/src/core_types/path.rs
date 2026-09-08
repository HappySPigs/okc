//! PathNormalizer — 볼트 루트 기준의 정규화된 POSIX 상대경로 `RelativePath` 를 정의한다.
//!
//! 정규화는 값 구성 시점에 1회 적용되며 멱등이다
//! (`normalize(normalize(p)) == normalize(p)`). 이 타입은 매니페스트 엔트리 식별자이자
//! `ChangeSet.deleted` 원소이며, 매니페스트 정렬·`manifest_digest` 결정성의 기반이다.
//!
//! 순수 리프 모듈로서 panic-free-total(U0-NFR-REL-02)을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use serde::{Deserialize, Deserializer, Serialize};

/// 경로 정규화 실패 사유. 볼트 경계를 벗어나거나 상대경로가 아닌 입력은 거부된다.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PathError {
    /// 절대경로(선행 `/`) 또는 Windows 드라이브 접두(`C:`)를 가진 입력 — 볼트 경계 밖.
    #[error("절대경로 또는 드라이브 접두는 허용되지 않는다(상대경로만 유효)")]
    Absolute,
    /// `..` 세그먼트로 볼트 루트를 이탈하려는 입력.
    #[error("`..` 세그먼트로 볼트 루트를 이탈할 수 없다")]
    Escape,
    /// 정규화 결과가 빈 경로(예: `.` 또는 `/` 단독) — 파일을 식별할 수 없다.
    #[error("정규화 결과가 빈 경로가 되어 파일을 식별할 수 없다")]
    Empty,
}

/// 볼트 루트 기준의 **정규화된 POSIX 상대경로**.
///
/// 불변식: 구성에 성공한 값은 항상 POSIX 구분자(`/`) · 상대 · `..`/`.` 세그먼트 없음.
/// 동등성/정렬은 바이트 단위(UTF-8) 사전식이며 로케일 비의존이다 — Manifest 1차 정렬 키.
/// 대소문자 폴딩 없음, Unicode NFC/NFD 정규화 없음(무손실 round-trip 보존, NFR-13).
///
/// `Serialize` 는 내부 문자열을 그대로 방출하고, `Deserialize` 는 재정규화를 거쳐
/// 불변식을 재확립한다(정규화는 멱등이므로 이미 정규화된 값의 round-trip 은 동일값 복원).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct RelativePath(String);

impl RelativePath {
    /// 원시 경로 문자열을 정규화해 `RelativePath` 를 구성한다.
    ///
    /// 규칙: (1) 백슬래시 `\` -> `/` 치환, (2) 절대경로/드라이브 접두 거부,
    /// (3) `..` 세그먼트 거부, (4) `.` 세그먼트·중복 슬래시·선행/후행 슬래시 제거,
    /// (5) 대소문자·Unicode 는 바이트 그대로 보존. 어떤 입력에도 패닉하지 않고
    /// 위반은 `PathError` 로 표면화한다.
    pub fn normalize(input: &str) -> Result<RelativePath, PathError> {
        let unified = input.replace('\\', "/");
        if unified.starts_with('/') {
            return Err(PathError::Absolute);
        }
        let mut segments: Vec<&str> = Vec::new();
        for segment in unified.split('/') {
            match segment {
                "" | "." => continue,
                ".." => return Err(PathError::Escape),
                other => segments.push(other),
            }
        }
        if let Some(first) = segments.first()
            && is_drive_prefix(first)
        {
            return Err(PathError::Absolute);
        }
        if segments.is_empty() {
            return Err(PathError::Empty);
        }
        Ok(RelativePath(segments.join("/")))
    }

    /// 정규화된 경로 문자열 참조를 반환한다.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 내부 문자열을 소유권째 반환한다.
    pub fn into_string(self) -> String {
        self.0
    }
}

/// 세그먼트가 Windows 드라이브 접두(`C:` 형태)인지 판정한다(인덱싱 없이 char 순회).
fn is_drive_prefix(segment: &str) -> bool {
    let mut chars = segment.chars();
    matches!(
        (chars.next(), chars.next()),
        (Some(c), Some(':')) if c.is_ascii_alphabetic()
    )
}

impl<'de> Deserialize<'de> for RelativePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        RelativePath::normalize(&raw).map_err(serde::de::Error::custom)
    }
}

//! Manifest 값 모델 — `ManifestEntry`, `Manifest`, `ChangeSet`.
//!
//! U0 는 타입과 결정성/정렬 계약만 정의한다. `manifest_digest` 계산·`ChangeSet` diff 계산은
//! U1 소관이다. 모두 무손실 CBOR round-trip 대상(NFR-13, US-E7-06)이다.
//!
//! 순수 리프 모듈로서 panic-free-total(U0-NFR-REL-02)을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use serde::{Deserialize, Serialize};

use super::path::RelativePath;
use super::primitives::{ByteCount, ManifestDigest, Sha256Digest};

/// 볼트 내 파일 1개의 콘텐츠 지문 레코드. mtime 필드는 포함하지 않는다.
///
/// 파생 `Ord` 는 필드 선언 순서(`relative_path`, `raw_sha256`, `size`)를 따르며, 이는
/// canonical 정렬 키(경로 오름차순 1차, tie-break `(raw_sha256, size)`)와 정확히 일치한다.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// 볼트 루트 기준 정규화 경로 — 엔트리 식별자.
    pub relative_path: RelativePath,
    /// 파일 콘텐츠의 표준 SHA-256 — dedup·수정 판정 기준.
    pub raw_sha256: Sha256Digest,
    /// 파일 바이트 크기.
    pub size: ByteCount,
}

/// 한 사이클에서 재계산된 볼트 전체의 콘텐츠 스냅샷 지문.
///
/// 불변식(계약; 계산·정렬 강제는 U1): `entries` 는 `relative_path` 오름차순 canonical 정렬,
/// 경로 유일, 0-엔트리 유효. `manifest_digest == digest(canonical(entries))`.
/// 논리적으로 동일한 매니페스트는 발견 순서와 무관하게 동일 다이제스트를 갖는다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// canonical 정렬된 엔트리 시퀀스(경로 오름차순).
    pub entries: Vec<ManifestEntry>,
    /// 정렬된 엔트리 시퀀스에 대한 로컬 no-op 판별자.
    pub manifest_digest: ManifestDigest,
}

/// 마지막 커밋 매니페스트(prior) 대비 현재 매니페스트(current)의 차이(diff).
///
/// 재스냅샷 산출물이며 이벤트 큐가 아니다(FQ-2=A) — 지속되지 않는다. diff 계산은 U1 소관.
/// 세 목록은 각각 `relative_path` 오름차순 정렬을 권장 형상으로 한다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeSet {
    /// current 에만 존재하는 파일(신규).
    pub added: Vec<ManifestEntry>,
    /// 양쪽에 있으나 콘텐츠/크기가 달라진 파일(current 값).
    pub modified: Vec<ManifestEntry>,
    /// prior 에만 존재하고 current 에 없는 파일(삭제) — 경로만.
    pub deleted: Vec<RelativePath>,
}

impl ChangeSet {
    /// `added`/`modified`/`deleted` 세 목록이 모두 비었을 때 `true`(no-op 사이클 판정 근거).
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.deleted.is_empty()
    }
}

//! U1 이 도입하는 도메인 값 타입 — `ScannedFile`, `VaultSnapshot`.
//!
//! `VaultScanner` 열거 산출물이며 `ManifestBuilder` 입력이다. U0 값 타입(`RelativePath`/`ByteCount`/
//! `Timestamp`)을 재사용하고 재정의하지 않는다.
//!
//! 순수 값 타입으로서 panic-free-total 을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::path::PathBuf;

use foundation::{ByteCount, RelativePath, Timestamp};

/// 볼트 열거에서 발견한 파일 1개의 경로+크기(해시 이전 중간 레코드).
///
/// `relative_path` 는 U0 `RelativePath::normalize` 를 통과한 정규화 경로다. 심링크는 산출되지 않는다
/// (R-SCAN-02 SKIP). `size` 는 열거 시점 값이며 해시 시점과 다를 수 있다(TOCTOU; 최종 정합은 U3 재검증).
/// 파생 `Ord` 는 `(relative_path, size)` 순으로 canonical 정렬 키와 일치한다.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScannedFile {
    /// 볼트 루트 기준 정규화 상대경로(엔트리 식별자).
    pub relative_path: RelativePath,
    /// 열거 시점의 파일 바이트 크기.
    pub size: ByteCount,
}

/// 한 사이클의 일관 시점(best-effort) 볼트 열거 결과.
///
/// OS 레벨 파일시스템 스냅샷이 아니라 단일 walk 패스 산출물이며(D11), 열거~해시~전송 간 TOCTOU 는
/// U3 전송 재검증이 폐쇄한다. `files` 는 `relative_path` 오름차순 canonical 정렬·경로 유일이고,
/// 0-파일 스냅샷(빈 볼트)도 유효값이다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultSnapshot {
    /// 열거 대상 볼트 루트(config `vault_path`).
    pub root: PathBuf,
    /// 발견된 파일 목록(경로 오름차순 canonical 정렬).
    pub files: Vec<ScannedFile>,
    /// 열거를 캡처한 논리 시점(진단·상태 표면화용).
    pub captured_at: Timestamp,
}

//! ErrorTaxonomy — U1 운영 오류 `ScanError`/`BuildError`.
//!
//! `thiserror` 파생으로 `Display`/`std::error::Error` 및 원인(`source: io::Error`)을 보존한다
//! (U0-NFR-MNT-01 워크스페이스 관례 승계). 지속 대상이 아니므로 NFR-13 round-trip 대상이 아니다.
//! 판정(vault-unavailable) 해석·표면화는 U1 이 하지 않고 값으로 반환하여 U2/U8 이 수행한다.
//!
//! `io::Error` 가 `Clone`/`PartialEq` 를 구현하지 않아 이 타입들은 `Debug` 만 파생한다.

use std::io;

use foundation::RelativePath;

/// `VaultScanner` 의 열거/리더 오류. `RelativePath` 는 `Display` 미구현이라 메시지에 `Debug` 로 표기한다.
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    /// 볼트 루트 부재/언마운트/접근 불가 — 볼트 경계 자체 도달 불가(0-파일 매니페스트 미생성).
    #[error("볼트 루트에 도달할 수 없다(부재/언마운트/접근 불가)")]
    RootUnavailable,
    /// 특정 파일/디렉터리 열거·읽기 I/O 오류.
    #[error("경로 {path:?} 열거/읽기 I/O 오류: {source}")]
    Io {
        /// 오류가 발생한 볼트 상대경로.
        path: RelativePath,
        /// 근본 원인 I/O 오류.
        source: io::Error,
    },
}

/// `ManifestBuilder::build` 의 스캔/해시 단계 실패. fail-fast — 부분 매니페스트를 반환하지 않는다(D12).
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// 열거 단계 실패(래핑).
    #[error("스캔 단계 실패: {0}")]
    Scan(#[from] ScanError),
    /// 특정 파일 스트리밍 해시 중 I/O 오류.
    #[error("파일 {path:?} 해시 중 I/O 오류: {source}")]
    Hash {
        /// 해시 실패한 파일의 볼트 상대경로.
        path: RelativePath,
        /// 근본 원인 I/O 오류.
        source: io::Error,
    },
}

//! BlobSource — 전송 전 재검증·전송용 blob 바이트 재-읽기 seam (도메인 포트, D-03).
//!
//! U3 는 `VaultScanner`(U1)에 직접 의존하지 않고 이 포트만 의존한다 — 재검증(R-REVERIFY-01)과
//! 전송(R-REVERIFY-03)이 각각 스트리밍 리더를 열어 전량 메모리 적재 없이 읽는다. U8 조립루트가
//! `VaultScanner::open_reader` 를 이 포트로 배선한다. 테스트는 임의/변조 바이트를 반환하는 fake
//! `BlobSource` 로 파일시스템 없이 재검증 가드(PROP-U3-06)와 청크 재조립(PROP-U3-03/04)을 구동한다.
//!
//! 트레이트/오류 정의만 담는 순수 표면이므로 lint-gate 를 적용한다(구현 IO 는 배선 측 소관).
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::io::Read;

use foundation::RelativePath;

/// blob 재-읽기 실패 사유(진단용 상세 — 토큰/콘텐츠 원문 미포함).
#[derive(Debug, thiserror::Error)]
pub enum BlobSourceError {
    /// 지정 경로의 blob 을 찾을 수 없음.
    #[error("blob 미발견")]
    NotFound,
    /// 리더 열기/읽기 I/O 실패(진단용 상세 동반).
    #[error("blob 읽기 I/O 실패: {0}")]
    Io(String),
}

/// 상대경로 blob 의 스트리밍 리더를 여는 재-읽기 seam. `Send + Sync`(데몬 멀티스레드 공유).
pub trait BlobSource: Send + Sync {
    /// 상대경로 blob 의 스트리밍 리더(`Box<dyn Read>`)를 연다(전량 적재 없음).
    ///
    /// 재검증 패스와 전송 패스가 **각각 독립적으로** 호출해 2회 재-읽기를 수행한다(R-REVERIFY-03).
    fn open(&self, path: &RelativePath) -> Result<Box<dyn Read>, BlobSourceError>;
}

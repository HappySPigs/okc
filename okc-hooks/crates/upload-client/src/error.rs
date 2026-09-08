//! ErrorTaxonomy — U3 사이클 실패 오류 `UploadError`(component-methods, `thiserror` 파생).
//!
//! 드라이버는 이 taxonomy 를 **그대로 상위(U8)에 반환**하며 재시도/sleep 하지 않는다
//! (R-ERR-01/D-16). `Transport` 는 U0 `TransportError`(및 `TransportErrorClass`)를 그대로
//! 감싸 상위 U4 `RetryBackoffController` 가 분류·백오프한다. U3 는 `ErrorClass` 를 재계산하지 않는다.
//!
//! 순수 리프 모듈로서 panic-free-total 을 컴파일타임 clippy lint-gate 로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use foundation::{LimitReport, RelativePath, Sha256Digest, TransportError};

/// 사이클 실패 taxonomy(R-ERR-01). 재시도 판정은 상위(U4)가 소유한다.
#[derive(Debug, thiserror::Error)]
pub enum UploadError {
    /// `AuthTransport::send` 실패 — U0 `TransportError`(분류 포함)를 그대로 감싼다(R-WANT-04).
    #[error("전송 실패: {0:?}")]
    Transport(TransportError),
    /// 전송 직전 재-해시 결과가 매니페스트 `raw_sha256` 과 불일치 -> 커밋 미발행(R-REVERIFY-02, Q8=A).
    #[error("해시 불일치(TOCTOU): path={path:?} expected={expected} actual={actual}")]
    HashMismatch {
        /// 불일치가 발생한 blob 의 볼트 상대경로.
        path: RelativePath,
        /// 매니페스트에 기록된 기대 해시.
        expected: Sha256Digest,
        /// 재-읽기 후 재-해시한 실제 해시.
        actual: Sha256Digest,
    },
    /// 런타임 SafetyLimits 재검사에서 한도 초과 -> halt, 마지막 정상 커밋 보존(R-LIMIT-RT-02).
    #[error("SafetyLimits 초과: {0:?}")]
    OverLimit(LimitReport),
    /// 그 외 사이클 중단 — blob 읽기 I/O 실패·프로토콜 본문 인코딩/디코딩 실패 등(R-REVERIFY-04/R-ERR-02).
    #[error("사이클 중단(I/O 또는 프로토콜 본문 코덱 실패)")]
    Aborted,
}

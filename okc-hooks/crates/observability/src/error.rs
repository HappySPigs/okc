//! U6 운영 오류 타입 — `LogError`/`HistoryError`(U0 Q4=A 관례 상속, `thiserror` 파생).
//!
//! push 표면(`Logger::log`/`event`, `HistorySink::append`, `StatusSink`, `CriticalEventSink`)은
//! infallible 이므로 이 오류는 그 경로에 전파되지 않고 내부에서 삼켜진다(R-PUSH-02). 오직
//! 읽기/재적용 경로(`UploadHistoryStore::query`, `StructuredLogger::reload`)만 이 오류를 반환한다.
//!
//! 순수 리프 모듈로서 panic-free-total 을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use foundation::CodecError;

/// `StructuredLogger` 내부 로그 파일 I/O·rotation·재적용 오류.
///
/// `log`/`event` push 표면에는 전파되지 않고 삼켜지며(best-effort), `reload()` 재적용 경로에서만
/// 반환될 수 있다.
#[derive(Debug, thiserror::Error)]
pub enum LogError {
    /// 로그 파일 append/read 중 I/O 실패.
    #[error("로그 파일 I/O 실패: {0}")]
    Io(String),
    /// 로그 파일 경로가 유효하지 않음.
    #[error("잘못된 로그 경로: {0}")]
    InvalidPath(String),
    /// rotation(rename/삭제) 중 실패.
    #[error("로그 rotation 실패: {0}")]
    RotationFailed(String),
}

impl From<std::io::Error> for LogError {
    fn from(err: std::io::Error) -> Self {
        LogError::Io(err.to_string())
    }
}

/// `UploadHistoryStore` 의 히스토리 파일 I/O·직렬화·손상 오류.
///
/// `append` push 표면에는 전파되지 않고 삼켜지며(best-effort), `query()` 읽기 경로에서만 반환된다.
/// `Corrupt` 는 파일 **중간** 프레임 손상에만 사용되며, 트레일링 truncated-tail 은 정상 prefix
/// 반환으로 관용된다(R-HIST-04).
#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    /// 히스토리 파일 append/read 중 I/O 실패.
    #[error("히스토리 파일 I/O 실패: {0}")]
    Io(String),
    /// CBOR 인코딩/디코딩(직렬화) 실패.
    #[error("히스토리 직렬화 실패: {0}")]
    Serde(String),
    /// 파일 중간 프레임 손상(트레일링 truncated-tail 아님).
    #[error("히스토리 파일 손상(중간 프레임): {0}")]
    Corrupt(String),
}

impl From<std::io::Error> for HistoryError {
    fn from(err: std::io::Error) -> Self {
        HistoryError::Io(err.to_string())
    }
}

impl From<CodecError> for HistoryError {
    fn from(err: CodecError) -> Self {
        HistoryError::Serde(err.to_string())
    }
}

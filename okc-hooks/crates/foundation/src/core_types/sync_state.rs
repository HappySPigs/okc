//! SyncStateModel — 동기화 사이클 상태 enum `SyncState` 와 전이 표(모델/데이터만).
//!
//! 전이 합법성 검증·영속·복구·stateful PBT 실행은 U4 `SyncStateStore` 소관이며,
//! U0 는 타입과 전이 참조 모델만 소유한다. 지속 표현은 무손실 round-trip 대상(NFR-13).
//!
//! 순수 리프 모듈로서 panic-free-total(U0-NFR-REL-02)을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use serde::{Deserialize, Serialize};

/// 최신 상태 대체 모델(FQ-2=A)의 동기화 사이클 상태(Q8=A: 선형 + dirty 재진입).
///
/// 상태 집합은 아래 4개로 폐쇄되며 별도의 `Failed`/`Paused` 변형이 없다.
/// `SyncState` 값과 **별개로** 유지되는 dirty boolean 플래그(단일 신호, 큐 아님)가
/// "다음 사이클이 재스냅샷해야 함" 을 표시한다 — dirty 플래그 자체의 지속은 U4 소관.
///
/// 전이 표(모델만; 가드·실패 처리·영속 검증은 U4):
/// - T1 `Idle` + 변경 감지 -> `Dirty`
/// - T2 `Dirty` + 사이클 시작 -> `Uploading` (진입 시 dirty CLEAR)
/// - T3 `Uploading` + 업로드 성공 -> `Committed`
/// - T4 `Committed` + 커밋 지속 완료 & dirty UNSET -> `Idle`
/// - T5 `Uploading` + 새 변경 -> `Uploading` (유지, dirty SET)
/// - T6 `Committed` + 새 변경 -> `Committed` (유지, dirty SET)
/// - T7 `Committed` + 커밋 지속 완료 & dirty SET -> `Dirty` (소비, 재진입)
/// - T8 `Uploading` + 업로드 실패 -> `Dirty` (별도 Failed 상태 없음; U4 retry/backoff 대기)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SyncState {
    /// 대기 — 미처리 변경 없음(마지막 커밋 = 현재 볼트 상태).
    Idle,
    /// 미커밋 변경 존재 — 다음 사이클이 재스냅샷해야 함.
    Dirty,
    /// 사이클 진행 중(협상/전송/커밋 수행).
    Uploading,
    /// 업로드 성공, 커밋 지속 직전/직후.
    Committed,
}

//! 트리거 규약 — 두 트리거 소스(`FilesystemWatcher` 디바운스, `ReconciliationScheduler`
//! 시작/주기)가 공유하는 값 타입과 전송 채널 계약.
//!
//! 트리거는 **ChangeSet 를 탑재하지 않는다**(FQ-2=A) — 코디네이터(U8)가 트리거를 받아 볼트를
//! 재스냅샷하므로 "무엇이 바뀌었는지"를 트리거가 기억할 필요가 없다.
//!
//! 순수 리프 모듈로서 panic-free 를 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::time::Instant;

/// 재조정 트리거의 국면(phase). 시작 1회(`Startup`) vs 주기 백스톱(`Periodic`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReconPhase {
    /// 데몬 기동 시 1회 전체 재조정(US-E1-03).
    Startup,
    /// 마지막 재조정 후 `T_recon` 경과로 발행한 백스톱(US-E1-04).
    Periodic,
}

/// 트리거의 발생 소스·의미를 구분하는 태그(3-변이 폐쇄; Overflow 유래도 `Debounced`).
///
/// 코디네이터의 사이클 동작은 kind 와 무관하게 동일한 재스냅샷 경로다(FQ-2=A) — kind 는
/// 진단/로그 구분용이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriggerKind {
    /// `FilesystemWatcher` 가 편집 버스트의 정적 구간 도달로 발행(US-E1-01).
    Debounced,
    /// `ReconciliationScheduler` 가 발행한 재조정 트리거(국면 동반).
    Reconciliation(ReconPhase),
}

/// 트리거 1건. `cause_summary` 는 진단 전용이며 사이클 로직의 입력이 아니다(FQ-2).
///
/// `observed_at` 은 벽시계가 아니라 스케줄링용 **단조 시점**(`Instant`)이며 지속 대상이 아니다.
/// `cycle_id` 는 U8 이 부여하며 트리거에 담기지 않는다(U2 순수성).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerSignal {
    /// 발생 소스/의미.
    pub kind: TriggerKind,
    /// 트리거 유발 요약(예: "3 events in burst", "periodic recon due"). 진단·로그 보조.
    pub cause_summary: String,
    /// 트리거 발행 시각(디바운스 만료·tick 도래 시점, 단조 시점).
    pub observed_at: Instant,
}

impl TriggerSignal {
    /// 주어진 kind·요약·시각으로 트리거를 구성한다.
    pub fn new(kind: TriggerKind, cause_summary: String, observed_at: Instant) -> Self {
        TriggerSignal {
            kind,
            cause_summary,
            observed_at,
        }
    }
}

/// 트리거 소스 -> 코디네이터로의 **순서 보존 단방향 전달 채널**의 수신단.
///
/// D14 계약(순서 보존 + 단방향 push + U8 유일 소비자)을 std `std::sync::mpsc` 로 실현한다.
/// `FilesystemWatcher::start` 가 반환하며 U8 `SyncCycleCoordinator` 가 유일 소비자다.
pub type TriggerStream = std::sync::mpsc::Receiver<TriggerSignal>;

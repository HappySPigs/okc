//! ConsentGate 값 타입 + 순수 상태 판정(게이트/projection/일관성 강등).
//!
//! `ConsentLifecycle`(4-상태, U0 `ConsentState` 와 별개), `ConsentGrant`/`GrantId`(부여 참조),
//! 조회/게이트 반환 형상, 운영 오류, 지속 레코드를 정의한다. 코덱·`Timestamp`·`ConsentState`
//! projection 타깃은 U0 소유를 소비한다(재정의 없음).
//!
//! 순수 값/판정 표면으로서 panic-free-total 을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use foundation::{ConsentState, Timestamp};
use serde::{Deserialize, Serialize};

/// 동의 4-상태 라이프사이클(DEC-U5-09). 업로드 허용은 오직 `Granted` 에서만 성립.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConsentLifecycle {
    /// RISK-01 고지 미확인(최초 실행) — 차단(`NeedsAcknowledgment`).
    NotAcknowledged,
    /// 고지 확인됨, 상시 동의 미부여 — 차단(`NotGranted`).
    AcknowledgedNotGranted,
    /// 상시 동의 부여됨 — 허용.
    Granted,
    /// 동의 철회됨(forward-only) — 차단(`Withdrawn`).
    Withdrawn,
}

/// 부여 유일 식별자(불투명). 구체 생성 방식(UUID vs 난수)은 Code Generation 이월(DEC-U5-10).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GrantId(pub String);

/// 상시 동의 부여 참조(FR-14) — 서버가 없어도 로컬 캡처로 표현한다(DEP-02 `[blocked-on-server]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentGrant {
    /// 부여 식별자.
    pub grant_id: GrantId,
    /// 부여 시각(UTC).
    pub granted_at: Timestamp,
}

/// CLI `consent view` 출력 형상(부여 참조·시각·현재 상태). 부수효과 없이 반환.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentStatus {
    /// 고지 확인 여부.
    pub acknowledged: bool,
    /// 부여 참조(있으면).
    pub grant: Option<ConsentGrant>,
    /// 현재 라이프사이클 상태.
    pub state: ConsentLifecycle,
}

/// 코디네이터 업로드 게이트 판정.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsentDecision {
    /// 업로드/커밋 허용(`Granted` 에서만).
    Permitted,
    /// 업로드/커밋 차단 — 상태별 사유 동반.
    Blocked(BlockReason),
}

/// 업로드 차단 사유(상태와 1:1 대응).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockReason {
    /// 고지 미확인.
    NeedsAcknowledgment,
    /// 고지 확인됨, 미부여.
    NotGranted,
    /// 철회됨.
    Withdrawn,
}

/// 동의 연산 오류(운영 오류).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConsentError {
    /// `grant()` 인데 고지 미확인(선확인 필수, DEC-U5-16).
    #[error("고지 미확인: grant 이전에 acknowledge 가 필요합니다")]
    NotAcknowledged,
    /// `grant()` 인데 이미 부여됨.
    #[error("이미 동의가 부여되었습니다")]
    AlreadyGranted,
    /// `withdraw()` 인데 활성 부여가 없음.
    #[error("철회할 활성 동의 부여가 없습니다")]
    NoGrant,
    /// `ConsentRecord` 원자적 지속 쓰기 실패(인메모리 롤백 동반).
    #[error("동의 레코드 지속 실패")]
    PersistFailed,
}

/// 재시작 후에도 지속되는 단일 로컬 동의 레코드(DEC-U5-11, CBOR round-trip 대상).
///
/// 일관성 불변식: `state == Granted` 이면 `grant.is_some()`; `state == NotAcknowledged` 이면
/// `acknowledged == false` 이고 `grant.is_none()`. `Withdrawn` 이어도 `acknowledged == true` 유지.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentRecord {
    /// RISK-01 고지 확인 여부.
    pub acknowledged: bool,
    /// 현재 라이프사이클 상태.
    pub state: ConsentLifecycle,
    /// 부여 참조(있으면).
    pub grant: Option<ConsentGrant>,
}

impl ConsentRecord {
    /// 최초 상태(파일 없음) — 고지 미확인.
    pub fn initial() -> Self {
        ConsentRecord {
            acknowledged: false,
            state: ConsentLifecycle::NotAcknowledged,
            grant: None,
        }
    }

    /// 이 레코드가 일관성 불변식(§3.5)을 만족하는지 여부.
    pub fn is_consistent(&self) -> bool {
        match self.state {
            ConsentLifecycle::NotAcknowledged => !self.acknowledged && self.grant.is_none(),
            ConsentLifecycle::AcknowledgedNotGranted => self.acknowledged && self.grant.is_none(),
            ConsentLifecycle::Granted => self.acknowledged && self.grant.is_some(),
            ConsentLifecycle::Withdrawn => self.acknowledged,
        }
    }
}

/// 업로드 게이트 판정(R-CG-GATE) — `Granted <-> Permitted`, 그 외 -> 대응 `BlockReason`.
pub fn decide(state: ConsentLifecycle) -> ConsentDecision {
    match state {
        ConsentLifecycle::Granted => ConsentDecision::Permitted,
        ConsentLifecycle::NotAcknowledged => {
            ConsentDecision::Blocked(BlockReason::NeedsAcknowledgment)
        }
        ConsentLifecycle::AcknowledgedNotGranted => ConsentDecision::Blocked(BlockReason::NotGranted),
        ConsentLifecycle::Withdrawn => ConsentDecision::Blocked(BlockReason::Withdrawn),
    }
}

/// `ConsentLifecycle -> ConsentState`(U0 3변이) projection(R-CG-PROJECT, total).
pub fn project(state: ConsentLifecycle) -> ConsentState {
    match state {
        ConsentLifecycle::Granted => ConsentState::Granted,
        ConsentLifecycle::Withdrawn => ConsentState::Blocked,
        ConsentLifecycle::NotAcknowledged => ConsentState::Unknown,
        ConsentLifecycle::AcknowledgedNotGranted => ConsentState::Unknown,
    }
}

/// 로드된 레코드를 일관성 불변식으로 강등한다(R-CG-PERSIST, fail-safe: 차단 우선).
///
/// 일관성을 만족하면 그대로 반환한다. 위반(손상) 레코드는 업로드 금지 방향으로 강등한다 —
/// `acknowledged` 이면 `AcknowledgedNotGranted`(부여 폐기), 아니면 `NotAcknowledged`.
pub fn sanitize(record: ConsentRecord) -> ConsentRecord {
    if record.is_consistent() {
        return record;
    }
    if record.acknowledged {
        ConsentRecord {
            acknowledged: true,
            state: ConsentLifecycle::AcknowledgedNotGranted,
            grant: None,
        }
    } else {
        ConsentRecord::initial()
    }
}

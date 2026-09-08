//! StatusVocab — Q9 2축 상태 어휘 값 타입.
//!
//! 축1 = 운영 라이프사이클(단일 값, `OperationalState`), 축2 = 동시 성립 활성 조건 집합
//! (`ActiveCondition`), startup 신호(`LivenessSignal`), 그리고 이 모두를 집계하는
//! `StatusSnapshot`. 조건 -> 운영상태 결합 판정 규칙은 U6 소관이며 U0 는 어휘만 정의한다.
//!
//! 순수 리프 모듈로서 panic-free-total(U0-NFR-REL-02)을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use serde::{Deserialize, Serialize};

use super::primitives::{ByteCount, Timestamp};

/// 운영 라이프사이클(축1) — 한 시점에 정확히 하나의 값.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperationalState {
    /// 대기(사이클 없음).
    Idle,
    /// 사이클 진행 중.
    Syncing,
    /// 오프라인(서버 도달 불가).
    Offline,
    /// 운영자 pause(감시 상주, 사이클 보류).
    Paused,
}

/// 활성 조건(축2 원소) — 동시에 여러 개가 성립할 수 있다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ActiveCondition {
    /// 인증 실패(401/토큰 거부) 조건 활성.
    AuthFailed,
    /// 동의 미부여/철회로 업로드 차단 조건 활성.
    ConsentBlocked,
    /// SafetyLimits 초과 조건 활성.
    OverLimit,
    /// 볼트 루트 부재/언마운트 조건 활성.
    VaultUnavailable,
    /// 자동 업데이트 롤백 발생 조건 활성.
    UpdateRolledBack,
}

/// startup liveness 신호 — 각 단위가 push 한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LivenessSignal {
    /// 기동 후 최초 idle 도달.
    IdleReached,
    /// 토큰 소스 해소 가능 확인.
    CredentialReadable,
}

/// 동의 상태 — **U5 소유 참조 타입의 최소 자리표시**(placeholder).
///
/// `StatusSnapshot` 컴파일을 위해 U0 에 최소 정의를 둔다. 최종 변형·의미는 U5
/// `ConsentGate` Functional Design 이 확정한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConsentState {
    /// 동의 부여됨.
    Granted,
    /// 동의 차단/철회됨.
    Blocked,
    /// 미결정/미확인.
    Unknown,
}

/// CLI `status` 표면용 현재 2축 상태 + 부가 필드 스냅샷.
///
/// `conditions` 는 동시 성립 조건 집합을 순서 있는 `Vec`(중복 없음 계약)으로 표현한다.
/// 무손실 round-trip 대상(NFR-13). `consent` 는 U5 소유 참조 타입(`ConsentState`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusSnapshot {
    /// 운영 라이프사이클(축1) 단일 상태.
    pub operational: OperationalState,
    /// 활성 조건(축2) 집합.
    pub conditions: Vec<ActiveCondition>,
    /// 마지막 성공 시각(있으면).
    pub last_success: Option<Timestamp>,
    /// 미커밋 변경 대기 표시(FQ-2 큐 깊이 대체).
    pub dirty: bool,
    /// 진행 중 전송의 (전송량, 전체) 바이트 쌍(있으면).
    pub resume: Option<(ByteCount, ByteCount)>,
    /// 동의 상태(U5 소유 참조 타입).
    pub consent: ConsentState,
    /// 오프라인 여부.
    pub offline: bool,
}

/// 운영 건강 저하 사유 — **U6 소유 참조 타입의 최소 자리표시**(placeholder).
///
/// 구체 변형·판정 임계값은 U6 `StatusService` Functional Design 이 확정한다.
/// U0 는 `health_check()` 반환 형상(return-shape) 계약만 소유한다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthReason(pub String);

/// `ReadJudgment::health_check()` 의 반환 형상 — 운영 건강 판정 결과.
///
/// CLI `status`/`health` 종료코드 매핑(US-E5-02)의 근거가 되는 값이며, `Healthy`/`Unhealthy`
/// 판정에 필요한 임계값 로직은 U6 소관이다(U0 는 형상만 정의). 무손실 round-trip 대상(NFR-13).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Health {
    /// 운영상 건강함(모든 운영 조건 정상).
    Healthy,
    /// 건강하지 않음 — 저하 사유 목록 동반.
    Unhealthy {
        /// 건강 저하 사유 집합.
        reasons: Vec<HealthReason>,
    },
}

/// `ReadJudgment::update_probe()` 의 반환 형상 — 순수 liveness 판정 결과.
///
/// 운영 조건(AuthFailed/OverLimit 등)과 **분리**된 순수 liveness 로, 일시적 운영 조건 때문에
/// 좋은 새 버전이 오판 롤백되는 롤백 루프를 방지한다(U7a AutoUpdater 전용 소비). 판정 임계값은
/// U6 소관이며 U0 는 형상만 정의한다. 무손실 round-trip 대상(NFR-13).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Liveness {
    /// startup 및 필수 liveness 신호(IdleReached + CredentialReadable) 충족.
    Alive,
    /// 아직 준비되지 않음 — 미충족 liveness 신호 목록 동반.
    NotReady {
        /// 아직 관측되지 않은 필수 liveness 신호 집합.
        missing: Vec<LivenessSignal>,
    },
}

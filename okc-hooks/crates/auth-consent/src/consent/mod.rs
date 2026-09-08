//! `ConsentGate` 애그리게이트 — 동의 4-상태 라이프사이클 권위 소스 + 원자 지속 + 업로드 게이트.
//!
//! 논리 컴포넌트: ConsentStateMachine(전이 합법성/게이트/projection/forward-only) · ConsentStore
//! (원자 지속 + fail-safe 로드) · DisclosureProvider(고지 문안). 인메모리 상태 + 지속을
//! `Mutex<ConsentInner>` 쓰기 직렬화로 all-or-nothing 보장한다(§4 동시성). 어떤 동시 게이트
//! 읽기도 완결된(지속 커밋된) 상태만 관측한다.
//!
//! 파일 IO/뮤텍스/시각/싱크 push 를 수행하므로 순수 lint-gate 를 적용하지 않는다(순수 판정은
//! `types.rs` 가 lint-gate 하에 소유).

pub mod disclosure;
pub mod store;
pub mod types;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use foundation::{ActiveCondition, ConfigReloadObserver, ConsentState, StatusSink, Timestamp};

use store::ConsentStore;
use types::{decide, project, sanitize};

pub use disclosure::disclosure_text;
// 도메인 값 타입을 원래 이름으로 crate 공개 표면에 재-export 한다.
pub use types::{
    BlockReason, ConsentDecision, ConsentError, ConsentGrant, ConsentLifecycle, ConsentRecord,
    ConsentStatus, GrantId,
};

/// 부여 시각을 제공하는 clock seam(테스트 결정성). 프로덕션 구현은 `SystemClock`. `Send + Sync`.
pub trait Clock: Send + Sync {
    /// 현재 UTC 시각을 반환한다.
    fn now(&self) -> Timestamp;
}

/// 실제 시스템 시각을 사용하는 `Clock` 구현체.
#[derive(Debug, Default, Clone)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let clamped = nanos.min(i64::MAX as u128) as i64;
        Timestamp::from_unix_nanos(clamped)
    }
}

/// 뮤텍스로 보호되는 인메모리 동의 상태.
struct ConsentInner {
    record: ConsentRecord,
}

/// 동의 상태머신 + 업로드 게이트. 전이+지속을 임계구역 안에서 직렬화한다.
pub struct ConsentGate {
    inner: Mutex<ConsentInner>,
    store: ConsentStore,
    clock: Arc<dyn Clock>,
    status_sink: Option<Arc<dyn StatusSink>>,
    seq: AtomicU64,
}

impl ConsentGate {
    /// 스토어에서 상태를 로드(fail-safe)해 게이트를 연다.
    pub fn open(
        store: ConsentStore,
        clock: Arc<dyn Clock>,
        status_sink: Option<Arc<dyn StatusSink>>,
    ) -> Self {
        let record = store.load();
        ConsentGate {
            inner: Mutex::new(ConsentInner { record }),
            store,
            clock,
            status_sink,
            seq: AtomicU64::new(0),
        }
    }

    /// 시스템 clock 으로 게이트를 연다(MVP 기본 배선).
    pub fn open_with_system_clock(
        path: impl Into<std::path::PathBuf>,
        status_sink: Option<Arc<dyn StatusSink>>,
    ) -> Self {
        ConsentGate::open(ConsentStore::new(path), Arc::new(SystemClock), status_sink)
    }

    /// RISK-01 고지 확인 기록(R-CG-TRANSITION). 이미 확인된 상태에서는 멱등 성공.
    pub fn acknowledge(&self) -> Result<(), ConsentError> {
        let mut inner = self.lock_inner();
        match inner.record.state {
            ConsentLifecycle::NotAcknowledged => {
                let new_record = ConsentRecord {
                    acknowledged: true,
                    state: ConsentLifecycle::AcknowledgedNotGranted,
                    grant: None,
                };
                self.commit(&mut inner, new_record)
            }
            // 이미 확인됨 -> 멱등 성공(상태 불변, 지속 불필요).
            _ => Ok(()),
        }
    }

    /// 상시 동의 부여(R-CG-TRANSITION). `AcknowledgedNotGranted`/`Withdrawn` 에서만 신규 부여.
    pub fn grant(&self) -> Result<(), ConsentError> {
        let mut inner = self.lock_inner();
        match inner.record.state {
            ConsentLifecycle::NotAcknowledged => Err(ConsentError::NotAcknowledged),
            ConsentLifecycle::Granted => Err(ConsentError::AlreadyGranted),
            ConsentLifecycle::AcknowledgedNotGranted | ConsentLifecycle::Withdrawn => {
                let granted_at = self.clock.now();
                let grant = ConsentGrant {
                    grant_id: self.next_grant_id(granted_at),
                    granted_at,
                };
                let new_record = ConsentRecord {
                    acknowledged: true,
                    state: ConsentLifecycle::Granted,
                    grant: Some(grant),
                };
                self.commit(&mut inner, new_record)
            }
        }
    }

    /// 동의 철회(forward-only, R-CG-FORWARD-ONLY). `Granted` 에서만 성립.
    ///
    /// 이미 업로드된 콘텐츠를 회수하지 않으며(불가), 이후 업로드/커밋만 차단한다. 부여 참조는
    /// 보존하되 게이트는 차단한다.
    pub fn withdraw(&self) -> Result<(), ConsentError> {
        let mut inner = self.lock_inner();
        match inner.record.state {
            ConsentLifecycle::Granted => {
                let new_record = ConsentRecord {
                    acknowledged: true,
                    state: ConsentLifecycle::Withdrawn,
                    grant: inner.record.grant.clone(),
                };
                self.commit(&mut inner, new_record)
            }
            _ => Err(ConsentError::NoGrant),
        }
    }

    /// 부수효과 없이 현재 동의 상태를 조회한다(CLI `consent view`).
    pub fn view(&self) -> ConsentStatus {
        let inner = self.lock_inner();
        ConsentStatus {
            acknowledged: inner.record.acknowledged,
            grant: inner.record.grant.clone(),
            state: inner.record.state,
        }
    }

    /// 업로드 게이트 판정(R-CG-GATE). `Granted` 에서만 `Permitted`.
    ///
    /// 판정에 따라 주입된 `StatusSink` 로 `ConsentBlocked` 조건을 push 한다(차단 -> raise,
    /// 허용 -> clear; 게이트가 동의 상태의 권위 소스이므로 자연 소유, §5). push 는 idempotent.
    pub fn is_upload_permitted(&self) -> ConsentDecision {
        let inner = self.lock_inner();
        let decision = decide(inner.record.state);
        if let Some(sink) = &self.status_sink {
            match decision {
                ConsentDecision::Permitted => sink.clear_condition(ActiveCondition::ConsentBlocked),
                ConsentDecision::Blocked(_) => sink.raise_condition(ActiveCondition::ConsentBlocked),
            }
        }
        decision
    }

    /// 고지 미확인 시 업로드 거부 판정(R-CG-ACK-GATE).
    pub fn ensure_acknowledged(&self) -> Result<(), ConsentError> {
        let inner = self.lock_inner();
        if inner.record.acknowledged {
            Ok(())
        } else {
            Err(ConsentError::NotAcknowledged)
        }
    }

    /// RISK-01 고지 문안을 반환한다(R-CG-DISCLOSURE).
    pub fn disclosure_text(&self) -> &'static str {
        disclosure::disclosure_text()
    }

    /// `StatusSnapshot.consent` 에 노출할 U0 `ConsentState` projection(R-CG-PROJECT).
    pub fn consent_state(&self) -> ConsentState {
        let inner = self.lock_inner();
        project(inner.record.state)
    }

    /// 성공 전이만 원자적으로 지속하고 성공 시에만 인메모리 상태를 확정한다(all-or-nothing).
    ///
    /// 지속 실패 -> `PersistFailed` + 인메모리 롤백(미변경). 로드 시 손상 방어는 `sanitize` 가
    /// 담당하나, 여기서도 커밋 대상이 일관적이도록 방어적으로 `sanitize` 를 통과시킨다.
    fn commit(&self, inner: &mut ConsentInner, new_record: ConsentRecord) -> Result<(), ConsentError> {
        let new_record = sanitize(new_record);
        self.store
            .persist(&new_record)
            .map_err(|_| ConsentError::PersistFailed)?;
        inner.record = new_record;
        Ok(())
    }

    /// 신규 부여 식별자를 생성한다(MVP: 시각 나노초 + 프로세스 내 시퀀스). UUID 는 code-gen 이월.
    fn next_grant_id(&self, now: Timestamp) -> GrantId {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        GrantId(format!("grant-{}-{}", now.as_unix_nanos(), seq))
    }

    /// 뮤텍스를 취득한다. poison 시 패닉 대신 내부 상태를 회수한다(회복력, U0 관례).
    fn lock_inner(&self) -> MutexGuard<'_, ConsentInner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// config 리로드 관찰자 — MVP no-op(config-grant DEFER, DEC-U5-17).
impl ConfigReloadObserver for ConsentGate {
    fn on_config_reload(&self) {}
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]
    use super::*;
    use crate::testing::FixedClock;
    use std::sync::atomic::AtomicU64;

    fn temp_file() -> std::path::PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut path = std::env::temp_dir();
        path.push(format!("okc_consent_{}_{}.cbor", std::process::id(), n));
        // 이전 잔여물 제거.
        let _ = std::fs::remove_file(&path);
        path
    }

    fn gate(path: std::path::PathBuf) -> ConsentGate {
        ConsentGate::open(ConsentStore::new(path), Arc::new(FixedClock::new(12345)), None)
    }

    /// PROP-U5-05/06 예제: 전체 라이프사이클 전이 + 게이트 판정.
    #[test]
    fn full_lifecycle_flow() {
        let path = temp_file();
        let g = gate(path);

        // 초기: 미확인 -> 차단(NeedsAcknowledgment).
        assert_eq!(
            g.is_upload_permitted(),
            ConsentDecision::Blocked(BlockReason::NeedsAcknowledgment)
        );
        // grant 전 ack 필수.
        assert_eq!(g.grant(), Err(ConsentError::NotAcknowledged));

        // acknowledge -> AcknowledgedNotGranted(차단 NotGranted).
        g.acknowledge().unwrap();
        assert_eq!(
            g.is_upload_permitted(),
            ConsentDecision::Blocked(BlockReason::NotGranted)
        );
        // acknowledge 멱등.
        g.acknowledge().unwrap();

        // grant -> Granted(허용).
        g.grant().unwrap();
        assert_eq!(g.is_upload_permitted(), ConsentDecision::Permitted);
        // 이미 부여 -> AlreadyGranted.
        assert_eq!(g.grant(), Err(ConsentError::AlreadyGranted));

        // withdraw -> Withdrawn(차단, forward-only).
        g.withdraw().unwrap();
        assert_eq!(
            g.is_upload_permitted(),
            ConsentDecision::Blocked(BlockReason::Withdrawn)
        );
        // 재부여 -> Granted.
        g.grant().unwrap();
        assert_eq!(g.is_upload_permitted(), ConsentDecision::Permitted);
    }

    /// R-CG-PERSIST 예제: 재시작(reopen) 후 상태 보존.
    #[test]
    fn state_persists_across_reopen() {
        let path = temp_file();
        {
            let g = gate(path.clone());
            g.acknowledge().unwrap();
            g.grant().unwrap();
            assert_eq!(g.is_upload_permitted(), ConsentDecision::Permitted);
        }
        // 새 게이트가 동일 파일에서 로드.
        let g2 = gate(path);
        assert_eq!(g2.is_upload_permitted(), ConsentDecision::Permitted);
        assert!(g2.view().grant.is_some());
    }

    /// R-CG-PERSIST 예제: 손상 파일 로드 -> fail-safe 강등(차단 우선).
    #[test]
    fn corrupt_file_fail_safe() {
        let path = temp_file();
        std::fs::write(&path, b"not valid cbor at all").unwrap();
        let g = gate(path);
        assert!(matches!(g.is_upload_permitted(), ConsentDecision::Blocked(_)));
    }

    /// R-CG-ACK-GATE 예제.
    #[test]
    fn ensure_acknowledged_gate() {
        let path = temp_file();
        let g = gate(path);
        assert_eq!(g.ensure_acknowledged(), Err(ConsentError::NotAcknowledged));
        g.acknowledge().unwrap();
        assert_eq!(g.ensure_acknowledged(), Ok(()));
    }

    /// R-CG-PROJECT 예제: projection total.
    #[test]
    fn projection_examples() {
        let path = temp_file();
        let g = gate(path);
        assert_eq!(g.consent_state(), ConsentState::Unknown);
        g.acknowledge().unwrap();
        assert_eq!(g.consent_state(), ConsentState::Unknown);
        g.grant().unwrap();
        assert_eq!(g.consent_state(), ConsentState::Granted);
        g.withdraw().unwrap();
        assert_eq!(g.consent_state(), ConsentState::Blocked);
    }

    /// §5 예제: 차단/허용 시 ConsentBlocked 조건 push.
    #[test]
    fn status_sink_push_on_gate() {
        let path = temp_file();
        let sink = Arc::new(crate::testing::RecordingStatusSink::new());
        let g = ConsentGate::open(
            ConsentStore::new(path),
            Arc::new(FixedClock::new(1)),
            Some(sink.clone()),
        );
        // 차단 상태 -> raise.
        let _ = g.is_upload_permitted();
        assert!(sink.raised().contains(&ActiveCondition::ConsentBlocked));
        // 허용 상태 -> clear.
        g.acknowledge().unwrap();
        g.grant().unwrap();
        let _ = g.is_upload_permitted();
        assert!(sink.cleared().contains(&ActiveCondition::ConsentBlocked));
    }

    /// PROP-U5-09 예제: 고지 4대 필수 내용 포함.
    #[test]
    fn disclosure_contains_four_elements() {
        let text = disclosure_text();
        assert!(text.contains("연속성"));
        assert!(text.contains("forward-only"));
        assert!(text.contains("필터"));
        assert!(text.contains("평문"));
    }
}

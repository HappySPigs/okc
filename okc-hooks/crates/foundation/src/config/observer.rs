//! ObserverRegistry — 기동 시 고정되는 불변 순서 관찰자 컬렉션 + per-observer 패닉 격리.
//!
//! 관찰자는 U8 조립 시점(데몬 스레드 기동 전)에 1회 등록되어 이후 변경/해제되지 않는다
//! (`Vec<Arc<dyn ConfigReloadObserver>>`, 결정적 순서 + 레지스트리 락 불필요). 성공 스왑 직후
//! 등록 순서대로 `on_config_reload()` 를 fan-out 하되, 각 관찰자 호출을 `std::panic::catch_unwind`
//! 로 격리한다 — 한 관찰자가 패닉해도 나머지 K-1 개는 계속 통지되며 `current()` 는 손상되지 않는다
//! (스왑은 팬아웃 이전에 이미 원자적으로 커밋됨). R-OBSERVER-01/02/03, R-RELOAD-05, U0-NFR-REL-04.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use crate::core_types::ConfigReloadObserver;

/// 기동 시 고정되는 불변 순서 관찰자 레지스트리.
///
/// 등록은 조립 단계에서만 이루어지고(가변 참조 필요), 런타임 fan-out 은 불변 참조로 수행된다.
pub struct ObserverRegistry {
    observers: Vec<Arc<dyn ConfigReloadObserver>>,
}

impl ObserverRegistry {
    /// 빈 레지스트리를 생성한다.
    pub fn new() -> Self {
        ObserverRegistry {
            observers: Vec::new(),
        }
    }

    /// 관찰자를 등록 순서대로 추가한다(기동 전 1회 등록 계약).
    ///
    /// 저장 타입이 `Arc<dyn ConfigReloadObserver>` 이므로 소유권 있는 `Arc` 를 받는다
    /// (불변 순서 컬렉션에 보존하기 위함). U8 조립 루트가 U5/U6 구현체를 하향 주입한다.
    pub fn subscribe(&mut self, observer: Arc<dyn ConfigReloadObserver>) {
        self.observers.push(observer);
    }

    /// 등록 순서대로 모든 관찰자에게 `on_config_reload()` 를 fan-out 한다.
    ///
    /// 각 호출은 `catch_unwind` 로 격리되어, 패닉한 관찰자가 나머지 통지나 활성 스냅샷을
    /// 오염시키지 못한다(best-effort, infallible). 성공 스왑 이후에만 호출되어야 한다.
    pub fn notify_all(&self) {
        for observer in &self.observers {
            let observer = observer.clone();
            // `Arc<dyn Trait>` 는 UnwindSafe 가 아니므로 AssertUnwindSafe 로 감싼다.
            // 스왑은 이미 커밋되었고 관찰자 상태는 관찰자 측에 격리되므로 안전하다.
            let _ = catch_unwind(AssertUnwindSafe(move || observer.on_config_reload()));
        }
    }

    /// 등록된 관찰자 수를 반환한다.
    pub fn len(&self) -> usize {
        self.observers.len()
    }

    /// 등록된 관찰자가 없는지 반환한다.
    pub fn is_empty(&self) -> bool {
        self.observers.is_empty()
    }
}

impl Default for ObserverRegistry {
    fn default() -> Self {
        Self::new()
    }
}

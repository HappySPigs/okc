//! Clock seam — 방출 시각 stamping 의 now-source(D4).
//!
//! `StructuredLogger` 는 방출 시점에 `Clock::now()` 로 U0 `Timestamp` 를 stamping 한다. 프로덕션은
//! `SystemClock`(`std::time::SystemTime`), 테스트는 결정적 clock 을 주입해 타임스탬프 stamping 을
//! 검증할 수 있다. `chrono`/`time` 등 신규 시간 크레이트는 도입하지 않는다(U0 `Timestamp` 재사용).

use std::time::{SystemTime, UNIX_EPOCH};

use foundation::Timestamp;

/// 방출 시각 now-source 계약. 스레드 간 공유를 위해 `Send + Sync`.
pub trait Clock: Send + Sync {
    /// 현재 벽시계 시각을 U0 `Timestamp`(UTC epoch 나노초)로 반환한다.
    fn now(&self) -> Timestamp;
}

/// `std::time::SystemTime` 기반 프로덕션 clock.
///
/// epoch 이전 시각/오버플로 같은 비정상 입력에도 패닉하지 않고 포화(saturating)로 처리한다.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| i64::try_from(d.as_nanos()).unwrap_or(i64::MAX))
            .unwrap_or(0);
        Timestamp::from_unix_nanos(nanos)
    }
}

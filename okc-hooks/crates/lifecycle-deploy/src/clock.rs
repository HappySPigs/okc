//! `Clock` seam — bounded-time 헬스 게이트(D-U7A-05)의 now-source + sleep.
//!
//! `AutoUpdater` 는 게이트 폴링에서 `Clock::now()` 로 데드라인을 계산하고 `Clock::sleep()` 로
//! 폴링 간격을 대기한다. 프로덕션은 `SystemClock`(`std::time`), 테스트는 결정적 fake clock 을
//! 주입해 실제 대기 없이 타임아웃/통과를 검증한다. `chrono`/`time` 등 신규 시간 크레이트는
//! 도입하지 않는다(U0 `Timestamp` 재사용). U6 `observability::Clock` 은 `now()` 만 노출하므로
//! sleep 이 필요한 게이트 루프를 위해 U7a 는 자체 `Clock` seam 을 둔다(MVP 트림: 두 seam 미통합).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use foundation::Timestamp;

/// 게이트 시간 모델 now-source + sleep 계약. 스레드 간 공유를 위해 `Send + Sync`.
pub trait Clock: Send + Sync {
    /// 현재 벽시계 시각을 U0 `Timestamp`(UTC epoch 나노초)로 반환한다.
    fn now(&self) -> Timestamp;

    /// 주어진 기간만큼 대기한다(폴링 간격 `T_poll`).
    fn sleep(&self, dur: Duration);
}

/// `std::time` 기반 프로덕션 clock.
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

    fn sleep(&self, dur: Duration) {
        std::thread::sleep(dur);
    }
}

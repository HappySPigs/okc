//! RetryBackoffController — 순수·결정적 인메모리 재시도 상태머신(I/O·네트워크·전역 시계 없음).
//!
//! U0 `TransportError` 를 `RetryClass` 로 total 분류(ErrorClassifier)하고, 지수 백오프 +
//! full jitter + 상한(BackoffScheduler/JitterSource)과 연속 실패 에스컬레이션 게이트
//! (EscalationGate)를 계산한다. `on_failure(err, now: Instant)` 는 주입 시계 + 주입 시드로
//! 순수하게 계산되어 재현 가능하다(D-13, PROP-U4-05). status/notify push 는 U4 소관이 아니다.
//!
//! 순수 리프 모듈로서 panic-free-total 을 컴파일타임으로 강제한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::time::{Duration, Instant};

use foundation::{TransportError, TransportErrorClass};

/// U0 `TransportErrorClass` 를 U4 재시도 정책 관점으로 재분류한 5원 enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RetryClass {
    /// 서버측 일시 오류(5xx) — 백오프 후 재시도, 카운트 대상(N 도달 시 escalate).
    Transient,
    /// 타임아웃/연결오류 — 백오프 후 재시도, 오프라인 판정(에스컬레이션 제외).
    Offline,
    /// PROJECT_BUSY/queue-full/429 — 정상 지연, 백오프 후 재시도(에스컬레이션 제외).
    Backpressure,
    /// HTTP 401 — 재시도 안 함(U5 위임), 카운터 불변.
    AuthFailed,
    /// Fatal 계열 — 재시도 무의미. 전송 경로에서는 산출되지 않는 예약 변형.
    Permanent,
}

/// full jitter 적용 여부.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitterMode {
    /// `[0, base_delay]` 균등 난수(thundering-herd 완화, 기본).
    Full,
    /// 지터 없음(`retry_after == base_delay`).
    None,
}

/// 지수 백오프 스케줄 파라미터(U4 소유 federated config 키 `backoff` 투영).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BackoffConfig {
    /// 첫 재시도 기준 지연(`> 0`).
    pub initial_delay: Duration,
    /// 지수 승수(`>= 1.0`).
    pub multiplier: f64,
    /// 지연 상한(`>= initial_delay`).
    pub cap: Duration,
    /// full jitter 적용 여부.
    pub jitter: JitterMode,
}

impl Default for BackoffConfig {
    /// 권장 기본값: `initial_delay=1s`, `multiplier=2.0`, `cap=300s`, `jitter=full`(R-BACKOFF-02).
    fn default() -> Self {
        BackoffConfig {
            initial_delay: Duration::from_secs(1),
            multiplier: 2.0,
            cap: Duration::from_secs(300),
            jitter: JitterMode::Full,
        }
    }
}

/// 한 번의 실패에 대한 재시도 판단 결과(`on_failure` 반환).
///
/// 불변식: `AuthFailed`/`Permanent` -> `retry_after == None`; `is_offline => !escalate`;
/// `retry_after == Some(d) => 0 <= d <= cap`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryDecision {
    /// 재시도 지연. `None` 이면 재시도하지 않는다(AuthFailed/Permanent).
    pub retry_after: Option<Duration>,
    /// `true` 이면 `OperationalState::Offline` push 근거(US-E3-04).
    pub is_offline: bool,
    /// `true` 이면 FR-19 케이스 2 에스컬레이션 신호(실제 알림은 U6).
    pub escalate: bool,
}

/// 소형 결정적 PRNG(splitmix64) — full jitter 난수원. 암호학적 품질 불필요.
#[derive(Debug, Clone)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// 시드로부터 PRNG 를 구성한다(같은 시드·호출 시퀀스 -> 같은 난수열).
    fn new(seed: u64) -> Self {
        SplitMix64 { state: seed }
    }

    /// 다음 64비트 난수를 산출한다(splitmix64, wrapping 산술만 사용).
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// 순수·결정적 재시도 백오프 상태머신.
#[derive(Debug, Clone)]
pub struct RetryBackoffController {
    /// 백오프 스케줄 파라미터.
    cfg: BackoffConfig,
    /// 에스컬레이션 임계 `N`(U0 `notify_consecutive_failures` 주입, 단일 출처).
    escalation_threshold: u32,
    /// 성공 이후 누적된 재시도 대상 실패 횟수(백오프 지수).
    attempt: u32,
    /// 연속 실패 카운트(재시도 대상 실패마다 증가, 성공 시 0).
    consecutive_failures: u32,
    /// 다음 재시도 예정 시각(status 관측용).
    next_retry_at: Option<Instant>,
    /// full jitter 난수원.
    rng: SplitMix64,
}

impl RetryBackoffController {
    /// 컨트롤러를 구성한다 — `cfg` 스케줄, `n` 에스컬레이션 임계, `seed` jitter 시드 주입.
    pub fn new(cfg: &BackoffConfig, n: u32, seed: u64) -> Self {
        RetryBackoffController {
            cfg: *cfg,
            escalation_threshold: n,
            attempt: 0,
            consecutive_failures: 0,
            next_retry_at: None,
            rng: SplitMix64::new(seed),
        }
    }

    /// U0 `TransportError` 를 `RetryClass` 로 total 분류한다(R-RETRY-01, 5 변형 소진).
    ///
    /// `AuthFailed -> AuthFailed`, `ServerError -> Transient`, `Backpressure -> Backpressure`,
    /// `Timeout -> Offline`, `Network -> Offline`. 전송 경로에서 `Permanent` 는 산출하지 않는다.
    pub fn classify(err: &TransportError) -> RetryClass {
        match err.class {
            TransportErrorClass::AuthFailed => RetryClass::AuthFailed,
            TransportErrorClass::ServerError => RetryClass::Transient,
            TransportErrorClass::Backpressure => RetryClass::Backpressure,
            TransportErrorClass::Timeout => RetryClass::Offline,
            TransportErrorClass::Network => RetryClass::Offline,
        }
    }

    /// 실패 1건을 처리하고 재시도 판단을 반환한다(주입 `now` 기준, 순수·결정적).
    ///
    /// 재시도 대상(`Transient`/`Offline`/`Backpressure`)은 `attempt`·`consecutive_failures` 를
    /// 증가시키고 백오프 지연을 산출한다. `AuthFailed`/`Permanent` 는 카운터를 불변으로 두고
    /// `retry_after = None` 을 반환한다(R-AUTH-01).
    pub fn on_failure(&mut self, err: &TransportError, now: Instant) -> RetryDecision {
        match Self::classify(err) {
            RetryClass::AuthFailed | RetryClass::Permanent => RetryDecision {
                retry_after: None,
                is_offline: false,
                escalate: false,
            },
            RetryClass::Transient => {
                let retry_after = self.advance_backoff(now);
                let escalate = self.consecutive_failures >= self.escalation_threshold;
                RetryDecision {
                    retry_after: Some(retry_after),
                    is_offline: false,
                    escalate,
                }
            }
            RetryClass::Backpressure => {
                let retry_after = self.advance_backoff(now);
                RetryDecision {
                    retry_after: Some(retry_after),
                    is_offline: false,
                    escalate: false,
                }
            }
            RetryClass::Offline => {
                let retry_after = self.advance_backoff(now);
                RetryDecision {
                    retry_after: Some(retry_after),
                    is_offline: true,
                    escalate: false,
                }
            }
        }
    }

    /// 성공 시 백오프·카운터를 모두 리셋한다(R-BACKOFF-03, R-ESCAL-01).
    pub fn on_success(&mut self) {
        self.attempt = 0;
        self.consecutive_failures = 0;
        self.next_retry_at = None;
    }

    /// 다음 재시도 예정 시각(없으면 `None`).
    pub fn next_retry_at(&self) -> Option<Instant> {
        self.next_retry_at
    }

    /// 현재 연속 실패 카운트(status 관측용).
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    /// 재시도 대상 실패의 공통 전이 — 카운터 증가 + 백오프 지연 산출 + `next_retry_at` 저장.
    fn advance_backoff(&mut self, now: Instant) -> Duration {
        let base = self.base_delay(self.attempt);
        let retry_after = self.apply_jitter(base);
        self.attempt = self.attempt.saturating_add(1);
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        self.next_retry_at = Some(now + retry_after);
        retry_after
    }

    /// `base_delay(attempt) = min(cap, initial * multiplier^attempt)`(`cap` 까지 비감소).
    fn base_delay(&self, attempt: u32) -> Duration {
        let exp = i32::try_from(attempt).unwrap_or(i32::MAX);
        let initial = self.cfg.initial_delay.as_secs_f64();
        let scaled = initial * self.cfg.multiplier.powi(exp);
        let cap = self.cfg.cap.as_secs_f64();
        // NaN/무한/음수·상한 초과는 모두 `cap` 으로 절단한다(비감소·유계 불변식 보장).
        let capped = if scaled.is_finite() && scaled >= 0.0 && scaled < cap {
            scaled
        } else {
            cap
        };
        Duration::try_from_secs_f64(capped).unwrap_or(self.cfg.cap)
    }

    /// full jitter 를 적용한다 — `[0, base]` 균등 난수(`None` 모드는 `base` 그대로).
    fn apply_jitter(&mut self, base: Duration) -> Duration {
        match self.cfg.jitter {
            JitterMode::None => base,
            JitterMode::Full => {
                let base_nanos = u64::try_from(base.as_nanos()).unwrap_or(u64::MAX);
                let modulus = base_nanos.saturating_add(1);
                let r = self.rng.next_u64() % modulus;
                Duration::from_nanos(r)
            }
        }
    }
}

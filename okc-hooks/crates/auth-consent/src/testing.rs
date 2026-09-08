//! 테스트 더블(mock/stub) — `[blocked-on-server]` 목 계약 + 결정적 우선순위/게이트 테스트용.
//!
//! 비-default `proptest-support` feature 또는 `test` 에서만 빌드되어 프로덕션 그래프에 유입되지
//! 않는다. `MockHttpTransport` 는 전송된 `RawHttpRequest` 를 캡처해 TLS/토큰/타임아웃 불변식
//! (PROP-U5-03)을 서버 없이 검증한다.

use std::sync::{Arc, Mutex};

use foundation::{
    ActiveCondition, ByteCount, ConfigSnapshot, LivenessSignal, LogLevel, OperationalState,
    StatusSink, Timestamp, TokenSecret, WatcherConfig,
};

use crate::config_source::ConfigSource;
use crate::consent::Clock;
use crate::credential::{EnvReader, SecureStore, SecureStoreError};
use crate::transport::{HttpError, HttpTransport, RawHttpRequest, RawHttpResponse};

/// 전송된 요청을 캡처하고 미리 설정한 결과를 돌려주는 목 `HttpTransport`.
pub struct MockHttpTransport {
    result: Result<RawHttpResponse, HttpError>,
    captured: Mutex<Vec<RawHttpRequest>>,
}

impl MockHttpTransport {
    /// 미리 설정한 결과(성공 응답 또는 전송 실패)로 목을 만든다.
    pub fn new(result: Result<RawHttpResponse, HttpError>) -> Self {
        MockHttpTransport {
            result,
            captured: Mutex::new(Vec::new()),
        }
    }

    /// 지정 상태코드의 빈 응답을 돌려주는 목.
    pub fn with_status(status: u16) -> Self {
        MockHttpTransport::new(Ok(RawHttpResponse {
            status,
            headers: crate::transport::Headers::new(),
            body: crate::transport::Body::empty(),
        }))
    }

    /// 지정 전송 실패를 돌려주는 목.
    pub fn with_error(error: HttpError) -> Self {
        MockHttpTransport::new(Err(error))
    }

    /// 지금까지 캡처된 요청 목록의 복제본.
    pub fn captured(&self) -> Vec<RawHttpRequest> {
        self.lock().clone()
    }

    /// 마지막으로 캡처된 요청(있으면).
    pub fn last(&self) -> Option<RawHttpRequest> {
        self.lock().last().cloned()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<RawHttpRequest>> {
        self.captured
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl HttpTransport for MockHttpTransport {
    fn execute(&self, req: RawHttpRequest) -> Result<RawHttpResponse, HttpError> {
        self.lock().push(req);
        self.result.clone()
    }
}

/// `WatcherConfig` 를 고정 보유하는 `ConfigSource` stub(파일 로드 없이 결정적 스냅샷 주입).
pub struct StaticConfig {
    config: Arc<WatcherConfig>,
}

impl StaticConfig {
    /// 지정 `WatcherConfig` 로 stub 을 만든다.
    pub fn new(config: WatcherConfig) -> Self {
        StaticConfig {
            config: Arc::new(config),
        }
    }

    /// server_endpoint + 선택 토큰/secure-store 플래그로 stub 을 만든다(테스트 편의).
    pub fn build(
        server_endpoint: &str,
        token: Option<TokenSecret>,
        secure_store_enabled: bool,
    ) -> Self {
        StaticConfig::new(WatcherConfig {
            vault_path: "/tmp/vault".to_string(),
            server_endpoint: server_endpoint.to_string(),
            token,
            secure_store_enabled,
            log_level: LogLevel::Info,
            notify_consecutive_failures: 3,
        })
    }
}

impl ConfigSource for StaticConfig {
    fn current(&self) -> ConfigSnapshot {
        ConfigSnapshot::new(self.config.clone())
    }
}

/// 고정 env 매핑을 돌려주는 `EnvReader` stub.
pub struct StaticEnv {
    value: Option<String>,
}

impl StaticEnv {
    /// 지정 키 값(있으면)을 돌려주는 stub. `None` 이면 미설정.
    pub fn new(value: Option<String>) -> Self {
        StaticEnv { value }
    }
}

impl EnvReader for StaticEnv {
    fn read(&self, _key: &str) -> Option<String> {
        self.value.clone()
    }
}

/// 고정 결과를 돌려주는 `SecureStore` stub.
pub struct StaticSecureStore {
    result: Result<Option<TokenSecret>, SecureStoreError>,
}

impl StaticSecureStore {
    /// 지정 결과로 stub 을 만든다.
    pub fn new(result: Result<Option<TokenSecret>, SecureStoreError>) -> Self {
        StaticSecureStore { result }
    }
}

impl SecureStore for StaticSecureStore {
    fn read_token(&self) -> Result<Option<TokenSecret>, SecureStoreError> {
        self.result.clone()
    }
}

/// 고정 시각을 돌려주는 `Clock` stub.
pub struct FixedClock {
    nanos: i64,
}

impl FixedClock {
    /// 지정 나노초 시각으로 stub 을 만든다.
    pub fn new(nanos: i64) -> Self {
        FixedClock { nanos }
    }
}

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_unix_nanos(self.nanos)
    }
}

/// `ConsentBlocked` raise/clear push 를 기록하는 `StatusSink` stub(관측 검증용).
#[derive(Default)]
pub struct RecordingStatusSink {
    raised: Mutex<Vec<ActiveCondition>>,
    cleared: Mutex<Vec<ActiveCondition>>,
}

impl RecordingStatusSink {
    /// 새 기록 sink 를 만든다.
    pub fn new() -> Self {
        RecordingStatusSink::default()
    }

    /// raise 된 조건 목록의 복제본.
    pub fn raised(&self) -> Vec<ActiveCondition> {
        self.raised
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    /// clear 된 조건 목록의 복제본.
    pub fn cleared(&self) -> Vec<ActiveCondition> {
        self.cleared
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }
}

impl StatusSink for RecordingStatusSink {
    fn set_operational(&self, _state: OperationalState) {}
    fn raise_condition(&self, cond: ActiveCondition) {
        self.raised.lock().unwrap_or_else(|p| p.into_inner()).push(cond);
    }
    fn clear_condition(&self, cond: ActiveCondition) {
        self.cleared
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(cond);
    }
    fn record_sync_success(&self, _at: Timestamp) {}
    fn set_dirty(&self, _dirty: bool) {}
    fn set_resume_progress(&self, _transferred: ByteCount, _total: ByteCount) {}
    fn set_liveness(&self, _signal: LivenessSignal) {}
}

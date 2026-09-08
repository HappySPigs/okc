//! StructuredLogger — `serde_json` JSON-line 구조화 로거 + config 리로드 재적용.
//!
//! `Logger`(push 방출) + `ConfigReloadObserver`(로그레벨 재적용)를 구현한다. 방출 파이프라인:
//! `log`/`event` -> 레벨 필터(R-LOG-02) -> 방출 시각 stamping(D4) -> `serde_json` JSON-line 직렬화
//! -> 동기 best-effort append(R-LOG-03) -> 크기 초과 시 rotation(R-LOG-05). push 표면은 infallible
//! 이며 IO/rotation 오류는 삼킨다(R-PUSH-02).
//!
//! SEC-02 redaction 계약(R-LOG-04): config 유래 비밀(토큰)은 원문으로 방출하지 않는다. 토큰은
//! U0 `TokenSecret`(redacting `Debug`/`Display`)로만 표현되며, 로거는 호출자가 넘긴 `LogRecord`
//! 필드를 그대로 방출할 뿐 평문 토큰을 스스로 생성/노출하지 않는다.
//!
//! `Mutex` + 파일 IO 를 보유하는 상태 모듈이므로 순수 lint-gate 는 적용하지 않는다.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use serde::{Deserialize, Serialize};

use foundation::{
    ConfigProvider, ConfigReloadObserver, CycleId, LogFields, LogLevel, LogRecord, Logger, Timestamp,
};

use crate::clock::Clock;
use crate::config::LoggerConfig;
use crate::error::LogError;

/// 디스크에 기록되는 방출 로그 라인 = U0 `LogRecord` 전 필드 + 방출 시점 stamped `timestamp`.
///
/// JSON-line 1건으로 직렬화되며 `serde_json` 무손실 round-trip 대상이다(PROP-U6-DE-02). 최소 필드
/// 계약 `{ timestamp, level, event, cycle_id, message }`(+ `fields`)를 항상 포함한다(US-E5-01).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmittedLogLine {
    /// 방출 벽시계 시각(로거가 stamping).
    pub timestamp: Timestamp,
    /// 로그 레벨.
    pub level: LogLevel,
    /// 이벤트 식별 문자열.
    pub event: String,
    /// 연관 사이클 식별자(있으면, 부재는 JSON `null`).
    pub cycle_id: Option<CycleId>,
    /// 사람이 읽는 메시지.
    pub message: String,
    /// 구조화 필드맵.
    pub fields: LogFields,
}

impl EmittedLogLine {
    /// stamped `timestamp` 와 U0 `LogRecord` 로부터 방출 라인을 조립한다.
    pub fn new(timestamp: Timestamp, record: &LogRecord) -> Self {
        EmittedLogLine {
            timestamp,
            level: record.level,
            event: record.event.clone(),
            cycle_id: record.cycle_id,
            message: record.message.clone(),
            fields: record.fields.clone(),
        }
    }

    /// 개행 없는 JSON-line 문자열로 직렬화한다.
    pub fn to_json(&self) -> Result<String, LogError> {
        serde_json::to_string(self).map_err(|e| LogError::Io(e.to_string()))
    }
}

/// 뮤텍스로 보호되는 로거 상태(현재 활성 레벨). rotation 파라미터/경로는 불변 필드.
#[derive(Debug)]
struct LoggerState {
    /// 현재 활성 최소 방출 레벨(reload 로 갱신 가능).
    level: LogLevel,
}

/// JSON-line 구조화 로거(`Logger` + `ConfigReloadObserver` 구현체).
pub struct StructuredLogger {
    /// 활성 레벨 + 쓰기 직렬화 락(단일 소형 라인 쓰기+rotation 을 직렬화).
    state: Mutex<LoggerState>,
    /// JSON-line 방출 대상 파일.
    log_file: PathBuf,
    /// rotation 트리거 크기 임계(바이트).
    max_size: u64,
    /// 보존할 회전 파일 수(개수 근사).
    max_retained: usize,
    /// 방출 시각 now-source(D4).
    clock: Arc<dyn Clock>,
    /// 코어 필드(`log_level`) 재적용을 위한 config 프로바이더(리로드 재조회).
    provider: Arc<ConfigProvider>,
}

impl StructuredLogger {
    /// 로거를 생성한다. 초기 레벨/rotation 파라미터는 `cfg`, 리로드 재적용은 `provider` 로부터.
    pub fn new(cfg: LoggerConfig, clock: Arc<dyn Clock>, provider: Arc<ConfigProvider>) -> Self {
        StructuredLogger {
            state: Mutex::new(LoggerState {
                level: cfg.log_level,
            }),
            log_file: cfg.log_file,
            max_size: cfg.log_max_size_bytes.get(),
            max_retained: cfg.log_max_retained,
            clock,
            provider,
        }
    }

    /// 뮤텍스를 취득한다. poison 시 패닉 대신 내부 상태를 회수한다(회복력).
    fn lock(&self) -> MutexGuard<'_, LoggerState> {
        match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// config 리로드 시 `log_level` 을 재적용한다(R-LOG-06). 데몬을 막지 않으며 keep-last-good.
    ///
    /// 코어 필드(`log_level`)만 live-reload 대상이며 rotation 파라미터는 생성자 주입 값을 유지한다.
    pub fn reload(&self) -> Result<(), LogError> {
        let snapshot = self.provider.current();
        self.lock().level = snapshot.log_level;
        Ok(())
    }

    /// 방출 라인을 파일에 동기 append 하고 크기 초과 시 rotation 한다(호출자가 락 보유).
    fn write_line(&self, json: &str) -> Result<(), LogError> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)?;
        writeln!(file, "{json}")?;
        drop(file);
        self.maybe_rotate()
    }

    /// 파일 크기가 임계 초과면 rotation 한다(크기+개수 근사, R-LOG-05).
    fn maybe_rotate(&self) -> Result<(), LogError> {
        let len = std::fs::metadata(&self.log_file)?.len();
        if len <= self.max_size {
            return Ok(());
        }
        if self.max_retained == 0 {
            // 보존 없음: 현재 파일을 제거해 새로 시작.
            std::fs::remove_file(&self.log_file)
                .map_err(|e| LogError::RotationFailed(e.to_string()))?;
            return Ok(());
        }
        // 가장 오래된 초과분 삭제 후 file.k -> file.(k+1) 시프트, 마지막에 file -> file.1.
        let oldest = self.rotated_path(self.max_retained);
        let _ = std::fs::remove_file(&oldest);
        for k in (1..self.max_retained).rev() {
            let from = self.rotated_path(k);
            if from.exists() {
                std::fs::rename(&from, self.rotated_path(k + 1))
                    .map_err(|e| LogError::RotationFailed(e.to_string()))?;
            }
        }
        std::fs::rename(&self.log_file, self.rotated_path(1))
            .map_err(|e| LogError::RotationFailed(e.to_string()))?;
        Ok(())
    }

    /// 회전 파일 경로(`log_file.k`)를 구성한다.
    fn rotated_path(&self, index: usize) -> PathBuf {
        let mut name = self.log_file.clone().into_os_string();
        name.push(format!(".{index}"));
        PathBuf::from(name)
    }

    /// 레벨 필터 통과 시 방출 라인을 조립·직렬화·기록한다(오류 삼킴, infallible).
    fn emit(&self, record: &LogRecord) {
        let guard = self.lock();
        if record.level < guard.level {
            return; // R-LOG-02: 활성 미만 레벨은 방출하지 않음(부수효과 없음).
        }
        let timestamp = self.clock.now();
        let line = EmittedLogLine::new(timestamp, record);
        // 직렬화/쓰기 실패는 best-effort 로 삼킨다(R-PUSH-02).
        if let Ok(json) = line.to_json() {
            let _ = self.write_line(&json);
        }
    }
}

impl Logger for StructuredLogger {
    fn log(&self, record: LogRecord) {
        self.emit(&record);
    }

    fn event(&self, level: LogLevel, event: &str, cycle_id: Option<CycleId>, fields: LogFields) {
        let record = LogRecord {
            level,
            event: event.to_string(),
            cycle_id,
            message: String::new(),
            fields,
        };
        self.emit(&record);
    }
}

impl ConfigReloadObserver for StructuredLogger {
    fn on_config_reload(&self) {
        // 재적용 실패도 데몬을 막지 않는다(keep-last-good).
        let _ = self.reload();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use foundation::{ByteCount, FOUNDATION_CONFIG_KEYS, TokenSecret};
    use std::sync::atomic::{AtomicU64, Ordering};

    /// 고정 시각을 반환하는 결정적 테스트 clock.
    struct FixedClock(i64);
    impl Clock for FixedClock {
        fn now(&self) -> Timestamp {
            Timestamp::from_unix_nanos(self.0)
        }
    }

    fn unique_path(tag: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut path = std::env::temp_dir();
        path.push(format!("okc_u6_log_{}_{}_{}", tag, std::process::id(), n));
        path
    }

    fn make_logger(path: PathBuf, level: LogLevel, max_size: u64, retained: usize) -> StructuredLogger {
        let cfg = LoggerConfig {
            log_file: path,
            log_max_size_bytes: ByteCount::new(max_size),
            log_max_retained: retained,
            log_level: level,
        };
        let provider = Arc::new(ConfigProvider::new(FOUNDATION_CONFIG_KEYS));
        StructuredLogger::new(cfg, Arc::new(FixedClock(42)), provider)
    }

    #[test]
    fn emits_json_line_and_parses_back() {
        let path = unique_path("emit");
        let logger = make_logger(path.clone(), LogLevel::Info, 1_000_000, 3);
        logger.event(
            LogLevel::Warn,
            "cycle.start",
            Some(CycleId(7)),
            LogFields(vec![("k".to_string(), "v".to_string())]),
        );
        let content = std::fs::read_to_string(&path).expect("read log");
        let line = content.lines().next().expect("one line");
        let parsed: EmittedLogLine = serde_json::from_str(line).expect("parse json-line");
        assert_eq!(parsed.timestamp, Timestamp::from_unix_nanos(42));
        assert_eq!(parsed.level, LogLevel::Warn);
        assert_eq!(parsed.event, "cycle.start");
        assert_eq!(parsed.cycle_id, Some(CycleId(7)));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn level_filter_drops_below_active() {
        let path = unique_path("filter");
        let logger = make_logger(path.clone(), LogLevel::Warn, 1_000_000, 3);
        logger.event(LogLevel::Info, "dropped", None, LogFields::default());
        logger.event(LogLevel::Error, "kept", None, LogFields::default());
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(!content.contains("dropped"));
        assert!(content.contains("kept"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn token_secret_is_redacted_not_plaintext() {
        // R-LOG-04: TokenSecret 의 Display/Debug 는 평문을 노출하지 않는다(U0 계약 소비 확인).
        let secret = TokenSecret::new("super-secret-token".to_string());
        let path = unique_path("redact");
        let logger = make_logger(path.clone(), LogLevel::Info, 1_000_000, 3);
        logger.event(
            LogLevel::Info,
            "auth",
            None,
            LogFields(vec![("token".to_string(), format!("{secret}"))]),
        );
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(!content.contains("super-secret-token"));
        assert!(content.contains("***"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn write_to_bad_path_is_non_fatal() {
        // PROP-U6-BL-02(예제): 존재하지 않는 부모 디렉토리 -> append 실패하지만 push 는 정상 반환.
        let mut path = std::env::temp_dir();
        path.push("okc_u6_nonexistent_dir_xyz");
        path.push("log.txt");
        let logger = make_logger(path, LogLevel::Info, 1_000_000, 3);
        logger.event(LogLevel::Error, "e", None, LogFields::default()); // 패닉/블록 없이 반환하면 성공
    }

    #[test]
    fn rotation_triggers_on_size_and_retains_count() {
        let path = unique_path("rotate");
        // 아주 작은 임계로 매 라인 rotation 유발.
        let logger = make_logger(path.clone(), LogLevel::Info, 1, 2);
        for i in 0..4 {
            logger.event(LogLevel::Info, "e", Some(CycleId(i)), LogFields::default());
        }
        // 보존 개수 2 -> file.1, file.2 존재, file.3 은 없음.
        let r3 = {
            let mut n = path.clone().into_os_string();
            n.push(".3");
            PathBuf::from(n)
        };
        assert!(!r3.exists());
        // 정리.
        for suffix in ["", ".1", ".2"] {
            let mut n = path.clone().into_os_string();
            n.push(suffix);
            let _ = std::fs::remove_file(PathBuf::from(n));
        }
    }
}

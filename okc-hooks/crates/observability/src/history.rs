//! UploadHistoryStore — append-only 무손실 CBOR 프레임 히스토리 + `query` 필터.
//!
//! `HistorySink`(append push)를 구현한다. 지속 포맷 = **고정폭 length prefix(`u64` LE) + U0
//! CBOR payload** 프레임(R-HIST-02/NFR-13). `append` 는 infallible — IO/코덱 오류를 삼키고
//! (있으면) 로거로 남긴다(R-HIST-03). `query` 만 `Result<_, HistoryError>` 를 반환한다.
//!
//! truncated-tail 관용(R-HIST-04): `query` 스캔 중 트레일링 프레임이 부분(헤더/페이로드 잘림)이면
//! append 도중 크래시로 간주해 정상 prefix 를 성공 반환하고, 파일 **중간** 프레임 손상만
//! `HistoryError::Corrupt` 로 표면화한다.
//!
//! `Mutex` + 파일 IO 를 보유하는 상태 모듈이므로 순수 lint-gate 는 적용하지 않는다.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use foundation::{
    HistorySink, LogFields, LogLevel, Logger, Timestamp, UploadHistoryRecord, decode, encode,
};

use crate::error::HistoryError;

/// length prefix 폭(바이트).
const LEN_PREFIX: usize = 8;

/// CLI `watcher history` 필터. 두 필터는 AND 결합이며 `None` 축은 무제약이다(R-HIST-05).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HistoryQuery {
    /// `record.timestamp >= since` 만 통과(`None` = 무제약).
    pub since: Option<Timestamp>,
    /// `Some(true)` = 실패만(`error_detail.is_some()`), `Some(false)` = 성공만, `None` = 무제약.
    pub only_failures: Option<bool>,
}

impl HistoryQuery {
    /// 필터 없는(전 레코드 통과) 질의를 생성한다.
    pub fn all() -> Self {
        HistoryQuery::default()
    }

    /// 레코드가 두 필터(AND)를 모두 만족하는지 판정한다(성공/실패는 `error_detail` 유무로 파생).
    pub fn matches(&self, record: &UploadHistoryRecord) -> bool {
        if let Some(since) = self.since
            && record.timestamp < since {
                return false;
            }
        if let Some(only_failures) = self.only_failures {
            let is_failure = record.error_detail.is_some();
            if only_failures != is_failure {
                return false;
            }
        }
        true
    }
}

/// append-only 히스토리 지속 저장소(`HistorySink` 구현체).
pub struct UploadHistoryStore {
    /// append-only 히스토리 파일.
    path: PathBuf,
    /// 동시 append 를 직렬화하는 쓰기 락.
    write_lock: Mutex<()>,
    /// append 실패를 남길 선택적 로거(R-HIST-03; 없으면 조용히 삼킴).
    logger: Option<Arc<dyn Logger>>,
}

impl UploadHistoryStore {
    /// 히스토리 저장소를 생성한다. `logger` 는 append 실패 로깅용(없으면 `None`).
    pub fn new(path: PathBuf, logger: Option<Arc<dyn Logger>>) -> Self {
        UploadHistoryStore {
            path,
            write_lock: Mutex::new(()),
            logger,
        }
    }

    /// 레코드 1건을 length-prefixed CBOR 프레임으로 파일에 append 한다(락 직렬화).
    fn try_append(&self, record: &UploadHistoryRecord) -> Result<(), HistoryError> {
        let payload = encode(record)?;
        let len = payload.len() as u64;
        let _guard = match self.write_lock.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let mut file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        file.write_all(&len.to_le_bytes())?;
        file.write_all(&payload)?;
        Ok(())
    }

    /// 필터를 적용해 히스토리 레코드를 조회한다(읽기 경로, `Result` 반환).
    ///
    /// 파일 부재는 빈 결과로 취급한다. 트레일링 부분 프레임은 관용하고(정상 prefix 반환) 중간
    /// 프레임 손상만 `HistoryError::Corrupt` 로 반환한다(R-HIST-04). 결과는 append 순서를 보존한다.
    pub fn query(&self, filter: HistoryQuery) -> Result<Vec<UploadHistoryRecord>, HistoryError> {
        let bytes = match std::fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(HistoryError::Io(err.to_string())),
        };
        let records = scan_frames(&bytes)?;
        Ok(records
            .into_iter()
            .filter(|record| filter.matches(record))
            .collect())
    }
}

/// 프레임 바이트열을 순차 디코드한다(truncated-tail 관용, 중간 손상은 `Corrupt`).
fn scan_frames(bytes: &[u8]) -> Result<Vec<UploadHistoryRecord>, HistoryError> {
    let mut out: Vec<UploadHistoryRecord> = Vec::new();
    let total = bytes.len();
    let mut pos: usize = 0;
    while pos < total {
        // 길이 헤더가 부분이면 트레일링 truncated-tail -> 정상 prefix 반환.
        let header_end = match pos.checked_add(LEN_PREFIX) {
            Some(end) if end <= total => end,
            _ => break,
        };
        let len_bytes: [u8; LEN_PREFIX] = match bytes[pos..header_end].try_into() {
            Ok(arr) => arr,
            Err(_) => break,
        };
        let len = u64::from_le_bytes(len_bytes) as usize;
        // 페이로드가 부분이면 트레일링 truncated-tail -> 정상 prefix 반환.
        let payload_end = match header_end.checked_add(len) {
            Some(end) if end <= total => end,
            _ => break,
        };
        let payload = &bytes[header_end..payload_end];
        match decode::<UploadHistoryRecord>(payload) {
            Ok(record) => {
                out.push(record);
                pos = payload_end;
            }
            // 완전히 존재하는 프레임의 디코드 실패 = 파일 중간 손상.
            Err(err) => return Err(HistoryError::Corrupt(err.to_string())),
        }
    }
    Ok(out)
}

impl HistorySink for UploadHistoryStore {
    fn append(&self, record: UploadHistoryRecord) {
        if let Err(err) = self.try_append(&record)
            && let Some(logger) = &self.logger {
                logger.event(
                    LogLevel::Error,
                    "history.append.failed",
                    None,
                    LogFields(vec![("error".to_string(), err.to_string())]),
                );
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use foundation::ByteCount;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn unique_path() -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut path = std::env::temp_dir();
        path.push(format!("okc_u6_hist_{}_{}", std::process::id(), n));
        path
    }

    fn rec(ts: i64, bytes: u64, err: Option<&str>) -> UploadHistoryRecord {
        UploadHistoryRecord {
            timestamp: Timestamp::from_unix_nanos(ts),
            bytes_transferred: ByteCount::new(bytes),
            error_detail: err.map(|s| s.to_string()),
        }
    }

    #[test]
    fn append_only_preserves_order_lossless() {
        let path = unique_path();
        let store = UploadHistoryStore::new(path.clone(), None);
        let a = rec(1, 10, None);
        let b = rec(2, 20, Some("boom"));
        let c = rec(3, 0, None);
        store.append(a.clone());
        store.append(b.clone());
        store.append(c.clone());
        let all = store.query(HistoryQuery::all()).expect("query");
        assert_eq!(all, vec![a, b, c]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn query_only_failures_and_since_filter() {
        let path = unique_path();
        let store = UploadHistoryStore::new(path.clone(), None);
        store.append(rec(1, 10, None));
        store.append(rec(2, 20, Some("e")));
        store.append(rec(3, 30, None));
        let failures = store
            .query(HistoryQuery {
                since: None,
                only_failures: Some(true),
            })
            .expect("query");
        assert_eq!(failures.len(), 1);
        assert!(failures[0].error_detail.is_some());
        let since = store
            .query(HistoryQuery {
                since: Some(Timestamp::from_unix_nanos(3)),
                only_failures: None,
            })
            .expect("query");
        assert_eq!(since.len(), 1);
        assert_eq!(since[0].timestamp, Timestamp::from_unix_nanos(3));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn truncated_tail_returns_prefix() {
        let path = unique_path();
        let store = UploadHistoryStore::new(path.clone(), None);
        store.append(rec(1, 10, None));
        store.append(rec(2, 20, None));
        // 마지막 몇 바이트를 잘라 truncated tail 을 만든다.
        let raw = std::fs::read(&path).expect("read");
        let cut = raw.len() - 3;
        std::fs::write(&path, &raw[..cut]).expect("truncate");
        let out = store.query(HistoryQuery::all()).expect("query tolerant");
        assert_eq!(out, vec![rec(1, 10, None)]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_file_is_empty() {
        let store = UploadHistoryStore::new(unique_path(), None);
        assert!(store.query(HistoryQuery::all()).expect("query").is_empty());
    }
}

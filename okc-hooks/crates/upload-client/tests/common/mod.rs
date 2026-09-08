//! 공유 테스트 더블(fake seam) + 헬퍼 — 예제/property 테스트가 네트워크/파일시스템/권한 없이
//! `UploadProtocolDriver` 를 구동하도록 지원한다.
//!
//! - `ScriptedHttpTransport`: `AuthTransport` 하위 `HttpTransport` mock seam. URL 경로별로 다른
//!   응답(negotiate/commit/blob)을 돌려주고 전송 요청을 캡처한다([blocked-on-server] mock 계약).
//! - `FakeBlobSource`/`FakeStore`/`RecordingStatusSink`: blob 재-읽기·재개 오프셋·상태 push seam.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{Cursor, Read};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use auth_consent::{
    AuthTransport, Body, ConfigSource, CredentialProvider, Headers, HttpError, HttpTransport,
    RawHttpRequest, RawHttpResponse,
};
use content_core::ContentAddressing;
use foundation::{
    ActiveCondition, ByteCount, ConfigSnapshot, LivenessSignal, LogLevel, Manifest, ManifestEntry,
    OperationalState, RelativePath, Sha256Digest, StatusSink, Timestamp, TokenSecret, WatcherConfig,
    encode,
};
use upload_client::{
    BlobSource, BlobSourceError, CommitOutcome, NegotiateResponse, SyncStore, UploadError,
    VaultContentId,
};

// ---------------------------------------------------------------------------
// ScriptedHttpTransport — 경로별 응답 + 요청 캡처(mock HttpTransport seam)
// ---------------------------------------------------------------------------

/// 스크립트된 전송 실패 지점(없으면 전부 2xx).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailAt {
    /// negotiate 응답에서 전송 실패.
    Negotiate,
    /// blob 청크 전송에서 전송 실패.
    Blob,
    /// commit 응답에서 전송 실패.
    Commit,
}

/// 경로별(negotiate/commit/blob) 응답을 돌려주고 요청을 캡처하는 mock `HttpTransport`.
pub struct ScriptedHttpTransport {
    negotiate_body: Vec<u8>,
    commit_body: Vec<u8>,
    fail: Option<FailAt>,
    calls: Mutex<Vec<RawHttpRequest>>,
}

impl ScriptedHttpTransport {
    /// 서버 보유 해시 집합 + 커밋 결과로 스크립트를 만든다(전송 실패 없음).
    pub fn new(server_has: BTreeSet<Sha256Digest>, commit: CommitOutcome) -> Self {
        ScriptedHttpTransport {
            negotiate_body: encode(&NegotiateResponse { server_has }).expect("encode negotiate"),
            commit_body: encode(&commit).expect("encode commit"),
            fail: None,
            calls: Mutex::new(Vec::new()),
        }
    }

    /// 지정 지점에서 전송 실패(`HttpError::Io`)를 내는 변형.
    pub fn with_failure(mut self, at: FailAt) -> Self {
        self.fail = Some(at);
        self
    }

    /// 캡처된 요청 목록 복제본.
    pub fn calls(&self) -> Vec<RawHttpRequest> {
        self.calls.lock().expect("lock").clone()
    }

    /// 실행 총 횟수.
    pub fn call_count(&self) -> usize {
        self.calls.lock().expect("lock").len()
    }

    /// URL 에 지정 부분문자열을 포함하는 요청 수(예: `/commit`, `/blob`).
    pub fn count_paths(&self, needle: &str) -> usize {
        self.calls
            .lock()
            .expect("lock")
            .iter()
            .filter(|req| req.url.contains(needle))
            .count()
    }
}

fn ok_response(body: Vec<u8>) -> Result<RawHttpResponse, HttpError> {
    Ok(RawHttpResponse {
        status: 200,
        headers: Headers::new(),
        body: Body::from_bytes(body),
    })
}

impl HttpTransport for ScriptedHttpTransport {
    fn execute(&self, req: RawHttpRequest) -> Result<RawHttpResponse, HttpError> {
        let url = req.url.clone();
        self.calls.lock().expect("lock").push(req);
        if url.contains("/negotiate") {
            if self.fail == Some(FailAt::Negotiate) {
                return Err(HttpError::Io("scripted negotiate 실패".to_string()));
            }
            return ok_response(self.negotiate_body.clone());
        }
        if url.contains("/commit") {
            if self.fail == Some(FailAt::Commit) {
                return Err(HttpError::Io("scripted commit 실패".to_string()));
            }
            return ok_response(self.commit_body.clone());
        }
        if url.contains("/blob") {
            if self.fail == Some(FailAt::Blob) {
                return Err(HttpError::Io("scripted blob 실패".to_string()));
            }
            return ok_response(Vec::new());
        }
        ok_response(Vec::new())
    }
}

// ---------------------------------------------------------------------------
// AuthTransport 배선(mock HttpTransport seam 주입)
// ---------------------------------------------------------------------------

struct StubConfig {
    snapshot: Arc<WatcherConfig>,
}

impl StubConfig {
    fn new() -> Self {
        StubConfig {
            snapshot: Arc::new(WatcherConfig {
                vault_path: "/tmp/vault".to_string(),
                server_endpoint: "https://server.example/api".to_string(),
                token: Some(TokenSecret::new("test-token".to_string())),
                secure_store_enabled: false,
                log_level: LogLevel::Info,
                notify_consecutive_failures: 3,
            }),
        }
    }
}

impl ConfigSource for StubConfig {
    fn current(&self) -> ConfigSnapshot {
        ConfigSnapshot::new(self.snapshot.clone())
    }
}

/// 주입된 하위 `HttpTransport` seam 으로 실 `AuthTransport`(유일 HTTP 경로)를 조립한다.
pub fn build_auth_transport(transport: Arc<dyn HttpTransport>) -> AuthTransport {
    let config: Arc<dyn ConfigSource> = Arc::new(StubConfig::new());
    let credential = Arc::new(CredentialProvider::with_defaults(config.clone()));
    AuthTransport::new(credential, transport, config, Duration::from_secs(30))
}

// ---------------------------------------------------------------------------
// FakeBlobSource — 경로별 인메모리 바이트(변조 포함) 재-읽기 seam
// ---------------------------------------------------------------------------

/// 경로 -> 바이트 매핑을 보유하는 fake `BlobSource`(Cursor 스트리밍 리더 반환).
#[derive(Default)]
pub struct FakeBlobSource {
    files: HashMap<String, Vec<u8>>,
}

impl FakeBlobSource {
    /// 빈 소스를 만든다.
    pub fn new() -> Self {
        FakeBlobSource::default()
    }

    /// 경로에 바이트를 등록/덮어쓴다(변조 시나리오 = 다른 바이트로 덮어쓰기).
    pub fn insert(&mut self, path: RelativePath, bytes: Vec<u8>) {
        self.files.insert(path.into_string(), bytes);
    }
}

impl BlobSource for FakeBlobSource {
    fn open(&self, path: &RelativePath) -> Result<Box<dyn Read>, BlobSourceError> {
        match self.files.get(path.as_str()) {
            Some(bytes) => Ok(Box::new(Cursor::new(bytes.clone()))),
            None => Err(BlobSourceError::NotFound),
        }
    }
}

// ---------------------------------------------------------------------------
// FakeStore — 재개 오프셋·마지막 커밋·커밋 지속 seam(interior mutability, 관측 가능)
// ---------------------------------------------------------------------------

#[derive(Default)]
struct FakeInner {
    last: Option<Manifest>,
    offsets: BTreeMap<Sha256Digest, ByteCount>,
    commit_calls: u32,
    persist_calls: u32,
}

/// 관측 가능한 fake `SyncStore`(clone 은 동일 상태 공유 -> 드라이버 소유분과 외부 핸들 공유).
#[derive(Clone, Default)]
pub struct FakeStore {
    inner: Arc<Mutex<FakeInner>>,
}

impl FakeStore {
    /// 빈 스토어(커밋 이력 없음).
    pub fn new() -> Self {
        FakeStore::default()
    }

    /// 마지막 커밋 매니페스트를 미리 세운 스토어(no-op 판정 대상).
    pub fn with_last(manifest: Manifest) -> Self {
        let store = FakeStore::new();
        store.inner.lock().expect("lock").last = Some(manifest);
        store
    }

    /// 현재 마지막 커밋 매니페스트(관측).
    pub fn last(&self) -> Option<Manifest> {
        self.inner.lock().expect("lock").last.clone()
    }

    /// `commit_manifest` 호출 횟수(관측).
    pub fn commit_calls(&self) -> u32 {
        self.inner.lock().expect("lock").commit_calls
    }

    /// `persist_resume_offset` 호출 횟수(관측).
    pub fn persist_calls(&self) -> u32 {
        self.inner.lock().expect("lock").persist_calls
    }

    /// 재개 오프셋을 미리 세운다(재개 시나리오).
    pub fn set_offset(&self, blob: Sha256Digest, off: ByteCount) {
        self.inner.lock().expect("lock").offsets.insert(blob, off);
    }
}

impl SyncStore for FakeStore {
    fn last_committed_manifest(&self) -> Option<Manifest> {
        self.inner.lock().expect("lock").last.clone()
    }

    fn resume_offset(&self, blob: &Sha256Digest) -> Option<ByteCount> {
        self.inner.lock().expect("lock").offsets.get(blob).copied()
    }

    fn persist_resume_offset(
        &mut self,
        blob: Sha256Digest,
        off: ByteCount,
    ) -> Result<(), UploadError> {
        let mut guard = self.inner.lock().expect("lock");
        guard.offsets.insert(blob, off);
        guard.persist_calls += 1;
        Ok(())
    }

    fn commit_manifest(&mut self, manifest: Manifest) -> Result<(), UploadError> {
        let mut guard = self.inner.lock().expect("lock");
        guard.last = Some(manifest);
        guard.offsets.clear();
        guard.commit_calls += 1;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// RecordingStatusSink — 진행률·조건 push 관측
// ---------------------------------------------------------------------------

/// raise/clear/progress push 를 기록하는 관측 `StatusSink`.
#[derive(Default)]
pub struct RecordingStatusSink {
    raised: Mutex<Vec<ActiveCondition>>,
    cleared: Mutex<Vec<ActiveCondition>>,
    progress: Mutex<Vec<(u64, u64)>>,
}

impl RecordingStatusSink {
    /// 새 관측 sink.
    pub fn new() -> Self {
        RecordingStatusSink::default()
    }

    /// raise 된 조건 목록.
    pub fn raised(&self) -> Vec<ActiveCondition> {
        self.raised.lock().expect("lock").clone()
    }

    /// clear 된 조건 목록.
    pub fn cleared(&self) -> Vec<ActiveCondition> {
        self.cleared.lock().expect("lock").clone()
    }

    /// 진행률 push `(transferred, total)` 목록.
    pub fn progress(&self) -> Vec<(u64, u64)> {
        self.progress.lock().expect("lock").clone()
    }
}

impl StatusSink for RecordingStatusSink {
    fn set_operational(&self, _state: OperationalState) {}
    fn raise_condition(&self, cond: ActiveCondition) {
        self.raised.lock().expect("lock").push(cond);
    }
    fn clear_condition(&self, cond: ActiveCondition) {
        self.cleared.lock().expect("lock").push(cond);
    }
    fn record_sync_success(&self, _at: Timestamp) {}
    fn set_dirty(&self, _dirty: bool) {}
    fn set_resume_progress(&self, transferred: ByteCount, total: ByteCount) {
        self.progress
            .lock()
            .expect("lock")
            .push((transferred.get(), total.get()));
    }
    fn set_liveness(&self, _signal: LivenessSignal) {}
}

// ---------------------------------------------------------------------------
// 매니페스트/blob 픽스처 헬퍼
// ---------------------------------------------------------------------------

/// 바이트열의 표준 SHA-256(테스트 오라클).
pub fn hash_bytes(bytes: &[u8]) -> Sha256Digest {
    ContentAddressing::hash_stream(bytes).expect("hash_stream 무오류")
}

/// `(경로, 바이트)` 목록으로 정합 매니페스트 + 그 바이트를 반환하는 `FakeBlobSource` 를 만든다.
///
/// 각 엔트리 `raw_sha256` 은 바이트의 실제 해시이며, `manifest_digest` 는 U1 계산으로 채운다 —
/// 따라서 재검증(R-REVERIFY-01)이 통과한다(변조 없음).
pub fn manifest_from_files(files: &[(&str, Vec<u8>)]) -> (Manifest, FakeBlobSource) {
    let mut entries = Vec::new();
    let mut source = FakeBlobSource::new();
    for (path, bytes) in files {
        let relative_path = RelativePath::normalize(path).expect("정규화");
        let raw_sha256 = hash_bytes(bytes);
        entries.push(ManifestEntry {
            relative_path: relative_path.clone(),
            raw_sha256,
            size: ByteCount::new(bytes.len() as u64),
        });
        source.insert(relative_path, bytes.clone());
    }
    entries.sort();
    let manifest_digest = ContentAddressing::manifest_digest(&entries);
    (
        Manifest {
            entries,
            manifest_digest,
        },
        source,
    )
}

/// 성공 커밋 결과(서버 식별자 부여, committed=true) 픽스처.
pub fn committed_outcome() -> CommitOutcome {
    CommitOutcome {
        server_vault_content_id: Some(VaultContentId("vault-content-1".to_string())),
        committed: true,
    }
}

/// 서버 no-op 커밋 결과(committed=false) 픽스처.
pub fn noop_outcome() -> CommitOutcome {
    CommitOutcome {
        server_vault_content_id: None,
        committed: false,
    }
}

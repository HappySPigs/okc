//! 예제 기반 앵커 테스트(PBT-10) — 상태머신 전이·no-op 조기 종료 등 business-critical 경로를
//! PBT 와 병행 검증한다. feature 게이트 없이 기본 `cargo test` 로 실행된다.
//!
//! 커버: no-op 조기 종료(R-NOOP-01), 성공 사이클 전이, 서버 보유분 미전송(NFR-09/R-WANT-03),
//! 전송 전 재검증 불일치 -> 커밋 중단(R-REVERIFY-02), 런타임 한도 초과 halt(R-LIMIT-RT-02),
//! 협상 전송 실패(R-WANT-04), 재개 청크 진행률·오프셋(NFR-10/R-RESUME-02/R-STATUS-01).

mod common;

use std::collections::BTreeSet;
use std::sync::Arc;

use auth_consent::HttpTransport;
use content_core::ContentAddressing;
use foundation::{
    ActiveCondition, ByteCount, Manifest, ManifestEntry, RelativePath, Sha256Digest, StatusSink,
    decode,
};
use upload_client::{ChunkFrame, CyclePhase, UploadError, UploadProtocolDriver};

use common::{
    FailAt, FakeStore, RecordingStatusSink, ScriptedHttpTransport, build_auth_transport,
    committed_outcome, hash_bytes, manifest_from_files,
};

/// 서버 미보유(server_has 공집합) 스크립트 + 지정 threshold/chunk_size 로 드라이버를 조립한다.
#[allow(clippy::too_many_arguments)]
fn driver_with(
    server_has: BTreeSet<Sha256Digest>,
    outcome: upload_client::CommitOutcome,
    fail: Option<FailAt>,
    source: common::FakeBlobSource,
    store: FakeStore,
    status: Arc<RecordingStatusSink>,
    threshold: u64,
    chunk_size: u64,
) -> (UploadProtocolDriver, Arc<ScriptedHttpTransport>) {
    let mut script = ScriptedHttpTransport::new(server_has, outcome);
    if let Some(at) = fail {
        script = script.with_failure(at);
    }
    let script = Arc::new(script);
    let transport: Arc<dyn HttpTransport> = script.clone();
    let auth = build_auth_transport(transport);
    let status_dyn: Arc<dyn StatusSink> = status;
    let driver = UploadProtocolDriver::new(
        auth,
        Box::new(source),
        Box::new(store),
        status_dyn,
        threshold,
        chunk_size,
    );
    (driver, script)
}

/// R-NOOP-01 예제: digest 동일 -> negotiate/transfer/commit 호출 0회, committed=false.
#[test]
fn noop_early_exit_when_digest_matches_last_commit() {
    let (manifest, source) = manifest_from_files(&[("a.txt", b"hello".to_vec())]);
    let store = FakeStore::with_last(manifest.clone());
    let status = Arc::new(RecordingStatusSink::new());
    let (mut driver, script) = driver_with(
        BTreeSet::new(),
        committed_outcome(),
        None,
        source,
        store.clone(),
        status,
        1024,
        64,
    );

    let report = driver.execute_cycle_traced(&manifest);
    let outcome = report.outcome.expect("no-op 은 성공");
    assert!(!outcome.committed, "no-op 은 committed=false");
    assert_eq!(script.call_count(), 0, "서버 왕복 없음");
    assert_eq!(report.phases, vec![CyclePhase::NoOp, CyclePhase::Done]);
    assert_eq!(store.commit_calls(), 0);
}

/// 성공 사이클 예제: server_has 공집합 -> 전 blob 전송 -> 커밋 성공, 전이 순서 정합.
#[test]
fn full_cycle_transfers_all_and_commits() {
    let (manifest, source) =
        manifest_from_files(&[("a.txt", b"alpha".to_vec()), ("b.txt", b"bravo".to_vec())]);
    let store = FakeStore::new();
    let status = Arc::new(RecordingStatusSink::new());
    let (mut driver, script) = driver_with(
        BTreeSet::new(),
        committed_outcome(),
        None,
        source,
        store.clone(),
        status,
        1024,
        64,
    );

    let report = driver.execute_cycle_traced(&manifest);
    let outcome = report.outcome.expect("성공 사이클");
    assert!(outcome.committed);
    assert_eq!(script.count_paths("/negotiate"), 1);
    assert_eq!(script.count_paths("/blob"), 2, "want blob 2건 전송");
    assert_eq!(script.count_paths("/commit"), 1);
    assert_eq!(store.commit_calls(), 1);
    assert_eq!(store.last().as_ref(), Some(&manifest));
    assert_eq!(
        report.phases,
        vec![
            CyclePhase::Negotiate,
            CyclePhase::Transfer,
            CyclePhase::Reverify,
            CyclePhase::Reverify,
            CyclePhase::Commit,
            CyclePhase::Done,
        ]
    );
}

/// R-WANT-03/NFR-09 예제: 서버가 전부 보유 -> want 공집합 -> blob 미전송, 커밋만.
#[test]
fn server_has_all_transfers_nothing() {
    let (manifest, source) = manifest_from_files(&[("a.txt", b"alpha".to_vec())]);
    let server_has: BTreeSet<Sha256Digest> =
        manifest.entries.iter().map(|e| e.raw_sha256).collect();
    let store = FakeStore::new();
    let status = Arc::new(RecordingStatusSink::new());
    let (mut driver, script) = driver_with(
        server_has,
        committed_outcome(),
        None,
        source,
        store.clone(),
        status,
        1024,
        64,
    );

    let report = driver.execute_cycle_traced(&manifest);
    assert!(report.outcome.is_ok());
    assert_eq!(script.count_paths("/blob"), 0, "서버 보유분 미전송");
    assert_eq!(script.count_paths("/commit"), 1);
    assert_eq!(
        report.phases,
        vec![
            CyclePhase::Negotiate,
            CyclePhase::Transfer,
            CyclePhase::Commit,
            CyclePhase::Done,
        ]
    );
}

/// R-REVERIFY-02 예제: 전송 직전 재-해시 불일치(변조) -> HashMismatch, 커밋 미발행, last 불변.
#[test]
fn reverify_mismatch_aborts_before_commit() {
    let (manifest, mut source) = manifest_from_files(&[("a.txt", b"original".to_vec())]);
    // 열거~전송 간 변경(TOCTOU) 모사: 같은 경로에 다른 바이트를 덮어쓴다.
    let tampered = RelativePath::normalize("a.txt").expect("정규화");
    source.insert(tampered, b"tampered-bytes".to_vec());

    let store = FakeStore::new();
    let status = Arc::new(RecordingStatusSink::new());
    let (mut driver, script) = driver_with(
        BTreeSet::new(),
        committed_outcome(),
        None,
        source,
        store.clone(),
        status,
        1024,
        64,
    );

    let report = driver.execute_cycle_traced(&manifest);
    match report.outcome {
        Err(UploadError::HashMismatch { .. }) => {}
        other => panic!("HashMismatch 기대, got {other:?}"),
    }
    assert_eq!(script.count_paths("/commit"), 0, "커밋 미발행");
    assert_eq!(script.count_paths("/blob"), 0, "변조 blob 전송 안 함");
    assert!(store.last().is_none(), "last_committed 불변");
    assert_eq!(report.phases.last(), Some(&CyclePhase::Abort));
}

/// R-LIMIT-RT-02 예제: 파일당 한도(2 GiB) 초과 -> halt, OverLimit raise, 서버 왕복 없음.
#[test]
fn over_limit_halts_and_raises_condition() {
    let entries = vec![ManifestEntry {
        relative_path: RelativePath::normalize("big.bin").expect("정규화"),
        raw_sha256: hash_bytes(b"x"),
        size: ByteCount::new(3 * 1024 * 1024 * 1024),
    }];
    let manifest_digest = ContentAddressing::manifest_digest(&entries);
    let manifest = Manifest {
        entries,
        manifest_digest,
    };

    let store = FakeStore::new();
    let status = Arc::new(RecordingStatusSink::new());
    let (mut driver, script) = driver_with(
        BTreeSet::new(),
        committed_outcome(),
        None,
        common::FakeBlobSource::new(),
        store.clone(),
        status.clone(),
        1024,
        64,
    );

    let report = driver.execute_cycle_traced(&manifest);
    assert!(matches!(report.outcome, Err(UploadError::OverLimit(_))));
    assert_eq!(script.call_count(), 0, "한도 초과 시 전송 없음");
    assert!(status.raised().contains(&ActiveCondition::OverLimit));
    assert_eq!(report.phases, vec![CyclePhase::Abort]);
    assert!(store.last().is_none());
}

/// R-WANT-04 예제: 협상 전송 실패 -> Transport 오류, 전이 [Negotiate, Abort].
#[test]
fn negotiate_transport_failure_aborts() {
    let (manifest, source) = manifest_from_files(&[("a.txt", b"alpha".to_vec())]);
    let store = FakeStore::new();
    let status = Arc::new(RecordingStatusSink::new());
    let (mut driver, _script) = driver_with(
        BTreeSet::new(),
        committed_outcome(),
        Some(FailAt::Negotiate),
        source,
        store.clone(),
        status,
        1024,
        64,
    );

    let report = driver.execute_cycle_traced(&manifest);
    assert!(matches!(report.outcome, Err(UploadError::Transport(_))));
    assert_eq!(report.phases, vec![CyclePhase::Negotiate, CyclePhase::Abort]);
    assert_eq!(store.commit_calls(), 0);
}

/// NFR-10/R-STATUS-01 예제: 대용량 blob(> S) -> 재개 청크 전송, 청크마다 오프셋 지속 + 진행률 push.
#[test]
fn large_blob_chunked_with_progress_and_offsets() {
    let data = (0u8..10).collect::<Vec<u8>>(); // 10 바이트
    let (manifest, source) = manifest_from_files(&[("big.bin", data.clone())]);
    let store = FakeStore::new();
    let status = Arc::new(RecordingStatusSink::new());
    // threshold=4 (< 10) -> 청크 경로, chunk_size=4 -> 청크 3개(4,4,2).
    let (mut driver, script) = driver_with(
        BTreeSet::new(),
        committed_outcome(),
        None,
        source,
        store.clone(),
        status.clone(),
        4,
        4,
    );

    let report = driver.execute_cycle_traced(&manifest);
    assert!(report.outcome.is_ok());
    assert_eq!(script.count_paths("/blob"), 3, "10바이트/4 -> 청크 3개");
    assert_eq!(store.persist_calls(), 3, "청크 ack 마다 오프셋 지속");
    // 진행률은 청크마다 누적 오프셋으로 push 된다.
    assert_eq!(status.progress(), vec![(4, 10), (8, 10), (10, 10)]);
    assert_eq!(store.commit_calls(), 1);
}

/// R-RESUME-01/02 예제: 재개 오프셋이 있으면 청크 경로로 진입하고 그 오프셋부터 전송한다.
#[test]
fn resume_offset_starts_from_persisted_position() {
    let data = (0u8..10).collect::<Vec<u8>>();
    let (manifest, source) = manifest_from_files(&[("big.bin", data.clone())]);
    let blob = manifest.entries.first().expect("엔트리").raw_sha256;
    let store = FakeStore::new();
    store.set_offset(blob, ByteCount::new(4)); // [0,4) 는 서버 보유로 간주.
    let status = Arc::new(RecordingStatusSink::new());
    // threshold=1024 (10 <= 1024) 이지만 resume>0 이므로 청크 경로로 진입한다.
    let (mut driver, script) = driver_with(
        BTreeSet::new(),
        committed_outcome(),
        None,
        source,
        store.clone(),
        status,
        1024,
        4,
    );

    let report = driver.execute_cycle_traced(&manifest);
    assert!(report.outcome.is_ok());
    // 첫 blob 요청 프레임은 오프셋 4 에서 시작한다([0,4) 재전송 없음, R-CHUNK-02).
    let calls = script.calls();
    let first_blob = calls
        .iter()
        .find(|req| req.url.contains("/blob"))
        .expect("blob 요청 존재");
    let frame: ChunkFrame = decode(first_blob.body.as_bytes()).expect("프레임 디코드");
    assert_eq!(frame.offset.get(), 4, "재개 오프셋 4 부터 전송");
    // 재개 후 남은 [4,10) = 청크 2개(4,2).
    assert_eq!(script.count_paths("/blob"), 2);
}

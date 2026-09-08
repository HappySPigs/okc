//! Property-Based Testing (PBT, `proptest-support` feature 게이트) — PROP-U3-01~08.
//!
//! feature 가 꺼진 기본 빌드에서는 빈 테스트 크레이트로 컴파일되고,
//! `cargo test --features proptest-support` 로만 실제 property 가 실행된다(PBT-07).
//!
//! 커버: PROP-U3-01/02(want 차집합·멱등), PROP-U3-03/04/05(청크 재조립·재개 동치·해시 정합),
//! PROP-U3-06(재검증 불일치 -> 커밋 미발행·last 불변), PROP-U3-07(no-op 조기 종료·왕복 0회),
//! PROP-U3-08(상태머신 합법 전이·단일 직렬 사이클).
#![cfg(feature = "proptest-support")]

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use auth_consent::HttpTransport;
use content_core::ContentAddressing;
use foundation::proptest_support::generators as fg;
use foundation::{ByteCount, Manifest, ManifestEntry, RelativePath, Sha256Digest, StatusSink};
use proptest::prelude::*;
use upload_client::proptest_support::generators as u3gen;
use upload_client::{
    CyclePhase, UploadError, UploadProtocolDriver, compute_want, frame_chunks, reassemble,
};

use common::{
    FailAt, FakeBlobSource, FakeStore, RecordingStatusSink, ScriptedHttpTransport,
    build_auth_transport, committed_outcome, hash_bytes,
};

// ---------------------------------------------------------------------------
// PROP-U3-01 / PROP-U3-02 — want 차집합 정확성 + 멱등
// ---------------------------------------------------------------------------

proptest! {
    /// `WantSet == manifest_hashes \ server_has`, want ⊆ hashes, want ∩ server_has == ∅.
    #[test]
    fn prop_want_is_exact_difference(
        hashes in u3gen::arb_digest_set(),
        server_has in u3gen::arb_digest_set(),
    ) {
        let want = compute_want(&hashes, &server_has);
        let expected: BTreeSet<Sha256Digest> = hashes.difference(&server_has).copied().collect();
        prop_assert_eq!(&want.blobs, &expected);
        prop_assert!(want.blobs.is_subset(&hashes));
        prop_assert!(want.blobs.is_disjoint(&server_has));
    }
}

proptest! {
    /// 서버가 want 를 보유한 뒤 재실행 시 want == ∅; `server_has ⊇ hashes` 이면 want == ∅.
    #[test]
    fn prop_want_idempotent(
        hashes in u3gen::arb_digest_set(),
        server_has in u3gen::arb_digest_set(),
    ) {
        let want1 = compute_want(&hashes, &server_has);
        let mut after: BTreeSet<Sha256Digest> = server_has.clone();
        after.extend(want1.blobs.iter().copied());
        prop_assert!(compute_want(&hashes, &after).blobs.is_empty());
        // superset(자기 자신 포함)이면 공집합.
        prop_assert!(compute_want(&hashes, &hashes).blobs.is_empty());
    }
}

// ---------------------------------------------------------------------------
// PROP-U3-03 / PROP-U3-04 / PROP-U3-05 — 청크 재조립·재개 동치·해시 정합
// ---------------------------------------------------------------------------

proptest! {
    /// 임의 blob·임의 청크 크기에 대해 `reassemble(frame_chunks(.., 0)) == original`.
    #[test]
    fn prop_chunk_reassembly_roundtrip(
        bytes in u3gen::arb_blob_bytes(),
        chunk_size in u3gen::arb_chunk_size(),
        blob in fg::arb_sha256_digest(),
    ) {
        let frames = frame_chunks(blob, &bytes, chunk_size, 0);
        prop_assert_eq!(reassemble(&frames), bytes);
    }
}

proptest! {
    /// 임의 유효 오프셋 `k` 에서 재개한 재조립이 무중단 전송 결과와 동일하다(R-CHUNK-02).
    #[test]
    fn prop_resume_offset_equivalence(
        bytes in u3gen::arb_blob_bytes(),
        chunk_size in u3gen::arb_chunk_size(),
        blob in fg::arb_sha256_digest(),
        k_raw in any::<u64>(),
    ) {
        let total = bytes.len() as u64;
        let k = if total == 0 { 0 } else { k_raw % (total + 1) };
        let frames = frame_chunks(blob, &bytes, chunk_size, k);
        // 재개 프레임은 모두 `offset >= k`(=`[0, k)` 재전송 없음).
        for frame in &frames {
            prop_assert!(frame.offset.get() >= k);
        }
        // `[0, k)`(서버 보유분) ++ 재개 재조립 == 원본.
        let prefix = bytes.get(..k as usize).unwrap_or_default();
        let mut full = prefix.to_vec();
        full.extend_from_slice(&reassemble(&frames));
        prop_assert_eq!(full, bytes);
    }
}

proptest! {
    /// 재조립 blob 의 `raw_sha256` == 원본 해시; 각 프레임 `chunk_sha256` == 청크 바이트 해시.
    #[test]
    fn prop_reassembled_hash_matches_manifest(
        bytes in u3gen::arb_blob_bytes(),
        chunk_size in u3gen::arb_chunk_size(),
        blob in fg::arb_sha256_digest(),
    ) {
        let frames = frame_chunks(blob, &bytes, chunk_size, 0);
        prop_assert_eq!(hash_bytes(&reassemble(&frames)), hash_bytes(&bytes));
        for frame in &frames {
            prop_assert_eq!(frame.chunk_sha256, hash_bytes(&frame.bytes));
        }
    }
}

// ---------------------------------------------------------------------------
// 상태머신 구동 헬퍼 (PROP-U3-06/07/08)
// ---------------------------------------------------------------------------

/// `(RelativePath, bytes)` 목록으로 정합 매니페스트 + `FakeBlobSource` 를 만든다(경로 유일).
fn build(files: Vec<(RelativePath, Vec<u8>)>) -> (Manifest, FakeBlobSource) {
    let mut entries = Vec::new();
    let mut source = FakeBlobSource::new();
    for (relative_path, bytes) in files {
        let raw_sha256 = hash_bytes(&bytes);
        entries.push(ManifestEntry {
            relative_path: relative_path.clone(),
            raw_sha256,
            size: ByteCount::new(bytes.len() as u64),
        });
        source.insert(relative_path, bytes);
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

/// 경로 유일 `(RelativePath, bytes)` 목록을 생성한다(BTreeMap dedup).
fn arb_files() -> impl Strategy<Value = Vec<(RelativePath, Vec<u8>)>> {
    prop::collection::vec((fg::arb_relative_path(), u3gen::arb_blob_bytes()), 0..4).prop_map(
        |pairs| {
            let mut map: BTreeMap<String, (RelativePath, Vec<u8>)> = BTreeMap::new();
            for (relative_path, bytes) in pairs {
                map.insert(relative_path.as_str().to_string(), (relative_path, bytes));
            }
            map.into_values().collect()
        },
    )
}

/// 스크립트된 전송 실패 지점(없음 포함)을 생성한다.
fn arb_fail() -> impl Strategy<Value = Option<FailAt>> {
    prop_oneof![
        Just(None),
        Just(Some(FailAt::Negotiate)),
        Just(Some(FailAt::Blob)),
        Just(Some(FailAt::Commit)),
    ]
}

/// `CyclePhase` 전이가 합법 경로(`Negotiate -> Transfer -> Commit -> Done` | `NoOp` | `Abort`)인지.
fn is_legal(phases: &[CyclePhase]) -> bool {
    use CyclePhase::*;
    match phases.first() {
        Some(NoOp) | Some(Negotiate) | Some(Abort) => {}
        _ => return false,
    }
    for window in phases.windows(2) {
        let legal = matches!(
            (window[0], window[1]),
            (NoOp, Done)
                | (Negotiate, Transfer)
                | (Negotiate, Abort)
                | (Transfer, Reverify)
                | (Transfer, Commit)
                | (Transfer, Abort)
                | (Reverify, Reverify)
                | (Reverify, Commit)
                | (Reverify, Abort)
                | (Commit, Done)
                | (Commit, Abort)
        );
        if !legal {
            return false;
        }
    }
    matches!(phases.last(), Some(Done) | Some(Abort))
}

// ---------------------------------------------------------------------------
// PROP-U3-06 — 재검증 불일치 -> HashMismatch, 커밋 미발행, last 불변
// ---------------------------------------------------------------------------

proptest! {
    /// 변조 바이트 주입 -> 최소 1개 want blob 이 불일치 -> HashMismatch, /commit 0회, last None.
    #[test]
    fn prop_reverify_mismatch_aborts_commit(
        files in arb_files().prop_filter("비어있지 않은 파일 집합", |f| !f.is_empty()),
    ) {
        let (manifest, mut source) = build(files.clone());
        // 첫 엔트리 경로에 다른 바이트를 덮어써 재검증을 깨뜨린다(TOCTOU 모사).
        if let Some(first) = manifest.entries.first() {
            source.insert(first.relative_path.clone(), b"MISMATCH-INJECTED-BYTES".to_vec());
        }

        let store = FakeStore::new();
        let status = Arc::new(RecordingStatusSink::new());
        let script = Arc::new(ScriptedHttpTransport::new(BTreeSet::new(), committed_outcome()));
        let transport: Arc<dyn HttpTransport> = script.clone();
        let auth = build_auth_transport(transport);
        let status_dyn: Arc<dyn StatusSink> = status;
        let mut driver = UploadProtocolDriver::new(
            auth,
            Box::new(source),
            Box::new(store.clone()),
            status_dyn,
            1024,
            64,
        );

        let report = driver.execute_cycle_traced(&manifest);
        prop_assert!(matches!(report.outcome, Err(UploadError::HashMismatch { .. })), "HashMismatch 결과 예상");
        prop_assert_eq!(script.count_paths("/commit"), 0);
        prop_assert!(store.last().is_none());
        prop_assert_eq!(store.commit_calls(), 0);
        prop_assert_eq!(report.phases.last(), Some(&CyclePhase::Abort));
    }
}

// ---------------------------------------------------------------------------
// PROP-U3-07 — no-op 조기 종료(digest 동일) -> 왕복 0회, committed == false
// ---------------------------------------------------------------------------

proptest! {
    /// `manifest_digest == last.manifest_digest` -> negotiate/transfer/commit 0회 & committed false.
    #[test]
    fn prop_noop_early_exit(files in arb_files()) {
        let (manifest, source) = build(files);
        let store = FakeStore::with_last(manifest.clone());
        let status = Arc::new(RecordingStatusSink::new());
        let script = Arc::new(ScriptedHttpTransport::new(BTreeSet::new(), committed_outcome()));
        let transport: Arc<dyn HttpTransport> = script.clone();
        let auth = build_auth_transport(transport);
        let status_dyn: Arc<dyn StatusSink> = status;
        let mut driver = UploadProtocolDriver::new(
            auth,
            Box::new(source),
            Box::new(store.clone()),
            status_dyn,
            1024,
            64,
        );

        let report = driver.execute_cycle_traced(&manifest);
        let outcome = report.outcome.expect("no-op 은 성공");
        prop_assert!(!outcome.committed);
        prop_assert_eq!(script.call_count(), 0);
        prop_assert_eq!(store.commit_calls(), 0);
        prop_assert_eq!(report.phases, vec![CyclePhase::NoOp, CyclePhase::Done]);
    }
}

// ---------------------------------------------------------------------------
// PROP-U3-08 — 상태머신 합법 전이(단일 직렬 사이클, 성공/실패/변조/no-op/over 시나리오)
// ---------------------------------------------------------------------------

proptest! {
    /// 임의 시나리오 조합에서 `CyclePhase` 전이는 항상 합법 경로만 밟고, 결과와 종료 상태가 정합.
    #[test]
    fn prop_transitions_are_legal(
        files in arb_files(),
        server_has_all in any::<bool>(),
        preload_last in any::<bool>(),
        tamper in any::<bool>(),
        fail in arb_fail(),
        threshold in prop::sample::select(vec![0u64, 4, 16, 1024]),
        chunk_size in u3gen::arb_chunk_size(),
    ) {
        let (manifest, mut source) = build(files);
        if tamper
            && let Some(first) = manifest.entries.first() {
                source.insert(first.relative_path.clone(), b"TAMPER".to_vec());
            }
        let server_has: BTreeSet<Sha256Digest> = if server_has_all {
            manifest.entries.iter().map(|e| e.raw_sha256).collect()
        } else {
            BTreeSet::new()
        };

        let store = if preload_last {
            FakeStore::with_last(manifest.clone())
        } else {
            FakeStore::new()
        };
        let status = Arc::new(RecordingStatusSink::new());
        let mut script = ScriptedHttpTransport::new(server_has, committed_outcome());
        if let Some(at) = fail {
            script = script.with_failure(at);
        }
        let script = Arc::new(script);
        let transport: Arc<dyn HttpTransport> = script.clone();
        let auth = build_auth_transport(transport);
        let status_dyn: Arc<dyn StatusSink> = status;
        let mut driver = UploadProtocolDriver::new(
            auth,
            Box::new(source),
            Box::new(store.clone()),
            status_dyn,
            threshold,
            chunk_size,
        );

        let report = driver.execute_cycle_traced(&manifest);
        prop_assert!(is_legal(&report.phases), "불법 전이 시퀀스: {:?}", report.phases);
        match &report.outcome {
            Ok(_) => prop_assert_eq!(report.phases.last(), Some(&CyclePhase::Done)),
            Err(_) => prop_assert_eq!(report.phases.last(), Some(&CyclePhase::Abort)),
        }
        // no-op(선반영된 last 와 digest 동일)은 서버 왕복 0회.
        if preload_last {
            prop_assert_eq!(script.call_count(), 0);
            prop_assert_eq!(report.phases, vec![CyclePhase::NoOp, CyclePhase::Done]);
        }
    }
}

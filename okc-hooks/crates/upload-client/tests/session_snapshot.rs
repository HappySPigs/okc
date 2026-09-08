//! Regression tests for the real receiver session contract and coherent blob bytes.

mod common;

use common::{
    FakeStore, RecordingStatusSink, ScriptedHttpTransport, build_auth_transport, committed_outcome,
    manifest_from_files,
};
use foundation::{ByteCount, RelativePath, decode};
use std::collections::BTreeSet;
use std::io::{Cursor, Read};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use upload_client::{BlobSource, BlobSourceError, ChunkFrame, UploadProtocolDriver};

struct ChangingSource {
    opens: Arc<AtomicUsize>,
    first: Vec<u8>,
}

impl BlobSource for ChangingSource {
    fn open(&self, _: &RelativePath) -> Result<Box<dyn Read>, BlobSourceError> {
        let bytes = if self.opens.fetch_add(1, Ordering::SeqCst) == 0 {
            self.first.clone()
        } else {
            b"changed between verification and upload".to_vec()
        };
        Ok(Box::new(Cursor::new(bytes)))
    }
}

fn captured_frames(script: &ScriptedHttpTransport) -> Vec<ChunkFrame> {
    script
        .calls()
        .iter()
        .filter(|req| req.url.contains("/blob/"))
        .map(|req| decode(req.body.as_bytes()).unwrap())
        .collect()
}

#[test]
fn upload_uses_the_same_verified_bytes_without_reopening_source() {
    for threshold in [2, 1024] {
        let bytes = b"original immutable snapshot".to_vec();
        let (manifest, _) = manifest_from_files(&[("note.md", bytes.clone())]);
        let opens = Arc::new(AtomicUsize::new(0));
        let script = Arc::new(ScriptedHttpTransport::new(
            BTreeSet::new(),
            committed_outcome(),
        ));
        let mut driver = UploadProtocolDriver::new(
            build_auth_transport(script.clone()),
            Box::new(ChangingSource {
                opens: opens.clone(),
                first: bytes.clone(),
            }),
            Box::new(FakeStore::new()),
            Arc::new(RecordingStatusSink::new()),
            threshold,
            4,
        );
        assert!(driver.execute_cycle(&manifest).is_ok());
        assert_eq!(opens.load(Ordering::SeqCst), 1);
        assert_eq!(upload_client::reassemble(&captured_frames(&script)), bytes);
    }
}

#[test]
fn session_uses_server_offset_instead_of_stale_local_ack() {
    for server_offset in [0, 3] {
        let bytes = b"0123456789".to_vec();
        let (manifest, source) = manifest_from_files(&[("note.md", bytes.clone())]);
        let blob = manifest.entries[0].raw_sha256;
        let store = FakeStore::new();
        store.set_offset(blob, ByteCount::new(8));
        let script = Arc::new(
            ScriptedHttpTransport::new(BTreeSet::new(), committed_outcome())
                .with_session("session-42", vec![(blob, ByteCount::new(server_offset))]),
        );
        let mut driver = UploadProtocolDriver::new(
            build_auth_transport(script.clone()),
            Box::new(source),
            Box::new(store),
            Arc::new(RecordingStatusSink::new()),
            2,
            4,
        );
        assert!(driver.execute_cycle(&manifest).is_ok());
        let frames = captured_frames(&script);
        assert_eq!(frames[0].offset.get(), server_offset);
        assert_eq!(
            upload_client::reassemble(&frames),
            bytes[server_offset as usize..]
        );
        for request in script.calls() {
            assert!(
                request
                    .headers
                    .iter()
                    .any(|(k, v)| k == "Content-Type" && v == "application/cbor")
            );
            if !request.url.ends_with("/negotiate") {
                assert!(
                    request
                        .headers
                        .iter()
                        .any(|(k, v)| k == "X-OKC-Upload-Session" && v == "session-42")
                );
            }
        }
    }
}

#[test]
fn rejects_server_offset_beyond_blob_length_before_transfer_or_commit() {
    let (manifest, source) = manifest_from_files(&[("note.md", b"a".to_vec())]);
    let blob = manifest.entries[0].raw_sha256;
    let script = Arc::new(
        ScriptedHttpTransport::new(BTreeSet::new(), committed_outcome())
            .with_session("session", vec![(blob, ByteCount::new(2))]),
    );
    let mut driver = UploadProtocolDriver::new(
        build_auth_transport(script.clone()),
        Box::new(source),
        Box::new(FakeStore::new()),
        Arc::new(RecordingStatusSink::new()),
        2,
        4,
    );
    assert!(driver.execute_cycle(&manifest).is_err());
    assert_eq!(script.count_paths("/blob"), 0);
    assert_eq!(script.count_paths("/commit"), 0);
}

#[cfg(feature = "proptest-support")]
mod properties {
    use super::*;
    use proptest::prelude::*;
    use upload_client::proptest_support::generators::{arb_blob_bytes, arb_chunk_size};

    proptest! {
        #[test]
        fn verified_snapshot_is_transmitted_for_arbitrary_content(
            bytes in arb_blob_bytes(), chunk in arb_chunk_size(),
        ) {
            let (manifest, _) = manifest_from_files(&[("note.md", bytes.clone())]);
            let opens = Arc::new(AtomicUsize::new(0));
            let script = Arc::new(ScriptedHttpTransport::new(BTreeSet::new(), committed_outcome()));
            let mut driver = UploadProtocolDriver::new(build_auth_transport(script.clone()),
                Box::new(ChangingSource { opens: opens.clone(), first: bytes.clone() }),
                Box::new(FakeStore::new()), Arc::new(RecordingStatusSink::new()), 0, chunk);
            prop_assert!(driver.execute_cycle(&manifest).is_ok());
            prop_assert_eq!(opens.load(Ordering::SeqCst), 1);
            prop_assert_eq!(upload_client::reassemble(&captured_frames(&script)), bytes);
        }

        #[test]
        fn session_response_cbor_roundtrips(
            hashes in upload_client::proptest_support::generators::arb_digest_set(),
            session in "[a-zA-Z0-9_-]{1,64}",
            offsets in proptest::collection::vec((foundation::proptest_support::generators::arb_sha256_digest(), 0u64..=2_147_483_648), 0..8),
        ) {
            let response = upload_client::NegotiateResponse {
                server_has: hashes, session_id: Some(session),
                resume_offsets: offsets.into_iter().map(|(hash, count)| (hash, ByteCount::new(count))).collect(),
            };
            let bytes = foundation::encode(&response).unwrap();
            prop_assert_eq!(decode::<upload_client::NegotiateResponse>(&bytes).unwrap(), response);
        }
    }
}

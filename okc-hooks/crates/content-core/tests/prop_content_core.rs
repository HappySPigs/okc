//! U1 Property-Based Tests (PBT, `proptest-support` feature 게이트).
//!
//! feature 가 꺼진 기본 빌드에서는 빈 테스트 크레이트로 컴파일되고,
//! `cargo test --features proptest-support` 로만 실제 property 가 실행된다(PBT-07).
//!
//! 구현 속성: PROP-U1-01(해싱 결정성 + sha256 오라클 + 청크 무관), PROP-U1-02(manifest_digest
//! 순서 무관 + injective), PROP-U1-03(diff apply 오라클 + 서로소 + no-op), PROP-U1-04(SafetyLimits
//! 경계-정확 + 단조), PROP-U1-05(U1 산출값 무손실 round-trip), REL-07(대값 no-panic).
#![cfg(feature = "proptest-support")]

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Read};

use content_core::proptest_support::generators as u1gen;
use content_core::{ContentAddressing, LimitVerdict, LimitViolation, ManifestDiffer, SafetyLimitsValidator};
use foundation::proptest_support::generators as g;
use foundation::{
    ByteCount, ChangeSet, MAX_FILE_BYTES, Manifest, ManifestDigest, ManifestEntry, RelativePath,
    Sha256Digest, decode, encode,
};
use proptest::prelude::*;
use sha2::{Digest, Sha256};

/// 매 `read` 마다 최대 `chunk` 바이트만 반환하는 리더(청크 경계 무관성 검증용).
struct ChunkReader<'a> {
    data: &'a [u8],
    pos: usize,
    chunk: usize,
}

impl<'a> ChunkReader<'a> {
    fn new(data: &'a [u8], chunk: usize) -> Self {
        ChunkReader {
            data,
            pos: 0,
            chunk: chunk.max(1),
        }
    }
}

impl Read for ChunkReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let remaining = &self.data[self.pos..];
        if remaining.is_empty() {
            return Ok(0);
        }
        let take = remaining.len().min(self.chunk).min(buf.len());
        buf[..take].copy_from_slice(&remaining[..take]);
        self.pos += take;
        Ok(take)
    }
}

/// 시드 기반 결정적 셔플(Fisher-Yates, xorshift)로 순열을 만든다.
fn shuffle<T>(mut items: Vec<T>, mut seed: u64) -> Vec<T> {
    if seed == 0 {
        seed = 0x9E37_79B9_7F4A_7C15;
    }
    let len = items.len();
    for i in (1..len).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let j = (seed % ((i as u64) + 1)) as usize;
        items.swap(i, j);
    }
    items
}

proptest! {
    /// PROP-U1-01 — 결정성 + 표준 sha256 오라클.
    #[test]
    fn hashing_deterministic_and_oracle(bytes in u1gen::arb_bytes()) {
        let d1 = ContentAddressing::hash_stream(bytes.as_slice()).unwrap();
        let d2 = ContentAddressing::hash_stream(bytes.as_slice()).unwrap();
        prop_assert_eq!(d1, d2);

        let mut oracle = Sha256::new();
        oracle.update(&bytes);
        let expected = oracle.finalize();
        prop_assert_eq!(d1.as_bytes().as_slice(), expected.as_slice());
    }

    /// PROP-U1-01 — 청크 분할 경계 무관성.
    #[test]
    fn hashing_chunk_boundary_invariant(bytes in u1gen::arb_bytes(), chunk in 1usize..131) {
        let whole = ContentAddressing::hash_stream(bytes.as_slice()).unwrap();
        let chunked = ContentAddressing::hash_stream(ChunkReader::new(&bytes, chunk)).unwrap();
        prop_assert_eq!(whole, chunked);
    }

    /// PROP-U1-02 — manifest_digest 순서 무관 결정성.
    #[test]
    fn manifest_digest_permutation_invariant(
        manifest in u1gen::arb_consistent_manifest(),
        seed in any::<u64>(),
    ) {
        let permuted = shuffle(manifest.entries.clone(), seed);
        prop_assert_eq!(ContentAddressing::manifest_digest(&permuted), manifest.manifest_digest);
    }

    /// PROP-U1-03 — diff apply 오라클 + 서로소 + no-op 동치.
    #[test]
    fn diff_apply_oracle(pair in u1gen::arb_manifest_pair()) {
        let (prev, current) = pair;
        let change = ManifestDiffer::diff(&prev, &current);

        // 서로소: added/modified/deleted 경로가 겹치지 않는다.
        let added: BTreeSet<&str> = change.added.iter().map(|e| e.relative_path.as_str()).collect();
        let modified: BTreeSet<&str> = change.modified.iter().map(|e| e.relative_path.as_str()).collect();
        let deleted: BTreeSet<&str> = change.deleted.iter().map(|p| p.as_str()).collect();
        prop_assert!(added.is_disjoint(&modified));
        prop_assert!(added.is_disjoint(&deleted));
        prop_assert!(modified.is_disjoint(&deleted));

        // apply(prev, diff) == current.
        let mut map: BTreeMap<RelativePath, ManifestEntry> = prev
            .entries
            .iter()
            .map(|e| (e.relative_path.clone(), e.clone()))
            .collect();
        for path in &change.deleted {
            map.remove(path);
        }
        for entry in change.added.iter().chain(change.modified.iter()) {
            map.insert(entry.relative_path.clone(), entry.clone());
        }
        let rebuilt: Vec<ManifestEntry> = map.into_values().collect();
        prop_assert_eq!(rebuilt, current.entries.clone());

        // no-op 동치: 동일 매니페스트 diff 는 empty.
        prop_assert!(ManifestDiffer::diff(&current, &current).is_empty());
    }

    /// PROP-U1-04 — 파일당 한도 경계-정확(limit-1/limit/limit+1).
    #[test]
    fn file_bytes_boundary_exact(
        delta in prop_oneof![Just(-1i64), Just(0i64), Just(1i64)],
        path in g::arb_relative_path(),
    ) {
        let size = (MAX_FILE_BYTES as i64 + delta) as u64;
        let entry = ManifestEntry {
            relative_path: path,
            raw_sha256: Sha256Digest::from_bytes([0u8; 32]),
            size: ByteCount::new(size),
        };
        let entries = vec![entry];
        let manifest = Manifest {
            manifest_digest: ContentAddressing::manifest_digest(&entries),
            entries,
        };
        match SafetyLimitsValidator::validate(&manifest) {
            LimitVerdict::WithinLimits => prop_assert!(delta <= 0),
            LimitVerdict::Exceeded(LimitViolation::FileBytes { limit, actual, .. }) => {
                prop_assert!(delta > 0);
                prop_assert_eq!(limit, MAX_FILE_BYTES);
                prop_assert_eq!(actual, size);
            }
            other => prop_assert!(false, "예상치 못한 판정: {:?}", other),
        }
    }

    /// PROP-U1-04 — 단조성: 이미 reject 된 매니페스트에 추가해도 reject 유지.
    #[test]
    fn limits_monotone_reject(
        base in u1gen::arb_consistent_manifest(),
        extra in prop::collection::vec(g::arb_manifest_entry(), 0..4),
    ) {
        let mut manifest = base;
        // 파일당 한도 초과 파일을 주입해 확정 reject 상태로 만든다.
        manifest.entries.push(ManifestEntry {
            relative_path: RelativePath::normalize("oversized__marker.bin").unwrap(),
            raw_sha256: Sha256Digest::from_bytes([1u8; 32]),
            size: ByteCount::new(MAX_FILE_BYTES + 1),
        });
        prop_assert!(matches!(SafetyLimitsValidator::validate(&manifest), LimitVerdict::Exceeded(_)));

        manifest.entries.extend(extra);
        prop_assert!(matches!(SafetyLimitsValidator::validate(&manifest), LimitVerdict::Exceeded(_)));
    }

    /// PROP-U1-05 — U1 산출 Manifest 의 무손실 round-trip(U0 코덱).
    #[test]
    fn manifest_roundtrip(manifest in u1gen::arb_consistent_manifest()) {
        let bytes = encode(&manifest).unwrap();
        let decoded: Manifest = decode(&bytes).unwrap();
        prop_assert_eq!(decoded, manifest);
    }

    /// PROP-U1-05 — U1 산출 ChangeSet 의 무손실 round-trip(U0 코덱).
    #[test]
    fn changeset_roundtrip(pair in u1gen::arb_manifest_pair()) {
        let change = ManifestDiffer::diff(&pair.0, &pair.1);
        let bytes = encode(&change).unwrap();
        let decoded: ChangeSet = decode(&bytes).unwrap();
        prop_assert_eq!(decoded, change);
    }

    /// REL-07 — 대값(u64::MAX 포함) 누적에도 validate 는 패닉 없이 종결한다.
    #[test]
    fn validate_no_panic_large(
        specs in prop::collection::vec((g::arb_relative_path(), g::arb_byte_count()), 0..8),
    ) {
        let entries: Vec<ManifestEntry> = specs
            .into_iter()
            .map(|(relative_path, size)| ManifestEntry {
                relative_path,
                raw_sha256: Sha256Digest::from_bytes([0u8; 32]),
                size,
            })
            .collect();
        let manifest = Manifest {
            manifest_digest: ManifestDigest::from_bytes([0u8; 32]),
            entries,
        };
        // 패닉하면 테스트 실패. 반환값 자체는 입력에 따라 다르다.
        let _ = SafetyLimitsValidator::validate(&manifest);
    }
}

//! U1 도메인 제약 준수 `proptest` 제너레이터.
//!
//! U0 `proptest_support::generators`(`arb_manifest_entry`/`arb_relative_path`/`arb_sha256_digest`
//! /`arb_byte_count`)를 재사용하며, U1 컴포넌트 산출값 불변식을 만족하는 제너레이터를 추가한다:
//! - `arb_bytes`: 빈/소량/버퍼 경계를 가로지르는 임의 `Vec<u8>`(PROP-U1-01).
//! - `arb_consistent_manifest`: 실제 `manifest_digest` 가 계산된 canonical `Manifest`(PROP-U1-02/05).
//! - `arb_manifest_pair`: add/modify/delete 로 상관된 두 digest-consistent `Manifest`(PROP-U1-03).

use std::collections::BTreeSet;

use foundation::proptest_support::generators as g;
use foundation::{Manifest, ManifestEntry};
use proptest::prelude::*;

use crate::content_addressing::{ContentAddressing, HASH_BUFFER_BYTES};

/// 빈/소량/64 KiB 버퍼 경계를 가로지르는 임의 바이트열(청크 무관 결정성·오라클 검증용).
pub fn arb_bytes() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        Just(Vec::new()),
        prop::collection::vec(any::<u8>(), 1..256),
        // 고정 버퍼(64 KiB) 경계를 확실히 가로지르는 크기 대역.
        prop::collection::vec(any::<u8>(), (HASH_BUFFER_BYTES - 4)..(HASH_BUFFER_BYTES + 260)),
    ]
}

/// 실제 계산된 `manifest_digest` 를 갖는 canonical(경로 유일·정렬) `Manifest`.
pub fn arb_consistent_manifest() -> impl Strategy<Value = Manifest> {
    prop::collection::vec(g::arb_manifest_entry(), 0..8).prop_map(|entries| {
        let entries = canonicalize(entries);
        Manifest {
            manifest_digest: ContentAddressing::manifest_digest(&entries),
            entries,
        }
    })
}

/// add/modify/delete 로 상관된 두 digest-consistent `Manifest`(prev, current) 쌍.
///
/// base 엔트리를 경로 유일화한 뒤 항목별 액션(keep/modify/delete)을 적용하고 신규 엔트리를 추가해
/// current 를 만든다. 두 매니페스트 모두 실제 `manifest_digest` 를 계산한다.
pub fn arb_manifest_pair() -> impl Strategy<Value = (Manifest, Manifest)> {
    (
        prop::collection::vec((g::arb_manifest_entry(), 0u8..3), 0..8),
        prop::collection::vec(g::arb_manifest_entry(), 0..4),
        g::arb_sha256_digest(),
    )
        .prop_map(|(base_specs, additions, alt_hash)| {
            let mut seen: BTreeSet<String> = BTreeSet::new();
            let mut prev_entries: Vec<ManifestEntry> = Vec::new();
            let mut current_entries: Vec<ManifestEntry> = Vec::new();

            for (entry, action) in base_specs {
                if !seen.insert(entry.relative_path.as_str().to_string()) {
                    continue; // 경로 유일성 보장.
                }
                prev_entries.push(entry.clone());
                match action {
                    0 => current_entries.push(entry), // keep
                    1 => {
                        // modify: 해시를 교체(원본과 같으면 no-change 로 취급되어도 오라클 성립).
                        let mut modified = entry;
                        modified.raw_sha256 = alt_hash;
                        current_entries.push(modified);
                    }
                    _ => {} // delete: current 에서 제외
                }
            }

            for addition in additions {
                if seen.insert(addition.relative_path.as_str().to_string()) {
                    current_entries.push(addition);
                }
            }

            let prev_entries = canonicalize(prev_entries);
            let current_entries = canonicalize(current_entries);
            let prev = Manifest {
                manifest_digest: ContentAddressing::manifest_digest(&prev_entries),
                entries: prev_entries,
            };
            let current = Manifest {
                manifest_digest: ContentAddressing::manifest_digest(&current_entries),
                entries: current_entries,
            };
            (prev, current)
        })
}

/// 엔트리를 canonical 정렬하고 경로 유일화한다(U0 `Manifest` 불변식).
fn canonicalize(mut entries: Vec<ManifestEntry>) -> Vec<ManifestEntry> {
    entries.sort();
    entries.dedup_by(|a, b| a.relative_path == b.relative_path);
    entries
}

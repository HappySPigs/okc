//! U3 도메인 제약 준수 `proptest` 제너레이터.
//!
//! U0 제너레이터(`foundation::proptest_support::generators`)를 재사용해 `Manifest`/`Sha256Digest`/
//! `ByteCount` 를 조합하고, U3 프로토콜/청크 속성(PROP-U3-01..05)의 제너레이터를 제공한다(빈/다수·
//! 경계값 포괄). 상태머신 속성(PROP-U3-06..08)은 fake seam(테스트 크레이트 측)으로 구동한다.

use std::collections::BTreeSet;

use foundation::Sha256Digest;
use foundation::proptest_support::generators as fg;
use proptest::prelude::*;

/// 빈/작은/경계 크기를 포괄하는 임의 blob 바이트열을 생성한다(청크 분할·재조립 대상).
pub fn arb_blob_bytes() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..512)
}

/// 경계(1) 및 다양한 청크 크기를 포괄하는 고정 `chunk_size` 후보를 생성한다.
pub fn arb_chunk_size() -> impl Strategy<Value = u64> {
    prop::sample::select(vec![1u64, 3, 7, 32, 64, 128, 512, 4096])
}

/// 빈/다수 해시를 포괄하는 임의 `Sha256Digest` 정렬 집합을 생성한다(`server_has` 대상).
pub fn arb_digest_set() -> impl Strategy<Value = BTreeSet<Sha256Digest>> {
    prop::collection::btree_set(fg::arb_sha256_digest(), 0..6)
}

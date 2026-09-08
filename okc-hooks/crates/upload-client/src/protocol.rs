//! Protocol — U3 도입 값 타입(협상/커밋 봉투·청크 프레이밍)과 순수 프로토콜 함수.
//!
//! 프로토콜 봉투는 okc-web `/api/sync` 수신 계약(DEP-03)이며,
//! 본문 바이트는 U0 CBOR 코덱(`encode`/`decode`)으로 직렬화되어 `OkcRequest.body`/`OkcResponse.body`
//! 에 실린다. 이 모듈의 순수 함수(`compute_want`/`plan_chunks`/`frame_chunks`/`reassemble`)는 전송과
//! 분리되어 PBT(PROP-U3-01..05)를 transport 없이 검증할 수 있게 한다.
//!
//! 순수 리프 모듈로서 panic-free-total 을 컴파일타임 clippy lint-gate 로 강제한다.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::{BTreeMap, BTreeSet};

use content_core::ContentAddressing;
use foundation::{ByteCount, Manifest, ManifestDigest, RelativePath, Sha256Digest};
use serde::{Deserialize, Serialize};

/// 협상 요청 엔드포인트 상대경로.
pub const NEGOTIATE_PATH: &str = "/negotiate";
/// 커밋 요청 엔드포인트 상대경로.
pub const COMMIT_PATH: &str = "/commit";
/// blob 청크 전송 엔드포인트 상대경로 접두(`{BLOB_PATH}/{blob}/{offset}`).
pub const BLOB_PATH: &str = "/blob";

// ---------------------------------------------------------------------------
// U3 도입 값 타입
// ---------------------------------------------------------------------------

/// 서버가 아직 보유하지 않아 전송해야 하는 blob 의 `raw_sha256` 집합(have/want 협상 산출).
///
/// `BTreeSet`(정렬 집합)으로 순회·round-trip 이 결정적이다(R-WANT-01, MVP). 다중 경로 동일 콘텐츠는
/// 하나의 해시로 dedup 된다. 불변식: `blobs ⊆ { e.raw_sha256 | e ∈ manifest.entries }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WantSet {
    /// 전송 대상 blob 해시(정렬 집합).
    pub blobs: BTreeSet<Sha256Digest>,
}

/// 커밋이 참조하는 권위 있는 경로->콘텐츠 해시 바인딩(Q6=C). `BTreeMap`(경로 오름차순 canonical).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathHashMap {
    /// `(relative_path -> raw_sha256)` canonical 정렬 맵.
    pub entries: BTreeMap<RelativePath, Sha256Digest>,
}

/// 협상 요청 — 매니페스트를 서버에 제시한다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NegotiateRequest {
    /// 로컬 no-op 판별자(참고용, 권위 아님).
    pub manifest_digest: ManifestDigest,
    /// `(경로, 해시, 크기)` 3-튜플 투영.
    pub entries: Vec<(RelativePath, Sha256Digest, ByteCount)>,
}

/// 협상 응답 — 서버 보유 blob, 세션 identity, 권위 재개 오프셋.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NegotiateResponse {
    /// 서버 보유 blob 해시 집합.
    pub server_has: BTreeSet<Sha256Digest>,
    /// 서버가 현재 source revision에 바인딩한 업로드 세션.
    #[serde(default)]
    pub session_id: Option<String>,
    /// 세션이 실제 보유한 blob별 연속 prefix 길이(누락 blob은 0).
    #[serde(default)]
    pub resume_offsets: Vec<(Sha256Digest, ByteCount)>,
}

/// 커밋 요청 — 권위 경로->해시 맵 + no-op 판별자.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitRequest {
    /// 권위 경로->해시 맵.
    pub path_hash_map: PathHashMap,
    /// 로컬 no-op 판별자(참고용).
    pub manifest_digest: ManifestDigest,
}

/// 서버 권위 볼트 콘텐츠 식별자(불투명, FQ-1=A) — 로컬 판정에 쓰지 않는다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VaultContentId(pub String);

/// 커밋 결과 — 서버 권위 식별자(있으면) + 실제 커밋 여부(`false` = 서버 no-op, R-NOOP-02).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitOutcome {
    /// 서버 권위 `vault_content_id`(있으면; 로컬 판정 미참여).
    pub server_vault_content_id: Option<VaultContentId>,
    /// 실제 커밋 여부(`false` = 이미 존재 -> 서버 no-op).
    pub committed: bool,
}

/// 재개 청크 분할 계획 — `start_offset` 부터 `total` 까지 `chunk_size` 로 분할한다(마지막 `<=`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkPlan {
    /// 대상 blob 해시.
    pub blob: Sha256Digest,
    /// blob 총 바이트.
    pub total: ByteCount,
    /// 고정 청크 크기(주입 config, 적응형 없음, D-07).
    pub chunk_size: ByteCount,
    /// 재개 시작 오프셋(= 재개 오프셋 또는 0).
    pub start_offset: ByteCount,
}

impl ChunkPlan {
    /// `[start_offset, total)` 를 `chunk_size` 로 분할한 `(offset, len)` 시퀀스를 반환한다.
    pub fn segments(&self) -> Vec<(u64, u64)> {
        plan_chunks(
            self.total.get(),
            self.chunk_size.get(),
            self.start_offset.get(),
        )
    }
}

/// 오프셋·길이·청크별 무결성 해시(FR-08)를 동반한 전송 프레임.
///
/// 오프셋 오름차순으로 `bytes` 를 concat 하면 원본 blob 바이트열을 재구성한다(R-CHUNK-01, NFR-10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkFrame {
    /// 대상 blob 해시.
    pub blob: Sha256Digest,
    /// 이 프레임이 담는 구간의 시작 오프셋.
    pub offset: ByteCount,
    /// 이 프레임 바이트 길이.
    pub len: ByteCount,
    /// 청크 바이트의 표준 SHA-256(per-chunk integrity, FR-08).
    pub chunk_sha256: Sha256Digest,
    /// 청크 바이트열.
    pub bytes: Vec<u8>,
}

// ---------------------------------------------------------------------------
// 순수 프로토콜 함수 (transport 무관 — PROP-U3-01..05)
// ---------------------------------------------------------------------------

/// 매니페스트 blob 해시 집합에서 서버 보유분을 뺀 **정확한 차집합**을 계산한다(R-WANT-01).
///
/// `WantSet.blobs == manifest_hashes \ server_has`. `server_has ⊇ manifest_hashes` 이면 공집합
/// (멱등, R-WANT-02). 전송 없이 검증되는 순수 함수다(PROP-U3-01/02).
pub fn compute_want(
    manifest_hashes: &BTreeSet<Sha256Digest>,
    server_has: &BTreeSet<Sha256Digest>,
) -> WantSet {
    WantSet {
        blobs: manifest_hashes.difference(server_has).copied().collect(),
    }
}

/// 매니페스트 엔트리의 `raw_sha256` 을 정렬 집합으로 투영한다(협상 입력·dedup).
pub fn manifest_hashes(manifest: &Manifest) -> BTreeSet<Sha256Digest> {
    manifest
        .entries
        .iter()
        .map(|entry| entry.raw_sha256)
        .collect()
}

/// 매니페스트를 협상 요청 엔트리(`(경로, 해시, 크기)`)로 투영한다.
pub fn negotiate_entries(manifest: &Manifest) -> Vec<(RelativePath, Sha256Digest, ByteCount)> {
    manifest
        .entries
        .iter()
        .map(|entry| (entry.relative_path.clone(), entry.raw_sha256, entry.size))
        .collect()
}

/// 매니페스트를 커밋용 권위 경로->해시 맵으로 투영한다(BTreeMap canonical 정렬, R-COMMIT-01).
pub fn path_hash_map(manifest: &Manifest) -> PathHashMap {
    PathHashMap {
        entries: manifest
            .entries
            .iter()
            .map(|entry| (entry.relative_path.clone(), entry.raw_sha256))
            .collect(),
    }
}

/// `[start_offset, total)` 를 고정 `chunk_size` 로 분할한 `(offset, len)` 시퀀스를 계산한다.
///
/// `chunk_size == 0` 은 `1` 로 보정하고(패닉 회피, MVP), `start_offset > total` 은 빈 결과를 낸다.
/// 마지막 청크만 `<= chunk_size`. 오프셋은 항상 오름차순·연속이다(R-CHUNK-01/02).
pub fn plan_chunks(total: u64, chunk_size: u64, start_offset: u64) -> Vec<(u64, u64)> {
    let step = chunk_size.max(1);
    let mut segments = Vec::new();
    let mut offset = start_offset.min(total);
    while offset < total {
        let len = step.min(total - offset);
        segments.push((offset, len));
        offset += len;
    }
    segments
}

/// 전체 blob 바이트에서 `[start_offset, total)` 구간의 청크 프레임 시퀀스를 만든다(각 프레임에
/// 청크별 SHA-256 동반). `bytes` 는 전체 blob 이며 프레임 `offset` 은 그 안의 절대 오프셋이다.
pub fn frame_chunks(
    blob: Sha256Digest,
    bytes: &[u8],
    chunk_size: u64,
    start_offset: u64,
) -> Vec<ChunkFrame> {
    let total = bytes.len() as u64;
    plan_chunks(total, chunk_size, start_offset)
        .into_iter()
        .filter_map(|(offset, len)| {
            // `get(..)` 는 인덱싱 연산자를 쓰지 않아 lint-gate 를 통과하며, plan_chunks 가
            // `offset + len <= total` 을 보장하므로 항상 `Some` 이다.
            let slice = bytes.get(offset as usize..(offset + len) as usize)?;
            // `&[u8]` 읽기는 무오류이므로 `ok()?` 는 항상 `Some`(스트리밍 SHA-256, U1 재사용).
            let chunk_sha256 = ContentAddressing::hash_stream(slice).ok()?;
            Some(ChunkFrame {
                blob,
                offset: ByteCount::new(offset),
                len: ByteCount::new(len),
                chunk_sha256,
                bytes: slice.to_vec(),
            })
        })
        .collect()
}

/// 청크 프레임을 `offset` 오름차순으로 concat 해 바이트열을 재구성한다(R-CHUNK-01 재조립).
pub fn reassemble(frames: &[ChunkFrame]) -> Vec<u8> {
    let mut sorted: Vec<&ChunkFrame> = frames.iter().collect();
    sorted.sort_by_key(|frame| frame.offset.get());
    let mut out = Vec::new();
    for frame in sorted {
        out.extend_from_slice(&frame.bytes);
    }
    out
}

#![deny(missing_docs)]
//! `upload-client` 크레이트 (U3) — okc-hooks Watcher 의 **업로드 프로토콜 클라이언트**.
//!
//! 유일 컴포넌트 `UploadProtocolDriver` 가 트리거당 **단일 직렬 사이클**(Q2=B)로 다단계 업로드
//! 프로토콜을 구동한다: have/want 협상(`raw_sha256` 차집합) -> 서버 결여 blob 순차 전송(임계값 `S`
//! 이하 단일 / 초과·재개는 재개 가능 청크) -> **전송 직전 재-읽기+재-해시 재검증**(불일치 시 커밋
//! 중단, Q8=A) -> 경로->해시 맵 + `manifest_digest` 커밋. digest 가 마지막 커밋과 동일하면 서버
//! 왕복 없이 no-op 조기 종료하며(FR-10), 매 사이클 런타임 SafetyLimits 를 재검사한다(US-E2-08).
//! 재시도/백오프(sleep)는 하지 않고 `UploadError` 를 상위(U8/U4)에 반환한다(drive-not-sleep, D-16).
//!
//! W3 소비 단위로서 상류 크레이트의 **공개 API 만 소비**하며 수정하지 않는다:
//! - U0 `foundation`: 값 타입·오류 taxonomy·CBOR 코덱(`encode`/`decode`)·`StatusSink` 계약.
//! - U1 `content-core`: `ContentAddressing::hash_stream`(재검증) · `SafetyLimitsValidator`(한도 재검사).
//! - U4 `sync-state`: `SyncStateStore`(재개 오프셋·마지막 커밋·커밋 지속) — `SyncStore` seam 경유.
//! - U5 `auth-consent`: `AuthTransport::send`(유일 아웃바운드 HTTP 경로).
//!
//! HTTP·blob-source·store IO 는 seam(`AuthTransport` 의 하위 `HttpTransport` · `BlobSource` ·
//! `SyncStore`)으로 주입되어 테스트가 네트워크/파일시스템/권한 없이 프로토콜을 구동한다. 서버
//! 프로토콜 미확정 구간([blocked-on-server], DEP-03)은 잠정 mock 계약으로 표현한다(`protocol` 모듈).
//!
//! 규약: 모든 doc 주석은 한국어, 코드 내 화살표는 ASCII `A -> B` 만 사용,
//! Rust 타입/식별자는 doc 주석에서 백틱으로 감싼다(`Box<dyn Read>`, `Result<CommitOutcome, UploadError>`).

/// UploadProtocolDriver: 다단계 업로드 프로토콜 상태머신 + `CyclePhase` 전이.
pub mod driver;

/// Protocol: U3 도입 값 타입(협상/커밋 봉투·청크 프레이밍) + 순수 프로토콜 함수.
pub mod protocol;

/// ErrorTaxonomy: U3 사이클 실패 오류 `UploadError`(taxonomy 보존 반환).
pub mod error;

/// BlobSource: 전송 전 재검증·전송용 blob 재-읽기 seam(도메인 포트).
pub mod blob_source;

/// SyncStore: 재개 오프셋·마지막 커밋·커밋 지속 seam(U4 `SyncStateStore` 어댑터).
pub mod store;

/// ProptestGenerators: U3 도메인 제너레이터(test-support, 런타임 그래프 밖).
///
/// 비-default cargo feature `proptest-support` 뒤에 게이트되어 릴리스 빌드에서 제외된다.
#[cfg(feature = "proptest-support")]
pub mod proptest_support;

// ---------------------------------------------------------------------------
// 공개 API 표면(crate-root 재-export). 재-export 된 아이템은 원본 doc 을 승계한다.
// ---------------------------------------------------------------------------

pub use blob_source::{BlobSource, BlobSourceError};
pub use driver::{CyclePhase, CycleReport, UploadProtocolDriver};
pub use error::UploadError;
pub use protocol::{
    BLOB_PATH, COMMIT_PATH, ChunkFrame, ChunkPlan, CommitOutcome, CommitRequest, NEGOTIATE_PATH,
    NegotiateRequest, NegotiateResponse, PathHashMap, VaultContentId, WantSet, compute_want,
    frame_chunks, manifest_hashes, negotiate_entries, path_hash_map, plan_chunks, reassemble,
};
pub use store::SyncStore;

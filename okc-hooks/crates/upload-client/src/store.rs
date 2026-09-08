//! SyncStore — 재개 오프셋 read/write · 마지막 커밋 기준선 · 커밋 지속 seam (U4 어댑터).
//!
//! U4 `SyncStateStore` 의 mutator 는 `std::fs` 원자 쓰기를 수행하므로, 드라이버 로직을
//! 파일시스템 없이 단위/property 테스트하기 위해 U3 가 소유하는 최소 조회/mutate 트레이트를 둔다
//! (BlobSource 와 동형 seam). 프로덕션 배선은 `SyncStateStore` 에 대한 impl 로 제공되며(아래),
//! U4 `StateError` 를 U3 `UploadError::Aborted` 로 사상한다(재시도 분류는 상위 U8/U4 소관).
//!
//! **MVP 결정(FD 대비 편차)**: `domain-entities.md` §3 은 `SyncStateStore` 를 concrete 주입으로
//! 열거하나, 유일 공개 생성자(`open_and_recover`)가 실제 fs IO 를 요구하므로 property 테스트의
//! 격리·속도를 위해 seam 트레이트로 감싼다(프롬프트의 "OS/IO seam + fake" 지침 준수). 또한
//! 트레이트 반환은 `Option<&Manifest>` 대신 소유값 `Option<Manifest>` 로 두어 interior-mutability
//! fake(Arc<Mutex>)가 no-op digest 비교에 참여할 수 있게 한다(매니페스트 1개 clone, 비용 무시가능).
//!
//! IO 어댑터 모듈이므로 순수 lint-gate 를 두지 않는다(단, 패닉 경로 없음).

use foundation::{ByteCount, Manifest, Sha256Digest};
use sync_state::{StateError, SyncStateStore};

use crate::error::UploadError;

/// 재개 오프셋·마지막 커밋·커밋 지속을 추상화하는 seam(단일 writer 전제, FR-23/Q2=B).
pub trait SyncStore: Send + Sync {
    /// 마지막으로 커밋된 매니페스트(no-op digest 비교 기준선, 없으면 `None`)를 소유값으로 반환한다.
    fn last_committed_manifest(&self) -> Option<Manifest>;

    /// 지정 blob(`raw_sha256` keying, R-RESUME-01)의 진행 중 재개 오프셋(없으면 `None` -> 0).
    fn resume_offset(&self, blob: &Sha256Digest) -> Option<ByteCount>;

    /// 청크 ack 마다 진행 오프셋을 지속한다(R-RESUME-02). I/O 실패는 `UploadError::Aborted`.
    fn persist_resume_offset(&mut self, blob: Sha256Digest, off: ByteCount)
    -> Result<(), UploadError>;

    /// 커밋 성공 시 `{ last_committed=m, dirty=false, resume_offsets={} }` 를 한 번의 원자 쓰기로
    /// 반영한다(R-RESUME-03: stale 오프셋 clear 흡수). I/O 실패는 `UploadError::Aborted`.
    fn commit_manifest(&mut self, manifest: Manifest) -> Result<(), UploadError>;
}

/// 프로덕션 배선: U4 `SyncStateStore` 를 seam 으로 노출한다(`StateError -> UploadError::Aborted`).
impl SyncStore for SyncStateStore {
    fn last_committed_manifest(&self) -> Option<Manifest> {
        SyncStateStore::last_committed_manifest(self).cloned()
    }

    fn resume_offset(&self, blob: &Sha256Digest) -> Option<ByteCount> {
        SyncStateStore::resume_offset(self, blob)
    }

    fn persist_resume_offset(
        &mut self,
        blob: Sha256Digest,
        off: ByteCount,
    ) -> Result<(), UploadError> {
        SyncStateStore::persist_resume_offset(self, blob, off).map_err(map_state_error)
    }

    fn commit_manifest(&mut self, manifest: Manifest) -> Result<(), UploadError> {
        SyncStateStore::commit_manifest(self, manifest).map_err(map_state_error)
    }
}

/// U4 `StateError`(I/O·직렬화·손상 복구 신호)를 U3 사이클 중단으로 사상한다.
///
/// 재시도 여부 분류(`ErrorClass`)는 U3 가 재계산하지 않는다(관심사 분리, R-ERR-01) — 상위 U8 이
/// 다음 트리거 사이클에서 재스냅샷·재시도한다.
fn map_state_error(_error: StateError) -> UploadError {
    UploadError::Aborted
}

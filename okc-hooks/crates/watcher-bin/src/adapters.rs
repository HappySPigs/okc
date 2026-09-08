//! 하위 크레이트의 공개 API 를 `SyncCycleCoordinator` 의 seam(port)에 연결하는 얇은 어댑터.
//!
//! 코디네이터는 여기 정의된 6개 port 트레이트(`RunStatePort`/`AvailabilityGuard`/`ManifestSource`/
//! `ConsentPort`/`CycleDriver`/`CoordinatorStore`)에만 의존한다 — 프로덕션은 하위 동결 크레이트의
//! 실체 타입에 이 port 를 구현하고(예: `RunStatePort for RunStateController`), property 테스트는
//! 동일 port 를 기록 fake 로 구현해 스레드/네트워크/파일시스템 없이 사이클 분기를 검증한다(PBT-01).
//! 세 concrete 어댑터(`SharedSyncStore`/`VaultBlobSource`/`VaultManifestSource`)는 U3 seam 계약
//! (`SyncStore`/`BlobSource`)과 port 를 하위 크레이트 위에 얇게 배선한다(신규 도메인 로직 없음).
//!
//! 순수-이하 배선 표면으로서 panic-free 를 컴파일타임으로 강제한다(어떤 경로에서도
//! `unwrap`/`expect`/`panic`/인덱싱을 쓰지 않는다).
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use auth_consent::{ConsentDecision, ConsentGate};
use change_detect::{Availability, GuardVerdict, VaultAvailabilityGuard};
use content_core::{ManifestBuilder, VaultScanner};
use foundation::{ByteCount, Manifest, RelativePath, Sha256Digest};
use ops_control::{RunState, RunStateController};
use sync_state::SyncStateStore;
use upload_client::{
    BlobSource, BlobSourceError, CycleReport, SyncStore, UploadError, UploadProtocolDriver,
};

use content_core::ScanError;

/// 매니페스트 재계산 실패 사유(진단용 문자열 보존).
#[derive(Debug, thiserror::Error)]
pub enum ManifestSourceError {
    /// `ManifestBuilder::build` 실패(scan/hash) — 상세는 진단용.
    #[error("볼트 매니페스트 재계산 실패: {0}")]
    Build(String),
}

/// 공유 상태 저장소 접근 실패 사유(진단용 문자열 보존).
#[derive(Debug, thiserror::Error)]
pub enum StoreAdapterError {
    /// U4 `SyncStateStore` 원자 쓰기 실패.
    #[error("공유 상태 저장소 접근 실패: {0}")]
    State(String),
}

/// run-state 읽기 전용 port — 코디네이터가 pause/stop 신호를 관측한다(no back-reference, US-E6-03).
pub trait RunStatePort: Send + Sync {
    /// 현재 run-state 스냅샷을 반환한다(mode + 휘발성 신호).
    fn current(&self) -> RunState;
}

/// 볼트 가용성 분류 + 파괴적-빈-커밋 가드 판정 port(U2 순수 판정 위임).
pub trait AvailabilityGuard: Send + Sync {
    /// 볼트 루트 도달 가능성을 사전 점검한다.
    fn check_reachable(&self, root: &Path) -> Availability;

    /// 재계산 매니페스트/마지막 커밋/가용성을 결합해 진행/보류 판정을 낸다.
    fn guard_diff(
        &self,
        new_manifest: &Manifest,
        last_committed: Option<&Manifest>,
        availability: Availability,
    ) -> GuardVerdict;
}

/// 현재 볼트 매니페스트 재계산 port(U1 scan + hash 위임).
pub trait ManifestSource: Send + Sync {
    /// 폴더를 진실 원천으로 현재 매니페스트를 재계산한다(fail-fast).
    fn rebuild(&self) -> Result<Manifest, ManifestSourceError>;
}

/// 업로드 동의 게이트 판정 port(U5 `ConsentGate` 위임).
pub trait ConsentPort: Send + Sync {
    /// 업로드 허용 여부를 판정한다(부수효과: `ConsentBlocked` 조건 push 는 구현이 소유).
    fn is_upload_permitted(&self) -> ConsentDecision;
}

/// 업로드 프로토콜 사이클 구동 port(U3 `UploadProtocolDriver` 위임).
pub trait CycleDriver {
    /// 한 사이클을 실행하고 밟은 단계 추적을 함께 반환한다.
    fn execute_cycle_traced(&mut self, manifest: &Manifest) -> CycleReport;
}

/// 코디네이터가 직접 읽고/dirty 표시하는 공유 상태 저장소 port(U4 `SyncStateStore` 위임).
pub trait CoordinatorStore: Send + Sync {
    /// 마지막 커밋 매니페스트(diff/guard 기준선)를 소유값으로 반환한다.
    fn last_committed_manifest(&self) -> Option<Manifest>;

    /// 미커밋 변경을 `dirty` 로 표시하고 원자 지속한다(커밋 전 유실 방지).
    fn mark_dirty(&self) -> Result<(), StoreAdapterError>;
}

// ---------------------------------------------------------------------------
// 프로덕션 port 구현 — 하위 동결 크레이트 실체 타입에 얇게 위임(로컬 트레이트, orphan-rule OK).
// ---------------------------------------------------------------------------

impl RunStatePort for RunStateController {
    fn current(&self) -> RunState {
        RunStateController::current(self)
    }
}

impl AvailabilityGuard for VaultAvailabilityGuard {
    fn check_reachable(&self, root: &Path) -> Availability {
        VaultAvailabilityGuard::check_reachable(self, root)
    }

    fn guard_diff(
        &self,
        new_manifest: &Manifest,
        last_committed: Option<&Manifest>,
        availability: Availability,
    ) -> GuardVerdict {
        VaultAvailabilityGuard::guard_diff(self, new_manifest, last_committed, availability)
    }
}

impl ConsentPort for ConsentGate {
    fn is_upload_permitted(&self) -> ConsentDecision {
        ConsentGate::is_upload_permitted(self)
    }
}

impl CycleDriver for UploadProtocolDriver {
    fn execute_cycle_traced(&mut self, manifest: &Manifest) -> CycleReport {
        UploadProtocolDriver::execute_cycle_traced(self, manifest)
    }
}

// ---------------------------------------------------------------------------
// SharedSyncStore — `Arc<Mutex<SyncStateStore>>` 공유 핸들(DEC-U8-04).
//
// 드라이버에는 `Box<dyn SyncStore>` 로, 코디네이터에는 `Arc<dyn CoordinatorStore>` 로 같은 상태
// 저장소를 공유 주입한다. 단일 직렬 사이클이라 락 경합이 없다.
// ---------------------------------------------------------------------------

/// U4 `SyncStateStore` 를 공유 가능한 핸들로 감싸 U3 seam + 코디네이터 port 를 동시에 충족한다.
pub struct SharedSyncStore {
    inner: Arc<Mutex<SyncStateStore>>,
}

impl SharedSyncStore {
    /// 공유 상태 저장소 핸들로 어댑터를 구성한다.
    pub fn new(store: Arc<Mutex<SyncStateStore>>) -> Self {
        SharedSyncStore { inner: store }
    }

    /// 뮤텍스를 취득한다. poison 시 패닉 대신 내부 상태를 회수한다(회복력, 하위 크레이트 관례).
    fn lock(&self) -> MutexGuard<'_, SyncStateStore> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl CoordinatorStore for SharedSyncStore {
    fn last_committed_manifest(&self) -> Option<Manifest> {
        self.lock().last_committed_manifest().cloned()
    }

    fn mark_dirty(&self) -> Result<(), StoreAdapterError> {
        self.lock()
            .mark_dirty()
            .map_err(|err| StoreAdapterError::State(err.to_string()))
    }
}

impl SyncStore for SharedSyncStore {
    fn last_committed_manifest(&self) -> Option<Manifest> {
        self.lock().last_committed_manifest().cloned()
    }

    fn resume_offset(&self, blob: &Sha256Digest) -> Option<ByteCount> {
        self.lock().resume_offset(blob)
    }

    fn persist_resume_offset(
        &mut self,
        blob: Sha256Digest,
        off: ByteCount,
    ) -> Result<(), UploadError> {
        self.lock()
            .persist_resume_offset(blob, off)
            .map_err(|_| UploadError::Aborted)
    }

    fn commit_manifest(&mut self, manifest: Manifest) -> Result<(), UploadError> {
        self.lock()
            .commit_manifest(manifest)
            .map_err(|_| UploadError::Aborted)
    }
}

// ---------------------------------------------------------------------------
// VaultBlobSource — U1 `VaultScanner::open_reader` 를 U3 `BlobSource` 로 배선(DEC-U8-05).
// ---------------------------------------------------------------------------

/// 볼트 루트 `VaultScanner` 를 U3 `BlobSource`(전송 전 재-읽기 seam)로 얇게 구현한다.
pub struct VaultBlobSource {
    scanner: VaultScanner,
}

impl VaultBlobSource {
    /// 볼트 루트 경로로 blob 소스를 구성한다.
    pub fn new(root: PathBuf) -> Self {
        VaultBlobSource {
            scanner: VaultScanner::new(root),
        }
    }
}

impl BlobSource for VaultBlobSource {
    fn open(&self, path: &RelativePath) -> Result<Box<dyn Read>, BlobSourceError> {
        self.scanner.open_reader(path).map_err(map_scan_error)
    }
}

/// U1 `ScanError` 를 U3 `BlobSourceError` 로 사상한다(루트 소실 -> `NotFound`, I/O -> `Io`).
fn map_scan_error(error: ScanError) -> BlobSourceError {
    match error {
        ScanError::RootUnavailable => BlobSourceError::NotFound,
        ScanError::Io { source, .. } => BlobSourceError::Io(source.to_string()),
    }
}

// ---------------------------------------------------------------------------
// VaultManifestSource — U1 scan + hash 를 코디네이터 `ManifestSource` port 로 배선.
// ---------------------------------------------------------------------------

/// 볼트 루트를 스캔·해시해 매니페스트를 재계산하는 `ManifestSource` 구현체.
pub struct VaultManifestSource {
    scanner: VaultScanner,
}

impl VaultManifestSource {
    /// 볼트 루트 경로로 매니페스트 소스를 구성한다.
    pub fn new(root: PathBuf) -> Self {
        VaultManifestSource {
            scanner: VaultScanner::new(root),
        }
    }
}

impl ManifestSource for VaultManifestSource {
    fn rebuild(&self) -> Result<Manifest, ManifestSourceError> {
        ManifestBuilder::new(&self.scanner)
            .build()
            .map_err(|err| ManifestSourceError::Build(err.to_string()))
    }
}

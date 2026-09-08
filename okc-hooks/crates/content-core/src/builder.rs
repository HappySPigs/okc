//! ManifestBuilder — scan + hash 조립 -> 재계산된 `Manifest` (fail-fast).
//!
//! `VaultScanner.scan` -> 파일별 `ContentAddressing::hash_stream` -> `ManifestEntry` -> canonical
//! 정렬 -> `manifest_digest` -> `Manifest` 를 조립한다. 어느 파일이라도 실패하면 전체를 중단하고
//! `BuildError` 를 반환한다(부분 매니페스트 없음, R-BUILD-01/D12). 파일은 순차 스트리밍되어 동시
//! 다수 파일 바이트가 적재되지 않는다(R-BUILD-02). 산출 `Manifest` 는 U0 불변식
//! `manifest_digest == digest(canonical(entries))` 를 만족한다(R-BUILD-04).
//!
//! 조립부는 순수하나 `VaultScanner` 경유 I/O 를 오케스트레이션하므로 §6 lint-gate 를 걸지 않는다
//! (U0 loader 관례). unwrap/expect/인덱싱/패닉을 사용하지 않는다.

use foundation::{Manifest, ManifestEntry};

use crate::content_addressing::ContentAddressing;
use crate::error::{BuildError, ScanError};
use crate::scanner::VaultScanner;

/// 주입된 `VaultScanner` 를 구동해 현재 볼트 상태의 `Manifest` 를 재계산하는 빌더.
pub struct ManifestBuilder<'a> {
    scanner: &'a VaultScanner,
}

impl<'a> ManifestBuilder<'a> {
    /// 스캐너 참조로 빌더를 구성한다(U8 조립 시 주입).
    pub fn new(scanner: &'a VaultScanner) -> Self {
        ManifestBuilder { scanner }
    }

    /// 볼트를 스캔·해시해 canonical `Manifest` 를 조립한다(fail-fast).
    ///
    /// 스캔 실패는 `BuildError::Scan`, 파일 리더 열기/해시 실패는 `BuildError::Hash { path, source }`
    /// 로 반환하며 부분 매니페스트를 만들지 않는다.
    pub fn build(&self) -> Result<Manifest, BuildError> {
        let snapshot = self.scanner.scan().map_err(BuildError::Scan)?;
        let mut entries: Vec<ManifestEntry> = Vec::with_capacity(snapshot.files.len());
        for file in &snapshot.files {
            let reader = self
                .scanner
                .open_reader(&file.relative_path)
                .map_err(open_error_to_build)?;
            let raw_sha256 = ContentAddressing::hash_stream(reader).map_err(|source| {
                BuildError::Hash {
                    path: file.relative_path.clone(),
                    source,
                }
            })?;
            entries.push(ManifestEntry {
                relative_path: file.relative_path.clone(),
                raw_sha256,
                size: file.size,
            });
        }
        // snapshot 이 이미 정렬되어 있으나 canonical 형상을 재확인한다(R-BUILD-04).
        entries.sort();
        let manifest_digest = ContentAddressing::manifest_digest(&entries);
        Ok(Manifest {
            entries,
            manifest_digest,
        })
    }
}

/// 리더 열기 단계의 `ScanError` 를 `BuildError` 로 사상한다.
///
/// 파일 I/O(`Io`)는 `Hash { path, source }` 로, 루트 소실(`RootUnavailable`)은 `Scan` 으로 전달한다.
fn open_error_to_build(error: ScanError) -> BuildError {
    match error {
        ScanError::Io { path, source } => BuildError::Hash { path, source },
        ScanError::RootUnavailable => BuildError::Scan(ScanError::RootUnavailable),
    }
}

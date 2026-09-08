//! VaultScanner — 볼트 루트 열거 + 파일별 스트리밍 리더 (U1 유일 파일 I/O).
//!
//! `std::fs` 만 사용한다(신규 walk 크레이트 없음, T2). 심링크 SKIP(R-SCAN-02), dotfile 포함
//! (R-SCAN-03), exclude 패턴 없음(R-SCAN-04). 루트 도달 불가 시 `ScanError::RootUnavailable` 값
//! 반환(R-SCAN-05, 0-파일 매니페스트 미생성). 출력은 `relative_path` 오름차순 canonical 정렬(R-SCAN-06).
//!
//! 순수 표면이 아니므로 §6 clippy lint-gate 대상은 아니나(U0 loader/store 관례), 실패는 오직
//! `ScanError` `Result` 로만 표면화하며 패닉하지 않는다.

use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use foundation::{ByteCount, RelativePath, Timestamp};

use crate::error::ScanError;
use crate::types::{ScannedFile, VaultSnapshot};

/// 주입된 볼트 루트를 열거·리드하는 스캐너. 생성자 주입(config 직접 읽기 없음, U8 조립).
pub struct VaultScanner {
    root: PathBuf,
}

impl VaultScanner {
    /// 볼트 루트 경로로 스캐너를 구성한다(U8 composition root 가 해소된 절대 경로를 주입).
    pub fn new(root: PathBuf) -> Self {
        VaultScanner { root }
    }

    /// 볼트 루트를 단일 패스 재귀 walk 로 열거해 canonical 정렬된 `VaultSnapshot` 을 산출한다.
    ///
    /// 심링크는 따라가지 않고 제외하며(R-SCAN-02), dotfile 은 포함한다(R-SCAN-03). 루트에
    /// 도달할 수 없으면 `ScanError::RootUnavailable` 을 반환하고 0-파일 스냅샷을 만들지 않는다
    /// (R-SCAN-05). 하위 항목 I/O 오류는 `ScanError::Io { path, source }` 로 반환한다.
    pub fn scan(&self) -> Result<VaultSnapshot, ScanError> {
        // 루트 도달 가능성 확인(R-SCAN-05): read_dir 실패 시 RootUnavailable.
        if fs::read_dir(&self.root).is_err() {
            return Err(ScanError::RootUnavailable);
        }
        let mut files: Vec<ScannedFile> = Vec::new();
        self.walk(&self.root, &mut files)?;
        // R-SCAN-06: 파일시스템 walk 순서와 무관하게 결정적 정렬 출력.
        files.sort();
        Ok(VaultSnapshot {
            root: self.root.clone(),
            files,
            captured_at: Timestamp::from_unix_nanos(now_unix_nanos()),
        })
    }

    /// 스캔된 상대경로에 대한 버퍼드 스트리밍 리더(`Box<dyn Read>`)를 연다.
    ///
    /// `hash_stream` 이 전량 적재 없이 소비한다. I/O 실패는 `ScanError::Io { path, source }`.
    pub fn open_reader(&self, path: &RelativePath) -> Result<Box<dyn Read>, ScanError> {
        let full = self.root.join(path.as_str());
        let file = File::open(&full).map_err(|source| ScanError::Io {
            path: path.clone(),
            source,
        })?;
        Ok(Box::new(BufReader::new(file)))
    }

    /// 디렉터리를 재귀 walk 하며 일반 파일을 `files` 에 수집한다(심링크 SKIP, dir 재귀).
    fn walk(&self, dir: &Path, files: &mut Vec<ScannedFile>) -> Result<(), ScanError> {
        let entries = fs::read_dir(dir).map_err(|source| self.io_error(dir, source))?;
        for entry in entries {
            let entry = entry.map_err(|source| self.io_error(dir, source))?;
            let file_type = entry
                .file_type()
                .map_err(|source| self.io_error(&entry.path(), source))?;
            if file_type.is_symlink() {
                continue; // R-SCAN-02: 심링크는 따라가지 않고 제외.
            }
            let path = entry.path();
            if file_type.is_dir() {
                self.walk(&path, files)?;
            } else if file_type.is_file() {
                let metadata = entry
                    .metadata()
                    .map_err(|source| self.io_error(&path, source))?;
                // 정상 볼트 내부 파일은 항상 정규화에 성공한다(R-SCAN-01). 방어적으로 실패 시 배제.
                if let Some(relative_path) = self.relative_path(&path) {
                    files.push(ScannedFile {
                        relative_path,
                        size: ByteCount::new(metadata.len()),
                    });
                }
            }
        }
        Ok(())
    }

    /// 절대 경로를 볼트 루트 기준 정규화 `RelativePath` 로 변환한다(루트 자신·이탈 경로는 `None`).
    fn relative_path(&self, path: &Path) -> Option<RelativePath> {
        let stripped = path.strip_prefix(&self.root).ok()?;
        RelativePath::normalize(&stripped.to_string_lossy()).ok()
    }

    /// I/O 오류를 `ScanError` 로 사상한다. 경로가 루트 자신(정규화 불가)이면 `RootUnavailable`.
    fn io_error(&self, path: &Path, source: std::io::Error) -> ScanError {
        match self.relative_path(path) {
            Some(relative_path) => ScanError::Io {
                path: relative_path,
                source,
            },
            None => ScanError::RootUnavailable,
        }
    }
}

/// 현재 시각을 Unix epoch 기준 나노초로 반환한다(단조성·정확성 요구 없는 진단 시점).
fn now_unix_nanos() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

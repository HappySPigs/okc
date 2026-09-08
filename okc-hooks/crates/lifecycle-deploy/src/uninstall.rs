//! `Uninstaller` — idempotent 로컬 산출물 정리 파이프라인 + 볼트-안전 이중 방어.
//!
//! 순서(R-UN-01): (1) `deregister_service` 이면 `ServiceManager.uninstall()`(등록 해제, 데몬 정지)
//! -> (2) 주입된 `ArtifactSet` 경로 기반 삭제(UploadHistory/SyncState/Log/ConfigToken) -> (3) 토큰
//! 3경로 정리(R-UN-05) -> (4) `UninstallReport` 조립·판정(R-UN-06). 파일 삭제는 데몬 실행 상태에
//! 의존하지 않아 **데몬 정지 상태에서도 안전**하다.
//!
//! 볼트-안전(R-UN-04): 삭제 직전 각 대상 경로가 `vault_root` 하위가 아님을 lexical 정규화 후
//! 재검사하고, 하위이면 절대 삭제하지 않고 `VaultProtected` 로 skip 한다(조립 배제에 더한 이중 방어).
//!
//! 이 모듈은 `std::fs` I/O 를 (seam 뒤에서) 수행하므로 U0 `store.rs`/`loader.rs` 처럼 순수 리프
//! clippy lint-gate 를 두지 않는다(단, 어떤 경로에서도 `unwrap`/`expect`/`panic`/인덱싱을 쓰지 않는다).

use std::path::{Component, Path, PathBuf};

use auth_consent::{CredentialProvider, TokenStatus};

use crate::service::ServiceManager;

/// 정리 대상 아티팩트 분류(리포트 분류 + `purge_logs`/`purge_token` 스위치 판정용).
///
/// Uninstaller 는 내용을 파싱하지 않고 **경로만** 근거로 삭제한다(R-UN-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    /// U6 업로드 히스토리 파일(FR-16).
    UploadHistory,
    /// U4 지속 상태 파일(FR-02/03).
    SyncState,
    /// U6 구조화 로그(FR-17) — `purge_logs` 로 게이트.
    Log,
    /// config 평문 토큰 보유 파일(RISK-01 정리 대상) — `purge_token` 으로 게이트.
    ConfigToken,
}

/// 정리 대상 아티팩트(주입 입력). 경로 해소는 조립 루트(U8)가 담당한다(D-U7A-08).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    /// 아티팩트 분류.
    pub kind: ArtifactKind,
    /// 절대 경로(U8 이 vault_root 하위를 구조적으로 배제해 조립).
    pub path: PathBuf,
}

/// 주입되는 정리 대상 집합.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArtifactSet(pub Vec<Artifact>);

/// 정리 범위 스위치.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UninstallOptions {
    /// config/env/secure-store 토큰 정리 시도(D-U7A-07).
    pub purge_token: bool,
    /// `Log` 아티팩트 포함 여부.
    pub purge_logs: bool,
    /// `ServiceManager.uninstall()` 선행 여부(기본 `true`).
    pub deregister_service: bool,
}

impl Default for UninstallOptions {
    fn default() -> Self {
        UninstallOptions {
            purge_token: false,
            purge_logs: false,
            deregister_service: true,
        }
    }
}

/// skip 대상 식별자(리포트).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipTarget {
    /// 아티팩트 경로 대상.
    Artifact(Artifact),
    /// 프로세스 밖 영속 env 토큰(`OKC_WATCHER_TOKEN`).
    EnvToken,
    /// OS 보안 저장소 토큰.
    SecureStoreToken,
}

/// skip 사유(리포트).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// 이미 없음(idempotent no-op) — `removed` 로 집계될 수도 있음(R-UN-02).
    AlreadyAbsent,
    /// 권한 부족 -> 부분 실패 항목.
    PermissionDenied,
    /// `vault_root` 하위 -> 안전 배제(절대 삭제 안 함).
    VaultProtected,
    /// 프로세스가 영속 env 를 해제 불가 -> 자문 skip(D-U7A-07b).
    EnvTokenExternal,
    /// U5 에 삭제 API 부재 + MVP `UnavailableSecureStore`(D-U7A-07c).
    SecureStoreRemovalUnsupported,
}

/// 정리 결과 요약.
///
/// **불변식(완결성, R-UN-06)**: `removed` 와 `skipped` 의 (아티팩트) 대상 합집합 = 전체 **시도**
/// 대상이며 교집합은 공집합이다. `purge_logs`/`purge_token` 으로 게이트되어 시도되지 않은
/// 아티팩트는 어느 목록에도 나타나지 않는다.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UninstallReport {
    /// 삭제 성공(또는 이미 부재 -> idempotent 성공)한 아티팩트.
    pub removed: Vec<Artifact>,
    /// 미삭제 대상 + 사유.
    pub skipped: Vec<(SkipTarget, SkipReason)>,
}

/// Uninstaller 오류 taxonomy(U7a 소유, D-U7A-11).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UninstallError {
    /// 하나 이상 `PermissionDenied` 등 잔존 -> 무엇이 남았는지 리포트 동반(US-E6-04 AC).
    #[error("부분 실패: 잔존 항목이 있습니다")]
    PartialFailure(UninstallReport),
    /// 리포트 조립조차 불가한 하위 실패.
    #[error("정리 I/O 실패: {0}")]
    Io(String),
}

/// 파일 삭제 seam 오류.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsError {
    /// 권한 부족.
    PermissionDenied,
    /// 기타 하위 I/O 실패.
    Io(String),
}

/// 경로 삭제 seam — 실 삭제(`StdFileSystem`) 또는 테스트 fake(권한 실패 주입) 뒤로 격리.
pub trait FileSystem: Send + Sync {
    /// 경로를 삭제한다. 반환 `bool` 은 삭제 이전 존재 여부. **부재는 오류가 아니라 `Ok(false)`**
    /// (idempotent, R-UN-02).
    fn remove(&self, path: &Path) -> Result<bool, FsError>;
}

/// `std::fs` 기반 프로덕션 파일시스템. 심볼릭 링크는 따라가지 않고 링크 자체만 제거한다
/// (볼트 원본을 링크 경유로도 건드리지 않음).
#[derive(Debug, Clone, Copy, Default)]
pub struct StdFileSystem;

impl FileSystem for StdFileSystem {
    fn remove(&self, path: &Path) -> Result<bool, FsError> {
        let meta = match std::fs::symlink_metadata(path) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(map_io(&e)),
        };
        let result = if meta.is_dir() {
            std::fs::remove_dir_all(path)
        } else {
            std::fs::remove_file(path)
        };
        match result {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(map_io(&e)),
        }
    }
}

/// `io::Error` 를 `FsError` 로 매핑한다(권한 부족 분리).
fn map_io(e: &std::io::Error) -> FsError {
    if e.kind() == std::io::ErrorKind::PermissionDenied {
        FsError::PermissionDenied
    } else {
        FsError::Io(e.to_string())
    }
}

/// secure-store 토큰 정리 결과(성공 경로).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurgeOutcome {
    /// 실제 제거됨.
    Removed,
    /// 이미 부재.
    AlreadyAbsent,
}

/// secure-store 토큰 정리 미지원 오류. MVP 기본 포트는 항상 이 값을 반환한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("secure-store 토큰 삭제 미지원")]
pub struct PurgeUnsupported;

/// secure-store 토큰 정리 seam(U7a 소유, D-U7A-07c).
///
/// U5 `CredentialProvider`/`SecureStore` 공개 API 에는 토큰 **삭제 메서드가 없으므로**, U7a 는
/// 존재하지 않는 API 를 호출하지 않고 이 seam 으로 의도를 표현한다. 향후 U5 가 삭제 API 를
/// 노출하면 이 seam 을 그 API 로 배선한다.
pub trait TokenPurgePort: Send + Sync {
    /// secure-store 토큰을 제거 시도한다. MVP 기본 구현은 항상 `Err(PurgeUnsupported)`.
    fn purge_secure_store_token(&self) -> Result<PurgeOutcome, PurgeUnsupported>;
}

/// MVP 기본 포트(null-object) — 항상 미지원을 반환한다.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnsupportedTokenPurge;

impl TokenPurgePort for UnsupportedTokenPurge {
    fn purge_secure_store_token(&self) -> Result<PurgeOutcome, PurgeUnsupported> {
        Err(PurgeUnsupported)
    }
}

/// 정리 전 토큰 존재/소스 진단(선택, R-UN-05 자문). U5 `CredentialProvider.token_status()` 에
/// 위임한다 — **삭제가 아니라 존재 확인만**이다(원문 미노출).
pub fn pre_purge_token_status(provider: &CredentialProvider) -> TokenStatus {
    provider.token_status()
}

/// 서비스 등록 해제 + 로컬 산출물 idempotent 삭제 + 토큰 정리 오케스트레이터.
pub struct Uninstaller {
    service: ServiceManager,
    fs: Box<dyn FileSystem>,
    token_purge: Box<dyn TokenPurgePort>,
}

impl Uninstaller {
    /// 주입 의존으로 uninstaller 를 구성한다.
    pub fn new(
        service: ServiceManager,
        fs: Box<dyn FileSystem>,
        token_purge: Box<dyn TokenPurgePort>,
    ) -> Self {
        Uninstaller {
            service,
            fs,
            token_purge,
        }
    }

    /// 정리 파이프라인을 1회 실행한다(R-UN-01..06). 잔존(`PermissionDenied`)이 있으면
    /// `Err(PartialFailure(report))`, 없으면 `Ok(report)`.
    pub fn uninstall(
        &self,
        opts: &UninstallOptions,
        artifacts: ArtifactSet,
        vault_root: &Path,
    ) -> Result<UninstallReport, UninstallError> {
        let mut report = UninstallReport::default();

        // (1) 서비스 등록 해제(선행, idempotent). MVP: 서비스 dereg 실패는 best-effort 로 무시하고
        // 아티팩트 정리를 계속한다(리포트에는 서비스 SkipTarget 이 없음 — 트림).
        if opts.deregister_service {
            let _ = self.service.uninstall();
        }

        // (2) 아티팩트 경로 기반 삭제.
        for artifact in artifacts.0 {
            // purge 스위치로 게이트된 아티팩트는 아예 시도하지 않는다(리포트 미기록).
            match artifact.kind {
                ArtifactKind::Log if !opts.purge_logs => continue,
                ArtifactKind::ConfigToken if !opts.purge_token => continue,
                _ => {}
            }

            // (R-UN-04) 볼트-안전 이중 방어: vault_root 하위면 절대 삭제하지 않는다.
            if is_descendant(&artifact.path, vault_root) {
                report
                    .skipped
                    .push((SkipTarget::Artifact(artifact), SkipReason::VaultProtected));
                continue;
            }

            match self.fs.remove(&artifact.path) {
                // 존재/부재 모두 삭제 성공으로 집계(idempotent, R-UN-02).
                Ok(_existed) => report.removed.push(artifact),
                Err(FsError::PermissionDenied) => report
                    .skipped
                    .push((SkipTarget::Artifact(artifact), SkipReason::PermissionDenied)),
                // io 실패도 잔존 항목이므로 PermissionDenied 로 집계(부분 실패 판정 대상).
                Err(FsError::Io(_)) => report
                    .skipped
                    .push((SkipTarget::Artifact(artifact), SkipReason::PermissionDenied)),
            }
        }

        // (3) 토큰 3경로 정리(R-UN-05). config 토큰은 위 루프에서 ConfigToken 아티팩트로 처리됨.
        if opts.purge_token {
            // env 토큰: 프로세스가 영속 env 를 해제 불가 -> 자문 skip.
            report
                .skipped
                .push((SkipTarget::EnvToken, SkipReason::EnvTokenExternal));
            // secure-store 토큰: seam 위임. MVP 기본 포트는 항상 미지원.
            match self.token_purge.purge_secure_store_token() {
                // 성공 경로는 MVP 기본 포트에서 발생하지 않는다(미래 U5 삭제 API 배선 시 도달).
                Ok(_) => {}
                Err(PurgeUnsupported) => report.skipped.push((
                    SkipTarget::SecureStoreToken,
                    SkipReason::SecureStoreRemovalUnsupported,
                )),
            }
        }

        // (4) 판정(R-UN-06): PermissionDenied 잔존 <=> PartialFailure.
        let has_residual = report
            .skipped
            .iter()
            .any(|(_, reason)| *reason == SkipReason::PermissionDenied);
        if has_residual {
            Err(UninstallError::PartialFailure(report))
        } else {
            Ok(report)
        }
    }
}

/// (R-UN-04) `path` 가 `root` 와 같거나 그 하위 경로인지 lexical 정규화 후 판정한다.
///
/// FS 를 건드리지 않는 순수 lexical 정규화(`.`/`..` 해소)를 쓰므로 대상이 존재하지 않아도
/// 안전하게 판정한다. 정규화 후 컴포넌트 prefix 비교(`starts_with`).
fn is_descendant(path: &Path, root: &Path) -> bool {
    let np = normalize_lexical(path);
    let nr = normalize_lexical(root);
    np == nr || np.starts_with(&nr)
}

/// FS 접근 없이 `.`/`..`/중복 구분자를 해소하는 lexical 경로 정규화.
fn normalize_lexical(p: &Path) -> PathBuf {
    let mut stack: Vec<Component> = Vec::new();
    for comp in p.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => match stack.last() {
                // 직전이 일반 세그먼트면 상쇄, 아니면(루트/접두/이미 ..) 보존.
                Some(Component::Normal(_)) => {
                    stack.pop();
                }
                _ => stack.push(Component::ParentDir),
            },
            other => stack.push(other),
        }
    }
    let mut out = PathBuf::new();
    for comp in stack {
        out.push(comp.as_os_str());
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )]
    use super::*;
    use crate::testing::{FakeController, FakeFileSystem};

    fn abs(p: &str) -> PathBuf {
        if cfg!(windows) {
            PathBuf::from(format!("C:\\{}", p.replace('/', "\\")))
        } else {
            PathBuf::from(format!("/{p}"))
        }
    }

    fn artifact(kind: ArtifactKind, p: &str) -> Artifact {
        Artifact {
            kind,
            path: abs(p),
        }
    }

    fn uninstaller(fs: FakeFileSystem) -> Uninstaller {
        Uninstaller::new(
            ServiceManager::new(Box::new(FakeController::empty())),
            Box::new(fs),
            Box::new(UnsupportedTokenPurge),
        )
    }

    #[test]
    fn is_descendant_detects_nested_and_equal() {
        assert!(is_descendant(&abs("vault/sub/f"), &abs("vault")));
        assert!(is_descendant(&abs("vault"), &abs("vault")));
        assert!(!is_descendant(&abs("data/f"), &abs("vault")));
        // .. 로 벗어난 경로는 하위가 아님.
        assert!(!is_descendant(&abs("vault/../data/f"), &abs("vault")));
    }

    #[test]
    fn removes_present_and_absent_artifacts_idempotently() {
        let fs = FakeFileSystem::new();
        fs.add_existing(&abs("data/history"));
        // syncstate 는 이미 부재.
        let un = uninstaller(fs);
        let set = ArtifactSet(vec![
            artifact(ArtifactKind::UploadHistory, "data/history"),
            artifact(ArtifactKind::SyncState, "data/state"),
        ]);
        let report = un.uninstall(&UninstallOptions::default(), set, &abs("vault")).unwrap();
        assert_eq!(report.removed.len(), 2);
        assert!(report.skipped.is_empty());
    }

    #[test]
    fn logs_and_config_token_gated_by_options() {
        let fs = FakeFileSystem::new();
        fs.add_existing(&abs("data/log"));
        fs.add_existing(&abs("data/config"));
        let un = uninstaller(fs);
        let set = ArtifactSet(vec![
            artifact(ArtifactKind::Log, "data/log"),
            artifact(ArtifactKind::ConfigToken, "data/config"),
        ]);
        // 기본 옵션(purge_logs=false, purge_token=false): 둘 다 시도 안 됨.
        let report = un
            .uninstall(&UninstallOptions::default(), set, &abs("vault"))
            .unwrap();
        assert!(report.removed.is_empty());
        assert!(report.skipped.is_empty());
    }

    #[test]
    fn purge_token_reports_env_and_secure_store_skips() {
        let fs = FakeFileSystem::new();
        fs.add_existing(&abs("data/config"));
        let un = uninstaller(fs);
        let opts = UninstallOptions {
            purge_token: true,
            purge_logs: false,
            deregister_service: true,
        };
        let set = ArtifactSet(vec![artifact(ArtifactKind::ConfigToken, "data/config")]);
        let report = un.uninstall(&opts, set, &abs("vault")).unwrap();
        // config 토큰은 removed, env/secure-store 는 자문 skip.
        assert_eq!(report.removed.len(), 1);
        assert!(report.skipped.iter().any(|(t, r)| matches!(t, SkipTarget::EnvToken)
            && *r == SkipReason::EnvTokenExternal));
        assert!(report.skipped.iter().any(|(t, r)| matches!(t, SkipTarget::SecureStoreToken)
            && *r == SkipReason::SecureStoreRemovalUnsupported));
    }

    #[test]
    fn vault_paths_are_never_deleted() {
        let fs = FakeFileSystem::new();
        fs.add_existing(&abs("vault/original"));
        let un = uninstaller(fs.clone());
        let set = ArtifactSet(vec![artifact(ArtifactKind::SyncState, "vault/original")]);
        let report = un
            .uninstall(&UninstallOptions::default(), set, &abs("vault"))
            .unwrap();
        assert!(report.removed.is_empty());
        assert!(report.skipped.iter().any(|(_, r)| *r == SkipReason::VaultProtected));
        // 볼트 경로는 여전히 존재해야 한다.
        assert!(fs.contains(&abs("vault/original")));
    }

    #[test]
    fn permission_failure_yields_partial_failure() {
        let fs = FakeFileSystem::new();
        fs.add_existing(&abs("data/history"));
        fs.deny_permission(&abs("data/history"));
        let un = uninstaller(fs);
        let set = ArtifactSet(vec![artifact(ArtifactKind::UploadHistory, "data/history")]);
        let err = un
            .uninstall(&UninstallOptions::default(), set, &abs("vault"))
            .unwrap_err();
        match err {
            UninstallError::PartialFailure(report) => {
                assert!(report.skipped.iter().any(|(_, r)| *r == SkipReason::PermissionDenied));
            }
            other => panic!("예상: PartialFailure, 실제: {other:?}"),
        }
    }
}

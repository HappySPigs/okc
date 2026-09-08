#![deny(missing_docs)]
//! `lifecycle-deploy` 크레이트 (U7a) — Watcher 바이너리의 배포/업데이트/제거 수명주기 단위.
//!
//! 세 개의 응집 애그리게이트를 노출한다:
//! - `ServiceManager`(모듈 `service`): launchd / systemd / Windows Service 에 대한 3-OS 동일
//!   계약(`install/uninstall/start/stop/restart/status` + 부팅/로그인 자동시작). per-OS 차이는
//!   `ServiceController` seam 뒤에 격리되고 상위 오케스트레이션은 OS-불변·순수 로직이다.
//! - `AutoUpdater`(모듈 `update`): 채널 config 스테이징 -> `ServiceManager.restart()` ->
//!   주입 `Clock` 기반 bounded-time 헬스 게이트(U0 `ReadJudgment::update_probe()`) -> 실패/타임아웃
//!   시 직전 정상 버전 자동 롤백 + U0 `CriticalEventSink::report_update_rollback()` 1회 통지 +
//!   롤백 루프 방지 hold 마커.
//! - `Uninstaller`(모듈 `uninstall`): 서비스 등록 해제 + 로컬 평문 산출물(히스토리/SyncState/
//!   로그/config 토큰) 경로 기반 idempotent 삭제 + 토큰 3경로 정리. 데몬 정지 상태에서도 동작하며
//!   볼트 원본은 어떤 경로로도 삭제하지 않는다(이중 방어).
//!
//! OS/프로세스(`ServiceController`)·시계(`Clock`)·업데이트 소스(`UpdateSource`)·파일시스템
//! (`FileSystem`)·secure-store 토큰 정리(`TokenPurgePort`)는 모두 seam 트레이트로 추상화되어,
//! 테스트가 특권/네트워크 없이 결정적으로 로직을 검증한다.
//!
//! 규약: 모든 doc 주석은 한국어, 코드 내 화살표는 ASCII `A -> B` 만 사용,
//! Rust 타입/식별자는 doc 주석에서 백틱으로 감싼다(`Arc<dyn ReadJudgment>`).

/// `Clock` seam — bounded-time 헬스 게이트의 now-source + sleep(`SystemClock`).
pub mod clock;
/// `ServiceManager` 애그리게이트 + `ServiceController` seam + per-OS best-effort 어댑터.
pub mod service;
/// `AutoUpdater` 애그리게이트 + `UpdateSource` seam + apply 상태머신/헬스 게이트.
pub mod update;
/// `Uninstaller` 애그리게이트 + `FileSystem`/`TokenPurgePort` seam + 볼트-안전 가드.
pub mod uninstall;

/// 테스트 더블(fake seam 구현) — 비-default `proptest-support` feature 또는 `test` 에서만 빌드.
#[cfg(any(test, feature = "proptest-support"))]
pub mod testing;

/// ProptestGenerators: U7a 도메인 제너레이터(test-support, 런타임 그래프 밖).
///
/// 비-default cargo feature `proptest-support` 뒤에 게이트되어 릴리스 빌드에서 제외된다(PBT-07).
#[cfg(feature = "proptest-support")]
pub mod proptest_support;

// ---------------------------------------------------------------------------
// 공개 API 표면(crate-root 재-export). 재-export 된 아이템은 원본 doc 을 승계한다.
// ---------------------------------------------------------------------------

pub use clock::{Clock, SystemClock};
pub use service::{
    ServiceAccount, ServiceController, ServiceError, ServiceManager, ServiceRegistration,
    ServiceSpec, native_controller,
};
pub use update::{
    ArtifactRef, AutoUpdater, Checksum, GateConfig, HoldReason, RollbackState, UpdateChannel,
    UpdateError, UpdateInfo, UpdateOutcome, UpdateSource,
};
pub use uninstall::{
    Artifact, ArtifactKind, ArtifactSet, FileSystem, FsError, PurgeOutcome, PurgeUnsupported,
    SkipReason, SkipTarget, StdFileSystem, TokenPurgePort, UninstallError, UninstallOptions,
    UninstallReport, Uninstaller, UnsupportedTokenPurge, pre_purge_token_status,
};

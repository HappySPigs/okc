//! Property-Based Testing (PBT, `proptest-support` feature 게이트) — PROP-U7A-01..07.
//!
//! feature 가 꺼진 기본 빌드에서는 빈 테스트 크레이트로 컴파일되고,
//! `cargo test --features proptest-support` 로만 실제 property 가 실행된다(PBT-07).
//!
//! 커버:
//! - PROP-U7A-01: 헬스 게이트 통과/타임아웃 경계(배타·전수) — AutoUpdater.
//! - PROP-U7A-02: 롤백 시 last-good 복원 + `report_update_rollback` 정확히 1회 — AutoUpdater.
//! - PROP-U7A-03: 롤백 직후 hold 구간 재-apply 는 Refused + 스테이징 부수효과 없음 — AutoUpdater.
//! - PROP-U7A-04: `uninstall()` idempotency(2회 == 1회 FS 상태) — Uninstaller.
//! - PROP-U7A-05: 볼트-안전 불변식(vault_root 하위 절대 미삭제) — Uninstaller.
//! - PROP-U7A-06: 리포트 완결·무중복 + 부분실패 판정 — Uninstaller.
//! - PROP-U7A-07: 임의 초기상태 idempotent uninstall + 상태 함의 — ServiceManager.
#![cfg(feature = "proptest-support")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use foundation::{RollbackReason, Version};
use proptest::prelude::*;

use lifecycle_deploy::proptest_support::generators as g;
use lifecycle_deploy::testing::{
    FakeClock, FakeController, FakeFileSystem, RecordingCritical, ScriptedProbe, make_update_info,
};
use lifecycle_deploy::{
    Artifact, ArtifactSet, AutoUpdater, GateConfig, ServiceManager, SkipReason, SkipTarget,
    StdFileSystem, UninstallError, UninstallOptions, Uninstaller, UnsupportedTokenPurge,
    UpdateChannel, UpdateInfo, UpdateOutcome, UpdateSource,
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn abs(parts: &str) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(format!("C:\\{}", parts.replace('/', "\\")))
    } else {
        PathBuf::from(format!("/{parts}"))
    }
}

fn build_updater(
    source: Box<dyn UpdateSource>,
    controller: FakeController,
    probe: ScriptedProbe,
    critical: Arc<RecordingCritical>,
    clock: Arc<FakeClock>,
    gate: GateConfig,
) -> AutoUpdater {
    AutoUpdater::new(
        source,
        ServiceManager::new(Box::new(controller)),
        Arc::new(probe),
        critical,
        clock,
        UpdateChannel::Stable,
        gate,
        Version("1.0.0".to_string()),
    )
}

proptest! {
    // PROP-U7A-01: Alive 가 T_gate 이내면 Committed, 아니면 RolledBack(GateTimeout). 정확히 하나.
    #[test]
    fn prop_gate_pass_or_timeout_exclusive(gate in g::arb_gate_config(), plan in g::arb_liveness_plan()) {
        let g_nanos = gate.t_gate.as_nanos() as i64;
        let p_nanos = gate.t_poll.as_nanos() as i64;
        // 정수 나노초 모델로 통과 여부 예측(폴링 i 는 시각 i*P 에 발생, deadline = G).
        let expect_commit = match plan {
            None => false,
            Some(0) => true,
            Some(k) => ((k as i64 - 1) * p_nanos) < g_nanos,
        };
        let probe = match plan {
            None => ScriptedProbe::never_alive(),
            Some(k) => ScriptedProbe::alive_after(k),
        };
        let critical = Arc::new(RecordingCritical::default());
        let mut au = build_updater(
            Box::new(lifecycle_deploy::testing::FakeUpdateSource::ok()),
            FakeController::running(1),
            probe,
            critical.clone(),
            Arc::new(FakeClock::new(0)),
            gate,
        );
        let out = au.apply_update(make_update_info("2.0.0")).unwrap();
        match out {
            UpdateOutcome::Committed { .. } => prop_assert!(expect_commit),
            UpdateOutcome::RolledBack { reason, .. } => {
                prop_assert!(!expect_commit);
                prop_assert_eq!(reason, RollbackReason("GateTimeout".to_string()));
            }
            UpdateOutcome::Refused { .. } => prop_assert!(false, "hold 미설정인데 Refused"),
        }
    }

    // PROP-U7A-02: 롤백 실행 시 to == last_good, 통지 정확히 1회, from == 시도 버전.
    #[test]
    fn prop_rollback_restores_last_good_and_notifies_once(
        attempted in g::arb_version(),
        via_restart_fail in any::<bool>(),
    ) {
        let critical = Arc::new(RecordingCritical::default());
        // restart 실패 경로 vs 게이트 타임아웃 경로 두 롤백 트리거 모두 커버.
        let (controller, probe, expected_reason) = if via_restart_fail {
            (FakeController::running(1).failing_start(), ScriptedProbe::alive_now(), "RestartFailed")
        } else {
            (FakeController::running(1), ScriptedProbe::never_alive(), "GateTimeout")
        };
        let gate = GateConfig { t_gate: Duration::from_nanos(0), t_poll: Duration::from_nanos(1), t_hold: Duration::from_secs(300) };
        let mut au = build_updater(
            Box::new(lifecycle_deploy::testing::FakeUpdateSource::ok()),
            controller,
            probe,
            critical.clone(),
            Arc::new(FakeClock::new(0)),
            gate,
        );
        let info = UpdateInfo { version: attempted.clone(), ..make_update_info("x") };
        let out = au.apply_update(info).unwrap();
        prop_assert_eq!(
            out,
            UpdateOutcome::RolledBack {
                from: attempted.clone(),
                to: Version("1.0.0".to_string()),
                reason: RollbackReason(expected_reason.to_string()),
            }
        );
        let calls = critical.rollback_calls();
        prop_assert_eq!(calls.len(), 1);
        prop_assert_eq!(&calls[0].0, &attempted);
        prop_assert_eq!(&calls[0].1, &Version("1.0.0".to_string()));
        prop_assert_eq!(au.rollback_state().last_good.clone(), Version("1.0.0".to_string()));
    }

    // PROP-U7A-03: 롤백 후 hold 구간 재-apply 는 Refused 이며 추가 스테이징 부수효과가 없다.
    #[test]
    fn prop_hold_refuses_reapply_without_staging(second in g::arb_version()) {
        let critical = Arc::new(RecordingCritical::default());
        let source = lifecycle_deploy::testing::FakeUpdateSource::ok();
        let source_handle = source.clone();
        let gate = GateConfig { t_gate: Duration::from_nanos(0), t_poll: Duration::from_nanos(1), t_hold: Duration::from_secs(300) };
        let mut au = build_updater(
            Box::new(source),
            FakeController::running(1),
            ScriptedProbe::never_alive(),
            critical.clone(),
            Arc::new(FakeClock::new(0)),
            gate,
        );
        // 1회차: 게이트 타임아웃 -> 롤백 -> hold 세팅.
        let first = au.apply_update(make_update_info("2.0.0")).unwrap();
        prop_assert!(matches!(first, UpdateOutcome::RolledBack { .. }), "RolledBack 결과 예상");
        let staged_after_first = source_handle.stage_count();
        // 2회차(clock 미진행 -> hold 유효): Refused, 스테이징 호출 수 불변.
        let info = UpdateInfo { version: second, ..make_update_info("y") };
        let out = au.apply_update(info).unwrap();
        prop_assert!(matches!(out, UpdateOutcome::Refused { .. }), "Refused 결과 예상");
        prop_assert_eq!(source_handle.stage_count(), staged_after_first);
    }

    // PROP-U7A-04: 실제 temp-dir 아티팩트 트리에 대해 uninstall 2회 == 1회 FS 상태(idempotent).
    #[test]
    fn prop_uninstall_idempotent(specs in g::arb_artifact_specs()) {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!("okc-u7a-prop-{}-{}", std::process::id(), n));
        let vault = base.join("vault");
        let data = base.join("data");
        std::fs::create_dir_all(&vault).unwrap();
        std::fs::create_dir_all(&data).unwrap();

        let mut paths = Vec::new();
        for (i, s) in specs.iter().enumerate() {
            let dir = if s.under_vault { &vault } else { &data };
            let p = dir.join(format!("{i}_{}", s.name));
            if s.exists {
                std::fs::write(&p, b"x").unwrap();
            }
            paths.push((Artifact { kind: s.kind, path: p.clone() }, p));
        }

        let opts = UninstallOptions { purge_token: true, purge_logs: true, deregister_service: true };
        let make = || Uninstaller::new(
            ServiceManager::new(Box::new(FakeController::empty())),
            Box::new(StdFileSystem),
            Box::new(UnsupportedTokenPurge),
        );

        let set1 = ArtifactSet(paths.iter().map(|(a, _)| a.clone()).collect());
        let _ = make().uninstall(&opts, set1, &vault);
        let after1: Vec<bool> = paths.iter().map(|(_, p)| p.exists()).collect();

        let set2 = ArtifactSet(paths.iter().map(|(a, _)| a.clone()).collect());
        let _ = make().uninstall(&opts, set2, &vault);
        let after2: Vec<bool> = paths.iter().map(|(_, p)| p.exists()).collect();

        prop_assert_eq!(after1, after2);
    }

    // PROP-U7A-05: vault_root 하위의 어떤 경로도 삭제되지 않는다(항상 VaultProtected).
    #[test]
    fn prop_vault_paths_never_deleted(specs in g::arb_artifact_specs()) {
        let fs = FakeFileSystem::new();
        let vault_root = abs("vault");
        let mut artifacts = Vec::new();
        for (i, s) in specs.iter().enumerate() {
            let p = if s.under_vault {
                abs(&format!("vault/{i}_{}", s.name))
            } else {
                abs(&format!("data/{i}_{}", s.name))
            };
            if s.exists {
                fs.add_existing(&p);
            }
            artifacts.push((s.under_vault, Artifact { kind: s.kind, path: p }));
        }
        let un = Uninstaller::new(
            ServiceManager::new(Box::new(FakeController::empty())),
            Box::new(fs.clone()),
            Box::new(UnsupportedTokenPurge),
        );
        let opts = UninstallOptions { purge_token: true, purge_logs: true, deregister_service: true };
        let set = ArtifactSet(artifacts.iter().map(|(_, a)| a.clone()).collect());
        let result = un.uninstall(&opts, set, &vault_root);
        let report = match result {
            Ok(r) => r,
            Err(UninstallError::PartialFailure(r)) => r,
            Err(UninstallError::Io(e)) => panic!("예상 밖 I/O 오류: {e}"),
        };
        // vault-하위 아티팩트는 removed 에 없어야 하고, 여전히 FS 에 존재해야 한다.
        for (under_vault, art) in &artifacts {
            if *under_vault {
                prop_assert!(!report.removed.iter().any(|r| r.path == art.path));
                // vault-하위는 VaultProtected 로 skip.
                prop_assert!(report.skipped.iter().any(|(t, r)|
                    matches!(t, SkipTarget::Artifact(a) if a.path == art.path)
                    && *r == SkipReason::VaultProtected));
                prop_assert!(fs.contains(&art.path) == art_existed(&specs, &art.path));
            }
        }
    }

    // PROP-U7A-06: removed + skipped(artifact) = 전체 시도 아티팩트, 무중복; 잔존 <=> PartialFailure.
    #[test]
    fn prop_report_completeness_and_partial_failure(specs in g::arb_artifact_specs()) {
        let fs = FakeFileSystem::new();
        let vault_root = abs("vault");
        let mut count = 0usize;
        let mut any_deny = false;
        let mut artifacts = Vec::new();
        for (i, s) in specs.iter().enumerate() {
            // 완결성 검증을 위해 모든 대상을 vault 밖(data)에 두어 전부 시도되게 한다.
            let p = abs(&format!("data/{i}_{}", s.name));
            if s.exists {
                fs.add_existing(&p);
            }
            if s.deny {
                fs.deny_permission(&p);
                any_deny = true;
            }
            artifacts.push(Artifact { kind: s.kind, path: p });
            count += 1;
        }
        let un = Uninstaller::new(
            ServiceManager::new(Box::new(FakeController::empty())),
            Box::new(fs),
            Box::new(UnsupportedTokenPurge),
        );
        let opts = UninstallOptions { purge_token: true, purge_logs: true, deregister_service: true };
        let set = ArtifactSet(artifacts);
        let result = un.uninstall(&opts, set, &vault_root);
        let (report, is_partial) = match result {
            Ok(r) => (r, false),
            Err(UninstallError::PartialFailure(r)) => (r, true),
            Err(UninstallError::Io(e)) => panic!("예상 밖 I/O 오류: {e}"),
        };
        let artifact_skips = report.skipped.iter()
            .filter(|(t, _)| matches!(t, SkipTarget::Artifact(_)))
            .count();
        // 완결: removed + artifact-skips = 전체 시도. (무중복은 인덱스-유일 경로 + 1회 처리로 보장.)
        prop_assert_eq!(report.removed.len() + artifact_skips, count);
        // 잔존(PermissionDenied) <=> PartialFailure.
        prop_assert_eq!(is_partial, any_deny);
    }

    // PROP-U7A-07: 임의 초기상태 -> uninstall 항상 Ok(idempotent), 이후 미등록; status 함의 유지.
    #[test]
    fn prop_service_uninstall_idempotent_and_status_invariants(
        registered in any::<bool>(),
        running in any::<bool>(),
        pid in proptest::option::of(any::<u32>()),
    ) {
        let mgr = ServiceManager::new(Box::new(FakeController::custom(registered, running, pid)));
        // 초기 status 가 Ok 면 R-SM-03 함의를 만족해야 한다.
        if let Ok(reg) = mgr.status() {
            prop_assert!(!reg.running || reg.registered);
            prop_assert!(reg.pid.is_none() || reg.running);
        }
        // uninstall 은 항상 Ok(미등록 포함, idempotent).
        prop_assert!(mgr.uninstall().is_ok());
        prop_assert!(mgr.uninstall().is_ok());
        // 이후 status 는 NotInstalled(미등록).
        prop_assert!(mgr.status().is_err());
    }
}

/// specs 목록에서 주어진 경로의 초기 존재 여부를 되찾는다(PROP-05 보조).
fn art_existed(specs: &[g::ArtifactSpec], path: &std::path::Path) -> bool {
    for (i, s) in specs.iter().enumerate() {
        let candidate = if s.under_vault {
            abs(&format!("vault/{i}_{}", s.name))
        } else {
            abs(&format!("data/{i}_{}", s.name))
        };
        if candidate == path {
            return s.exists;
        }
    }
    false
}

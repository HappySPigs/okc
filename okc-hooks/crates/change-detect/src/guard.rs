//! `VaultAvailabilityGuard` — 볼트 가용성 분류 + 파괴적-빈-커밋 방지 total-function 판정
//! (US-E1-06 / FR-09).
//!
//! `check_reachable` 는 파일시스템 stat 으로 볼트 루트의 도달 가능성을 사전 점검하고,
//! `guard_diff` 는 `(new_manifest, last_committed, availability)` 를 결합해 **순수 판정**을 낸다
//! (파일시스템 stat 외 컴포넌트 의존 없음). 가드는 hold 상태를 보유하지 않으므로 볼트가 복구되면
//! 즉시 정상 diff 로 자동 재개된다(R-GUARD-04, 무상태·멱등).
//!
//! 순수 판정(`guard_diff`)과 fs stat(`check_reachable`)만 사용하며 unwrap/expect/index/panic 을
//! 컴파일타임으로 금지한다.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use std::path::Path;

use foundation::Manifest;

/// 볼트 루트의 도달 가능성 분류(스냅샷 전 사전 점검). 4 명명 조건만(D9).
///
/// 세 unavailable 변이(`RootMissing`/`Unmounted`/`Inaccessible`)는 동일한 안전 동작(HOLD)을
/// 유발하며 구분은 best-effort 진단 라벨이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// 루트 존재·마운트·접근 권한 OK — 정상 diff 경로 후보.
    Reachable,
    /// 루트 경로 부재(삭제/이동) — HOLD.
    RootMissing,
    /// 마운트 지점이나 볼륨 미마운트 — HOLD.
    Unmounted,
    /// 존재하나 권한/오류로 접근 불가 — HOLD.
    Inaccessible,
}

/// 보류 사유 진단 문구(`Availability` 변이 유래).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoldReason(pub String);

/// 스냅샷/diff 에 대한 가드 판정. 코디네이터(U8)가 받아 진행/보류를 결정·표면화한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardVerdict {
    /// 정상 diff·업로드 진행 허용.
    Proceed,
    /// 볼트 도달 불가 — 보류(U8: `VaultUnavailable` 조건 raise + 사이클 중단).
    HoldVaultUnavailable(HoldReason),
    /// 0-파일 매니페스트인데 마지막 커밋엔 파일 존재 + confirm-empty 아님 -> 전부-삭제 방지 보류.
    HoldDestructiveEmpty,
}

/// 볼트 가용성·파괴적-빈-커밋 가드. `confirm_empty` 영속 플래그를 생성자 주입받는다(D10).
#[derive(Debug, Clone)]
pub struct VaultAvailabilityGuard {
    confirm_empty: bool,
}

impl VaultAvailabilityGuard {
    /// 주어진 confirm-empty 영속 플래그로 가드를 구성한다(`false` = 가드 활성).
    pub fn new(confirm_empty: bool) -> Self {
        VaultAvailabilityGuard { confirm_empty }
    }

    /// confirm-empty 영속 플래그 값(R-GUARD-05).
    pub fn is_confirm_empty(&self) -> bool {
        self.confirm_empty
    }

    /// 볼트 루트의 도달 가능성을 파일시스템 stat 으로 사전 점검한다(R-GUARD-01).
    ///
    /// 존재하지 않으면 `RootMissing`, 존재하나 접근/열람 불가면 `Inaccessible`, 디렉터리로
    /// 열람 가능하면 `Reachable`. 크로스 OS 에서 `Unmounted` 를 신뢰성 있게 구분하기 어려운
    /// 경우 best-effort 로 `Inaccessible`/`RootMissing` 에 흡수하되 안전 동작(HOLD)은 불변이다
    /// (D9). stat 실패는 패닉이 아니라 unavailable 판정으로 흡수한다.
    pub fn check_reachable(&self, root: &Path) -> Availability {
        match std::fs::metadata(root) {
            Ok(meta) => {
                if !meta.is_dir() {
                    // 파일/기타 -> 볼트 루트로 접근 불가.
                    return Availability::Inaccessible;
                }
                // 디렉터리 열람 가능 여부까지 확인(권한/마운트 드롭 흡수).
                match std::fs::read_dir(root) {
                    Ok(_) => Availability::Reachable,
                    Err(_) => Availability::Inaccessible,
                }
            }
            Err(err) => match err.kind() {
                std::io::ErrorKind::NotFound => Availability::RootMissing,
                std::io::ErrorKind::PermissionDenied => Availability::Inaccessible,
                _ => Availability::Inaccessible,
            },
        }
    }

    /// 파괴적-빈-커밋 방지 판정 표(G1~G4, total function). R-GUARD-02/03.
    ///
    /// - G1: `availability != Reachable` -> `HoldVaultUnavailable`.
    /// - G2: `Reachable` AND `empty(new)` AND `last_committed` 비어있지 않음 AND `!confirm_empty`
    ///   -> `HoldDestructiveEmpty`.
    /// - G3: `Reachable` AND `empty(new)` AND (`last_committed` None|빈 | `confirm_empty`)
    ///   -> `Proceed`.
    /// - G4: `Reachable` AND `!empty(new)` -> `Proceed`.
    ///
    /// 불변식(R-GUARD-03): G1·G2 조건에서 `Proceed` 는 절대 반환되지 않는다 — 0-파일 매니페스트가
    /// 확인 없이 "전부 삭제"로 커밋되는 경로가 존재하지 않는다.
    pub fn guard_diff(
        &self,
        new_manifest: &Manifest,
        last_committed: Option<&Manifest>,
        availability: Availability,
    ) -> GuardVerdict {
        // G1: 도달 불가면 어떤 매니페스트든 진행하지 않는다.
        if availability != Availability::Reachable {
            return GuardVerdict::HoldVaultUnavailable(HoldReason(reason_for(availability)));
        }

        let empty_new = new_manifest.entries.is_empty();
        if !empty_new {
            return GuardVerdict::Proceed; // G4
        }

        // 여기서부터 empty(new) == true.
        let last_non_empty = matches!(last_committed, Some(m) if !m.entries.is_empty());
        if last_non_empty && !self.confirm_empty {
            GuardVerdict::HoldDestructiveEmpty // G2
        } else {
            GuardVerdict::Proceed // G3
        }
    }
}

/// unavailable `Availability` 변이에 대한 진단 문구를 만든다(HOLD 사유).
fn reason_for(availability: Availability) -> String {
    match availability {
        Availability::Reachable => "reachable".to_string(),
        Availability::RootMissing => "vault root missing".to_string(),
        Availability::Unmounted => "vault volume unmounted".to_string(),
        Availability::Inaccessible => "vault root inaccessible".to_string(),
    }
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
    use foundation::{ManifestDigest, ManifestEntry, RelativePath, Sha256Digest};
    use foundation::ByteCount;

    fn empty_manifest() -> Manifest {
        Manifest {
            entries: Vec::new(),
            manifest_digest: ManifestDigest::from_bytes([0u8; 32]),
        }
    }

    fn non_empty_manifest() -> Manifest {
        let entry = ManifestEntry {
            relative_path: RelativePath::normalize("a.txt").expect("정규화"),
            raw_sha256: Sha256Digest::from_bytes([1u8; 32]),
            size: ByteCount::new(3),
        };
        Manifest {
            entries: vec![entry],
            manifest_digest: ManifestDigest::from_bytes([2u8; 32]),
        }
    }

    #[test]
    fn g1_unavailable_never_proceeds() {
        let guard = VaultAvailabilityGuard::new(false);
        for avail in [
            Availability::RootMissing,
            Availability::Unmounted,
            Availability::Inaccessible,
        ] {
            let v = guard.guard_diff(&non_empty_manifest(), Some(&non_empty_manifest()), avail);
            assert!(matches!(v, GuardVerdict::HoldVaultUnavailable(_)));
        }
    }

    #[test]
    fn g2_destructive_empty_holds_without_confirm() {
        let guard = VaultAvailabilityGuard::new(false);
        let v = guard.guard_diff(
            &empty_manifest(),
            Some(&non_empty_manifest()),
            Availability::Reachable,
        );
        assert_eq!(v, GuardVerdict::HoldDestructiveEmpty);
    }

    #[test]
    fn g3_empty_with_confirm_proceeds() {
        let guard = VaultAvailabilityGuard::new(true);
        let v = guard.guard_diff(
            &empty_manifest(),
            Some(&non_empty_manifest()),
            Availability::Reachable,
        );
        assert_eq!(v, GuardVerdict::Proceed);
    }

    #[test]
    fn g3_empty_with_no_prior_proceeds() {
        let guard = VaultAvailabilityGuard::new(false);
        let v = guard.guard_diff(&empty_manifest(), None, Availability::Reachable);
        assert_eq!(v, GuardVerdict::Proceed);
        let v2 = guard.guard_diff(
            &empty_manifest(),
            Some(&empty_manifest()),
            Availability::Reachable,
        );
        assert_eq!(v2, GuardVerdict::Proceed);
    }

    #[test]
    fn g4_non_empty_proceeds() {
        let guard = VaultAvailabilityGuard::new(false);
        let v = guard.guard_diff(
            &non_empty_manifest(),
            Some(&non_empty_manifest()),
            Availability::Reachable,
        );
        assert_eq!(v, GuardVerdict::Proceed);
    }

    #[test]
    fn auto_resume_is_stateless() {
        let guard = VaultAvailabilityGuard::new(false);
        // 먼저 hold 를 유발.
        let held = guard.guard_diff(
            &empty_manifest(),
            Some(&non_empty_manifest()),
            Availability::Reachable,
        );
        assert_eq!(held, GuardVerdict::HoldDestructiveEmpty);
        // 복구(reachable + non-empty) 시 이력과 무관하게 즉시 Proceed(멱등).
        let resumed = guard.guard_diff(
            &non_empty_manifest(),
            Some(&non_empty_manifest()),
            Availability::Reachable,
        );
        assert_eq!(resumed, GuardVerdict::Proceed);
    }
}

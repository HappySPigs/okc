//! U7a 도메인 제약 준수 `proptest` 제너레이터.
//!
//! `arb_gate_config` 는 `t_poll > 0`(fake clock 진행 보장) 불변식을 유지하고, `arb_artifact_specs`
//! 는 인덱스 접두로 경로 유일성을 보장하는 아티팩트 트리 스펙을 생성한다(존재/부재/vault-하위/권한
//! 실패 혼합). `arb_liveness_plan` 은 헬스 게이트 통과/타임아웃 경계 property(PROP-U7A-01)를 위한
//! "언제 Alive 가 되는가" 계획을 생성한다.

use std::time::Duration;

use foundation::Version;
use proptest::prelude::*;

use crate::update::GateConfig;
use crate::uninstall::ArtifactKind;

/// 임의 버전 문자열(`Version`). 비어있지 않은 짧은 식별자.
pub fn arb_version() -> impl Strategy<Value = Version> {
    "[0-9]{1,2}\\.[0-9]{1,2}\\.[0-9]{1,2}".prop_map(Version)
}

/// bounded-time 게이트 파라미터 — `t_poll` 은 항상 `>= 1ns`(진행 보장), `t_gate` 는 `0..=40ns`.
///
/// property 테스트가 정수 나노초 모델로 통과/타임아웃 경계를 예측하기 위해 작은 나노초 범위를 쓴다.
pub fn arb_gate_config() -> impl Strategy<Value = GateConfig> {
    (0i64..=40, 1i64..=8, 0i64..=1_000_000).prop_map(|(g, p, h)| GateConfig {
        t_gate: Duration::from_nanos(g as u64),
        t_poll: Duration::from_nanos(p as u64),
        t_hold: Duration::from_nanos(h as u64),
    })
}

/// 헬스 게이트 liveness 계획 — `Some(k)` 는 `k` 회 `NotReady` 후 `Alive`, `None` 은 영원히 `NotReady`.
pub fn arb_liveness_plan() -> impl Strategy<Value = Option<usize>> {
    prop_oneof![Just(None), (0usize..8).prop_map(Some)]
}

/// 단일 아티팩트 스펙(제너레이터 산출 형상 — 테스트가 실제 경로/파일시스템으로 물질화).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactSpec {
    /// 아티팩트 분류.
    pub kind: ArtifactKind,
    /// 파일명 세그먼트(테스트가 인덱스 접두로 유일화).
    pub name: String,
    /// 초기 존재 여부.
    pub exists: bool,
    /// `vault_root` 하위 여부(볼트-안전 대상).
    pub under_vault: bool,
    /// 삭제 시 권한 실패 주입 여부(FakeFileSystem 대상).
    pub deny: bool,
}

/// 아티팩트 분류 제너레이터.
pub fn arb_artifact_kind() -> impl Strategy<Value = ArtifactKind> {
    prop_oneof![
        Just(ArtifactKind::UploadHistory),
        Just(ArtifactKind::SyncState),
        Just(ArtifactKind::Log),
        Just(ArtifactKind::ConfigToken),
    ]
}

/// 단일 아티팩트 스펙 제너레이터.
pub fn arb_artifact_spec() -> impl Strategy<Value = ArtifactSpec> {
    (
        arb_artifact_kind(),
        "[a-z]{1,6}",
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(|(kind, name, exists, under_vault, deny)| ArtifactSpec {
            kind,
            name,
            exists,
            under_vault,
            deny,
        })
}

/// 0..8 개 아티팩트 스펙 벡터 제너레이터(경로 유일성은 테스트가 인덱스 접두로 보장).
pub fn arb_artifact_specs() -> impl Strategy<Value = Vec<ArtifactSpec>> {
    prop::collection::vec(arb_artifact_spec(), 0..8)
}

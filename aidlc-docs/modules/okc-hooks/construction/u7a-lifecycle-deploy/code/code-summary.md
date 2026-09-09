# U7a lifecycle-deploy — Code Summary (구현 요약)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U7a Deploy & Update** -> Code Generation
**크레이트**: `lifecycle-deploy` (lib) · **소속 컴포넌트**: `ServiceManager`, `AutoUpdater`, `Uninstaller`
**근거 산출물**: FD 3종(`domain-entities.md` / `business-rules.md` / `business-logic-model.md`)
**NFR 참고**: NFR 단계는 사용자 지시로 **SKIP** -> 코드는 FD + 상류 크레이트 실제 공개 API 에 직접 근거한다.

---

## 1. 구현된 컴포넌트 + 공개 API 표면

세 개의 응집 애그리게이트를 노출한다. OS/프로세스/시계/파일시스템/업데이트-소스 상호작용은 전부 seam 트레이트
뒤로 격리되어 특권·네트워크·실시간 없이 결정적으로 단위테스트된다.

- **`ServiceManager`**(모듈 `service`): 3-OS 동일 계약 수명주기(`install`/`uninstall`/`start`/`stop`/`restart`/`status`).
  상위는 OS-불변 순수 로직으로 idempotency(R-SM-02)·상태 정합(R-SM-03)·`ServiceSpec` 검증(R-SM-01)만 담당하고,
  실 launchd plist / systemd unit / Windows SCM 호출은 `ServiceController` seam(`native_controller()` 로 OS 디스패치)에 격리.
  공개: `ServiceSpec`/`ServiceAccount`/`ServiceRegistration`/`ServiceError`/`ServiceController`.
- **`AutoUpdater`**(모듈 `update`): 채널 스테이징 -> `ServiceManager.restart()` 위임(D-U7A-12) -> 주입 `Clock` 기반
  bounded-time 헬스 게이트(`ReadJudgment::update_probe()` **전용**, D-U7A-04) -> 통과 시 `Committed`(last-good 승계),
  실패/타임아웃 시 `RolledBack`(last-good 복원 + `report_update_rollback` 1회 + hold 마커). apply 는 `Committed`/`RolledBack`/
  `Refused` 로 종료하며 스테이징 실패(`Download`/`Verify`)만 `Err` 조기 반환(상태 변경 없음).
  공개: `UpdateChannel`/`UpdateInfo`/`ArtifactRef`/`Checksum`/`RollbackState`/`HoldReason`/`UpdateOutcome`/`UpdateError`/`GateConfig`/`UpdateSource`.
- **`Uninstaller`**(모듈 `uninstall`): 서비스 등록 해제(선행) -> 경로 기반 idempotent 삭제(UploadHistory/SyncState/Log/
  ConfigToken, `purge_logs`/`purge_token` 게이트) -> 토큰 3경로 정리(R-UN-05) -> `UninstallReport` 판정(잔존 시 `PartialFailure`).
  삭제 직전 각 경로가 `vault_root` 하위가 아님을 lexical 정규화 후 재검사하는 볼트-안전 이중 방어(R-UN-04). 데몬 정지 상태에서도 동작.
  공개: `Artifact`/`ArtifactKind`/`ArtifactSet`/`UninstallOptions`/`UninstallReport`/`SkipTarget`/`SkipReason`/`UninstallError`/`FileSystem`/`FsError`/`StdFileSystem`/`TokenPurgePort`/`PurgeOutcome`/`PurgeUnsupported`/`UnsupportedTokenPurge`/`pre_purge_token_status`.
- seam now-source: `Clock`/`SystemClock`(모듈 `clock`).

## 2. 모듈 레이아웃

- `service.rs` — `ServiceManager` + `ServiceController` seam + per-OS best-effort 어댑터(`LaunchdController`/`SystemdController`/`WindowsScmController`). `std::process::Command` IO 라 순수 리프 lint-gate 없음(단 패닉 경로 없음).
- `update.rs` — `AutoUpdater` + `UpdateSource` seam + apply 상태머신/`run_gate` 폴링. 순수 오케스트레이션 -> `#![deny(clippy::unwrap_used/expect_used/indexing_slicing/panic)]`.
- `uninstall.rs` — `Uninstaller` + `FileSystem`/`TokenPurgePort` seam + `is_descendant`/`normalize_lexical` 볼트-안전 가드. `std::fs` IO(seam 뒤).
- `clock.rs` — `Clock` seam + `SystemClock`.
- `testing.rs`(`test`/`proptest-support` 게이트) — fake 더블(`FakeController`/`FakeClock`/`ScriptedProbe`/`RecordingCritical`/`FakeUpdateSource`/`FakeFileSystem`).
- `proptest_support/generators.rs` — U7a 도메인 제너레이터(게이트 config / 아티팩트 스펙 / liveness 계획 / 버전).

## 3. 의존성

**신규 외부 크레이트 0건.** `foundation`(`ReadJudgment`/`Liveness`/`CriticalEventSink`/`Version`/`RollbackReason`/
`Timestamp` 등 소비), `auth-consent`(`CredentialProvider.token_status()` — 정리 전 진단만, 삭제 API 아님),
`observability`(U6 구현은 조립 루트 U8 이 U0 트레이트 객체로 하향 주입 -> U7a 코드는 U6 구체 타입 미참조, 배선
정합을 위해 의존만 선언 — 비순환 하향 주입), `thiserror`, `proptest`(optional). OS/프로세스/시계/파일시스템은
`std::process`/`std::fs`/`std::time` 을 seam(`ServiceController`/`Clock`/`FileSystem`) 뒤에서 사용한다.
`proptest` 는 비-default feature 게이트라 프로덕션 그래프에 유입되지 않는다(PBT-07).

## 4. MVP 트림 (FD 대비 편차 — 코드 주석에 명시)

- `ServiceSpec` 은 전용 `AbsolutePath` newtype 대신 `PathBuf` + 설치 시점 절대성 검증(taxonomy 에 Validation 변형이 없어 실패는 `Io` 로 매핑).
- per-OS 어댑터는 얇은 best-effort shell-out 이며 root/OS 의존이라 단위테스트 대상 아님(Infra/통합테스트 이월); 오케스트레이션은 `FakeController` 로 완전 검증.
- `UpdateInfo.version` no-op 판정은 문자열 동등 비교만(semver 정렬 비교 이연).
- `HoldReason` 는 단일 사유 `Backoff`(지수 백오프 미도입).
- 롤백 시 last-good 아티팩트 실 스왑은 `UpdateSource` seam 뒤로 이연 -> MVP 는 best-effort `restart()` 만 재호출하고 결과 무시.
- secure-store 토큰: U5 에 삭제 API 부재 -> `TokenPurgePort` seam 으로 의도 표현, MVP 기본 포트(`UnsupportedTokenPurge`)는 항상 `Err(PurgeUnsupported)` -> skip 보고.
- Uninstaller 의 서비스 dereg 실패는 best-effort 무시(리포트에 서비스 SkipTarget 없음).
- U6 `observability::Clock` 은 `now()` 만 노출 -> sleep 필요한 게이트 루프를 위해 U7a 자체 `Clock` seam 을 둠(두 seam 미통합).

## 5. 테스트 커버리지 (총 25개 통과, 0 실패)

- **단위 테스트 18개**(`src` 내 `#[cfg(test)]`, 기본 `cargo test`): `service.rs` 6개(절대경로 검증·등록+autostart·미등록 idempotent uninstall·미등록 start/stop·restart·상태 불변식), `update.rs` 6개(게이트 통과 커밋·타임아웃 롤백+통지 1회·재기동 실패 롤백·스테이징 실패 무변경·hold 재-apply 거부·Disabled 채널), `uninstall.rs` 6개(descendant 판정·존재/부재 idempotent 삭제·옵션 게이트·토큰 3경로 skip·볼트 보호·권한 실패 PartialFailure).
- **property 테스트 7개**(`tests/prop_deploy.rs`, `proptest-support` 게이트):
  PROP-U7A-01(헬스 게이트 통과/타임아웃 배타·전수), PROP-U7A-02(롤백 last-good 복원 + 통지 정확히 1회),
  PROP-U7A-03(hold 구간 재-apply Refused + 스테이징 부수효과 없음), PROP-U7A-04(실 temp-dir uninstall idempotency),
  PROP-U7A-05(볼트-안전 불변식 — vault_root 하위 절대 미삭제), PROP-U7A-06(리포트 완결·무중복 + 부분실패 판정),
  PROP-U7A-07(임의 초기상태 idempotent uninstall + 상태 함의).

## 6. 검증 사실 (확인됨)

- 전체 workspace 빌드 성공(8개 크레이트).
- U7a 테스트 25개 전부 통과(0 실패).
- `cargo clippy --all-targets --features proptest-support -- -D warnings` CLEAN.

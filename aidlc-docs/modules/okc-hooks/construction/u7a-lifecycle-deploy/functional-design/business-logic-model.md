# U7a Lifecycle-Deploy — Business Logic Model (알고리즘 / 워크플로 / 상태머신)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U7a Deploy & Update** -> Functional Design -> 산출물 3/3 (`business-logic-model.md`)
**작성일**: 2026-09-08
**크레이트**: `lifecycle-deploy` (lib) · **소속 컴포넌트**: `ServiceManager`, `AutoUpdater`, `Uninstaller`
**전제(AUTOPILOT 확정, 계획 §3)**: D-U7A-01..12

> **문서 성격**: 이 문서는 U7a의 **핵심 로직·알고리즘·워크플로·상태머신·데이터 흐름·교차단위 경계**를 정의한다. 타입은 `domain-entities.md`, 규칙/불변식은 `business-rules.md`가 소유하며 이 문서는 그것을 재사용·참조한다(재정의 없음).
>
> **표기 규약**: 비즈니스 의미 중심 **기술중립 설계**. Rust스러운 시그니처는 **참고용**. 다이어그램은 ASCII 화살표(`A -> B`)와 표/목록으로만 기술(박스드로잉/유니코드 화살표 금지). English 식별자명은 원문 유지.

---

## 0. 소유 vs 소비 경계 (실제 API 기준)

| 관계 | 대상 | 실제 API / 타입 | 방향 |
|---|---|---|---|
| **소유(정의)** | `ServiceSpec`/`ServiceRegistration`/`ServiceError`, `UpdateInfo`/`UpdateOutcome`/`UpdateError`/`RollbackState`/`UpdateChannel`, `UninstallOptions`/`UninstallReport`/`Artifact*`/`SkipReason`/`UninstallError`, `ServiceController`/`UpdateSource`/`TokenPurgePort` seam | U7a `domain-entities.md` | 신규 |
| **소비(주입)** | 헬스 게이트 liveness | U0 `ReadJudgment::update_probe() -> Liveness`(U6 `StatusService` 구현, `Arc<dyn ReadJudgment>` 주입) | AutoUpdater -> U6 |
| **소비(주입)** | 롤백 알림 | U0 `CriticalEventSink::report_update_rollback(Version, Version, RollbackReason)`(U6 `CriticalErrorNotifier` 구현, `Arc<dyn CriticalEventSink>` 주입) | AutoUpdater -> U6 |
| **소비(조회)** | 볼트 루트 | U0 `ConfigProvider.current().vault_path`(볼트-안전 배제 기준) | Uninstaller -> U0 |
| **소비(진단)** | 토큰 존재 | U5 `CredentialProvider.token_status()`(정리 전 진단용, 선택) — **삭제 API 없음** | Uninstaller -> U5 |
| **위임(동일 크레이트)** | 재기동/등록 해제 | `ServiceManager.restart()` / `.uninstall()` | AutoUpdater/Uninstaller -> ServiceManager |

> **비순환·하향 주입**: U7a는 U6/U5 **구체 타입을 참조하지 않고** U0 트레이트 객체만 주입받는다. 조립 루트(U8)가 U6 구현(`StatusService`/`CriticalErrorNotifier`)을 생성해 U0 트레이트로 U7a에 주입한다 -> `lifecycle-deploy`의 `[dependencies]`는 `foundation`(+ 배선상 `observability`/`auth-consent`)이며 역참조 없음(`unit-of-work.md` §3.1).

---

## 1. ServiceManager — per-OS seam 디스패치

### 1.1 seam 구조 (D-U7A-01)

```
ServiceManager (OS-불변 오케스트레이션)
  -> ServiceController (seam trait): register / deregister / start / stop / query
       -> LaunchdController   (macOS, best-effort 얇은 어댑터)
       -> SystemdController   (Linux)
       -> WindowsScmController(Windows)
       -> FakeController      (테스트: 인메모리 등록/실행 상태)
```

- 상위 메서드(`install/uninstall/start/stop/restart/status`)는 `ServiceController`를 호출하고, idempotency(R-SM-02)·상태 정합(R-SM-03)·`ServiceSpec` 검증(R-SM-01)만 순수 로직으로 담당한다. 실 launchd plist / systemd unit / SCM 호출은 어댑터에 격리(MVP: best-effort, 단위테스트는 `FakeController`).

### 1.2 install 알고리즘

```
install(spec):
  validate(spec)                       // exec_path/working_dir 절대경로, R-SM-01
  controller.register(spec)            // 유닛 등록 + autostart(spec.autostart 기본 true)
  if spec.autostart: controller.enable_autostart()
  -> Ok(()) | Err(UnsupportedPlatform|PermissionDenied|Io)
```

- `restart()`는 `controller.stop()` 후 `controller.start()`(AutoUpdater 재기동 진입점, R-AU-02). 미등록이면 `NotInstalled`.

---

## 2. AutoUpdater — apply 상태머신 + 헬스 게이트

### 2.1 상태머신 (텍스트 전이표)

| 현재 | 이벤트/조건 | 다음 | 부수효과 |
|---|---|---|---|
| Idle | `apply_update` 호출 & `hold_until` 미래 | Refused | `UpdateOutcome::Refused{Backoff}` 반환(R-AU-04) |
| Idle | `apply_update` 호출 & hold 없음 | Staging | `UpdateSource.fetch`+`verify`(checksum) |
| Staging | verify 실패 | Idle | `Err(UpdateError::Download\|Verify)`, 상태 변경 없음(R-AU-05) |
| Staging | verify 통과 | Restarting | `ServiceManager.restart()` |
| Restarting | restart 실패 | RollingBack | reason = RestartFailed(R-AU-03, D-U7A-12) |
| Restarting | restart 성공 | Gating | 게이트 폴링 시작(clock) |
| Gating | `update_probe()==Alive` (T_gate 내) | Committed | `last_good` 승계, hold 해제(R-AU-05) |
| Gating | T_gate 경과 & Alive 미관측 | RollingBack | reason = GateTimeout |
| RollingBack | 롤백 완료 | RolledBack | `rollback(last_good)` + `report_update_rollback` 1회 + `hold_until=now+T_hold`(R-AU-03/04) |

- **종료 상태**: `Committed` / `RolledBack` / `Refused`. `Staging` 실패는 `UpdateError`로 조기 반환(재기동 이전이라 롤백 불필요).

### 2.2 헬스 게이트 폴링 루프 (bounded-time, R-AU-01/02)

```
gate(probe: &dyn ReadJudgment, clock, T_gate, T_poll) -> GateResult:
  deadline = clock.now() + T_gate
  loop:
    match probe.update_probe():                 // 순수 liveness만 (health_check 미소비, D-U7A-04)
      Alive               -> return Passed
      NotReady{ missing } -> if clock.now() >= deadline: return TimedOut(missing)
                             else: clock.sleep(T_poll)
```

- `update_probe()`는 `IdleReached`+`CredentialReadable` 두 신호에만 의존(U6 `status.rs` 확인) -> US-E6-02 헬스체크 3조건("시작·idle 도달·토큰 읽기")과 정합하며, 운영 조건(`AuthFailed`/`OverLimit`) 변화에 영향받지 않아 롤백 루프를 방지한다.

### 2.3 데이터 흐름 (apply 1회)

```
UpdateChannel/UpdateInfo
  -> UpdateSource.fetch+verify         (seam, mock 가능)
  -> ServiceManager.restart()          (동일 크레이트 위임)
  -> ReadJudgment.update_probe()*N     (U6 주입, 폴링)
  -> [통과] RollbackState.last_good <- 직전버전 ; UpdateOutcome::Committed
  -> [실패] rollback(last_good)
          ; CriticalEventSink.report_update_rollback(from, to, reason)   (U6 주입, 1회)
          ; RollbackState.hold_until <- now+T_hold
          ; UpdateOutcome::RolledBack
```

---

## 3. Uninstaller — 정리 파이프라인

### 3.1 워크플로 (R-UN-01, 데몬-정지에서도 동작)

```
uninstall(opts, artifact_set, vault_root):
  report = empty
  // 1) 서비스 등록 해제(선행)
  if opts.deregister_service:
     ServiceManager.uninstall()         // idempotent(R-SM-02); 데몬 정지 유발
  // 2) 아티팩트 경로 기반 삭제
  for a in artifact_set:
     if a.kind == Log and not opts.purge_logs: continue
     if a.kind == ConfigToken and not opts.purge_token: continue   // R-UN-05: config 토큰은 purge_token일 때만 삭제
     if is_descendant(a.path, vault_root):        // 볼트-안전 이중 방어(R-UN-04)
        report.skip(a, VaultProtected); continue
     match delete_path(a.path):
        Ok(existed=true|false) -> report.removed(a)      // 부재도 성공(R-UN-02)
        Err(Permission)        -> report.skip(a, PermissionDenied)
        Err(io)                -> report.skip(a, PermissionDenied)  // 잔존 항목
  // 3) 토큰 정리(R-UN-05)
  if opts.purge_token:
     // config 토큰: ConfigToken 아티팩트는 위 루프에서 purge_token 게이트 하에 경로 삭제됨(위 가드)
     report.skip(EnvToken, EnvTokenExternal)             // 프로세스가 영속 env 해제 불가
     match TokenPurgePort.purge_secure_store_token():
        Ok(_)              -> report.removed(SecureStoreToken)
        Err(Unsupported)   -> report.skip(SecureStoreToken, SecureStoreRemovalUnsupported)
  // 4) 결과 판정(R-UN-06)
  if report.skipped has any PermissionDenied:
     return Err(UninstallError::PartialFailure(report))
  else:
     return Ok(report)
```

### 3.2 볼트-안전 판정 (R-UN-04, RISK-01)

- `is_descendant(path, vault_root)`: 정규화 후 `path`가 `vault_root` 또는 그 하위이면 `true`. `true`면 절대 삭제하지 않고 `VaultProtected` skip. 조립 루트(U8)의 `ArtifactSet` 구성 배제(D-U7A-08)에 더한 실행시 이중 방어.

### 3.3 토큰 3경로 데이터 흐름 (R-UN-05, 삭제 API 부재 반영)

```
config token    : config 파일 = ConfigToken 아티팩트 -> 경로 삭제(전체 폐기)
env token       : 프로세스 권한 밖 -> Skipped(EnvTokenExternal)  [자문]
secure-store    : U5 삭제 API 없음 + MVP UnavailableSecureStore
                  -> TokenPurgePort.purge_secure_store_token() = Err(Unsupported)
                  -> Skipped(SecureStoreRemovalUnsupported)
```

> **정합 주석**: `unit-of-work.md`/task는 "secure-store 삭제를 U5 `CredentialProvider`에 위임"으로 기술하나, U5 공개 API에 삭제 메서드가 **부재**하다(`credential/mod.rs`: `resolve_token`/`token_status`, `secure_store.rs`: `read_token`만). 동결 크레이트를 편집하지 않고 존재하지 않는 API도 발명하지 않으므로, U7a는 `TokenPurgePort` seam으로 의도를 표현하고 MVP에서 "미지원 -> skip"으로 정직하게 보고한다. 향후 U5가 삭제 API를 노출하면 이 seam을 그 API로 배선한다.

---

## 4. 컴포넌트별 Testable-Properties 노트 (PBT-01)

- **ServiceManager**: `FakeController`로 임의 등록/실행 초기상태를 생성 -> PROP-U7A-07(idempotent uninstall + 상태 함의). 실 OS 어댑터는 PBT 대상 아님(root 필요, 통합테스트/Infra 이월).
- **AutoUpdater**: 주입 `Clock` + 기록 `CriticalEventSink` + 임의 `update_probe` 준비 시퀀스 + fake restart 성공/실패 -> PROP-U7A-01(게이트 배타·전수), PROP-U7A-02(롤백 복원 + 알림 1회), PROP-U7A-03(hold 거부 + probe 격리). 오케스트레이션 로직이 순수하므로 실 네트워크/재기동 없이 완전 검증.
- **Uninstaller**: temp-dir 임의 아티팩트 트리(존재/부재/vault_root-하위/권한실패 혼합) -> PROP-U7A-04(idempotency), PROP-U7A-05(볼트-안전 불변식), PROP-U7A-06(리포트 완결·무중복 + 부분실패 판정).
- **UpdateSource 실 획득/서명검증**: No PBT properties identified — seam 뒤 단위/통합테스트 대상(NFR/Code Gen 이월).

---

## 5. 확장 컴플라이언스 요약

| 확장 | 활성 | 판정 |
|---|---|---|
| **Resiliency Baseline** | ON | RESILIENCY-04(자동 롤백)이 §2 상태머신·게이트로 직접 구현 — 이 단위의 핵심 회복력. RESILIENCY-15(사건 표면화): §2.3 롤백 알림. Uninstaller §3은 데몬 정지·부분실패에도 안전(idempotent). RPO/RTO·HA/DR·배포 자동화 파이프라인은 Infra/Ops 이월(부분 N/A) |
| **Property-Based Testing** | ON (Full) | §4 컴포넌트별 속성 + 제너레이터(주입 clock / fake controller / temp-dir / 임의 probe 시퀀스). 프레임워크(proptest) NFR 이월(PBT-09), shrinking/시드/CI는 PBT-08 |
| **Security Baseline** | OFF | N/A. config 평문 토큰(RISK-01)은 문서화된 수용 위험, §3.3 정리 대상 |

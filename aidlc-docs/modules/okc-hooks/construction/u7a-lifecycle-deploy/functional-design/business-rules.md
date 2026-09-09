# U7a Lifecycle-Deploy — Business Rules (결정 규칙 / 검증 로직 / 제약)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U7a Deploy & Update** -> Functional Design -> 산출물 2/3 (`business-rules.md`)
**작성일**: 2026-09-08
**크레이트**: `lifecycle-deploy` (lib) · **소속 컴포넌트**: `ServiceManager`, `AutoUpdater`, `Uninstaller`
**전제(AUTOPILOT 확정, 계획 §3)**: D-U7A-01..12

> **문서 성격**: 이 문서는 U7a가 소유하는 **결정 규칙·검증 로직·제약·불변식·엣지 케이스**를 정의한다. 타입 정의는 자매 산출물 `domain-entities.md`가 소유하며 이 문서는 그 타입명·필드명을 **그대로 재사용**한다(재정의·모순 없음). 알고리즘/흐름/시퀀스는 `business-logic-model.md`가 소유한다.
>
> **표기 규약**: 비즈니스 의미 중심 **기술중립 설계**. Rust스러운 시그니처는 **참고용**이며 규칙의 개념 형상을 표현한다. 다이어그램은 ASCII 화살표(`A -> B`)와 표/목록으로 기술(박스드로잉 금지). English 식별자명은 원문 유지.

---

## 1. ServiceManager 규칙 (US-E6-01 / FR-21 / NFR-05)

**규칙 R-SM-01 (3-OS 동일 계약 + autostart 기본값)**: `install/uninstall/start/stop/restart/status`는 macOS(launchd) / Linux(systemd) / Windows(Service)에서 **동일한 계약**으로 노출된다. per-OS 차이는 `ServiceController` seam 뒤에 격리되고(D-U7A-01), 상위 오케스트레이션 로직은 OS-불변이다. `ServiceSpec.autostart`는 기본 `true`이며 설치 시 부팅/로그인 자동시작 유닛으로 등록된다(FR-21, US-E6-01 "재부팅/로그인 시 자동 기동").

**규칙 R-SM-02 (idempotent uninstall)**: `uninstall()`은 대상 서비스가 **미등록이어도 성공**(`Ok`)한다 — 미등록은 오류가 아니라 목표 상태 달성이다. 반복 호출은 동일 결과(무변화). 반면 `start/stop/status`는 미등록 시 `ServiceError::NotInstalled`를 반환한다(질의/제어는 존재 전제).

**규칙 R-SM-03 (상태 질의 정합성)**: `status()`가 반환하는 `ServiceRegistration`은 실제 OS 상태를 진실하게 반영한다: `running == true`이면 반드시 `registered == true`, `pid.is_some()`이면 반드시 `running == true`. 미지원 OS는 `ServiceError::UnsupportedPlatform`, 특권 부족은 `PermissionDenied`.

**엣지 케이스**: 이미 실행 중인 서비스에 `start()` -> no-op 성공(중복 기동 방지). 이미 정지에 `stop()` -> no-op 성공.

---

## 2. AutoUpdater 규칙 (US-E6-02 / FR-20 / NFR-16 / RESILIENCY-04)

**규칙 R-AU-01 (헬스 게이트 = `update_probe` 전용, 운영 health 미소비)**: apply 후 헬스 게이트는 **오직 `ReadJudgment::update_probe() -> Liveness`만** 소비한다. 운영 `health_check()`는 **소비하지 않는다**(D-U7A-04, `unit-of-work.md` §0 노트3). 근거: 일시적 운영 조건(`AuthFailed`/`OverLimit`)은 좋은 새 버전을 오판 롤백시키는 **롤백 루프**를 유발하므로, 순수 liveness(startup + `IdleReached` + `CredentialReadable`)만으로 판정한다. `update_probe()`가 `Liveness::Alive`이면 통과, `NotReady`가 지속되면 미통과.

**규칙 R-AU-02 (bounded-time 게이트 판정)**: 게이트는 주입 `Clock` 기준 상한 `T_gate` 내에서 간격 `T_poll`로 `update_probe()`를 폴링한다(D-U7A-05). `T_gate` 이전에 `Alive` 관측 -> 통과(`Committed`). `T_gate` 경과까지 `Alive` 미관측 -> `UpdateError::GateTimeout` -> 롤백 트리거. `ServiceManager.restart()` 자체 실패 -> `UpdateError::RestartFailed` -> 즉시 롤백 트리거(D-U7A-12). 세 값(`T_gate`/`T_poll`/기본치)은 입력이며 config 배선은 NFR/U8 이월.

**규칙 R-AU-03 (자동 롤백 + last-good 복원 + 알림)**: 게이트 미통과(GateTimeout) 또는 재기동 실패(RestartFailed)가 확정되면:
1. `RollbackState.last_good`(단일 슬롯, D-U7A-02)로 `rollback(to: last_good)` 수행 -> 데몬이 이전 버전으로 다시 idle 도달.
2. `CriticalEventSink::report_update_rollback(from, to, reason)`를 **정확히 1회** 호출한다(FR-19 케이스4). `from` = 시도 버전, `to` = `last_good`, `reason` = `RollbackReason`(GateTimeout/RestartFailed 서술).
3. 최종 결과 = `UpdateOutcome::RolledBack { from, to, reason }`.

**규칙 R-AU-04 (롤백 루프 방지 — hold 마커)**: 롤백 완료 시 `RollbackState.hold_until`을 `now + T_hold`로 세팅하고 `last_rollback = from`을 기록한다(D-U7A-06). `hold_until`이 미래인 동안 동일/신규 `apply_update()` 호출은 스테이징/재기동을 수행하지 않고 `UpdateOutcome::Refused { reason: Backoff }`로 즉시 종료한다. 성공적 `Committed` 시 `hold_until`은 해제된다. (정교한 지수 백오프는 미도입 — 단순 hold, MVP.)

**규칙 R-AU-05 (커밋 시 last-good 승계)**: `Committed`가 확정되면 새 버전이 정상 버전이 되고, 직전 버전이 `RollbackState.last_good`으로 갱신(승계)되며 이전 아티팩트가 롤백 지점으로 보존된다(US-E6-02 "이전 버전 아티팩트는 롤백 지점으로 보존"). 스테이징 검증 실패(`Download`/`Verify`)는 재기동 이전이므로 롤백 불필요 — 현재 버전 그대로 유지하고 `UpdateError`를 반환(no state change).

**엣지 케이스**: `UpdateChannel::Disabled`이면 `check_for_update()`는 항상 `None`(apply 미발생). `check`가 반환한 버전이 현재 이하이면 no-op(단조 위반 무시).

---

## 3. Uninstaller 규칙 (US-E6-04 / RISK-01 / FR-16/02/03/17/13)

**규칙 R-UN-01 (정리 순서 및 데몬-정지 전제)**: `uninstall()`은 (1) `deregister_service`이면 `ServiceManager.uninstall()`(등록 해제 + 데몬 정지) -> (2) `ArtifactSet` 경로 기반 삭제(UploadHistory/SyncState/Log/ConfigToken) -> (3) 토큰 정리(§R-UN-05) -> (4) `UninstallReport` 조립 순으로 진행한다. **데몬이 실행 중이 아닐 때도 안전하게 동작**한다(US-E6-04 불변식) — 파일 삭제는 실행 상태에 의존하지 않는다.

**규칙 R-UN-02 (idempotency)**: 각 아티팩트 삭제는 멱등이다 — **이미 부재한 대상은 오류가 아니라 성공**(`Removed`로 집계, 사유 `AlreadyAbsent`)이다. 전체 `uninstall()`을 반복 호출하면 첫 호출과 동일 종료 상태에 도달하며 두 번째 호출의 실제 삭제 부수효과는 없다(US-E6-04 "이미 삭제된 항목은 no-op").

**규칙 R-UN-03 (경로 기반 삭제만 — 콘텐츠 해석 없음)**: Uninstaller는 아티팩트의 **경로만** 근거로 삭제하며 파일 내용을 파싱/해석하지 않는다(순수·단순). `ArtifactKind`는 리포트 분류·`purge_logs`/`purge_token` 스위치 판정에만 쓰인다.

**규칙 R-UN-04 (볼트-안전 불변식 — 절대 미접촉)**: 삭제 직전 각 대상 `path`가 `vault_root`(= `ConfigProvider.current().vault_path`)의 **하위 경로가 아님**을 검사한다. 하위이면 삭제하지 않고 `Skipped(_, VaultProtected)`로 기록한다(D-U7A-09). 이는 조립 루트의 구조적 배제(D-U7A-08)에 더한 **이중 방어**다 — 볼트 원본은 어떤 경로로도 삭제되지 않는다(US-E6-04 불변식, RISK-01).

**규칙 R-UN-05 (토큰 3경로 정리, 삭제 API 부재 반영)**: `purge_token`이 `true`일 때:
- **config 평문 토큰**: config 파일을 `ConfigToken` 아티팩트로 경로 기반 삭제한다(전체 폐기 의미, D-U7A-07a). config 토큰은 RISK-01 로컬 평문 산출물이다(requirements §13).
- **env 토큰**: 프로세스는 부모가 설정한 영속 환경변수(`OKC_WATCHER_TOKEN`)를 제거할 수 없다 -> `Skipped(EnvToken, EnvTokenExternal)` 자문 보고(D-U7A-07b).
- **secure-store 토큰**: U5 `CredentialProvider`/`SecureStore`에 **삭제 API가 존재하지 않고** MVP 기본이 `UnavailableSecureStore`이므로, U7a `TokenPurgePort.purge_secure_store_token()`이 `Err(PurgeUnsupported)`를 반환 -> `Skipped(SecureStoreToken, SecureStoreRemovalUnsupported)` 보고(D-U7A-07c). **존재하지 않는 U5 API를 호출하지 않는다.**

**규칙 R-UN-06 (부분 실패 완결 보고)**: 모든 대상은 정확히 한 번 `removed` 또는 `skipped`에 기록되며 두 목록의 대상 합집합 = 전체 시도 대상(중복 없음). 권한 등으로 하나 이상 삭제 실패(`PermissionDenied`)가 잔존하면 `Err(UninstallError::PartialFailure(report))`로 **어떤 항목이 남았는지 명확히 보고**한다(US-E6-04 AC). 잔존 없으면 `Ok(report)`.

---

## 4. Testable Properties (PBT-01 강제, 확장 ON Full)

각 속성은 MVP seam(주입 `Clock`, fake `ServiceController`, temp-dir `ArtifactSet`, 임의 `update_probe` 준비 시퀀스)으로 결정적으로 검증 가능하다. 프레임워크(proptest)는 NFR Requirements 이월(PBT-09), shrinking/고정시드/CI는 PBT-08.

| ID | 컴포넌트 | 속성(불변식) | 제너레이터 | 근거 |
|---|---|---|---|---|
| **PROP-U7A-01** | AutoUpdater | 임의의 probe 준비 시퀀스와 `T_gate`에 대해: `Alive`가 `T_gate` 이내에 관측되면 결과는 `Committed`, 그렇지 않으면 결과는 `RolledBack`(GateTimeout). 배타·전수(둘 중 정확히 하나) | 임의 `Liveness` 시퀀스 + 임의 `T_gate`/`T_poll` + 주입 clock | R-AU-01/02, NFR-16 |
| **PROP-U7A-02** | AutoUpdater | 롤백이 발생하는 모든 실행에서 `rollback(to)`는 항상 `last_good`을 복원하고 `report_update_rollback(from, to, reason)`은 **정확히 1회** 호출된다(중복/누락 없음) | 실패 유발 probe 시퀀스 + fake restart 실패 주입 + 기록 `CriticalEventSink` | R-AU-03, FR-19 케이스4 |
| **PROP-U7A-03** | AutoUpdater | 롤백 직후 `hold_until` 유효 구간 내 모든 `apply_update` 호출은 `Refused`이며 스테이징/재기동 부수효과가 없다; 또한 임의의 운영 조건(`AuthFailed`/`OverLimit` raise)은 게이트 판정을 바꾸지 않는다(probe 격리) | 임의 재-apply 타이밍 + 임의 운영 조건 주입 + 주입 clock | R-AU-04, D-U7A-04(롤백 루프 방지) |
| **PROP-U7A-04** | Uninstaller | 임의 `ArtifactSet`에 대해 `uninstall()`을 2회 적용한 종료 파일시스템 상태 == 1회 적용 상태(idempotent); 2회차 실제 삭제 부수효과 = 없음 | temp-dir 임의 아티팩트 트리(존재/부재 혼합) | R-UN-02, US-E6-04 |
| **PROP-U7A-05** | Uninstaller | 임의 `ArtifactSet`와 임의 `vault_root`에 대해, `vault_root` 하위의 어떤 경로도 삭제되지 않는다(전 실행에서 불변) | 임의 경로 트리(일부는 vault_root 하위로 생성) | R-UN-04, RISK-01, US-E6-04 |
| **PROP-U7A-06** | Uninstaller | 임의의 삭제 실패 부분집합(권한 주입)에 대해, `removed`와 `skipped` 대상 합집합 = 전체 대상이며 교집합 = 공집합(완결·무중복); 잔존 존재 <=> `PartialFailure` 반환 | temp-dir + 임의 per-artifact 권한 실패 주입 seam | R-UN-06, US-E6-04 AC |
| **PROP-U7A-07** | ServiceManager | 임의 초기 등록 상태에 대해 `uninstall()`은 항상 `Ok`(미등록 포함, idempotent)이며 이후 `status().registered == false`; `status()`가 보고하는 `ServiceRegistration`은 `running -> registered`, `pid.is_some() -> running` 함의를 항상 만족 | fake `ServiceController`(임의 등록/실행 상태) | R-SM-02/03 |

> **속성 없는 요소**: `check_for_update`의 실 네트워크/서명 검증(`UpdateSource` seam 뒤)은 이 단위의 PBT 대상이 아니다 — "No PBT properties identified"(단위테스트/통합테스트 대상, NFR/Code Gen 이월).

---

## 5. 확장 컴플라이언스 요약

| 확장 | 활성 | 판정 |
|---|---|---|
| **Resiliency Baseline** | ON | RESILIENCY-04(롤백): R-AU-01..05가 직접 구현(헬스게이트+자동롤백+루프방지). RESILIENCY-15(사건 표면화): R-AU-03 롤백 알림. R-UN-01/02 데몬-정지·idempotent = 회복력. RPO/RTO·HA/DR는 Infra/Ops 이월(부분 N/A) |
| **Property-Based Testing** | ON (Full) | §4 PROP-U7A-01..07 + 제너레이터 명시. 프레임워크는 NFR 이월(PBT-09) |
| **Security Baseline** | OFF | N/A. config 평문 토큰(RISK-01)은 수용 위험이며 R-UN-05 정리 대상 |

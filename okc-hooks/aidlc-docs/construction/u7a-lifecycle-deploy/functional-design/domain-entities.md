# U7a Lifecycle-Deploy — Domain Entities (도메인 엔티티 / 값 타입)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U7a Deploy & Update** -> Functional Design -> 산출물 1/3 (`domain-entities.md`)
**작성일**: 2026-09-08
**크레이트**: `lifecycle-deploy` (lib) · **소속 컴포넌트**: `ServiceManager`, `AutoUpdater`, `Uninstaller`
**전제(AUTOPILOT 확정, 계획 §3)**: D-U7A-01(ServiceController seam) · D-U7A-02(단일 last-good 슬롯) · D-U7A-03(UpdateSource seam) · D-U7A-04(update_probe 전용) · D-U7A-06(단순 hold 백오프) · D-U7A-07(토큰 3경로) · D-U7A-08(ArtifactSet 주입) · D-U7A-10(ServiceSpec 입력) · D-U7A-11(오류 taxonomy)

> **문서 성격**: 이 문서는 U7a가 **도입/특화하는 값 타입·엔티티**를 정의한다. U0(`foundation`)/U5(`auth-consent`)/U6(`observability`)가 소유하는 타입은 **이름으로 참조만** 하며 **재정의하지 않는다**(§0 표). 규칙(검증·불변식)은 자매 산출물 `business-rules.md`, 알고리즘/흐름은 `business-logic-model.md`가 소유한다.
>
> **표기 규약**: 비즈니스 의미 중심 **기술중립 설계**(기술/크레이트 선택은 후속 NFR 단계). Rust스러운 시그니처는 **참고용(reference-only)** 이며 인프라·스레딩·async·I/O 메커니즘이 아니라 개념 형상을 표현한다. 다이어그램은 ASCII 화살표(`A -> B`)와 표/목록으로만 기술. English 식별자명은 원문 유지.

---

## 0. 소비하는 타입 (재정의 금지 — 참조만)

`lifecycle-deploy`는 아래 동결 크레이트 타입을 이름으로만 참조한다.

| 원 소유 | 타입 / API | U7a에서의 사용 |
|---|---|---|
| U0 `foundation` | `ReadJudgment` trait (`update_probe() -> Liveness`) | `AutoUpdater`가 `Arc<dyn ReadJudgment>`로 주입받아 헬스 게이트에 소비(운영 `health_check()` **미소비**, D-U7A-04) |
| U0 `foundation` | `Liveness { Alive \| NotReady { missing: Vec<LivenessSignal> } }` | 게이트 판정 반환 형상. `Alive`만 통과 |
| U0 `foundation` | `CriticalEventSink` trait (`report_update_rollback(from: Version, to: Version, reason: RollbackReason)`) | `AutoUpdater`가 `Arc<dyn CriticalEventSink>`로 주입받아 롤백 시 호출(FR-19 케이스4) |
| U0 `foundation` | `Version(pub String)` · `RollbackReason(pub String)` | 업데이트/롤백 버전 식별·사유(placeholder newtype, U0 소유). U7a는 값 생성·전달만 |
| U0 `foundation` | `ConfigProvider` / `ConfigSnapshot` (`current()`) | `vault_path` 조회(볼트-안전 배제 기준) + 업데이트 채널 등 배선. 산출물 경로 자체는 U8이 해소(D-U7A-08) |
| U6 `observability` | `StatusService` (`ReadJudgment` 구현체) | 조립 루트가 `Arc<dyn ReadJudgment>`로 U7a에 주입하는 **구체 구현**. U7a는 구체 타입 미참조(트레이트만) |
| U6 `observability` | `CriticalErrorNotifier` (`CriticalEventSink` 구현체) | 조립 루트가 `Arc<dyn CriticalEventSink>`로 주입하는 **구체 구현** |
| U5 `auth-consent` | `CredentialProvider` (`resolve_token`/`token_status`/`read_token`) | 토큰 **존재 확인**(정리 전 진단)에 소비 가능. **삭제 API 부재** -> secure-store 삭제는 U7a `TokenPurgePort` seam으로 처리(D-U7A-07, `business-rules.md` R-UN-05) |

> **핵심 제약(재확인)**: U5 `CredentialProvider`/`SecureStore` 공개 API에는 토큰 **삭제/제거 메서드가 없다**. U7a는 존재하지 않는 API를 호출하지 않으며, secure-store 토큰 정리는 자체 seam이 "미지원 -> skip"으로 보고한다.

---

## 1. ServiceManager 값 타입

### 1.1 `ServiceSpec` (설치 스펙 — 입력값)

- **목적**: 서비스 등록에 필요한 배포 파라미터. U7a가 config에서 파싱하지 않고 **조립 루트(U8)가 조립해 입력**한다(D-U7A-10). MVP는 `account`=기동/현재 사용자, `working_dir`=기본 data-dir을 기본값으로 쓰며, 이 둘의 config 설정화는 **명시 이연**이다(구조적 불가 아님 — U7a가 federated config 키(예: `service`)를 등록하면 후속 성립. U2 `debounce_ms` 선례처럼 U0 federated union이 다운스트림 키를 허용). `autostart`는 부팅/로그인 자동시작으로 지원한다(FR-21, core).
- **필드 스키마**(참고용):

```
ServiceSpec {
  exec_path:    AbsolutePath,   // 배포 바이너리 절대 경로(U8: current_exe 유래)
  working_dir:  AbsolutePath,   // 데몬 작업 디렉터리(data-dir)
  account:      ServiceAccount, // 실행 계정(현재 사용자 기본)
  autostart:    bool,           // 부팅/로그인 자동시작(기본 true, FR-21)
}
```

- **불변식**: `exec_path`/`working_dir`는 절대 경로. `autostart` 기본 `true`(US-E6-01 자동시작). 세부는 `business-rules.md` R-SM-01.

### 1.2 `ServiceRegistration` (상태 질의 결과)

```
ServiceRegistration {
  registered: bool,           // OS 서비스 매니저에 유닛 등록됨
  running:    bool,           // 현재 실행 중
  pid:        Option<u32>,    // 실행 중일 때 PID
}
```

- **불변식**: `running == true`이면 `registered == true`(실행하려면 등록 필요). `pid.is_some()`이면 `running == true`.

### 1.3 `ServiceError` (오류 taxonomy, U7a 소유)

```
ServiceError {
  UnsupportedPlatform,   // 알 수 없는/미지원 OS
  PermissionDenied,      // 특권 부족(등록/해제)
  NotInstalled,          // start/stop/status 대상 미등록 (uninstall에선 idempotent no-op)
  Io(detail: String),    // 하위 seam I/O 실패
}
```

---

## 2. AutoUpdater 값 타입

### 2.1 `UpdateChannel` / `UpdateInfo` (업데이트 후보)

```
UpdateChannel { Stable, Beta, Disabled }   // 채널 config(D-U7A-03; Disabled면 check 항상 None)

UpdateInfo {
  version:      Version,        // U0 타입(참조)
  artifact_ref: ArtifactRef,    // UpdateSource seam이 해석하는 불투명 참조
  checksum:     Checksum,       // 스테이징 검증용(무결성)
}
```

- **불변식**: `check_for_update()`가 반환하는 `UpdateInfo.version`은 현재 실행 버전보다 상위(단조). `Disabled` 채널은 항상 `None`.

### 2.2 `RollbackState` (last-good + hold 마커 — 지속 상태)

- **목적**: 직전 정상 버전(단일 슬롯, D-U7A-02)과 롤백 루프 방지용 hold(D-U7A-06)를 지속.

```
RollbackState {
  last_good:        Version,           // 롤백 복원 대상(단일 슬롯)
  hold_until:       Option<Timestamp>, // 세팅 시 이 시각까지 재-apply 거부
  last_rollback:    Option<Version>,   // 마지막으로 롤백된(거부된) 버전 — 재시도 억제 진단
}
```

- **불변식**: `hold_until`이 미래이면 apply는 `refused`(no-op). `last_good`는 항상 "기동+idle+토큰읽기"를 통과한 적 있는 버전.

### 2.3 `UpdateOutcome` (apply 종료 결과)

```
UpdateOutcome {
  Committed { version: Version },                    // 게이트 통과 -> 새 버전 확정
  RolledBack { from: Version, to: Version, reason: RollbackReason }, // 롤백 완료
  Refused { reason: HoldReason },                    // hold 유효 구간 내 재-apply 거부(루프 방지)
}

HoldReason {
  Backoff,   // 롤백 직후 hold_until 유효 구간 -> 재-apply 즉시 거부(R-AU-04, 롤백 루프 방지). MVP는 단일 사유(지수 백오프 미도입)
}
```

- **참고**: `HoldReason`은 apply가 스테이징/재기동을 수행하지 않고 거부되는 사유다. MVP 흐름에서 거부는 hold 마커(`Backoff`) 한 경우뿐이며, "현재 이하 버전/업데이트 없음"은 `Refused`가 아니라 `check_for_update() -> None`/no-op으로 처리된다(`business-rules.md` R-AU-04, 엣지 케이스).

### 2.4 `UpdateError` (오류 taxonomy, U7a 소유)

```
UpdateError {
  Download,        // UpdateSource fetch 실패
  Verify,          // checksum/스테이징 검증 실패
  RestartFailed,   // ServiceManager.restart() 실패 -> 롤백 트리거(D-U7A-12)
  GateTimeout,     // bounded-time 내 update_probe Alive 미도달 -> 롤백 트리거
  Io(detail: String),
}
```

- **참고**: `RestartFailed`/`GateTimeout`은 apply 흐름 안에서 **롤백으로 귀결**되며 최종 결과는 `RolledBack`(오류가 아니라 정의된 결과). 순수 `UpdateError`는 스테이징 단계(Download/Verify) 실패처럼 재기동 이전 실패에 반환된다. 상세 매핑은 `business-logic-model.md` §2.

---

## 3. Uninstaller 값 타입

### 3.1 `Artifact` / `ArtifactKind` (정리 대상 — 주입 입력)

- **목적**: 조립 루트(U8)가 `ConfigProvider` + 플랫폼 data-dir 관례로 해소해 **주입하는** 로컬 평문 산출물 대상(D-U7A-08). U7a는 경로를 재해소하지 않고 소비만 한다.

```
ArtifactKind {
  UploadHistory,   // U6 히스토리 파일(FR-16)
  SyncState,       // U4 지속 상태 파일(FR-02/03 = 마지막 매니페스트 + dirty + 재개 오프셋)
  Log,             // U6 구조화 로그(FR-17)
  ConfigToken,     // config 평문 토큰 보유 파일(RISK-01 정리 대상, §13)
}

Artifact { kind: ArtifactKind, path: AbsolutePath }

ArtifactSet(Vec<Artifact>)   // 주입되는 정리 대상 집합(vault_root 하위 구조적 배제)
```

- **불변식(볼트-안전, D-U7A-09)**: `ArtifactSet`의 어떤 `path`도 `vault_root`의 하위가 아니다. 실행 시에도 재검사(`business-rules.md` R-UN-04).

### 3.2 `UninstallOptions` (정리 범위 스위치)

```
UninstallOptions {
  purge_token: bool,   // config/env/secure-store 토큰 정리 시도(D-U7A-07)
  purge_logs:  bool,   // Log 아티팩트 포함 여부
  deregister_service: bool, // ServiceManager.uninstall() 선행 여부(기본 true)
}
```

### 3.3 `UninstallReport` / `SkipReason` (결과 요약)

```
Removed(Artifact)                     // 실제 삭제됨(또는 이미 부재 -> idempotent 성공)
Skipped(target: SkipTarget, reason: SkipReason)

SkipTarget { Artifact(Artifact), EnvToken, SecureStoreToken }

SkipReason {
  AlreadyAbsent,                   // 이미 없음(idempotent no-op) — Removed로 집계 가능(R-UN-02)
  PermissionDenied,                // 권한 부족 -> 부분 실패 항목
  VaultProtected,                  // vault_root 하위 -> 안전 배제(절대 삭제 안 함)
  EnvTokenExternal,                // 프로세스가 영속 env 해제 불가 -> 자문 skip(D-U7A-07b)
  SecureStoreRemovalUnsupported,   // U5에 삭제 API 부재 + MVP UnavailableSecureStore(D-U7A-07c)
}

UninstallReport {
  removed: Vec<Artifact>,               // 삭제 성공/이미부재
  skipped: Vec<(SkipTarget, SkipReason)>, // 미삭제 항목 + 사유
}
```

- **불변식(완결성)**: `removed`와 `skipped`의 대상 합집합 = 전체 시도 대상(중복 없음). `business-rules.md` R-UN-06.

### 3.4 `UninstallError` (오류 taxonomy, U7a 소유)

```
UninstallError {
  PartialFailure(UninstallReport),  // 하나 이상 PermissionDenied 등 잔존 -> 리포트 동반
  Io(detail: String),               // 리포트 조립조차 불가한 하위 실패
}
```

- **참고**: 정상/완전 정리는 `Ok(UninstallReport)`. 일부만 실패해 잔존 항목이 있으면 `Err(PartialFailure(report))`로 **무엇이 남았는지 명확히 보고**(US-E6-04 AC).

### 3.5 `TokenPurgePort` (secure-store 삭제 seam — U7a 소유)

- **목적**: U5에 삭제 API가 없으므로, secure-store 토큰 정리 의도를 U7a 자체 seam으로 표현한다(D-U7A-07c). MVP 구현은 항상 `Unsupported`를 반환하는 null-object.

```
TokenPurgePort {
  purge_secure_store_token() -> Result<PurgeOutcome, PurgeUnsupported>
}
PurgeOutcome { Removed, AlreadyAbsent }
// MVP 기본 구현: 항상 Err(PurgeUnsupported) -> report.skipped(SecureStoreToken, SecureStoreRemovalUnsupported)
```

---

## 4. 엔티티 관계 (텍스트 표기)

```
ServiceSpec        -> ServiceManager.install()  -> ServiceRegistration | ServiceError
(U8 조립)                                           (status() 질의 결과)

UpdateChannel/UpdateInfo -> AutoUpdater.apply_update()
   AutoUpdater --(consumes)--> ReadJudgment.update_probe() : Liveness      [U6 주입]
   AutoUpdater --(delegates)-> ServiceManager.restart()                     [동일 크레이트]
   AutoUpdater --(persists)--> RollbackState (last_good + hold_until)
   AutoUpdater --(on rollback)-> CriticalEventSink.report_update_rollback() [U6 주입]
   결과: UpdateOutcome { Committed | RolledBack | Refused }

ArtifactSet + UninstallOptions -> Uninstaller.uninstall()
   Uninstaller --(delegates)-> ServiceManager.uninstall()
   Uninstaller --(path-delete)-> Artifact(UploadHistory|SyncState|Log|ConfigToken)
   Uninstaller --(seam)------> TokenPurgePort.purge_secure_store_token()
   Uninstaller --(guard)-----> vault_root 하위 배제 (VaultProtected)
   결과: UninstallReport (removed / skipped) | UninstallError::PartialFailure
```

---

## 5. 컴포넌트 -> 규칙 -> 속성 매핑(추적성)

| 컴포넌트 | 정의 타입 | 관련 규칙 | Testable Property |
|---|---|---|---|
| `ServiceManager` | `ServiceSpec`/`ServiceRegistration`/`ServiceError` | R-SM-01..03 | PROP-U7A-07 |
| `AutoUpdater` | `UpdateInfo`/`RollbackState`/`UpdateOutcome`/`UpdateError`/`UpdateChannel` | R-AU-01..05 | PROP-U7A-01/02/03 |
| `Uninstaller` | `Artifact`/`ArtifactSet`/`UninstallOptions`/`UninstallReport`/`SkipReason`/`UninstallError`/`TokenPurgePort` | R-UN-01..06 | PROP-U7A-04/05/06 |

# U7a Lifecycle-Deploy — Functional Design 계획 (AUTOPILOT)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U7a Deploy & Update** -> Functional Design (계획 + 결정)
**작성일**: 2026-09-08
**크레이트**: `lifecycle-deploy` (lib) · **소속 컴포넌트**: `ServiceManager`, `AutoUpdater`, `Uninstaller`
**대표 에픽**: E6 수명주기 & 업데이트 — 배포/업데이트/제거 슬라이스(US-E6-01/02/04)
**입력 아티팩트**: `unit-of-work.md`(§U7a 책임·얇은-단위 근거 + §0 노트3), `component-methods.md`(§ServiceManager/AutoUpdater/Uninstaller 시그니처·이월 항목), `unit-of-work-story-map.md`(U7a = US-E6-01/02/04), `stories.md`(US-E6-01/02/04 인수기준), `requirements.md`(FR-19 케이스4 / FR-20 / FR-21 / NFR-16 / RISK-01 / §13 토큰 부록)
**소비 크레이트(실제 공개 API, 동결)**: `foundation`(U0: `ReadJudgment::update_probe`/`Liveness`/`CriticalEventSink::report_update_rollback`/`Version`/`RollbackReason`/`ConfigProvider`/`ConfigSnapshot`), `observability`(U6: `StatusService`(=`ReadJudgment` 구현) + `CriticalErrorNotifier`(=`CriticalEventSink` 구현), 조립 루트가 U0 트레이트로 주입), `auth-consent`(U5: `CredentialProvider`)
**규칙**: `construction/functional-design.md` · `common/content-validation.md` · 활성 확장 `property-based-testing.md`(PBT-01 강제) · `resiliency-baseline.md`
**모드**: AUTOPILOT — 모든 미결정을 권장안 + MVP 최소범위로 자가 확정(§3). 사용자 질문 없음. drop-list(§5) 항목은 재개봉 금지.

> **표기 규약**: 비즈니스 의미 중심 **기술중립 설계**(기술/크레이트 선택은 후속 NFR 단계). Rust스러운 시그니처는 **참고용**이며 개념 형상을 표현한다. 다이어그램은 ASCII 화살표(`A -> B`)와 표/목록으로만 기술(박스드로잉/유니코드 화살표 금지). English 식별자명은 원문 유지.

---

## 1. 단위 컨텍스트

U7a는 기존 U7 분할의 **바이너리 생명주기** 절반이다 — Watcher 배포 아티팩트를 OS 네이티브 서비스로 **설치·업데이트·제거**하는 응집 단위. 실행 중 데몬 제어(U7b)와는 변경·설계 주기가 다르다. 세 컴포넌트가 각각 하나의 스토리를 소유한다:

| 컴포넌트 | 소유 스토리 | 책임 요지 |
|---|---|---|
| `ServiceManager` | US-E6-01 | launchd/systemd/Windows Service에 대한 3-OS 동일 계약 `install/uninstall/start/stop/restart/status` + 부팅/로그인 자동시작(FR-21) |
| `AutoUpdater` | US-E6-02 | 채널 config 스테이징 -> `ServiceManager.restart()` -> bounded-time 헬스 게이트(U6 `update_probe()`) -> 실패/타임아웃 시 직전 정상 버전 자동 롤백(FR-20/NFR-16) + 롤백 시 U6 `report_update_rollback`(FR-19 케이스4) |
| `Uninstaller` | US-E6-04 | 서비스 등록 해제 + 로컬 평문 산출물(히스토리/SyncState/로그/config 토큰) 경로 기반 idempotent 삭제 + 토큰 제거. 데몬 정지 상태에서도 동작, 볼트 원본 절대 미접촉(RISK-01) |

### 1.1 소비하는 실제 API 경계(동결 크레이트, 재정의 금지)

- **U0 `foundation`**: `ReadJudgment`(`update_probe() -> Liveness`), `Liveness{Alive|NotReady{missing}}`, `CriticalEventSink`(`report_update_rollback(from: Version, to: Version, reason: RollbackReason)`), `Version(pub String)`, `RollbackReason(pub String)`, `ConfigProvider`(`current() -> ConfigSnapshot`).
- **U6 `observability`**: `StatusService`(`ReadJudgment` + `StatusSink` 구현체), `CriticalErrorNotifier`(`CriticalEventSink` 구현체). U7a는 **구체 타입이 아니라 U0 트레이트 객체**(`Arc<dyn ReadJudgment>`/`Arc<dyn CriticalEventSink>`)로 주입받아 소비한다(하향 주입, 역참조 없음).
- **U5 `auth-consent`**: `CredentialProvider`. **중요 제약**: `CredentialProvider`/`SecureStore` 공개 API에는 **토큰 삭제 메서드가 존재하지 않는다**(`read_token`/`resolve_token`/`token_status`만). 따라서 secure-store 토큰 삭제는 U5에 위임 불가 -> §3 D-U7A-07의 seam+미지원 보고로 처리(존재하지 않는 API를 발명하지 않음).

### 1.2 Application Design에서 이월된 미결정 -> §3 AUTOPILOT 결정으로 확정

component-methods 이월 항목: 플랫폼별 유닛 파일 템플릿/권한(ServiceManager), 롤백 백오프/보류 지속·아티팩트 레이아웃·게이트 폴링 주기(AutoUpdater), 산출물 경로 최종 목록·플랫폼별 토큰 삭제(Uninstaller). 아래 §3에서 전부 자가 확정한다.

---

## 2. Functional Design 산출물 체크리스트 (기술중립, 프론트엔드 파일 없음)

`aidlc-docs/construction/u7a-lifecycle-deploy/functional-design/` 하위에 생성한다.

- [x] **`domain-entities.md`** — U7a가 도입/특화하는 값 타입: `ServiceSpec`/`ServiceRegistration`/`ServiceError`(ServiceManager), `UpdateInfo`/`UpdateOutcome`/`UpdateError`/`RollbackState`/`UpdateChannel`(AutoUpdater), `UninstallOptions`/`UninstallReport`/`Artifact`/`ArtifactKind`/`SkipReason`/`UninstallError`(Uninstaller). U0/U5/U6 타입(`Version`/`RollbackReason`/`Liveness`/`ReadJudgment`/`CriticalEventSink`/`ConfigSnapshot`/`CredentialProvider`)은 **이름 참조만**(§0 표). 컴포넌트 -> 규칙 -> 속성 매핑 포함.
- [x] **`business-rules.md`** — `R-*` 규칙: ServiceManager 3-OS 동일계약·idempotent uninstall·autostart 기본값; AutoUpdater 헬스게이트 경계·롤백 트리거·롤백 루프 방지 백오프·`update_probe` 전용 소비(운영 health_check 미소비); Uninstaller idempotency·볼트-안전 불변식·부분실패 보고·토큰 3경로(config/env/secure-store) 처리·경로 기반 삭제. `PROP-U7A-*` Testable Properties(PBT-01 강제).
- [x] **`business-logic-model.md`** — 컴포넌트별 알고리즘/워크플로/상태머신: AutoUpdater apply 상태머신(Staged -> Restarting -> Gating -> Committed | RolledBack), 헬스 게이트 폴링 루프(주입 clock + `update_probe`), Uninstaller 정리 파이프라인(서비스 해제 -> 아티팩트 순회 삭제 -> 토큰 정리 -> 리포트), ServiceManager per-OS seam 디스패치. 교차단위 경계(소유 vs 소비 + 실제 API), 컴포넌트별 Testable-Properties 노트 + 확장 컴플라이언스 요약.
- [x] **PBT-01 "Testable Properties" 섹션**(각 산출물, 확장 강제) — 주입 clock / fake `ServiceController` / temp-dir 아티팩트 트리 / 임의 `update_probe` 준비 시퀀스를 제너레이터로 명시(§3 MVP seam 기반).
- [x] 산출물 작성 전 `content-validation.md` 검증(특수문자, 표/코드블록 파싱, 박스드로잉 0건, ASCII 화살표만).

---

## 3. AUTOPILOT 결정 (권장 + MVP 편향, 인용 포함)

| # | 주제 | 확정(권장) | MVP 트림? | 근거 / 인용 |
|---|---|---|---|---|
| D-U7A-01 | OS 서비스 통합 경계 | `ServiceController` **seam 트레이트** + per-OS 구현(launchd/systemd/SCM)을 얇은 best-effort 어댑터로 분리. 오케스트레이션(설치 스펙 조립·idempotency·상태 질의)은 순수 로직으로 seam 위에서 테스트 | 예 | 특권 OS 서비스 호출은 root 없이 단위테스트 불가 -> seam 격리(MVP GUIDANCE). `component-methods` "플랫폼별 유닛 파일/권한: FD/Infra 이월" |
| D-U7A-02 | AutoUpdater 롤백 슬롯 수 | **직전 정상 버전 1개**(last-good 단일 슬롯) 보존. 다버전 히스토리 미도입 | 예 | US-E6-02 "이전 버전 아티팩트를 롤백 지점으로 보존"(단수). 다버전은 범위 확대 |
| D-U7A-03 | 업데이트 아티팩트 획득/검증 | `UpdateSource` seam(check/fetch/verify). MVP는 mock/fake 소스로 로직 검증, 실 네트워크·서명검증은 seam 뒤 이월 | 예 | 업로드 경로(U3/U5)와 무관한 별도 채널. 실 다운로드는 단위테스트 부적합 -> seam |
| D-U7A-04 | 헬스 게이트 판정 소스 | **`update_probe() -> Liveness`만** 소비(운영 `health_check()` 미소비). `Alive`면 통과, bounded-time 내 미도달 시 타임아웃 롤백 | 아니오 | `unit-of-work.md` §0 노트3 / `component-methods` 노트3: 일시 AuthFailed/OverLimit 오판 롤백 루프 방지. `status.rs` `update_probe`는 조건 격리 확인 |
| D-U7A-05 | 헬스 게이트 시간 모델 | **주입 `Clock` + 폴링 간격 `T_poll` + 상한 `T_gate`**. 둘 다 입력값(기본 제안값 명시, config 배선은 U8/NFR 이월) | 예 | 결정적 테스트 위해 clock 주입(MVP GUIDANCE). US-E6-02 "bounded time는 config 설정 가능(기본값 FD/NFR 이월)" |
| D-U7A-06 | 롤백 루프 방지 | 롤백 후 **hold 마커**(지속 플래그 + `retry_not_before` 시각) 세팅; hold 유효 구간 내 재-apply는 `refused`(no-op). 지수 백오프 스케줄은 미도입(단순 hold) | 예 | US-E6-02 불변식 "롤백 후 백오프/보류로 롤백 루프 회피". 정교한 백오프는 U4 도메인 -> 여기선 단순 hold |
| D-U7A-07 | 토큰 제거 3경로 | (a) **config 평문 토큰**: `purge_token` 시 config 파일을 아티팩트로 경로 기반 제거(전체 폐기 의미). (b) **env 토큰**: 프로세스가 영속 env를 해제 불가 -> `Skipped(EnvTokenExternal)` 자문 보고. (c) **secure-store 토큰**: U5에 삭제 API 부재 + MVP `UnavailableSecureStore` -> `TokenPurgePort` seam이 `Skipped(SecureStoreRemovalUnsupported)` 보고 | 예 | `credential/mod.rs`/`secure_store.rs`에 삭제 API **없음**(read만) -> 발명 금지. §13 "config 평문 토큰 = RISK-01 정리 대상". US-E6-04 AC "config/env/보안저장소 토큰 제거" |
| D-U7A-08 | 아티팩트 경로 출처 | Uninstaller는 **주입된 `ArtifactSet`**(각 `Artifact{kind, path}`)를 소비만 한다. 경로 해소(ConfigProvider `vault_path` 제외 + 플랫폼 data-dir 관례)는 조립 루트(U8) 책임 | 예 | `WatcherConfig`는 6개 core 키뿐(history/log/state 경로 필드 없음, 미지 키 strict-reject Q4=B). U4 `SyncStateStore`/U6 history·log가 각자 `PathBuf` 주입받는 실제 구조와 정합 -> U7a는 순수·temp-dir 테스트 가능 |
| D-U7A-09 | 볼트-안전 불변식 | `vault_root` 하위 경로는 `ArtifactSet`에서 **구조적으로 배제**되며, Uninstaller는 삭제 전 각 대상이 vault_root의 하위가 아님을 검사(위반 시 즉시 `Skipped(VaultProtected)`) | 아니오 | US-E6-04 불변식 "볼트 원본 절대 미접촉", RISK-01. 이중 방어(조립 배제 + 실행시 가드) |
| D-U7A-10 | ServiceSpec 조립 출처 | `ServiceSpec{exec_path, working_dir, account, autostart}`는 **입력값**으로 받는다(U8이 런타임 `current_exe` + data-dir + 현재 사용자 + `autostart=true` 기본으로 조립). **autostart**는 부팅/로그인 자동시작으로 지원(FR-21, core). **account/working_dir**은 기본값(기동/현재 사용자로 실행, 기본 data-dir)으로 충족하고 config 설정화는 **MVP 명시 이연**(구조적 불가 아님) | 부분(account/working_dir 이연) | US-E6-01 불변식("계정/작업디렉터리/자동시작 config 설정 가능")은 MVP에서 autostart만 충족. account/working_dir 설정화 메커니즘은 **U7a가 federated config 키(예: `service`)를 등록**하면 성립 — U0 federated known-key union이 다운스트림 기여 키를 허용하므로(U2 `debounce_ms`/U5 `AUTH_CONSENT_CONFIG_KEYS` 선례), strict-reject는 union 밖 키만 거부할 뿐 구조적 차단이 아니다. 이연 해제 시 NFR/U8에서 `service` 키 등록. FR-21 "로그인 자동시작"(confirm) -> `autostart` 기본 true |
| D-U7A-11 | 세 컴포넌트 오류 taxonomy | `component-methods` 명세 그대로 채택: `ServiceError{UnsupportedPlatform, PermissionDenied, NotInstalled, Io}`, `UpdateError{Download, Verify, RestartFailed, GateTimeout, Io}`, `UninstallError{PartialFailure(UninstallReport), Io}` | 아니오 | `component-methods` §각 컴포넌트 오류 정의. U7a 소유 신규 타입 |
| D-U7A-12 | `restart()` 의미(업데이트 재기동) | AutoUpdater는 자기 로직으로 재기동하지 않고 **`ServiceManager.restart()`에 위임**. restart 자체 실패는 `UpdateError::RestartFailed` -> 즉시 롤백 트리거 | 아니오 | `component-methods` AutoUpdater 흐름 "스테이징 -> ServiceManager.restart() -> 게이트". restart 실패도 게이트 실패와 동일 롤백 경로 |

---

## 4. MANDATORY 카테고리 N/A + 확장 컴플라이언스

### 4.1 MANDATORY 프로세스 항목
- **Welcome Message / Rule Loading**: 워크플로 시작 시 1회 처리 완료(재로딩 안 함).
- **Question Format**: AUTOPILOT 모드 — 사용자 질문 없음(게이트 waived, 권장+MVP 자가확정). N/A.
- **Content Validation**: 적용 — 산출물 박스드로잉 0건, ASCII 화살표(`->`)만, 한국어 산문, 백틱 Rust 타입.

### 4.2 확장 컴플라이언스

| 확장 | 활성 | 이 단계 적용 | 계획/판정 |
|---|---|---|---|
| **Resiliency Baseline** | ON | 적용 | RESILIENCY-04(배포자동화/롤백): FR-20/NFR-16 자동 롤백이 **이 단위의 1차 관심사** — AutoUpdater 헬스게이트+자동롤백+루프방지(D-U7A-04/06)가 직접 구현. RESILIENCY-15(사건 표면화): 롤백 시 `report_update_rollback`(FR-19 케이스4). Uninstaller는 데몬 정지 상태·부분실패에도 idempotent 동작(회복력). RPO/RTO 세부·HA/DR은 Infra/Ops 이월 -> 부분 N/A |
| **Property-Based Testing** | ON (Full) | **강제(PBT-01)** | 각 산출물에 "Testable Properties"(`PROP-U7A-*`) + 제너레이터(주입 clock / fake `ServiceController` / temp-dir 아티팩트 / 임의 probe 준비 시퀀스) 명시. 미준수 시 blocking. 프레임워크(proptest)는 NFR Requirements 이월(PBT-09) |
| **Security Baseline** | OFF | N/A | 미로딩·미강제. config 평문 토큰(RISK-01)은 문서화된 수용 위험이며 Uninstaller 정리 대상(D-U7A-07) |

---

## 5. Drop-list (이미 확정 — 재개봉 금지)

| 항목 | 확정 내용 | 출처 |
|---|---|---|
| U0 CoreTypes/codec/traits/error-taxonomy | 값 타입·CBOR 코덱·싱크 트레이트·오류 분류 동결 | U0 Functional Design |
| Q2=B | 트리거당 1회 단일 직렬 사이클(U7a 무관, 참고) | requirements/units |
| Q6=C | have/want = `raw_sha256` 기준(U3 소관, 참고) | requirements |
| Q8=A | 전송 직전 재검증(U3 소관, 참고) | requirements §0 노트1 |
| 토큰 저장 경로 | 1차=config 평문 `token`(+env 폴백), secure-store=선택적 opt-in | requirements §13 / story-map §확정 |
| TLS-only / latest-state-replacement | NFR-06 / FQ-2=A(U7a 무관, 참고) | requirements |
| W2 크레이트 API 동결 | U0/U5/U6 공개 API 고정 — 편집 금지, 소비만 | WAVE CONTEXT |
| FR-21 autostart | "로그인/부팅 자동시작"은 confirm된 가정 -> `autostart` 기본 true | requirements FR-21 / stories US-E6-01 |
| 서버 프로토콜 | [blocked-on-server] — U7a는 서버 무관(로컬 배포 생명주기) | requirements DEP |

---

## 6. 다음 단계
U7a Functional Design 산출물 3종 생성 완료 -> **U7a NFR Requirements**(per-unit 루프 다음 스테이지: proptest 프레임워크·config 배선·bounded-time 기본값·per-OS 기술 선택). 이후 U7a NFR Design -> Infrastructure Design(플랫폼 유닛 파일 템플릿) -> Code Generation.

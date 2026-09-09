# U7b Ops-Control — Functional Design 계획 (AUTOPILOT)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U7b Operations Control** -> Functional Design (계획 + 결정)
**작성일**: 2026-09-08
**크레이트**: `ops-control` (lib) · **소속 컴포넌트**: `OperatorCli`, `ControlPlane`, `RunStateController`
**대표 에픽**: E6 수명주기 & 업데이트 — 운영 제어 슬라이스(US-E6-03); 또한 E5 CLI 표면(US-E5-02/03) 서비스
**입력 아티팩트**: `unit-of-work.md`(§U7b 책임·얇은-단위 근거), `component-methods.md`(§OperatorCli/ControlPlane/RunStateController 시그니처·이월 항목), `unit-of-work-story-map.md`(U7b = US-E6-03; 서비스 US-E5-02/03), `stories.md`(US-E6-03 / US-E5-02 / US-E5-03 인수기준), `requirements.md`(FR-21/FR-01/FR-06 derived per Q11=D / NFR-15 / RISK-01)
**소비 크레이트(실제 공개 API, 동결)**: `foundation`(U0), `observability`(U6), `auth-consent`(U5), `lifecycle-deploy`(U7a) — 아래 §1.1
**규칙**: `construction/functional-design.md` · `common/content-validation.md` · 활성 확장 `property-based-testing.md`(PBT-01 강제) · `resiliency-baseline.md`
**모드**: AUTOPILOT — 모든 미결정을 권장안 + MVP 최소범위로 자가 확정(§3). 사용자 질문 없음. drop-list(§5) 항목은 재개봉 금지.

> **표기 규약**: 비즈니스 의미 중심 **기술중립 설계**(기술/크레이트 선택은 code-gen 인라인 확정). Rust스러운 시그니처는 **참고용**이며 개념 형상을 표현한다. 다이어그램은 ASCII 화살표(`A -> B`)와 표/목록으로만 기술(박스드로잉/유니코드 화살표 금지). English 식별자명은 원문 유지.

---

## 1. 단위 컨텍스트

U7b는 기존 U7 분할의 **실행 중 데몬 제어** 절반이다 — GUI 없는 데몬의 운영자 제어 표면(사실상의 운영자 API). 배포 생명주기(U7a)와 변경·설계 주기가 다르다. 세 컴포넌트가 하나의 응집된 "런타임 제어" 관심사를 이룬다:

| 컴포넌트 | 소유 스토리 | 책임 요지 |
|---|---|---|
| `OperatorCli` | US-E5-02/03(표면) · US-E6-03(제어) | 서브커맨드(`status`/`health`/`pause`/`resume`/`sync-now`/`stop`/`history`/`consent`/`reload`/`install`/`uninstall`) 파싱·디스패치; `health` 결과 -> 프로세스 종료코드(US-E5-02); 실행 중 데몬 대상 명령은 `ControlPlane` IPC 클라이언트로, 서비스 수명주기(`install`/`uninstall`)는 U7a `ServiceManager`/`Uninstaller`를 **직접 호출** |
| `ControlPlane` | US-E6-03(전송) | 로컬 IPC 서버/클라이언트(Q5=A): Unix 도메인 소켓(mac/Linux) + 명명 파이프(Windows), 소유자 권한 바인딩; 버전드 요청/응답 프로토콜로 status/health -> `StatusService`, history -> `UploadHistoryStore`, pause/resume/sync-now/stop -> `RunStateController`, consent -> U5 `ConsentGate`, reload -> U0 `ConfigProvider` 디스패치 |
| `RunStateController` | US-E6-03 | 권위 있는 run-state 보유자(`running`\|`paused`, `sync_requested`, `stop_requested`); `paused`를 재시작 후에도 지속(상태 파일); U8 오케스트레이션이 이 상태를 **읽어** 동작(역참조 회피, US-E6-03) |

### 1.1 소비하는 실제 API 경계(동결 크레이트, 재정의 금지 — src 확인)

- **U0 `foundation`**: `StatusSnapshot`/`Health`/`ConsentState`/`UploadHistoryRecord`(모두 `Serialize`+`Deserialize` 확인), `encode`/`decode`(CBOR 코덱, `CodecError`), `ReadJudgment::health_check() -> Health`, `ConfigProvider::reload() -> Result<(), ConfigError>`, `Timestamp`/`ByteCount`.
- **U6 `observability`**: `StatusService`(`snapshot() -> StatusSnapshot`(concrete) + `ReadJudgment::health_check()` 구현), `UploadHistoryStore`(`query(HistoryQuery) -> Result<Vec<UploadHistoryRecord>, HistoryError>`), `HistoryQuery{since, only_failures}`. **주입된 concrete `Arc<...>`로 소비**(ops-control `[dependencies]`에 `observability` 포함).
- **U5 `auth-consent`**: `ConsentGate`(`view() -> ConsentStatus`, `grant()`/`withdraw()`/`acknowledge() -> Result<(), ConsentError>`, `consent_state() -> ConsentState`). 와이어에는 U0 `ConsentState` + `acknowledged`만 노출(U5 내부 타입 와이어-직렬화 회피).
- **U7a `lifecycle-deploy`**: `ServiceManager`(`install(&ServiceSpec)`/`uninstall()`/`start()`/`stop()`/`status()`), `ServiceSpec`, `Uninstaller`(`uninstall(&UninstallOptions, ArtifactSet, &Path)`), `UninstallOptions`, `ArtifactSet`. `install`/`uninstall` CLI 경로가 **직접**(in-process, IPC 우회) 호출.

### 1.2 Application Design 이월 미결정 -> §3 AUTOPILOT 확정

`component-methods` 이월 항목: 명령별 인자 스키마·`--json` 출력 스키마(OperatorCli), 프레이밍·peer 자격증명 확인·버전 협상 상세(ControlPlane), 셧다운 유예 정책·신호 전달 메커니즘(RunStateController). 아래 §3에서 전부 자가 확정한다.

---

## 2. Functional Design 산출물 체크리스트 (기술중립, 프론트엔드 파일 없음)

`aidlc-docs/construction/u7b-ops-control/functional-design/` 하위에 생성한다.

- [x] **`domain-entities.md`** — U7b 도입/특화 값 타입: 버전드 프로토콜 메시지(`ControlRequest`/`ControlResponse` + `ControlOp`/`ControlResult`/`ControlError`/`ConsentOp`/`ConsentView`, `PROTO_VERSION`), IPC 경계(`IpcEndpoint`/`IpcError` + `IpcListener`/`IpcConnector` seam), run-state(`RunState`/`RunMode`/`StopMode`/`PersistedRunState`/`RunStateError`), CLI(`Command`/`CliArgs`/`CliResult`/`ExitCode`). U0/U5/U6/U7a 타입은 **이름 참조만**(§0 표). 컴포넌트 -> 규칙 -> 속성 매핑 포함.
- [x] **`business-rules.md`** — `R-U7B-*` 규칙: CLI 디스패치 분기(데몬 대상 IPC vs install/uninstall 직접 호출), health -> 종료코드 매핑, IPC 프레이밍/버전드 프로토콜, run-state 전이 + 지속(paused만 재시작-지속), 소유자-전용 소켓 권한, 오류 처리. `PROP-U7B-*` Testable Properties(PBT-01 강제).
- [x] **`business-logic-model.md`** — 컴포넌트별 알고리즘/워크플로/상태머신: OperatorCli 파싱->라우팅->렌더/종료코드, ControlPlane 서버 accept-loop + 프레이밍 + 디스패치 테이블 + 클라이언트 요청, RunStateController 전이 + crash-atomic 지속(U4 temp+rename 미러). 교차단위 경계(소유 vs 소비 + 실제 API; RunStateController read-by-U8 no-back-reference), 컴포넌트별 Testable-Properties 노트 + 확장 컴플라이언스 요약.
- [x] **PBT-01 "Testable Properties" 섹션**(각 산출물, 확장 강제) — 프로토콜 라운드트립/프레이밍 제너레이터, 임의 명령 시퀀스(run-state), 임의 `Health`, in-memory fake `IpcTransport` seam 명시.
- [x] 산출물 작성 전 `content-validation.md` 검증(특수문자, 표/코드블록 파싱, 박스드로잉 0건, ASCII 화살표만, 한국어 산문).

---

## 3. AUTOPILOT 결정 (권장 + MVP 편향, 인용 포함)

| # | 주제 | 확정(권장) | MVP 트림? | 근거 / 인용 |
|---|---|---|---|---|
| D-U7B-01 | IPC 전송 경계 | `IpcListener`/`IpcConnector` **seam 트레이트** 뒤에 전송을 격리. 1차 구현 = std Unix 도메인 소켓(`std::os::unix::net`, mac/Linux = dev/test 플랫폼). Windows 명명 파이프 구현은 동일 seam 뒤에 **이연/스텁**(un-defer 경로 = `interprocess` 크레이트) | 예 | MVP GUIDANCE: "primary impl = std UDS ...; Windows named-pipe ... MAY be deferred/stubbed". Q5=A(로컬 IPC). std만으로 mac/Linux 완전 테스트 -> 신규 의존 0 |
| D-U7B-02 | 와이어 직렬화 | 프로토콜 메시지를 **U0 코덱**(`encode`/`decode` = CBOR)으로 직렬화. `StatusSnapshot`/`Health`/`ConsentState`/`UploadHistoryRecord`가 모두 `Serialize`+`Deserialize`임을 src로 확인 -> 별도 와이어 타입 재정의 불필요 | 예 | MVP GUIDANCE: "serialized via the U0 codec (or serde_json)". U0 코덱 재사용 = NFR-13 무손실 라운드트립 승계 |
| D-U7B-03 | 프로토콜 메시지 집합 | **요청 enum 1개(`ControlRequest`) + 응답 enum 1개(`ControlResponse`)**, 정확히 필요한 op만. `ControlRequest{proto_version:u16, op:ControlOp}`, `ControlResponse{proto_version:u16, result:ControlResult}` | 예 | MVP GUIDANCE: "one request enum + one response enum". `component-methods` ControlRequest/ControlResponse 형상 채택 |
| D-U7B-04 | 버전 협상 | **엄격 동일성 검사**: 요청 `proto_version != PROTO_VERSION`이면 핸들러 디스패치 전에 `ControlResult::Error(VersionMismatch)` 반환(또는 `IpcError::VersionMismatch`). 다운그레이드/협상 없음 | 예 | MVP: 단일 배포 바이너리이므로 클라이언트=서버 버전 동일. `component-methods` `IpcError::VersionMismatch` |
| D-U7B-05 | 프레이밍 | **길이-프리픽스 프레이밍**(8바이트 big-endian len + CBOR 페이로드), U6 history `LEN_PREFIX=8` 관례 미러. 프레임당 요청/응답 1개. 절단/과대 길이 -> `IpcError::Protocol`(패닉 없음) | 예 | U6 `history.rs` `LEN_PREFIX` 선례. 단순 동기 요청/응답 = MVP |
| D-U7B-06 | 소켓 경로 & 소유자-전용 권한 | `IpcEndpoint::SocketPath`는 **U8이 data-dir 관례로 해소해 주입**. 바인딩 시 소유자-전용: Unix는 소켓을 소유자-전용 디렉터리(0700) 안에 생성 + best-effort `chmod 0600`, stale 소켓은 bind 전 unlink. peer 자격증명 검사(SO_PEERCRED)는 **이연**(소유자-전용 디렉터리가 1차 방어) | 부분(peer-cred 이연) | `component-methods` "소유자 권한". MVP: 로컬 단일 사용자 데몬 -> 디렉터리 권한이 충분. 경로 해소는 U8(하향 주입) |
| D-U7B-07 | run-state 지속 범위 | 상태 파일은 **`paused` 여부만** 지속(`PersistedRunState{paused:bool}`), U4 `SyncStateStore` crash-atomic(temp+rename+fsync) 파이프라인 미러. `sync_requested`/`stop_requested`는 **휘발성 런타임 신호**(재시작 시 기본값 리셋, 재요청 필요) | 예 | US-E6-03 불변식 "일시정지 상태는 재시작 후에도 보존". sync/stop은 실행 중 신호이지 지속 상태 아님 -> 최소 지속. U4 `store.rs` R-STATE-01 미러 |
| D-U7B-08 | U8로의 신호 전달 | RunStateController는 U8을 호출하지 않는다. U8 오케스트레이션이 `current() -> RunState`를 트리거 경계에서 **읽고**, `sync_requested`/`stop_requested`를 소비(consume-and-clear)한다. 즉시 wake(채널/notify)는 **이연**(다음 트리거 틱까지 관측, un-defer = 주입 `Notify` waker seam) | 예 | US-E6-03 "오케스트레이션이 이 상태를 읽어 동작"(역참조 회피). drop-list "no back-reference to U8". `component-methods` `current()` "오케스트레이션이 read" |
| D-U7B-09 | health -> 종료코드 | `Health::Healthy` -> **exit 0**; `Health::Unhealthy{reasons}` -> **non-zero(=1)** + 사유 출력. 데몬 미도달/전송 실패 -> **non-zero(=2)** 구분 | 아니오 | US-E5-02 AC "정상 종료코드 0, 비정상 0 아님 + 사유". `component-methods` "health -> ExitCode 매핑" |
| D-U7B-10 | 명령 라우팅 분기 | 데몬 대상(status/health/pause/resume/sync-now/stop/history/consent/reload) -> `ControlPlane` IPC 클라이언트. 서비스 수명주기(install/uninstall) -> U7a `ServiceManager`/`Uninstaller` **직접 호출**(데몬 미기동 상태에서도 동작, IPC 우회) | 아니오 | task/`unit-of-work.md` §U7b. install/uninstall은 데몬-독립(U7a는 데몬 정지에서도 동작) |
| D-U7B-11 | ControlPlane 디스패치 테이블 | status/health -> `StatusService`(snapshot/health_check), history -> `UploadHistoryStore.query`, pause/resume/sync-now/stop -> `RunStateController`, consent -> U5 `ConsentGate`(view/grant/withdraw/acknowledge), reload -> U0 `ConfigProvider.reload` | 아니오 | `component-methods` ControlPlane `handle` 주석. 모두 실제 공개 API |
| D-U7B-12 | 오류 taxonomy | `component-methods` 채택: `IpcError{Bind, Permission, Connect, Protocol, VersionMismatch, Io}`, `RunStateError{Persist, Io}`. 핸들러 도메인 오류(consent/reload/run-state)는 와이어에서 `ControlResult::Error(ControlError)`로 매핑(전송 오류 `IpcError`와 분리) | 아니오 | `component-methods` §ControlPlane/RunStateController 오류 정의 |
| D-U7B-13 | consent 서브커맨드 | `consent view|grant|withdraw|acknowledge` -> `ConsentGate`의 동명 API. 와이어 응답 `ConsentView{consent:ConsentState, acknowledged:bool}`(U0 타입만) | 아니오 | `ConsentGate` 실제 공개 API(`view`/`grant`/`withdraw`/`acknowledge`/`consent_state`). drop-list "consent" CLI |
| D-U7B-14 | `--json` 출력 | status/health/history에 `--json` 기계판독 출력 제공. U0 코덱 대상 타입(`StatusSnapshot`/`Health`/`UploadHistoryRecord`)이 serde 지원 -> JSON 렌더는 code-gen에서 serde_json 또는 얇은 CLI 뷰로 확정 | 부분(렌더 메커니즘 code-gen) | US-E5-02 체크리스트 "`--json` 기계 판독 출력 제공" |
| D-U7B-15 | install/uninstall 조립 출처 | `install`/`uninstall` CLI 경로가 `ServiceSpec`(current_exe + data-dir + 현재 사용자 + autostart=true)와 `ArtifactSet`/`UninstallOptions`를 **CLI 프로세스 시점에** 조립(U7a 입력 타입 재사용). vault_root 하위 배제는 U7a `Uninstaller`가 이중 방어(D-U7A-09). 플랫폼-백드 인스턴스는 `ServiceManager::new(native_controller()?)` + `Uninstaller::new(ServiceManager, StdFileSystem, UnsupportedTokenPurge)`로 CLI 프로세스가 직접 구성(in-process) | 예 | `ServiceSpec::new`/`UninstallOptions::default`/`native_controller()`/`Uninstaller::new`/`StdFileSystem`/`UnsupportedTokenPurge` 실제 API. U7a "U8/조립자가 입력 조립"(D-U7A-08/10) — CLI 프로세스가 이 경로의 조립자 |
| D-U7B-16 | config 주도 run-state on reload | **이연(post-MVP)**: `reload`는 `ConfigProvider.reload()`만 수행하고, 리로드된 config에 지정된 pause/resume/stop을 `RunStateController`에 적용하지 않는다. MVP run-state 제어 경로 = CLI/IPC(pause/resume/stop 서브커맨드) | 예(명시적 트림) | US-E6-03 AC "pause/resume/stop을 config로 지정하면 데몬이 config를 다시 읽을 때 반영"은 U7b-등록 federated run-state config 키 + reload-apply 배선이 추가로 필요 -> 명시적 MVP 이연(silent drop 아님). CLI/IPC 경로로 MVP 제어 요구는 충족 |

---

## 4. MANDATORY 카테고리 N/A + 확장 컴플라이언스

### 4.1 MANDATORY 프로세스 항목
- **Welcome Message / Rule Loading**: 워크플로 시작 시 1회 처리 완료(재로딩 안 함).
- **Question Format**: AUTOPILOT 모드 — 사용자 질문 없음(게이트 waived, 권장+MVP 자가확정). N/A.
- **Content Validation**: 적용 — 산출물 박스드로잉 0건, ASCII 화살표(`->`)만, 한국어 산문, 백틱 Rust 타입.

### 4.2 확장 컴플라이언스

| 확장 | 활성 | 이 단계 적용 | 계획/판정 |
|---|---|---|---|
| **Resiliency Baseline** | ON | 적용 | RESILIENCY(회복력): run-state crash-atomic 지속(D-U7B-07, U4 미러) — 재시작 후 paused 보존, 손상/절단 상태 파일은 last-good/default로 회복. ControlPlane 프레이밍은 절단/적대적 입력에 패닉 없이 `Protocol` 오류(회복력). CLI health->exit-code(US-E5-02)는 서비스 매니저/모니터링 회복 판정 근거. RPO/RTO 세부·HA/DR은 Infra/Ops 이월 -> 부분 N/A |
| **Property-Based Testing** | ON (Full) | **강제(PBT-01)** | 각 산출물에 "Testable Properties"(`PROP-U7B-*`) + 제너레이터(임의 `ControlRequest`/`ControlResponse` 라운드트립, 임의 바이트 프레임, 임의 명령 시퀀스, 임의 `Health`, in-memory fake `IpcTransport`) 명시. 미준수 시 blocking. 프레임워크(proptest)는 NFR Requirements 이월(PBT-09) |
| **Security Baseline** | OFF | N/A | 미로딩·미강제. 소유자-전용 소켓 권한(D-U7B-06)은 회복력/정합성 관점의 설계이지 Security 확장 강제 아님. config 평문 토큰(RISK-01)은 문서화된 수용 위험 |

---

## 5. Drop-list (이미 확정 — 재개봉 금지)

| 항목 | 확정 내용 | 출처 |
|---|---|---|
| U0 CoreTypes/codec/traits/error-taxonomy/ConfigProvider | 값 타입·CBOR 코덱·싱크 트레이트·오류 분류·config 리로드 동결 | U0 Functional Design |
| Q5=A 로컬 IPC | Unix 도메인 소켓(mac/Linux) + 명명 파이프(Windows) | requirements / units |
| CLI 서브커맨드 표면 | status/health/pause/resume/sync-now/stop/history/consent/reload/install/uninstall | `component-methods` OperatorCli |
| health -> 종료코드 | 0 healthy / non-zero unhealthy(US-E5-02) | stories US-E5-02 |
| run-state 재시작 지속 | paused를 상태 파일에 지속 | stories US-E6-03 |
| 하향 주입 / U8 역참조 금지 | U8이 U5/U6/U0/U7a 핸들을 U7b에 주입; RunStateController는 U8을 호출하지 않고 read-by-U8 | task / US-E6-03 |
| 토큰 저장 / TLS | 1차=config 평문 토큰(+env), TLS-only(U7b 무관, 참고) | requirements §13 / NFR-06 |
| W2/W3 크레이트 API 동결 | U0/U5/U6/U7a 공개 API 고정 — 편집 금지, 소비만 | WAVE CONTEXT |
| 서버 프로토콜 | [blocked-on-server] — U7b는 서버 무관(로컬 운영 제어) | requirements DEP |

---

## 6. 다음 단계
U7b Functional Design 산출물 3종 생성 완료 -> **U7b Code Generation**(NFR Requirements/Design 단계는 사용자 지시로 SKIP — 이 FD가 code-gen 직전 유일 설계 산출물). code-gen에서 인라인 확정: proptest 프레임워크 배선, `--json` 렌더(serde_json vs 뷰), CLI 파서(hand-rolled vs 경량 크레이트), UDS 전송 구현, Windows 명명 파이프 스텁, config/data-dir 경로 배선.

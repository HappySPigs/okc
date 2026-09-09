# U7b Ops-Control — Business Rules (결정 규칙 / 검증 로직 / 제약)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U7b Operations Control** -> Functional Design -> 산출물 2/3 (`business-rules.md`)
**작성일**: 2026-09-08
**크레이트**: `ops-control` (lib) · **소속 컴포넌트**: `OperatorCli`, `ControlPlane`, `RunStateController`
**전제(AUTOPILOT 확정, 계획 §3)**: D-U7B-01..15

> **문서 성격**: 이 문서는 U7b가 소유하는 **결정 규칙·검증 로직·제약·불변식·엣지 케이스**를 정의한다. 타입 정의는 자매 산출물 `domain-entities.md`가 소유하며 이 문서는 그 타입명·필드명을 **그대로 재사용**한다(재정의·모순 없음). 알고리즘/흐름/시퀀스는 `business-logic-model.md`가 소유한다.
>
> **표기 규약**: 비즈니스 의미 중심 **기술중립 설계**. Rust스러운 시그니처는 **참고용**이며 규칙의 개념 형상을 표현한다. 다이어그램은 ASCII 화살표(`A -> B`)와 표/목록으로 기술(박스드로잉 금지). English 식별자명은 원문 유지.

---

## 1. OperatorCli 규칙 (US-E5-02/03 / US-E6-03 / NFR-15)

**규칙 R-U7B-01 (명령 라우팅 이분법)**: `OperatorCli`는 파싱된 `Command`를 **정확히 두 경로 중 하나로** 디스패치한다(D-U7B-10):
- **데몬 대상**(status/health/pause/resume/sync-now/stop/history/consent/reload) -> `Command`를 `ControlOp`로 사상해 `ControlPlane` **IPC 클라이언트**로 전송한다(실행 중 데몬 상태를 조회/변경).
- **서비스 수명주기**(install/uninstall) -> U7a `ServiceManager`/`Uninstaller`를 **직접 호출**한다(IPC 우회). 데몬이 실행 중이 아니어도 동작해야 하므로 IPC에 의존하지 않는다(US-E6-04 정합, R-UN-01).
분기 집합은 배타적이며 전체 `Command`를 덮는다(PROP-U7B-07).

**규칙 R-U7B-02 (install/uninstall 입력 CLI-측 조립)**: `install`/`uninstall`은 데몬 밖 CLI 프로세스에서 실행되므로, 이 프로세스가 U7a 입력을 조립한다(D-U7B-15): `ServiceSpec::new(current_exe, data_dir)`(기본 `account=CurrentUser`, `autostart=true`) 및 `UninstallOptions`(기본값) + `ArtifactSet`(config data-dir 관례로 해소, vault_root 하위 배제). U7b는 U7a 오류(`ServiceError`/`UninstallError`)를 사람이 읽는 출력 + 종료코드로 렌더할 뿐 재해석하지 않는다.

**규칙 R-U7B-03 (`--json` 기계판독 출력)**: `status`/`health`/`history`는 `--json`이 주어지면 기계판독 표현을 출력한다(US-E5-02 체크리스트). 대상 페이로드(`StatusSnapshot`/`Health`/`Vec<UploadHistoryRecord>`)는 모두 serde 지원이므로 렌더 메커니즘(serde_json vs 얇은 뷰)은 code-gen 이월(D-U7B-14). `--json` 부재 시 사람용 텍스트. 트레이/GUI 부재와 무관하게 동작한다(NFR-15, US-E5-02 "트레이 없어도 동작").

**규칙 R-U7B-04 (health -> 종료코드 매핑, US-E5-02 계약)**: `health` 명령의 종료코드는 다음으로 **전수·배타** 매핑된다(D-U7B-09):
- `Health::Healthy` -> `ExitCode(0)`.
- `Health::Unhealthy { reasons }` -> `ExitCode(1)` + 사유(`reasons`) 출력.
- 데몬 미도달/전송 실패(`IpcError::Connect` 등) -> `ExitCode(2)`(데몬 미기동을 unhealthy와 구분).
`status`는 상태를 표면화만 하며 종료코드는 성공 시 0(전송 실패 시 2). 상태 전이는 U7b가 소유하지 않는다(US-E5-02 INVEST: 표면화만).

**엣지 케이스**: 알 수 없는 서브커맨드/인자 -> 사용법 출력 + non-zero(파싱 오류; MVP는 2 재사용 또는 별도 usage 코드, code-gen 확정). `consent`의 하위 동작 누락 -> 사용법.

---

## 2. ControlPlane 규칙 (US-E6-03 / Q5=A)

**규칙 R-U7B-05 (프레이밍 = 길이-프리픽스 + U0 코덱, 패닉 없음)**: 한 요청/응답은 **8바이트 big-endian 길이 프리픽스 + CBOR 페이로드** 1프레임으로 전송된다(D-U7B-05, U6 `history.rs` `LEN_PREFIX=8` 관례 미러). 페이로드는 U0 `encode`/`decode`로 직렬화되며 `decode(encode(m)) == m` 무손실 라운드트립을 만족한다(NFR-13 승계). 절단 프레임·선언 길이 초과·CBOR 디코드 실패는 **패닉하지 않고** `IpcError::Protocol`로 반환한다(적대적/부분 입력 회복력, PROP-U7B-02). 과대 길이는 상한(config/기본 상수)으로 거부한다.

**규칙 R-U7B-06 (버전드 프로토콜 — 엄격 동일성, 디스패치 전 차단)**: 수신 `ControlRequest.proto_version`이 `PROTO_VERSION`과 **다르면**, 어떤 핸들러도 호출하지 않고 `ControlResult::Error(VersionMismatch{server, client})`를 반환한다(D-U7B-04). 부작용 있는 op(pause/resume/sync-now/stop/reload/consent 변경)는 버전 불일치 시 **실행되지 않는다**(PROP-U7B-03). 단일 배포 바이너리라 정상 경로에서 버전은 항상 일치하며, 이 규칙은 버전 스큐(부분 업그레이드) 방어다.

**규칙 R-U7B-07 (소유자-전용 소켓 바인딩)**: 서버는 `IpcEndpoint::SocketPath`를 **소유자-전용 디렉터리(0700) 하위**에 바인딩하고 소켓 파일에 best-effort `chmod 0600`을 적용한다(D-U7B-06, `component-methods` "소유자 권한"). 바인딩 전 stale 소켓 경로는 unlink 후 재바인딩한다(idempotent 기동). 권한 설정 실패는 `IpcError::Permission`, 경로 점유/바인딩 실패는 `IpcError::Bind`. peer 자격증명 검사(SO_PEERCRED)는 MVP 이연이며 디렉터리 권한이 1차 접근 통제다.

**규칙 R-U7B-08 (디스패치 테이블 = 실제 공개 API로만)**: `handle(req)`는 op별로 아래 **실제 공개 API**에만 위임한다(D-U7B-11). 어떤 프로토콜 핸들러도 동결 크레이트 API를 재정의하거나 U8을 역참조하지 않는다:

| op | 대상 (주입 핸들) | 실제 API | 응답 |
|---|---|---|---|
| Status | `Arc<StatusService>` | `snapshot()` | `Status(StatusSnapshot)` |
| Health | `Arc<StatusService>` (as `ReadJudgment`) | `health_check()` | `Health(Health)` |
| History(q) | `Arc<UploadHistoryStore>` | `query(q)` | `History(Vec<..>)` / `Error(HistoryUnavailable)` |
| Pause/Resume | `Arc<RunStateController>` | `pause()`/`resume()` | `Ack` / `Error(RunState)` |
| SyncNow | `Arc<RunStateController>` | `request_sync_now()` | `Ack` |
| Stop(mode) | `Arc<RunStateController>` | `request_stop(mode)` | `Ack` |
| Consent(View) | `Arc<ConsentGate>` | `view()` + `consent_state()` -> `ConsentView` | `Consent(..)` |
| Consent(Grant/Withdraw/Acknowledge) | `Arc<ConsentGate>` | `grant()`/`withdraw()`/`acknowledge()` | `Ack` / `Error(ConsentRejected)` |
| Reload | `Arc<ConfigProvider>` | `reload()` | `Ack` / `Error(ReloadFailed)` |

**규칙 R-U7B-09 (도메인 오류 vs 전송 오류 분리)**: 핸들러가 반환한 도메인 오류(`ConsentError`/`ConfigError`/`RunStateError`/`HistoryError`)는 **성공적으로 전송되는 응답 프레임 안**의 `ControlResult::Error(ControlError)`로 매핑된다(전송은 성공). 소켓/프레이밍/버전 실패만 `IpcError`로 표면화된다. 예: `ConsentGate.grant()`가 `Err(AlreadyGranted)` -> `ControlResult::Error(ConsentRejected("이미 부여됨"))`. `ConfigProvider.reload()` 검증 실패(keep-last-good) -> `Error(ReloadFailed(..))`.

**엣지 케이스**: 클라이언트 `request` 시 소켓 부재/데몬 미기동 -> `IpcError::Connect`(OperatorCli가 exit 2 매핑). 서버 accept 루프의 개별 연결 처리 실패는 로그 후 다음 연결로 진행한다(1개 나쁜 연결이 서버를 죽이지 않음, 회복력).

---

## 3. RunStateController 규칙 (US-E6-03)

**규칙 R-U7B-10 (run-state 전이 + 멱등)**: `mode` 전이는 `pause() -> Paused`, `resume() -> Running`이며 **멱등**하다(이미 `Paused`에서 `pause()`는 no-op 성공, 지속 재기록 불필요). `request_sync_now()`는 `sync_requested = true`(이미 true면 no-op), `request_stop(mode)`는 `stop_requested = Some(mode)`로 세팅한다. `sync_requested`/`stop_requested`는 **U8이 관측 시 consume-and-clear**하며 RunStateController는 U8을 호출하지 않는다(no back-reference, US-E6-03). 전이는 임계구역 안에서 직렬화한다(U5 `ConsentGate` 선례).

**규칙 R-U7B-11 (paused만 재시작-지속, crash-atomic)**: `mode` 전이(`pause`/`resume`) 시 `PersistedRunState{paused}`를 **원자적으로** 지속한다 — U4 `SyncStateStore` 파이프라인(`encode(CBOR) -> 같은 디렉터리 temp -> fsync -> atomic rename -> 부모 dir fsync`, R-STATE-01) 미러(D-U7B-07). `sync_requested`/`stop_requested`는 **지속하지 않는다**(휘발성 신호). 기동 시 상태 파일을 읽어 `paused`를 복원하고, 손상/절단/부재/잔존-temp는 기본값(`paused=false` = Running)으로 안전 회복한다(패닉 없음). 지속 쓰기 실패는 `RunStateError::Persist`이되 **인메모리 전이는 이미 반영**된다(다음 지속에서 재동기화 가능; 인메모리가 권위, 지속은 재시작 대비).

**규칙 R-U7B-12 (pause -> StatusService 운영상태 반영)**: `pause()` 성공 시 `RunStateController`는 주입된 `Arc<dyn StatusSink>`(U0 `foundation` 소유 트레이트, U6 `StatusService` 구현)에 `set_operational(OperationalState::Paused)`를 호출해 운영 라이프사이클(축1)을 반영한다. 이는 인메모리 전이(R-U7B-10)·crash-atomic 지속(R-U7B-11)에 더한 부수효과이며, `ControlPlane`의 `status` op가 읽는 `StatusService.snapshot()`에 `Paused`가 나타나게 한다(US-E6-03 AC "상태가 paused로 표시된다", `component-methods` "pause()는 StatusService.set_operational(Paused)도 호출"). 이미 `Paused`에서의 멱등 no-op pause는 재반영이 불필요하다(`set_operational` 자체가 멱등이라 재호출해도 무해). `StatusSink` 주입은 U6로의 **동위-하향** 방향이며(sink 트레이트는 U0 소유, 구현체만 U6) U8 역참조가 아니다(US-E6-03 no-back-reference 유지).

**규칙 R-U7B-13 (resume -> StatusService 운영상태 반영)**: `resume()` 성공 시 `RunStateController`는 같은 주입 핸들에 `set_operational(OperationalState::Idle)`(비-paused 정지 상태)을 호출해 운영 라이프사이클을 pause 이전으로 되돌린다. 재개 직후 실제 상태(`Idle`/`Syncing`/`Offline`)는 이후 U6/U8 push로 갱신되며, 이 규칙은 paused 표시 해제만 보장한다(US-E6-03 "감시가 재개된다"). `OperationalState`에 `Running` 변형이 없으므로 재개 정지 상태는 `Idle`로 매핑한다(`StatusService` 초기값과 정합). `set_operational`은 infallible이므로 이 반영은 오류를 반환하지 않는다(`RunStateError`와 무관).

**엣지 케이스**: `request_stop(Immediate)`와 `request_stop(Graceful)`이 연달아 오면 마지막 값이 유지된다(U8가 소비 전이면). 셧다운 유예 정책·실제 종료 실행은 U8 `WatcherDaemon` 소관(U7b는 신호만 보유; `component-methods` "셧다운 유예/신호 전달: FD 이월" -> 신호 보유+read-by-U8로 확정).

---

## 4. Testable Properties (PBT-01 강제, 확장 ON Full)

각 속성은 MVP seam(in-memory fake `IpcTransport`(버퍼 쌍), temp-dir 상태 파일, 주입 fake 핸들)으로 소켓/데몬 없이 결정적으로 검증 가능하다. 프레임워크(proptest)는 code-gen 이월(PBT-09), shrinking/고정시드/CI는 PBT-08.

| ID | 컴포넌트 | 속성(불변식) | 제너레이터 | 근거 |
|---|---|---|---|---|
| **PROP-U7B-01** | ControlPlane | 임의의 `ControlRequest`/`ControlResponse` 값에 대해 `decode(encode(m)) == m`(무손실 라운드트립). 모든 op/result 변형 포함 | 임의 `ControlOp`(Stop/History/Consent 페이로드 포함) + 임의 `ControlResult`(임의 `StatusSnapshot`/`Health`/레코드 벡터) | R-U7B-05, NFR-13 |
| **PROP-U7B-02** | ControlPlane | 임의 바이트 버퍼에 대해: 정상 프레임(len 프리픽스+CBOR)은 정확히 원 페이로드로 복원되고, 절단/과대-길이/손상 CBOR은 `Err(IpcError::Protocol)`이며 **결코 패닉하지 않는다** | 임의 페이로드 + 임의 절단 오프셋 + 임의 손상 바이트 | R-U7B-05(회복력) |
| **PROP-U7B-03** | ControlPlane | 임의 `proto_version`에 대해: `!= PROTO_VERSION`이면 결과는 `Error(VersionMismatch)`이고 어떤 주입 핸들의 부작용 메서드(pause/resume/reload/grant 등)도 호출되지 않는다(기록 fake로 0회 확인). `== PROTO_VERSION`이면 정확히 대응 핸들러 1개가 호출된다 | 임의 u16 버전 + 임의 op + 호출-기록 fake 핸들 | R-U7B-06/08 |
| **PROP-U7B-04** | RunStateController | 임의의 pause/resume/sync-now/stop 명령 시퀀스에 대해: 최종 `current().mode`는 마지막 pause/resume에 의해 결정(pause->Paused, resume->Running; 무-전이 시 초기값); 전이는 멱등(연속 동일 명령 == 1회) | 임의 명령 시퀀스 | R-U7B-10, US-E6-03 |
| **PROP-U7B-05** | RunStateController | 임의 전이 시퀀스 후 지속+재로드하면 `paused`가 보존되고 `sync_requested`/`stop_requested`는 기본값으로 리셋된다; 임의 지점에서 상태 파일을 절단/손상시켜 로드해도 `paused=false`로 안전 회복(패닉 없음) | 임의 전이 시퀀스 + temp-dir + 임의 절단/손상 주입 | R-U7B-11, US-E6-03, U4 R-STATE-01 |
| **PROP-U7B-06** | OperatorCli | 임의 `Health` 값에 대해 `Healthy -> ExitCode(0)`, `Unhealthy -> ExitCode(1)`(배타·전수); 전송 실패(`IpcError::Connect`) -> `ExitCode(2)`. 세 결과는 상호 배타 | 임의 `Health`(Healthy / 임의 reasons Unhealthy) + 전송 성공/실패 플래그 | R-U7B-04, US-E5-02 |
| **PROP-U7B-07** | OperatorCli | 임의 `Command`에 대해: install/uninstall은 U7a 경로(ServiceManager/Uninstaller)로만 라우팅되고 IPC 클라이언트를 호출하지 않으며, 그 외 모든 Command는 IPC 경로로만 라우팅된다(두 경로 배타·전수) | 임의 `Command` + 호출-기록 fake(IPC 클라이언트 / U7a 핸들) | R-U7B-01, D-U7B-10 |

> **속성 없는 요소**: 실 UDS 소켓 바인딩/권한(`std::os::unix::net` 뒤), Windows 명명 파이프 스텁, 실 `ServiceManager` OS 어댑터는 이 단위의 PBT 대상이 아니다 — "No PBT properties identified"(통합/Infra 테스트 이월). 프로토콜/전이/라우팅 순수 로직만 seam으로 격리해 PBT한다.

---

## 5. 확장 컴플라이언스 요약

| 확장 | 활성 | 판정 |
|---|---|---|
| **Resiliency Baseline** | ON | 회복력이 이 단위 곳곳: R-U7B-05(프레이밍 절단/적대적 입력에 패닉 없이 `Protocol` 오류), R-U7B-11(run-state crash-atomic 지속 + 손상 상태 파일 안전 회복, U4 미러), R-U7B-06(버전 스큐 방어), R-U7B-09 엣지(1개 나쁜 연결이 서버 accept 루프를 죽이지 않음), R-U7B-04(health->exit-code로 서비스매니저/모니터링 회복 판정, US-E5-02). RPO/RTO 세부·HA/DR은 Infra/Ops 이월 -> 부분 N/A |
| **Property-Based Testing** | ON (Full) | §4 PROP-U7B-01..07 + 제너레이터(임의 메시지 라운드트립 / 임의 프레임 바이트 / 임의 명령 시퀀스 / 임의 `Health` / in-memory fake seam) 명시. 프레임워크(proptest)는 code-gen 이월(PBT-09) |
| **Security Baseline** | OFF | N/A. 소유자-전용 소켓 권한(R-U7B-07)은 정합성/회복력 설계이지 Security 확장 강제가 아님. config 평문 토큰(RISK-01)은 문서화된 수용 위험 |

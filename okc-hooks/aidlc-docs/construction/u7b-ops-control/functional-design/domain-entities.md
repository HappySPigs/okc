# U7b Ops-Control — Domain Entities (도메인 엔티티 / 값 타입)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U7b Operations Control** -> Functional Design -> 산출물 1/3 (`domain-entities.md`)
**작성일**: 2026-09-08
**크레이트**: `ops-control` (lib) · **소속 컴포넌트**: `OperatorCli`, `ControlPlane`, `RunStateController`
**전제(AUTOPILOT 확정, 계획 §3)**: D-U7B-01(IPC seam) · D-U7B-02(U0 코덱 직렬화) · D-U7B-03(단일 요청/응답 enum) · D-U7B-04(엄격 버전 검사) · D-U7B-05(길이-프리픽스 프레이밍) · D-U7B-06(소유자-전용 소켓) · D-U7B-07(paused만 지속) · D-U7B-12(오류 taxonomy) · D-U7B-13(consent op)

> **문서 성격**: 이 문서는 U7b가 **도입/특화하는 값 타입·엔티티**를 정의한다. U0(`foundation`)/U5(`auth-consent`)/U6(`observability`)/U7a(`lifecycle-deploy`)가 소유하는 타입은 **이름으로 참조만** 하며 **재정의하지 않는다**(§0 표). 규칙(검증·불변식)은 자매 산출물 `business-rules.md`, 알고리즘/흐름은 `business-logic-model.md`가 소유한다.
>
> **표기 규약**: 비즈니스 의미 중심 **기술중립 설계**. Rust스러운 시그니처는 **참고용(reference-only)** 이며 스레딩/async/I/O 메커니즘이 아니라 개념 형상을 표현한다. 다이어그램은 ASCII 화살표(`A -> B`)와 표/목록으로만 기술(박스드로잉 금지). English 식별자명은 원문 유지.

---

## 0. 소비하는 타입 (재정의 금지 — 참조만)

`ops-control`은 아래 동결 크레이트 타입을 이름으로만 참조한다. (`ops-control` `[dependencies]` = `foundation`, `observability`, `auth-consent`, `lifecycle-deploy`.)

| 원 소유 | 타입 / API | U7b에서의 사용 |
|---|---|---|
| U0 `foundation` | `encode<T: Serialize>() -> Result<Vec<u8>, CodecError>` · `decode<T: DeserializeOwned>() -> Result<T, CodecError>` | 프로토콜 메시지 CBOR 직렬화(D-U7B-02). 무손실 라운드트립(NFR-13) 승계 |
| U0 `foundation` | `StatusSnapshot`(serde) | `status` 응답 페이로드(concrete `StatusService.snapshot()` 결과) |
| U0 `foundation` | `Health { Healthy \| Unhealthy { reasons: Vec<HealthReason> } }`(serde) | `health` 응답 페이로드 + 종료코드 매핑 근거(US-E5-02) |
| U0 `foundation` | `ConsentState { Granted \| Blocked \| Unknown }`(serde) | `ConsentView` 와이어 필드(U5 내부 타입 회피) |
| U0 `foundation` | `UploadHistoryRecord`(serde) | `history` 응답 페이로드(레코드 벡터) |
| U0 `foundation` | `ReadJudgment::health_check() -> Health` | ControlPlane이 `health`에서 소비(`StatusService` 구현) |
| U0 `foundation` | `ConfigProvider::reload() -> Result<(), ConfigError>` | `reload` 디스패치 대상(concrete `Arc<ConfigProvider>` 주입) |
| U0 `foundation` | `StatusSink::set_operational(state: OperationalState)`(트레이트, U6 `StatusService` 구현) · `OperationalState { Idle \| Syncing \| Offline \| Paused }` | `RunStateController`가 pause/resume 성공 시 `Arc<dyn StatusSink>`로 run-state를 `StatusService`에 반영(R-U7B-12/13) -> `watcher status`가 paused 표면화. U6 `StatusService` 핸들 하향 주입 |
| U6 `observability` | `StatusService`(`snapshot() -> StatusSnapshot`, `ReadJudgment` 구현) | 조립 루트가 `Arc<StatusService>`로 주입. status/health 디스패치 |
| U6 `observability` | `UploadHistoryStore`(`query(HistoryQuery) -> Result<Vec<UploadHistoryRecord>, HistoryError>`) · `HistoryQuery{since, only_failures}` | 조립 루트가 `Arc<UploadHistoryStore>` 주입. history 디스패치 |
| U5 `auth-consent` | `ConsentGate`(`view() -> ConsentStatus`, `grant`/`withdraw`/`acknowledge() -> Result<(), ConsentError>`, `consent_state() -> ConsentState`) · `ConsentError` | 조립 루트가 `Arc<ConsentGate>` 주입. consent 디스패치 |
| U7a `lifecycle-deploy` | `ServiceManager`(+`::new(Box<dyn ServiceController>)`)/`install(&ServiceSpec)`/`uninstall()`/`start`/`stop`/`status` · `native_controller() -> Result<Box<dyn ServiceController>, ServiceError>`(플랫폼 팩토리) · `ServiceSpec`(+`::new`) | `install`/`uninstall` CLI 경로가 `ServiceManager::new(native_controller()?)`로 구성해 **직접** 호출(D-U7B-10/15) |
| U7a `lifecycle-deploy` | `Uninstaller`(+`::new(ServiceManager, Box<dyn FileSystem>, Box<dyn TokenPurgePort>)`)/`uninstall(&UninstallOptions, ArtifactSet, &Path)` · `UninstallOptions`(+`::default`) · `ArtifactSet` · `StdFileSystem`(실 FS) · `UnsupportedTokenPurge`(MVP U5-backed `TokenPurgePort`) | `uninstall` CLI 경로가 `Uninstaller::new(ServiceManager, StdFileSystem, UnsupportedTokenPurge)`로 조립해 직접 호출 |

> **핵심 제약(재확인)**: ControlPlane은 U5/U6 **구체 타입을 `Arc`로 주입**받아 소비하되(하향 주입, U8이 조립), 어떤 U8 타입도 참조하지 않는다. `RunStateController`는 U8을 **호출하지 않으며** U8이 `current()`를 읽는다(no back-reference, US-E6-03). 또한 `RunStateController`는 pause/resume 결과를 `Arc<dyn StatusSink>`(U6 `StatusService` 구현)로 반영하는데, 이는 U6로의 **동위-하향 주입**이지 U8 역참조가 아니다(`StatusSink` 트레이트는 U0 소유). `ConsentGate.view()`가 반환하는 `ConsentStatus`는 U5 내부 타입(`ConsentLifecycle`/`ConsentGrant`)을 포함하므로, 와이어에는 U0 `ConsentState`(+`acknowledged`)만 실어 직렬화 결합을 피한다.

---

## 1. ControlPlane — 프로토콜 메시지 타입 (U7b 소유)

### 1.1 `PROTO_VERSION` (프로토콜 버전 상수)

```
const PROTO_VERSION: u16 = 1;   // 단일 배포 바이너리 -> 클라이언트=서버 동일 버전(D-U7B-04)
```

### 1.2 `ControlRequest` / `ControlOp` (요청 — 단일 enum, D-U7B-03)

```
ControlRequest {
  proto_version: u16,     // 발신 시 PROTO_VERSION. 수신 시 엄격 동일성 검사(R-U7B-06)
  op:            ControlOp,
}

ControlOp {
  Status,                       // -> StatusService.snapshot()
  Health,                       // -> ReadJudgment.health_check()
  Pause,                        // -> RunStateController.pause()
  Resume,                       // -> RunStateController.resume()
  SyncNow,                      // -> RunStateController.request_sync_now()
  Stop(StopMode),               // -> RunStateController.request_stop(mode)
  History(HistoryQuery),        // -> UploadHistoryStore.query(filter)   [HistoryQuery = U6 타입]
  Consent(ConsentOp),           // -> ConsentGate (view/grant/withdraw/acknowledge)
  Reload,                       // -> ConfigProvider.reload()
}

ConsentOp { View, Grant, Withdraw, Acknowledge }   // D-U7B-13
```

- **참고**: `install`/`uninstall`은 `ControlOp`에 **없다** — 데몬 IPC를 우회해 U7a를 직접 호출하므로 프로토콜 표면 밖이다(D-U7B-10). `History`의 필터는 U6 `HistoryQuery{since: Option<Timestamp>, only_failures: Option<bool>}`를 그대로 실어 나른다(재정의 금지).

### 1.3 `ControlResponse` / `ControlResult` / `ControlError` (응답 — 단일 enum)

```
ControlResponse {
  proto_version: u16,     // 서버 PROTO_VERSION
  result:        ControlResult,
}

ControlResult {
  Status(StatusSnapshot),                 // U0 타입(serde)
  Health(Health),                         // U0 타입(serde)
  History(Vec<UploadHistoryRecord>),      // U0 타입(serde)
  Consent(ConsentView),                   // U7b 와이어 타입(§1.4)
  Ack,                                    // pause/resume/sync-now/stop/reload 성공(부수효과 완료)
  Error(ControlError),                    // 핸들러 도메인 오류(전송 오류 IpcError와 분리)
}

ControlError {
  VersionMismatch { server: u16, client: u16 },  // 디스패치 전 거부(R-U7B-06)
  ConsentRejected(String),                        // ConsentError 매핑(NotAcknowledged/AlreadyGranted/NoGrant/PersistFailed)
  ReloadFailed(String),                           // ConfigError 매핑(검증 실패 등 keep-last-good)
  RunState(String),                               // RunStateError 매핑(Persist/Io)
  HistoryUnavailable(String),                     // HistoryError 매핑
}
```

- **불변식(라운드트립)**: `decode(encode(req)) == req`, `decode(encode(resp)) == resp`(R-U7B-05, PROP-U7B-01). 모든 페이로드 타입이 serde 지원임을 U0 src로 확인.
- **참고**: 핸들러 도메인 오류는 `ControlResult::Error(ControlError)`로 **정상 응답 프레임 안에서** 전달된다(전송은 성공). 전송/프레이밍 실패는 별개 `IpcError`(§2.2)로 표면화된다.

### 1.4 `ConsentView` (consent 와이어 응답, U7b 소유)

```
ConsentView {
  consent:      ConsentState,   // U0 타입 = ConsentGate.consent_state()
  acknowledged: bool,           // ConsentGate.view().acknowledged
}
```

- **근거**: `ConsentGate.view() -> ConsentStatus`는 U5 내부 `ConsentLifecycle`/`ConsentGrant`를 포함한다. 와이어 결합을 피하려 U0 `ConsentState`(serde) + `acknowledged` 두 필드로 투영한다(D-U7B-13).

---

## 2. ControlPlane — IPC 경계 타입 (U7b 소유)

### 2.1 `IpcEndpoint` (엔드포인트 주소 — 주입 입력)

```
IpcEndpoint {
  SocketPath(AbsolutePath),   // Unix 도메인 소켓 경로(mac/Linux, 1차 구현)
  NamedPipe(String),          // Windows 명명 파이프(이연/스텁, D-U7B-01)
}
```

- **불변식**: 경로는 조립 루트(U8)가 data-dir 관례로 해소해 주입한다(D-U7B-06). U7b는 재해소하지 않는다. 소켓은 소유자-전용 디렉터리(0700) 하위에 생성된다(R-U7B-07).

### 2.2 `IpcError` (전송 오류 taxonomy, U7b 소유 — D-U7B-12)

```
IpcError {
  Bind,             // 서버 바인딩 실패(경로 점유/권한 등)
  Permission,       // 소유자-전용 권한 설정/검증 실패
  Connect,          // 클라이언트 연결 실패(데몬 미기동 -> CLI exit 2)
  Protocol,         // 프레이밍 위반: 절단/과대 길이/디코드 실패(패닉 없음, R-U7B-05)
  VersionMismatch,  // 버전 불일치(전송 계층 관측 시)
  Io(detail: String),
}
```

- **참고**: `Connect`는 데몬 미기동/소켓 부재를 포함하며 OperatorCli가 종료코드 2(데몬 미도달)로 매핑한다(R-U7B-04). `Protocol`은 어떤 적대적/절단 바이트에도 패닉하지 않고 반환된다(회복력, PROP-U7B-02).

### 2.3 전송 seam 트레이트 (D-U7B-01)

```
IpcListener {                                   // 서버 측(데몬)
  accept() -> Result<IpcStream, IpcError>       // 다음 연결 수락(블로킹)
}
IpcConnector {                                  // 클라이언트 측(CLI)
  connect(endpoint: &IpcEndpoint) -> Result<IpcStream, IpcError>
}
IpcStream {                                     // 양방향 프레임 채널(read/write 프레임)
  read_frame()  -> Result<Vec<u8>, IpcError>    // 길이-프리픽스 1프레임 수신
  write_frame(payload: &[u8]) -> Result<(), IpcError>
}
```

- **MVP 구현**: `UdsListener`/`UdsConnector`/`UdsStream`(`std::os::unix::net`) = mac/Linux 1차. Windows 명명 파이프 구현은 동일 seam 뒤 스텁(un-defer = `interprocess`). 테스트는 in-memory fake(`Vec<u8>` 버퍼 쌍)를 주입 -> 소켓 없이 프로토콜 로직 결정적 검증(PROP-U7B-01/02/03).

---

## 3. RunStateController — run-state 타입 (U7b 소유)

### 3.1 `RunState` (권위 있는 실행 상태 — 인메모리 보유)

```
RunState {
  mode:           RunMode,           // running | paused (지속 대상, D-U7B-07)
  sync_requested: bool,              // sync-now 신호(휘발성, U8가 consume-and-clear)
  stop_requested: Option<StopMode>,  // stop 신호(휘발성)
}

RunMode { Running, Paused }
StopMode { Graceful, Immediate }
```

- **불변식**: `mode`만 재시작 후에도 보존된다(US-E6-03). `sync_requested`/`stop_requested`는 실행 중 신호이며 재시작 시 기본값(`false`/`None`)으로 리셋된다(D-U7B-07). U8은 `current()`로 읽고 소비한다(read-by-U8, no back-reference).

### 3.2 `PersistedRunState` (재시작-지속 최소 부분집합)

```
PersistedRunState {
  paused: bool,     // RunState.mode == Paused 여부만 지속(D-U7B-07)
}
```

- **불변식(crash-atomic)**: 상태 파일 쓰기는 U4 `SyncStateStore` 파이프라인(`encode -> 같은 디렉터리 temp -> fsync -> atomic rename`)을 미러한다(R-U7B-08). 로드 시 손상/절단/잔존-temp 입력은 기본값(`paused=false`)으로 안전 회복(패닉 없음). 직렬화는 U0 코덱(CBOR).

### 3.3 `RunStateError` (오류 taxonomy, U7b 소유 — D-U7B-12)

```
RunStateError {
  Persist,          // 상태 파일 원자 쓰기 실패(인메모리 전이는 반영, 지속만 실패)
  Io(detail: String),
}
```

### 3.4 `RunStateController` 주입 의존 (run-state -> StatusService 반영)

```
RunStateController {
  // 권위 있는 RunState 보유 + crash-atomic 지속 경로(§3.1/3.2)
  status_sink: Arc<dyn StatusSink>,   // U0 트레이트, U6 StatusService 구현 (하향 주입)
}
```

- **근거**: `pause()`/`resume()` 성공 시 `RunStateController`는 인메모리 전이·지속에 더해 주입된 `status_sink.set_operational(_)`을 호출해 `StatusService`에 mode를 반영한다 — `pause -> OperationalState::Paused`, `resume -> OperationalState::Idle`(R-U7B-12/13, `component-methods` "pause()는 StatusService.set_operational(Paused)도 호출"). 이로써 `watcher status`(ControlPlane이 `StatusService.snapshot()`을 읽음)가 paused를 표면화한다(US-E6-03 AC "상태가 paused로 표시된다"). `set_operational`은 infallible이라 이 반영은 `RunStateError`를 늘리지 않는다.
- **비순환(재확인)**: `StatusSink`는 U0 `foundation` 소유 트레이트이고 구현체는 U6 `StatusService`다. 주입은 U6로의 **동위-하향** 방향이며(조립 루트 U8이 `Arc<dyn StatusSink>`를 주입) U8 타입을 참조하지 않는다 -> no back-reference 유지.

---

## 4. OperatorCli — CLI 타입 (U7b 소유)

### 4.1 `Command` (파싱된 서브커맨드)

```
Command {
  Status, Health, Pause, Resume, SyncNow, Stop(StopMode),
  History(HistoryQuery),        // --since / --status 파싱 -> U6 HistoryQuery
  Consent(ConsentOp),           // view|grant|withdraw|acknowledge
  Reload,
  Install,                      // -> U7a ServiceManager.install (직접, IPC 우회)
  Uninstall(UninstallOptions),  // -> U7a Uninstaller.uninstall (직접)
}
```

- **참고**: `Status..Reload`는 `ControlOp`로 사상되어 IPC 클라이언트로 전송된다(D-U7B-10). `Install`/`Uninstall`은 U7a를 직접 호출하므로 `ControlOp`에 대응이 없다.

### 4.2 `CliArgs` / `CliResult` / `ExitCode`

```
CliArgs {
  command: Command,
  json:    bool,          // --json 기계판독 출력(status/health/history, D-U7B-14)
}

CliResult {
  output:    String,      // 사람용 또는 --json 렌더 결과
  exit_code: ExitCode,
}

ExitCode(u8)              // 0 = healthy/성공, 1 = unhealthy, 2 = 데몬 미도달/전송 오류(R-U7B-04)
```

- **불변식(health 매핑)**: `Health::Healthy -> ExitCode(0)`, `Health::Unhealthy -> ExitCode(1)`(+ 사유 출력), `IpcError::Connect -> ExitCode(2)`. 배타·전수(PROP-U7B-06). US-E5-02 종료코드 계약.

---

## 5. 엔티티 관계 (텍스트 표기)

```
CliArgs -> OperatorCli.run()
  [데몬 대상] Command(Status..Reload) -> ControlOp
       -> ControlPlane(client).request(ControlRequest{PROTO_VERSION, op})
       -> IpcConnector.connect(endpoint) -> IpcStream.write_frame(encode(req))
       -> IpcStream.read_frame() -> decode -> ControlResponse{result}
       -> 렌더 + ExitCode
  [직접]     Command(Install)   -> ServiceManager::new(native_controller()).install(ServiceSpec::new(current_exe, data_dir))   [U7a]
             Command(Uninstall) -> Uninstaller::new(ServiceManager::new(native_controller()), StdFileSystem, UnsupportedTokenPurge)
                                     .uninstall(UninstallOptions, ArtifactSet, vault_root)                                       [U7a]

ControlPlane(server).serve(endpoint):
  IpcListener.accept() -> IpcStream.read_frame() -> decode(ControlRequest)
    if proto_version != PROTO_VERSION: ControlResult::Error(VersionMismatch)   [디스패치 전, R-U7B-06]
    else dispatch(op):
        Status/Health          -> StatusService.snapshot() / .health_check()          [U6 주입]
        History(q)             -> UploadHistoryStore.query(q)                          [U6 주입]
        Pause/Resume/SyncNow/Stop -> RunStateController.(pause|resume|request_sync_now|request_stop)
        Consent(op)            -> ConsentGate.(view|grant|withdraw|acknowledge)        [U5 주입]
        Reload                 -> ConfigProvider.reload()                              [U0 주입]
    -> IpcStream.write_frame(encode(ControlResponse{PROTO_VERSION, result}))

RunStateController:
  pause/resume        -> RunState.mode 전이 + PersistedRunState 원자 지속(paused만) + status_sink.set_operational(Paused|Idle)   [U6, 하향]
  request_sync_now    -> RunState.sync_requested = true (휘발성)
  request_stop(mode)  -> RunState.stop_requested = Some(mode) (휘발성)
  current()           <- U8 오케스트레이션이 read + consume-and-clear   [no back-reference]
```

---

## 6. 컴포넌트 -> 규칙 -> 속성 매핑(추적성)

| 컴포넌트 | 정의 타입 | 관련 규칙 | Testable Property |
|---|---|---|---|
| `OperatorCli` | `Command`/`CliArgs`/`CliResult`/`ExitCode` | R-U7B-01/02/03/04 | PROP-U7B-06/07 |
| `ControlPlane` | `ControlRequest`/`ControlResponse`/`ControlOp`/`ControlResult`/`ControlError`/`ConsentView`/`IpcEndpoint`/`IpcError`/`PROTO_VERSION` + seam | R-U7B-05/06/07/09 | PROP-U7B-01/02/03 |
| `RunStateController` | `RunState`/`RunMode`/`StopMode`/`PersistedRunState`/`RunStateError` + 주입 `Arc<dyn StatusSink>` | R-U7B-08/10/11/12/13 | PROP-U7B-04/05 |

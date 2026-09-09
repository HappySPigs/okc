# U7b Ops-Control — Business Logic Model (알고리즘 / 워크플로 / 상태머신)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U7b Operations Control** -> Functional Design -> 산출물 3/3 (`business-logic-model.md`)
**작성일**: 2026-09-08
**크레이트**: `ops-control` (lib) · **소속 컴포넌트**: `OperatorCli`, `ControlPlane`, `RunStateController`
**전제(AUTOPILOT 확정, 계획 §3)**: D-U7B-01..15

> **문서 성격**: 이 문서는 U7b의 **핵심 로직·알고리즘·워크플로·상태머신·데이터 흐름·교차단위 경계**를 정의한다. 타입은 `domain-entities.md`, 규칙/불변식은 `business-rules.md`가 소유하며 이 문서는 그것을 재사용·참조한다(재정의 없음).
>
> **표기 규약**: 비즈니스 의미 중심 **기술중립 설계**. Rust스러운 시그니처는 **참고용**. 다이어그램은 ASCII 화살표(`A -> B`)와 표/목록으로만 기술(박스드로잉/유니코드 화살표 금지). English 식별자명은 원문 유지.

---

## 0. 소유 vs 소비 경계 (실제 API 기준)

| 관계 | 대상 | 실제 API / 타입 | 방향 |
|---|---|---|---|
| **소유(정의)** | `ControlRequest`/`ControlResponse`/`ControlOp`/`ControlResult`/`ControlError`/`ConsentView`/`PROTO_VERSION`, `IpcEndpoint`/`IpcError` + `IpcListener`/`IpcConnector`/`IpcStream` seam, `RunState`/`RunMode`/`StopMode`/`PersistedRunState`/`RunStateError`, `Command`/`CliArgs`/`CliResult`/`ExitCode` | U7b `domain-entities.md` | 신규 |
| **소비(주입)** | 상태 스냅샷/헬스 | U6 `StatusService`(`snapshot() -> StatusSnapshot` concrete + `ReadJudgment::health_check() -> Health`), `Arc<StatusService>` 주입 | ControlPlane -> U6 |
| **소비(주입)** | run-state -> 운영상태 반영 | U0 `StatusSink::set_operational(state: OperationalState)`(U6 `StatusService` 구현), `Arc<dyn StatusSink>` 주입 | RunStateController -> U6(하향) |
| **소비(주입)** | 업로드 히스토리 | U6 `UploadHistoryStore.query(HistoryQuery) -> Result<Vec<UploadHistoryRecord>, HistoryError>`, `Arc<UploadHistoryStore>` 주입 | ControlPlane -> U6 |
| **소비(주입)** | 동의 제어 | U5 `ConsentGate.view()/grant()/withdraw()/acknowledge()/consent_state()`, `Arc<ConsentGate>` 주입 | ControlPlane -> U5 |
| **소비(주입)** | config 리로드 | U0 `ConfigProvider.reload() -> Result<(), ConfigError>`, `Arc<ConfigProvider>` 주입 | ControlPlane -> U0 |
| **소비(코덱)** | 와이어/상태 직렬화 | U0 `encode`/`decode`(CBOR, `CodecError`) | ControlPlane/RunStateController -> U0 |
| **소비(직접, IPC 우회)** | 서비스 수명주기 | U7a `ServiceManager::new(Box<dyn ServiceController>)`/`native_controller()`/`install`/`uninstall`, `Uninstaller::new(ServiceManager, Box<dyn FileSystem>, Box<dyn TokenPurgePort>)`/`uninstall`, `ServiceSpec::new`, `UninstallOptions::default`, `ArtifactSet`, `StdFileSystem`, `UnsupportedTokenPurge` | OperatorCli -> U7a(in-process) |
| **read-by(no back-ref)** | run-state 관측 | `RunStateController.current() -> RunState`(U8이 read + consume-and-clear) | U8 -> U7b (U7b는 U8 미참조) |

> **비순환·하향 주입**: ControlPlane은 U0/U5/U6 **구체 타입을 `Arc`로 주입**받아 소비하되(조립 루트 U8이 조립) U8 타입을 참조하지 않는다. `RunStateController`는 권위 있는 상태를 보유하고 **U8이 이를 읽어** 사이클을 구동한다 — RunStateController가 U8을 호출하지 않으므로 역참조가 없다(US-E6-03, `unit-of-work.md` §3.1). `RunStateController`는 추가로 pause/resume를 `Arc<dyn StatusSink>`(U6 `StatusService` 구현)에 반영하는데, `StatusSink`는 U0 소유 트레이트이고 주입 방향은 U6로의 **동위-하향**이라 U8 역참조가 아니다. `ops-control`의 `[dependencies]` = `foundation`, `observability`, `auth-consent`, `lifecycle-deploy`(모두 하위/동위 크레이트) -> 컴파일타임 비순환.

---

## 1. OperatorCli — 파싱 -> 라우팅 -> 렌더/종료코드

### 1.1 실행 흐름 (R-U7B-01, 라우팅 이분법)

```
run(args) -> ExitCode:
  cmd = parse(args)                         // 고정 서브커맨드 집합(경량 파서, code-gen 확정)
  match cmd:
    // (A) 데몬 대상 -> IPC 클라이언트
    Status|Health|Pause|Resume|SyncNow|Stop|History|Consent|Reload:
        op   = to_control_op(cmd)
        resp = ControlPlane_client.request(ControlRequest{PROTO_VERSION, op})   // §2.4
        return render(cmd, resp, args.json)     // Health면 exit-code 매핑(§1.2)
    // (B) 서비스 수명주기 -> U7a 직접 호출 (데몬 미기동에서도 동작, in-process)
    Install:
        mgr  = ServiceManager::new( native_controller()? )    // U7a 플랫폼 네이티브 팩토리(mac launchd / Linux systemd / Win SCM)
        spec = ServiceSpec::new(current_exe(), data_dir())    // R-U7B-02, autostart=true 기본
        return render_result(mgr.install(&spec))
    Uninstall(opts):
        mgr       = ServiceManager::new( native_controller()? )                   // 등록 해제(deregister_service) 선행용
        uninst    = Uninstaller::new( mgr, StdFileSystem, UnsupportedTokenPurge )  // 실 FileSystem + U5-backed TokenPurgePort(MVP null-object)
        artifacts = assemble_artifact_set(config_data_dir())  // vault_root 하위 배제(D-U7B-15)
        return render_result(uninst.uninstall(&opts, artifacts, vault_root()))
```

- 두 경로는 배타적이며 전체 `Command`를 덮는다(PROP-U7B-07). install/uninstall은 IPC 클라이언트를 절대 건드리지 않는다.
- **U7a 인스턴스 구성(CLI 프로세스 내)**: install/uninstall은 데몬 밖 CLI 프로세스에서 실행되므로 이 프로세스가 U7a 인스턴스를 직접 구성한다 — `ServiceManager::new(native_controller()?)`(U7a 플랫폼 네이티브 컨트롤러 팩토리 = 현재 OS 어댑터)로 매니저를 만들고, uninstall은 `Uninstaller::new(ServiceManager, StdFileSystem, UnsupportedTokenPurge)`로 애그리게이트를 조립한다(실 `FileSystem` 구현 `StdFileSystem` + U5-backed `TokenPurgePort` — U5에 토큰 삭제 API가 없어 MVP 기본은 미지원 null-object `UnsupportedTokenPurge`). 이들은 IPC가 아니라 in-process로 직접 호출된다(R-U7B-02, D-U7B-15). `native_controller()`가 `UnsupportedPlatform`을 반환하면 CLI는 이를 렌더 + non-zero로 표면화한다.

### 1.2 health -> 종료코드 매핑 (R-U7B-04, US-E5-02)

```
render_health(resp, json) -> (output, ExitCode):
  match resp:
    Ok(ControlResponse{ result: Health(Healthy) })          -> (render, ExitCode(0))
    Ok(ControlResponse{ result: Health(Unhealthy{reasons}) })-> (render(reasons), ExitCode(1))
    Ok(ControlResponse{ result: Error(e) })                 -> (render(e), ExitCode(1))
    Err(IpcError::Connect)                                  -> ("데몬 미도달", ExitCode(2))
    Err(other_ipc)                                          -> (render(other_ipc), ExitCode(2))
```

- `status`도 동일 전송이나 스냅샷을 렌더하며 종료코드는 성공 0 / 전송 실패 2. `--json`이면 `StatusSnapshot`/`Health`/레코드 벡터를 기계판독 렌더(R-U7B-03). 배타·전수는 PROP-U7B-06.

---

## 2. ControlPlane — 서버 accept 루프 + 프레이밍 + 디스패치

### 2.1 seam 구조 (D-U7B-01)

```
ControlPlane (전송-불변 프로토콜 로직)
  -> IpcListener / IpcConnector / IpcStream (seam trait): accept / connect / read_frame / write_frame
       -> Uds*   (mac/Linux, std::os::unix::net 1차 구현)
       -> NamedPipe* (Windows, 이연/스텁; un-defer = interprocess 크레이트)
       -> FakeTransport (테스트: 인메모리 Vec<u8> 버퍼 쌍 — 소켓 없이 프로토콜 검증)
```

- 프레이밍/버전/디스패치는 `IpcStream`을 호출하는 **순수 로직**이며 실 소켓 없이 fake로 결정적 테스트(PROP-U7B-01/02/03).

### 2.2 서버 워크플로 (R-U7B-05/06/07/08)

```
serve(endpoint):
  bind_owner_only(endpoint)              // stale unlink -> 0700 dir 하위 bind -> chmod 0600 (R-U7B-07)
  loop:
    stream = listener.accept()?          // 실패한 개별 연결은 로그 후 continue (R-U7B-09 엣지)
    handle_conn(stream)                  // 1 연결 = 1 요청/응답(MVP 동기)

handle_conn(stream):
  bytes = stream.read_frame()            // 길이-프리픽스 1프레임 (절단/과대 -> IpcError::Protocol)
  req: ControlRequest = decode(bytes)?   // 디코드 실패 -> Protocol (패닉 없음)
  result =
     if req.proto_version != PROTO_VERSION:               // R-U7B-06 (디스패치 전 차단)
         ControlResult::Error(VersionMismatch{server:PROTO_VERSION, client:req.proto_version})
     else dispatch(req.op)               // §2.3
  stream.write_frame(encode(ControlResponse{PROTO_VERSION, result}))
```

### 2.3 디스패치 테이블 (R-U7B-08, 실제 공개 API로만)

```
dispatch(op):
  Status        -> Status( status_service.snapshot() )
  Health        -> Health( status_service.health_check() )                    // ReadJudgment
  History(q)    -> match history_store.query(q):
                       Ok(v)  -> History(v)
                       Err(e) -> Error(HistoryUnavailable(msg(e)))
  Pause         -> run_state.pause()   |> ack_or_runstate_err
  Resume        -> run_state.resume()  |> ack_or_runstate_err
  SyncNow       -> run_state.request_sync_now();  Ack
  Stop(mode)    -> run_state.request_stop(mode);  Ack
  Consent(View) -> Consent(ConsentView{ consent: gate.consent_state(),
                                        acknowledged: gate.view().acknowledged })
  Consent(Grant)-> map_consent( gate.grant() )
  Consent(Withdraw)   -> map_consent( gate.withdraw() )
  Consent(Acknowledge)-> map_consent( gate.acknowledge() )
  Reload        -> match config_provider.reload():
                       Ok(())  -> Ack
                       Err(e)  -> Error(ReloadFailed(msg(e)))

ack_or_runstate_err(r) = match r { Ok(())->Ack, Err(e)->Error(RunState(msg(e))) }
map_consent(r)         = match r { Ok(())->Ack, Err(ce)->Error(ConsentRejected(msg(ce))) }
```

- 도메인 오류는 **응답 프레임 안** `ControlResult::Error`로(전송 성공), 전송/프레이밍/버전 실패만 `IpcError`(R-U7B-09).
- **MVP 트림(reload -> run-state 미적용)**: `Reload`는 `ConfigProvider.reload()`만 호출하고, 리로드된 config에 지정된 pause/resume/stop을 `RunStateController`에 **적용하지 않는다**. config 주도 run-state 반영은 post-MVP 이연이다(D-U7B-16, silent drop 아님) — MVP의 run-state 제어 경로는 CLI/IPC(pause/resume/stop 서브커맨드)이며, config 주도 경로는 U7b-등록 federated run-state config 키 + reload-apply 배선이 추가로 필요하다.

### 2.4 클라이언트 요청 (OperatorCli 사용)

```
request(req: ControlRequest) -> Result<ControlResponse, IpcError>:
  stream = connector.connect(endpoint)?         // 데몬 미기동 -> IpcError::Connect (CLI exit 2)
  stream.write_frame(encode(req))?
  bytes = stream.read_frame()?                  // 절단/과대 -> Protocol
  decode::<ControlResponse>(bytes)              // 디코드 실패 -> Protocol
```

---

## 3. RunStateController — 전이 + crash-atomic 지속

### 3.1 상태 전이 (텍스트 전이표, R-U7B-10)

| 현재 mode | 연산 | 다음 mode | 부수효과 |
|---|---|---|---|
| Running | `pause()` | Paused | `PersistedRunState{paused:true}` 원자 지속(R-U7B-11) + `status_sink.set_operational(OperationalState::Paused)`(R-U7B-12) |
| Paused | `pause()` | Paused | no-op(멱등, 재지속·재반영 불필요) |
| Paused | `resume()` | Running | `PersistedRunState{paused:false}` 원자 지속 + `status_sink.set_operational(OperationalState::Idle)`(R-U7B-13) |
| Running | `resume()` | Running | no-op(멱등) |
| (any) | `request_sync_now()` | (불변) | `sync_requested = true`(휘발성, 미지속) |
| (any) | `request_stop(mode)` | (불변) | `stop_requested = Some(mode)`(휘발성, 미지속) |

- 전이는 임계구역(mutex) 안에서 직렬화. 지속 실패 -> `RunStateError::Persist`이되 인메모리 전이는 반영됨(인메모리 권위, 지속은 재시작 대비).
- **StatusService 반영(R-U7B-12/13)**: pause/resume는 주입된 `Arc<dyn StatusSink>`(U0 `foundation` 트레이트, U6 `StatusService` 구현)의 `set_operational(_)`을 호출한다 — `pause -> OperationalState::Paused`, `resume -> OperationalState::Idle`. `status` op는 `StatusService.snapshot()`만 읽으므로(§2.3), 이 반영이 있어야 `watcher status`가 paused를 표면화한다(US-E6-03 AC "상태가 paused로 표시된다"). `set_operational`은 infallible이며 U6로의 동위-하향 주입이라 U8 역참조가 아니다.

### 3.2 crash-atomic 지속/복구 (R-U7B-11, U4 미러)

```
persist(paused):                           // U4 SyncStateStore R-STATE-01 파이프라인 미러
  bytes = encode(PersistedRunState{paused}) // CBOR (U0 코덱)
  write(<state>.tmp, bytes); fsync(tmp)
  atomic_rename(<state>.tmp -> <state>); fsync(parent_dir)   // best-effort

load() -> RunState:
  match read(<state>):
     Ok(bytes) & decode ok -> RunState{ mode: if paused {Paused} else {Running},
                                        sync_requested:false, stop_requested:None }
     absent | truncated | corrupt | leftover-temp -> RunState{ Running, false, None }  // 안전 회복(패닉 없음)
```

- `sync_requested`/`stop_requested`는 로드 시 항상 기본값(휘발성 신호는 재시작에서 재요청 필요, D-U7B-07). PROP-U7B-05.

### 3.3 U8 관측 데이터 흐름 (read-by-U8, no back-reference)

```
U8 SyncCycleCoordinator (트리거 경계마다):
  st = RunStateController.current()                  // read (RunStateController는 U8 미호출)
  if st.mode == Paused: 사이클 스킵(감시만 유지)
  if st.stop_requested.is_some(): graceful 종료 진입(유예 정책 = U8 소관)
  if st.sync_requested: 즉시 사이클 트리거 + consume-and-clear(sync_requested=false)
```

- 즉시 wake(채널/notify)는 MVP 이연 — U8이 다음 트리거 틱에서 관측(D-U7B-08). un-defer 경로 = 주입 `Notify` waker seam. `RunStateController`는 신호를 **보유**만 하고 U8을 호출하지 않는다(US-E6-03 역참조 회피).

---

## 4. 컴포넌트별 Testable-Properties 노트 (PBT-01)

- **ControlPlane**: in-memory fake `IpcStream`(Vec<u8> 버퍼 쌍)으로 소켓 없이 -> PROP-U7B-01(메시지 라운드트립), PROP-U7B-02(프레이밍 절단/손상에 패닉 없이 `Protocol`), PROP-U7B-03(버전 불일치 시 부작용 핸들러 0회 호출 = 기록 fake로 검증). 실 UDS bind/권한은 PBT 대상 아님(통합/Infra 이월).
- **RunStateController**: temp-dir 상태 파일 + 임의 명령 시퀀스 -> PROP-U7B-04(전이·멱등), PROP-U7B-05(지속-재로드 paused 보존 + 휘발성 리셋 + 손상 파일 안전 회복). U4 crash-atomic 선례를 미러하므로 동일 PBT 프로파일.
- **OperatorCli**: 기록 fake(IPC 클라이언트 / U7a 핸들) + 임의 `Health`/`Command` -> PROP-U7B-06(health->exit-code 배타·전수), PROP-U7B-07(라우팅 이분법 배타·전수). 실 프로세스 종료·current_exe 해소는 PBT 대상 아님(통합 이월).
- **Windows 명명 파이프 / 실 UDS / 실 ServiceManager 어댑터**: No PBT properties identified — seam 뒤 통합/Infra 테스트 대상(code-gen 이월).

---

## 5. 확장 컴플라이언스 요약

| 확장 | 활성 | 판정 |
|---|---|---|
| **Resiliency Baseline** | ON | 회복력이 핵심: §2.2 프레이밍/디코드가 절단·적대적 입력에 패닉 없이 `IpcError::Protocol`, §3.2 run-state crash-atomic 지속 + 손상 상태 파일 안전 회복(U4 R-STATE-01 미러), §2.2 서버 accept 루프가 1개 나쁜 연결에 죽지 않음, §1.2 health->exit-code(US-E5-02)로 서비스매니저/모니터링 회복 판정. RPO/RTO·HA/DR·배포 파이프라인은 Infra/Ops 이월 -> 부분 N/A |
| **Property-Based Testing** | ON (Full) | §4 컴포넌트별 속성 + 제너레이터(in-memory fake `IpcStream` / temp-dir 상태 파일 / 임의 명령 시퀀스 / 임의 `Health`·`Command` / 기록 fake 핸들). 프레임워크(proptest)는 code-gen 이월(PBT-09), shrinking/시드/CI는 PBT-08 |
| **Security Baseline** | OFF | N/A. 소유자-전용 소켓(§2.2 bind_owner_only)은 정합성/회복력 설계이지 Security 확장 강제 아님. config 평문 토큰(RISK-01)은 문서화된 수용 위험 |

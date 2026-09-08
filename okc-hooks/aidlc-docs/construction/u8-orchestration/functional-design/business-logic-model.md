# U8 Orchestration — Business Logic Model (조립 시퀀스 / 사이클 알고리즘 / 스레딩)

**단계**: CONSTRUCTION -> U8 Orchestration -> Functional Design (산출물 3/3)
**작성일**: 2026-09-08 · **크레이트**: `watcher-bin` (binary)
**범위**: WatcherDaemon 기동/조립 시퀀스, SyncCycleCoordinator 트리거당 사이클 알고리즘, 트리거 소스 스레딩 모델, 교차-크레이트 배선(각 단계의 실제 API), 컴포넌트별 Testable-Properties.

> 규약: 한국어 산문, ASCII 화살표(`A -> B`)만, 박스드로잉/유니코드 화살표 금지, Rust 타입/식별자는 백틱.

---

## 0. 소유 vs 소비 경계 (실제 API 기준)

U8은 **로직을 최소로** 담는 얇은 조립 루트다(`unit-of-work.md` §3.3). 사이클의 실질 로직 대부분은 동결 하위 크레이트가 소유하며, U8은 배선·직렬화·표면화만 소유한다.

| 관심사 | 소유 | U8 역할 |
|---|---|---|
| config 로드/검증/리로드 팬아웃 | U0 `ConfigProvider` | `new(known_keys)`/`load`, federated raw-parse + typed 주입(고유) |
| 매니페스트 재계산·diff·한도 | U1 | 호출만(scan/build/diff) — 한도/재검증은 U3 내부 재사용 |
| 트리거 발행·가드 판정 | U2(순수) | 트리거 소스 배선 + **판정 push 표면화(고유)** |
| 업로드 프로토콜(preflight/have-want/전송/재검증/commit) | **U3 `UploadProtocolDriver` 내부** | 매니페스트 넘겨 `execute_cycle_traced` 위임(중복 없음) |
| 상태 지속·복구·재시도 분류 | U4 | `open_and_recover`/`mark_dirty` 호출, `RetryBackoffController` 구동 |
| 전송/자격/동의 | U5 | 배선 + consent 게이트 평가(사이클 직전) |
| 로그/상태/히스토리/중대오류 | U6 | 싱크 구축 + 하향 주입 + 사이클 결과 push |
| 배포/업데이트/운영 제어 | U7a/U7b | 배선 + ControlPlane serve(U8 고유 핸들러) + run-state 읽기. **AutoUpdater는 MVP 미배선**(post-MVP), ServiceManager(install/autostart)+Uninstaller는 CLI 경로로만 노출(FIX3) |
| **조립·단일 인스턴스·트리거 직렬화·graceful 종료·표면화** | **U8(고유)** | 본 문서 |

---

## 1. WatcherDaemon — 기동/조립 시퀀스 (services.md 1~8단계)

### 1.1 진입: `run(config_path) -> Result<(), DaemonError>`

```text
[S1] config 로드 + federated 해소 (R-U8-03)
     known_keys = FOUNDATION_CONFIG_KEYS ∪ AUTH_CONSENT_CONFIG_KEYS
                  ∪ OBSERVABILITY_CONFIG_KEYS ∪ U8_KEYS         (DEC-U8-07)
     provider = ConfigProvider::new(&known_keys); snap = provider.load(cli_path)?
     raw = serde_json 파싱(config_path)  ->  typed 해소:
         request_timeout_s -> validate_request_timeout_s -> Duration      (U5)
         obs 경로/rotation -> ObservabilityConfig / LoggerConfig(PathBuf)  (U6)
         chunk_threshold/size -> u64                                      (U3)
         t_debounce/t_recon -> Duration                                   (U2)
         update 채널/gate, data_dir, socket_path                          (U7a/U8)
     무효 config -> 명확한 오류로 즉시 종료 (lock 이전)
[S2] lock = SingleInstanceLock::acquire(&LockConfig)?                     (R-U8-01/02)
     Err(AlreadyRunning{pid,owner}) -> 보고 후 비정상 종료
[S3] store = Arc<Mutex<SyncStateStore::open_and_recover(&StateConfig)?>>  (NFR-03)
[S4] U6 싱크 구축 (먼저) -> U0 트레이트 핸들                              (R-U8-04)
     status = Arc::new(StatusService::new())
     logger = Arc::new(StructuredLogger::new(logger_cfg, Arc<SystemClock>, provider.clone()))
     history = Arc::new(UploadHistoryStore::new(history_path, Some(logger.clone())))
     tray = Arc::new(TrayIndicator::new())               // no-op (§0 수정2)
     critical = Arc::new(CriticalErrorNotifier::new(logger, status.clone(), tray, threshold))
     // 로그레벨 live-reload: provider.subscribe(logger) 하지 않음(FIX2).
     //   StructuredLogger가 Arc<ConfigProvider>를 보유 -> subscribe(&mut self)는 참조 순환이라
     //   Arc::get_mut == None 으로 컴파일 불가. 대신 U8 고유 DaemonControlHandlers.reload가
     //   config.reload() 성공 후 logger.reload()를 호출해 캐시 레벨을 재적용한다(§5.3, R-U8-04).
[S5] 하위 단위 구축 + 하향 주입 (domain-entities §6 B6..B12)
     cred = CredentialProvider::with_defaults(provider.clone() as Arc<dyn ConfigSource>)
     transport = AuthTransport::new(cred, Arc::new(UreqAdapter::new()), provider.clone(), timeout)
     consent = Arc::new(ConsentGate::open_with_system_clock(consent_path, Some(status.clone())))
     retry = RetryBackoffController; scanner/builder/differ/limits(U1);
     guard = VaultAvailabilityGuard::new(confirm_empty); recon = ReconciliationScheduler::new(t_recon)
     driver = UploadProtocolDriver::new(transport, Box<VaultBlobSource>,
                Box<SharedSyncStore(store.clone())>, status.clone(), chunk_threshold, chunk_size)
     run_state = Arc::new(RunStateController::new(run_state_path, status.clone()))
     // AutoUpdater는 MVP 미배선(FIX3): AutoUpdater::new는 Box<dyn UpdateSource> seam을 요구하나
     //   기본 UpdateSource를 구성하지 않는다(auto-update opt-in / post-MVP). ServiceManager/Uninstaller는
     //   CLI install/uninstall 경로(§4 main.rs)에서만 구성되며 데몬 run 루프는 이를 구동하지 않는다.
[S6] ControlPlane 배선 (DEC-U8-12) — U8 고유 핸들러 주입
     (trigger_tx, trigger_rx) = mpsc::channel::<CycleTrigger>()   // U8 합류 채널(§1.2), 이 지점 이전 생성
     inner    = WatcherHandlers::new(status, history, consent, provider, run_state)  // 동결 배선
     handlers = Arc::new(DaemonControlHandlers::new(inner, trigger_tx.clone(),       // §5.3, U8 소유
                            logger.clone(), provider.clone()))
     //   sync_now -> trigger_tx.send(CycleTrigger::SyncNow) (엣지 이벤트, FIX1)
     //   reload   -> config.reload() 후 logger.reload()     (로그레벨 재적용, FIX2)
     //   그 외 op -> inner(WatcherHandlers) 위임
     listener = bind_native(socket_endpoint)?
     spawn(move || ControlPlane::new(handlers).serve_forever(&listener))   // 전용 스레드
     // ControlPlane<H: ControlHandlers>는 제네릭 -> U8 고유 H를 그대로 주입(control_plane.rs new(Arc<H>))
[S7] coordinator = SyncCycleCoordinator::new(store, guard, scanner/builder/differ,
                consent, driver, retry, status, logger, history, critical, run_state)
     startup 트리거 enqueue: trigger_tx.send(Reconcile(recon.run_startup_scan(now)))  (DEC-U8-16)
[S8] 트리거 소스 배선 + 실행 (§3)
     (watcher, fs_rx) = FilesystemWatcher::start(root, &NotifyBackend, t_debounce)?
     spawn(forwarder: fs_rx -> trigger_tx as Filesystem)
     spawn(timer: recon.tick -> trigger_tx as Reconcile)   // sync-now 폴링 없음(FIX1: 핸들러 엣지)
     // auto_updater loop 없음(FIX3: AutoUpdater MVP 미배선)
     coordinator.run_loop(trigger_rx, shutdown_flag)   // 소비자(현 스레드)
[S9] graceful 종료 (R-U8-10): watcher drop -> timer/forwarder stop -> in-flight drain
     -> ControlPlane stop -> lock(LockGuard) drop
```

### 1.2 상태 집계 + liveness (DEC-U8-13, services.md 7단계)

- 운영 라이프사이클(축1) + 활성 조건(축2)은 `StatusService`가 단일 집계(Q9=B). 코디네이터/run-state가 `set_operational`/`raise_condition`/`clear_condition`을 push한다.
- WatcherDaemon(또는 timer)이 주기 heartbeat로 `status.set_liveness(Alive)`를 갱신 -> `update_probe() -> Liveness`(순수 liveness, AutoUpdater 소비 — MVP 미배선이므로 소비자 부재이나 메서드/분리 계약은 유지)와 `health_check() -> Health`(운영 헬스, CLI/모니터링)가 **분리**된다(롤백 루프 방지). `ControlPlane`의 status/health op가 `snapshot()`/`health_check()`를 읽는다.

---

## 2. SyncCycleCoordinator — 트리거당 사이클 알고리즘

### 2.1 소비 루프

```text
run_loop(trigger_rx, shutdown_flag):
  loop:
    if shutdown_flag.get(): break
    trigger = trigger_rx.recv()            // blocking, 단일 소비자 == 직렬 잠금 (R-U8-06)
    drain_to_latest(trigger_rx, &mut trigger)   // 폭주 흡수 (DEC-U8-02)
    cycle_in_progress.set(true)
    outcome = run_cycle(trigger)
    cycle_in_progress.set(false)
    if let Failed(err) = outcome:                                // 변형별 처리 (R-U8-09, FIX4)
        if let Transport(te) = err && retry.classify(te)==retryable: schedule_backoff(retry, te)
        // HashMismatch/OverLimit/Aborted 및 AuthFailed: 백오프 없이 종료(다음 트리거 재스냅샷)
```

### 2.2 `run_cycle(trigger) -> CoordinatorOutcome` (services.md 1~12 + 동결 U3 흡수)

```text
[C1] cycle_id 발급; 트리거 원인 요약을 logger.event(cycle_id, cause) push   (US-E1-01, R-U8-07 push 소유)
[C2] rs = run_state.current();  // mode만 읽음 — sync_requested는 읽지 않음(SyncNow는 채널 엣지) (R-U8-06)
     if rs.mode == Paused: return SkippedPaused
[C3] avail = guard.check_reachable(root)                                     (U2)
     if avail != Reachable:
        status.raise_condition(VaultUnavailable); logger push               (US-E1-06, R-U8-07)
        return HeldVaultUnavailable
[C4] snapshot = scanner.scan()?;  new_manifest = builder(snapshot).build()?  (U1, 폴더=진실원천)
[C5] last = store.lock().last_committed_manifest().cloned()
     verdict = guard.guard_diff(&new_manifest, last.as_ref(), avail)         (U2)
     if verdict == HoldDestructiveEmpty:
        status.raise_condition(...) ; logger push ; return HeldDestructiveEmpty  (US-E1-06)
[C6] change = differ.diff(last, &new_manifest)                               (U1)
     if change.is_empty(): status idle push ; return NoOp                    (R-U8-08)
[C7] decision = consent.is_upload_permitted()                               (U5, R-U8-08)
     if Blocked(reason): status.raise_condition(...) ; return BlockedConsent(reason)
[C8] store.lock().mark_dirty()?                                             (커밋 전 유실 방지, services 9)
[C9] report = driver.execute_cycle_traced(&new_manifest)                     (U3 위임 — 아래 흡수)
     // 드라이버 내부: SafetyLimits preflight(over-limit -> OverLimit push) ->
     //   digest no-op 조기종료 -> have/want(raw_sha256) -> 재개 청크 전송 ->
     //   전송 직전 재-읽기+재-해시 재검증(불일치 abort) -> commit_manifest(SharedSyncStore)
[C10] match report.outcome:
        Ok(commit):  status.record_sync_success(now); status.set_dirty(false);
                     status.set_operational(Idle); history.append(success);
                     retry.on_success(); return Uploaded(commit)             (R-U8-09, services 11)
        Err(err):    // 공통: history.append(failure/partial); critical.report_cycle_result(map_to(err)) (FR-19 case2)
                     //        offline 상태 push; dirty 유지; return Failed(err)               (services 12)
                     // 백오프는 변형별로만(FIX4, R-U8-09):
                     UploadError::Transport(te):  retry가 te(TransportError) 분류
                        -> retryable 클래스: 다음 재시도 시각 계산 + 전체 사이클 백오프 재예약
                        -> AuthFailed 클래스: 재시도 제외(백오프 없음), U5 인증 흐름에 위임(사이클 종료)
                     UploadError::HashMismatch{..} | OverLimit(_) | Aborted:  분류 대상 아님
                        -> 백오프 재예약 없음, 사이클 종료(로그); 다음 트리거가 자동 재스냅샷
```

- **AuthFailed**(err)는 `RetryBackoffController`가 재시도에서 제외해 U5 인증 흐름에 위임한다(U4 분류 승계, R-U8-09). 오직 `UploadError::Transport(TransportError)`만 `RetryBackoffController`에 넘겨 분류·백오프하며(`TransportError`가 유일한 분류 가능 값, upload-client/src/error.rs), `HashMismatch`/`OverLimit`/`Aborted`는 백오프 없이 사이클을 종료한다(다음 트리거 재스냅샷). 진행 중 재개 오프셋 push는 드라이버가 주입된 `status`로 직접 수행한다(`set_resume_progress`).

---

## 3. 트리거 소스 스레딩 모델 (DEC-U8-01, blocking/std만)

```text
소스                         스레드                         -> 합류 채널 (mpsc<CycleTrigger>)
FilesystemWatcher            (U2 내부 감시 스레드)           fs_rx(mpsc<TriggerSignal>)
  -> forwarder 스레드         drain fs_rx                    -> trigger_tx.send(Filesystem(sig))
ReconciliationScheduler      timer 스레드(주기 sleep)        tick(now, busy=cycle_in_progress)
                                                            -> Some(sig) => trigger_tx.send(Reconcile)
ControlPlane sync-now        serve_forever 스레드            DaemonControlHandlers.sync_now(§5.3)
  (엣지 이벤트, 폴링 아님)                                    -> trigger_tx.send(CycleTrigger::SyncNow)
AutoUpdater                  (MVP 미배선 — updater 스레드 없음, FIX3)
coordinator                  소비자(main 흐름)               trigger_rx.recv() 직렬 실행
```

- async 런타임 없음 — 전반 blocking 설계 정합(MVP GUIDANCE). `FilesystemWatcher::start`가 이미 자체 감시 스레드 + `mpsc::Receiver`를 반환하므로 forwarder는 이를 공용 채널로 재전송만 한다.
- `RunStateController`는 U8을 역참조하지 않으며, U8은 `current()`를 **읽기 전용**으로만 소비한다(pause 모드/stop 신호 관측, no back-reference, US-E6-03). sync-now는 `sync_requested` 폴링이 아니라 `DaemonControlHandlers.sync_now`가 `trigger_tx`로 직접 push하는 **엣지 이벤트**다(FIX1) — 동결 `RunStateController`에 `sync_requested` clear/take API가 없어(request_sync_now만 존재, run_state.rs) 폴링 시 사이클 폭주 또는 fire-once가 되기 때문. 따라서 `sync_requested`/`request_sync_now`는 U8이 소비하지 않는다(동결·무해).
- 단일 소비자 == 사이클 직렬화(별도 배출 루프/UploadDrainWorker 없음, services.md).

---

## 4. main.rs — 얇은 진입 디스패치 (DEC-U8-09)

```text
main():
  argv = 파싱
  if argv 첫 인자 == "run" (또는 서비스-구동 진입):
      WatcherDaemon::run(resolve_config_path(cli_path))      // 데몬 전경 실행
  else:
      cli = OperatorCli::parse(&argv)?                        // U7b
      client = IpcControlClient::new(Box<native_connector()>, endpoint)   // 데몬 대상 IPC
      service = NativeServiceOps                              // install/uninstall 직접 호출
      exit_code = OperatorCli::run(&cli, &client, &service)   // health -> ExitCode 매핑
      process::exit(exit_code)
```

- 데몬 진입과 운영자 CLI 진입이 같은 바이너리(RESILIENCY-01)에서 분기한다. `ServiceManager`가 install 시 `run` 인자로 launchd/systemd/Windows Service에 등록하므로 서비스가 데몬 경로를 구동한다. main.rs는 조립 로직 외 로직을 담지 않는다.

---

## 5. 교차-크레이트 경계 (소비 vs 소유, no back-reference)

- **소비(호출만)**: U1 scan/build/diff, U3 `execute_cycle_traced`, U4 `open_and_recover`/`mark_dirty` + `RetryBackoffController`, U5 `send`/`resolve_token`/`is_upload_permitted`, U7a `AutoUpdater`/`ServiceManager`, U7b `RunStateController::current`(read)/`ControlPlane::serve_forever`.
- **소유(U8)**: 배선 그래프, `SingleInstanceLock`, 트리거 합류·직렬화, U2 판정의 push 표면화, graceful 종료, federated raw-parse.
- **역참조 0**: 모든 하위 단위는 U0 트레이트만 의존; `RunStateController`/U6 싱크는 U8을 호출하지 않는다. 배선 그래프(domain-entities §6)에 역엣지 없음(비순환, 컴파일타임 강제).

---

## 6. 컴포넌트별 Testable-Properties 노트 (PBT-01)

| 컴포넌트 | 속성(요지) | in-memory fake seam |
|---|---|---|
| `SingleInstanceLock` | free/stale->획득, live->AlreadyRunning, drop->정리(PROP-U8-01) | temp-dir 락파일 + fake liveness |
| `WatcherDaemon` | shutdown drain 비손상(PROP-U8-02), 배선 acyclic(PROP-U8-03), known_keys union + federated 라운드트립(PROP-U8-07) | 기록 fake 하위 유닛 + 임의 config 키맵 |
| `SyncCycleCoordinator` | 배타적 `CoordinatorOutcome` + 드라이버 조건부 1회 호출(PROP-U8-04), 사이클 무중첩·drain-to-latest(PROP-U8-05), U2 판정 push 1회(PROP-U8-06) | 기록 fake 드라이버/`StatusSink`/`Logger`, 임의 트리거 버스트 |

- 제너레이터/프레임워크(proptest)는 code-gen 이월(PBT-09). 실제 스레드·네트워크·파일시스템 없이 순수 로직(분기·순서·라운드트립)을 결정적으로 검증한다.

---

## 7. 확장 컴플라이언스 요약

| 확장 | 활성 | 판정 |
|---|---|---|
| **Resiliency Baseline** | ON | RESILIENCY-01 단일 바이너리. 기동 복구(open_and_recover)·단일 인스턴스·백오프 재예약·graceful drain·stale 회수가 조립 회복력을 구성. RPO/RTO·HA/DR는 Infra/Ops 이월 -> 부분 N/A |
| **Property-Based Testing** | ON (Full) | §6 + business-rules §5 PROP-U8-01..07. 미준수 blocking. 프레임워크 code-gen 이월 |
| **Security Baseline** | OFF | N/A. config 평문 토큰/락파일 PID(RISK-01)는 문서화된 수용 위험 |

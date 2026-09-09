# U8 Orchestration — Functional Design 계획 (AUTOPILOT)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U8 Orchestration** -> Functional Design (계획 + 결정)
**작성일**: 2026-09-08
**크레이트**: `watcher-bin` (binary) · **소속 컴포넌트**: `WatcherDaemon`, `SyncCycleCoordinator`, `SingleInstanceLock`
**대표 에픽**: 접착/조립(대표 에픽 없음) — 전 에픽 E1~E7을 관통하는 **시스템 조립 루트**
**입력 아티팩트**: `unit-of-work.md`(§U8 + §4 조립 루트 + §0), `services.md`(WatcherDaemon + SyncCycleCoordinator — 조립·사이클 단계 권위), `component-methods.md`(§WatcherDaemon/SyncCycleCoordinator/SingleInstanceLock), `components.md`, `application-design.md`, `unit-of-work-story-map.md`(U8 행), `stories.md`(FR-23 단일 인스턴스 / 사이클 오케스트레이션 / graceful 종료 / US-E1/E2/E3/E7 척추), `requirements.md`(RESILIENCY-01 / FR-23 / FQ)
**소비 크레이트(실제 공개 API, 전부 동결)**: `foundation`(U0), `content-core`(U1), `change-detect`(U2), `upload-client`(U3), `sync-state`(U4), `auth-consent`(U5), `observability`(U6), `lifecycle-deploy`(U7a), `ops-control`(U7b) — 9개 전부. 아래 §1.1
**규칙**: `construction/functional-design.md` · `common/content-validation.md` · 활성 확장 `property-based-testing.md`(PBT-01 강제) · `resiliency-baseline.md`
**모드**: AUTOPILOT — 모든 미결정을 권장안 + MVP 최소범위로 자가 확정(§3). 사용자 질문 없음. drop-list(§5) 항목은 재개봉 금지.

> **표기 규약**: 비즈니스/조립 의미 중심 **기술중립 설계**(기술·크레이트 선택은 code-gen 인라인 확정). Rust스러운 시그니처는 **참고용** 개념 형상이다. 다이어그램은 ASCII 화살표(`A -> B`)와 표/목록으로만 기술(박스드로잉/유니코드 화살표 금지). English 식별자명은 원문 유지.

---

## 1. 단위 컨텍스트 (조립 루트 / 단일 배포 바이너리 / 전 단위 의존)

U8 `watcher-bin`은 시스템의 **조립 루트(Composition Root)** 이며 **유일한 배포 산출물**(RESILIENCY-01)이다. 9개 lib 크레이트를 `[dependencies]`로 링크해 단일 실행파일을 산출하고(§3.3 `unit-of-work.md`), 그 안에서 전 컴포넌트를 생성·의존성 하향 주입(DI)·구동한다. 하위 단위는 U0가 소유한 값 타입·싱크 계약 트레이트에만 의존하고 상위를 역참조하지 않으므로, 크레이트 의존 그래프가 곧 비순환 DAG이다. 3개 컴포넌트:

| 컴포넌트 | 소유 스토리/책임 | 책임 요지 |
|---|---|---|
| `WatcherDaemon` | FR-21 / FR-23 배선 / NFR-03 복구 오케스트레이션 / NFR-05 3-OS 조립 | 조립 루트: config 로드 + **raw 파싱으로 federated(비-core) 키를 TYPED 값으로 해소** -> `SingleInstanceLock::acquire()`(FR-23) -> `SyncStateStore::open_and_recover()` -> **전 단위 컴포넌트 생성·하향 주입**(U6 싱크를 U0 트레이트 객체로 구축해 U1..U7에 주입) -> 실행/graceful 종료 -> 상태 집계 |
| `SyncCycleCoordinator` | US-E1-01 / US-E3-02/04 / US-E7-04/09/11 · FR-01/04/11/12 · NFR-03/11 | 트리거당 **1회 직렬 사이클**(Q2=B): 트리거 소스(U2 `FilesystemWatcher` 디바운스 + U2 `ReconciliationScheduler` 주기/시작 + U7b `ControlPlane` sync-now)를 단일 직렬 스트림으로 합류; 각 트리거에 대해 run-state 확인(paused skip) -> vault 가용성 -> 매니페스트 재계산 -> diff -> consent 게이트 -> `UploadProtocolDriver` 실행 -> 결과를 U6 싱크로 push(U2 순수성이 남긴 push 공백 충전) |
| `SingleInstanceLock` | FR-23 | 크로스플랫폼 단일 인스턴스 가드: 이미 실행 중이면 활성 PID/소유자 보고 후 비정상 종료, stale이면 회수 후 진행 |

### 1.1 소비하는 실제 API 경계 (동결 크레이트 — src 확인, 재정의/편집 금지)

- **U0 `foundation`**: `ConfigProvider::new(known_keys)`/`load(cli_path)`/`current()`/`reload()`/`subscribe(observer)`, `read_and_validate`/`resolve_config_path`, `WatcherConfig`(core 6필드)/`ConfigSnapshot`, `FOUNDATION_CONFIG_KEYS`; 싱크 계약 트레이트 `Logger`/`StatusSink`/`HistorySink`/`CriticalEventSink`/`ReadJudgment`/`ConfigReloadObserver`; 값 타입 `Manifest`/`ChangeSet`/`CycleOutcome`/`Version`/`Timestamp`/`OperationalState`/`ActiveCondition`/`LivenessSignal`/`RelativePath`; `encode`/`decode`.
- **U1 `content-core`**: `VaultScanner::new(root)`/`scan()`/`open_reader(&RelativePath)`, `ManifestBuilder::new(&VaultScanner)`/`build()`, `ManifestDiffer::diff(last, current)`, `SafetyLimitsValidator::validate`(주: U3가 사이클 내부에서 재사용), `ContentAddressing`.
- **U2 `change-detect`**: `FilesystemWatcher::start(root, &dyn WatchBackend, t_debounce) -> (FilesystemWatcher, TriggerStream)`(`TriggerStream = mpsc::Receiver<TriggerSignal>`), `NotifyBackend`, `ReconciliationScheduler::new(t_recon)`/`run_startup_scan(now)`/`tick(now, busy) -> Option<TriggerSignal>`, `VaultAvailabilityGuard::new(confirm_empty)`/`check_reachable(root) -> Availability`/`guard_diff(new, last, availability) -> GuardVerdict`, `TriggerSignal`/`TriggerKind`.
- **U3 `upload-client`**: `UploadProtocolDriver::new(AuthTransport, Box<dyn BlobSource>, Box<dyn SyncStore>, Arc<dyn StatusSink>, chunk_threshold_bytes, chunk_size_bytes)`/`execute_cycle(&mut, &Manifest)`/`execute_cycle_traced(&mut, &Manifest) -> CycleReport`; seam 트레이트 `BlobSource`(`open(&RelativePath) -> Box<dyn Read>`)·`SyncStore`(U4 위에 `impl SyncStore for SyncStateStore` 존재). **드라이버가 내부에서 preflight limits·no-op digest·have/want·전송·재검증·`commit_manifest`까지 수행**(§0 finding).
- **U4 `sync-state`**: `SyncStateStore::open_and_recover(&StateConfig)`/`last_committed_manifest()`/`mark_dirty()`/`commit_manifest(m)`/재개 오프셋 API, `StateConfig`, `RetryBackoffController::new(&BackoffConfig, n, seed)`(분류/백오프/에스컬레이션, 순수).
- **U5 `auth-consent`**: `AuthTransport::new(Arc<CredentialProvider>, Arc<dyn HttpTransport>, Arc<dyn ConfigSource>, Duration)`/`send`, `CredentialProvider::new/with_defaults`, `UreqAdapter::new`(concrete `HttpTransport`), `ConsentGate::open`/`open_with_system_clock`/`is_upload_permitted() -> ConsentDecision`/`acknowledge`/`grant`/`withdraw`, `impl ConfigSource for ConfigProvider`(어댑터 불필요), `validate_request_timeout_s(i64) -> Result<Duration,_>`, `AUTH_CONSENT_CONFIG_KEYS`.
- **U6 `observability`**: `StructuredLogger::new(LoggerConfig, Arc<dyn Clock>, Arc<ConfigProvider>)`(also `ConfigReloadObserver`), `StatusService::new()`(also `StatusSink`+`ReadJudgment`: `set_operational`/`raise_condition`/`clear_condition`/`record_sync_success`/`set_dirty`/`set_resume_progress`/`set_liveness`/`snapshot`/`health_check`/`update_probe`), `UploadHistoryStore::new(path, Option<Arc<dyn Logger>>)`/`query`(also `HistorySink`), `CriticalErrorNotifier::new(Arc<dyn Logger>, Arc<StatusService>, Arc<TrayIndicator>, threshold)`(also `CriticalEventSink`), `TrayIndicator::new()`/`start`(MVP no-op), `ObservabilityConfig`/`LoggerConfig`, `OBSERVABILITY_CONFIG_KEYS`, `SystemClock`.
- **U7a `lifecycle-deploy`**: `ServiceManager::new(Box<dyn ServiceController>)`/`native_controller()`, `AutoUpdater::new(source, ServiceManager, Arc<dyn ReadJudgment>, Arc<dyn CriticalEventSink>, Arc<dyn Clock>, UpdateChannel, GateConfig, Version)`/`check_for_update`/`apply_update`, `Uninstaller`.
- **U7b `ops-control`**: `RunStateController::new(state_path, Arc<dyn StatusSink>)`/`current() -> RunState`(`mode`/`sync_requested`/`stop_requested`)/`pause`/`resume`/`request_sync_now`/`request_stop`, `WatcherHandlers::new(Arc<StatusService>, Arc<UploadHistoryStore>, Arc<ConsentGate>, Arc<ConfigProvider>, Arc<RunStateController>)`, `ControlPlane::new(Arc<H>)`/`serve_forever(&dyn IpcListener)`, `bind_native`, `OperatorCli::parse`/`run(&CliArgs, &dyn ControlClient, &dyn ServiceOps)`, `IpcControlClient`/`native_connector`, `NativeServiceOps`.

### 1.2 Application Design 이월 미결정 -> §3 AUTOPILOT 확정

`services.md`/`component-methods` 이월 항목: 트리거 소스 배선·직렬화 스레딩 모델, 단일-사이클 잠금 구현, `SingleInstanceLock` OS별 회수/stale 판정, federated raw-parse + 타입 해소·주입 경계, 공유 `SyncStateStore` 소유권, `BlobSource` 실체, main.rs 진입 디스패치, graceful 종료 신호 메커니즘, AutoUpdater 스케줄링. 아래 §3에서 전부 자가 확정한다.

---

## 2. Functional Design 산출물 체크리스트 (기술중립, 프론트엔드 파일 없음)

`aidlc-docs/construction/u8-orchestration/functional-design/` 하위에 생성한다.

- [x] **`domain-entities.md`** — U8 도입/조립 타입: 트리거 enum + 채널 모델(`CoordinatorInput`/합류 스트림), shutdown 신호(`ShutdownFlag`), 단일 인스턴스 락 레코드(PID/owner, `LockRecord`/`LockError`/`InstanceInfo`/`LockGuard`), **데몬 배선 그래프(표 — 실제 크레이트 생성자명 참조)**, 사이클 컨텍스트(`CycleContext`/`CycleTrigger`/`CoordinatorOutcome`), U8-소유 어댑터(`SharedSyncStore`/`VaultBlobSource`). U0..U7b 타입은 **이름 참조만**(§0 표). 컴포넌트 -> 규칙 -> 속성 매핑 포함.
- [x] **`business-rules.md`** — `R-U8-*` 규칙: 단일 인스턴스 FR-23(acquire/stale-reclaim/report-and-exit), federated raw-parse + inject, 하향 주입 acyclic 배선, 트리거당 직렬 사이클 순서, paused-skip(`RunStateController`), vault-unavailable hold, consent 게이트, graceful 종료가 in-flight 사이클 drain. `PROP-U8-*` Testable Properties(PBT-01 강제).
- [x] **`business-logic-model.md`** — WatcherDaemon 기동/조립 시퀀스(config -> lock -> recover -> construct+inject -> run -> shutdown), SyncCycleCoordinator 트리거당 사이클 알고리즘(`services.md` 순서 + 동결 U3 흡수 반영), 트리거 소스 스레딩 모델, 교차-크레이트 배선(각 단계가 호출하는 실제 API), 컴포넌트별 Testable-Properties + 확장 컴플라이언스.
- [x] **PBT-01 "Testable Properties" 섹션**(각 산출물, 확장 강제) — 임의 트리거 시퀀스, 임의 lock 상태(live/stale/free), in-memory fake seam(fake `WatchBackend`/fake `ControlClient`/fake sink) 명시.
- [x] 산출물 작성 전 `content-validation.md` 검증(특수문자, 표/코드블록 파싱, 박스드로잉 0건, ASCII 화살표만, 한국어 산문, 백틱 Rust 타입).

---

## 3. AUTOPILOT 결정 (권장 + MVP 편향, 인용 포함)

| # | 주제 | 확정(권장) | MVP 트림? | 근거 / 인용 |
|---|---|---|---|---|
| DEC-U8-01 | 트리거 스레딩 모델 | **std::thread + 단일 `std::sync::mpsc` 채널**이 코디네이터에 합류(async 런타임 없음, 전반 blocking 설계 정합). `FilesystemWatcher`가 반환하는 자체 `TriggerStream`(mpsc rx)을 forwarder 스레드가 공용 채널로 재전송; timer 스레드가 `ReconciliationScheduler::tick` 구동 + `RunStateController.sync_requested` 폴링해 트리거 발행; 소비자 루프 1개가 직렬 실행 | 예 | MVP GUIDANCE "trigger sources run on std::thread with a std::sync::mpsc channel ... no async runtime". U2 `start` 실제 반환형 `mpsc::Receiver<TriggerSignal>` |
| DEC-U8-02 | 단일-사이클 직렬화 | **단일 소비자 스레드 == 암묵적 사이클 잠금**(한 번에 1 사이클). `cycle_in_progress: AtomicBool`을 `scheduler.tick(now, busy)`에 전달해 recon 중복 억제; 트리거 폭주 시 사이클 시작 전 채널을 **drain-to-latest**로 합침(busy면 최신 트리거만 유효) | 예(coalescing 최소) | services.md `SyncCycleCoordinator` "단일-사이클 잠금으로 직렬화 — busy면 최신 트리거만 유효". Q2=B |
| DEC-U8-03 | `SingleInstanceLock` 구현 | **std `File::lock`/`try_lock` advisory 락**(외부 크레이트 0, 1.97.1 stable) on 락파일; 파일에 PID+owner 기록. 획득 실패(live holder) -> 파일에서 PID/owner 읽어 보고 후 **비정상 종료**; holder dead면 OS가 락 해제 -> `try_lock` 성공 = stale 회수(레코드 덮어쓰기). `LockGuard` drop 시 해제 | 예 | MVP GUIDANCE "SingleInstanceLock via std File::lock (stabilized, 1.97.1 — NO external crate) on a lockfile storing PID/owner". FR-23. MSRV 1.85 대비 실 툴체인 1.97.1 사용은 code-gen 인라인 확인 |
| DEC-U8-04 | 공유 `SyncStateStore` 소유권 | **`Arc<Mutex<SyncStateStore>>`** + U8 얇은 어댑터 `SharedSyncStore`(impl `SyncStore`, 락 뒤 위임)를 `UploadProtocolDriver`에 주입. 코디네이터는 동일 `Arc`를 보유해 `last_committed_manifest`(diff/guard) + `mark_dirty`를 직접 호출. 직렬 사이클이라 경합 없음 | 예 | U3 `driver::new`가 `Box<dyn SyncStore>` 소유. `impl SyncStore for SyncStateStore`는 소유 값 기준 -> 코디네이터 공동 접근 위해 공유 핸들 필요(SyncStore=`Send+Sync` 이므로 `Arc<Mutex<>>`). 어댑터는 wiring 전용 |
| DEC-U8-05 | `BlobSource` 실체 | U8 `VaultBlobSource`(볼트 루트 `VaultScanner` 래핑, `open(path) -> VaultScanner::open_reader(path)`, `ScanError -> BlobSourceError` 매핑)를 드라이버에 주입. 스캔용 `VaultScanner`(ManifestBuilder)와 blob용 `VaultScanner`는 **별도 인스턴스**(둘 다 동일 root, `new(root)` 저비용) | 예 | U3 `BlobSource` 트레이트만 정의(실체 없음). U1 `VaultScanner::open_reader` 실제 API로 충족 — 신규 파일 I/O 재작성 0 |
| DEC-U8-06 | federated raw-parse + typed 주입 | WatcherDaemon이 원본 config 파일을 **raw JSON(`serde_json::Value`)으로 병렬 파싱**해 비-core 키를 TYPED로 해소: `request_timeout_s` -> U5 `validate_request_timeout_s` -> `Duration`; obs 키 -> `ObservabilityConfig`/`LoggerConfig`(부재 경로는 플랫폼 기본으로 해소한 `PathBuf`); `chunk_threshold_bytes`/`chunk_size_bytes` -> U3 `u64`; `t_debounce`/`t_recon` -> `Duration`(U2); update 채널/gate -> U7a; data-dir/socket 경로 -> U8. `ConfigProvider`는 core 6필드만 노출하므로 이 값들은 U8이 파싱·주입 | 예(일부 키 기본값) | task "federated-config = U8 parses raw config + injects typed values". U5 lib.rs 주석 "request_timeout_s 값은 U8 조립루트가 파싱해 Duration 으로 하향 주입". U6 `ObservabilityConfig` 주석 "U8 이 플랫폼 기본 경로로 해소한 뒤 PathBuf 로 주입" |
| DEC-U8-07 | `known_keys` union | `ConfigProvider::new(known_keys)`에 **`FOUNDATION_CONFIG_KEYS ∪ AUTH_CONSENT_CONFIG_KEYS ∪ OBSERVABILITY_CONFIG_KEYS ∪ U8-소유 키`** 를 전달(미지-키 수용 허용 목록). U8-소유 키 = data_dir·socket_path·t_debounce_s·t_recon_s·chunk_threshold_bytes·chunk_size_bytes·update_channel 등 | 아니오 | U5 lib.rs "watcher-bin(U8)이 FOUNDATION_CONFIG_KEYS + 이 상수 + 타 단위 키를 union 으로 집계해 ConfigProvider::new 에 주입" |
| DEC-U8-08 | 하향 주입 그래프(acyclic) | U6 싱크를 **먼저** 구축(`StructuredLogger`/`StatusService`/`UploadHistoryStore`/`TrayIndicator` no-op/`CriticalErrorNotifier`) -> `Arc<dyn Logger/StatusSink/HistorySink/CriticalEventSink/ReadJudgment>` 핸들 확보 -> U1/U4/U5/U3/U2/U7a/U7b에 U0 트레이트 타입으로 주입. 어떤 하위도 U8/U6 구체를 역참조하지 않음 | 아니오 | `unit-of-work.md` §4.2 하향 주입. services.md 4단계 "(§0 수정1)" 5개 U3/U5->U6 엣지 파운데이션-계약화 |
| DEC-U8-09 | main.rs 진입 디스패치 | **얇은 main.rs**: 예약어 첫 인자 `run`(또는 서비스-구동 진입) -> `WatcherDaemon::run(config_path)`; 그 외 operator 서브커맨드(status/health/pause/.../install/uninstall) -> `OperatorCli::run(&args, &IpcControlClient, &NativeServiceOps)`. `ServiceManager`가 install 시 `run` 인자로 데몬을 등록 | 예(부분) | task "main.rs is a THIN entry that builds + runs WatcherDaemon". U7b `Command` enum에 daemon `run` 부재 -> U8이 `run` 예약 추가 |
| DEC-U8-10 | graceful 종료 | **`AtomicBool` shutdown 플래그** + 신호 핸들러(SIGTERM/SIGINT)가 set; `RunStateController.stop_requested`도 병합. 종료 시: watcher 정지(drop) -> timer 스레드 정지(플래그) -> in-flight 사이클 완주(drain) 또는 dirty/재개 오프셋 보존 -> `ControlPlane` 종료 -> `LockGuard` drop. 신호 크레이트(ctrlc/self-pipe)는 **code-gen 인라인 확정**, FD는 기술중립 | 예 | MVP GUIDANCE "graceful shutdown via an AtomicBool shutdown flag set by a signal handler ... prefer a tiny one like ctrlc, or a minimal self-pipe". services.md 8단계 |
| DEC-U8-11 | AutoUpdater 스케줄링 | **AutoUpdater MVP 미배선(FIX3)**: `AutoUpdater::new`는 `Box<dyn UpdateSource>` seam을 요구하나 기본 `UpdateSource`를 구성하지 않는다 -> 데몬은 auto-update 백그라운드 루프를 배선하지 않는다(auto-update opt-in / post-MVP). `ServiceManager`(install/autostart) + `Uninstaller`는 CLI install/uninstall 경로로 계속 노출된다(데몬 run 루프 밖). 활성 롤아웃 정책은 post-MVP | 예 | `AutoUpdater::new`가 `Box<dyn UpdateSource>` 주입을 요구(U7a) — MVP 기본 소스 부재. MVP GUIDANCE "AutoUpdater loop scheduling minimal" 을 미배선으로 트림. `update_probe` 소비자(AutoUpdater) 부재이나 메서드 유지(§0 노트3) |
| DEC-U8-12 | ControlPlane 서버 배선 | WatcherDaemon이 동결 `WatcherHandlers::new(status, history, consent, config, run_state)`를 U8 고유 `DaemonControlHandlers`(domain-entities §5.3)로 감싸 `ControlPlane::new(Arc<DaemonControlHandlers>)` 구성, `bind_native`로 소켓 바인딩, `serve_forever`를 전용 스레드에 spawn. **sync-now는 handler가 `CycleTrigger::SyncNow`를 U8 trigger 채널로 직접 push하는 엣지 이벤트(FIX1)** — `RunStateController.request_sync_now` 폴링 아님 | 아니오 | `ControlPlane<H: ControlHandlers>`가 제네릭(`new(Arc<H>)`, control_plane.rs) -> U8 고유 핸들러 주입 가능. `RunStateController`에 `sync_requested` clear/take API 부재(run_state.rs `request_sync_now`만) -> 폴링 시 폭주/fire-once. services.md 6단계 |
| DEC-U8-13 | 상태 집계 + liveness | 상태는 `StatusService`(단일 집계 지점)에서 읽음(`snapshot`/`health_check`). WatcherDaemon이 주기 heartbeat로 `set_liveness(Alive)` 갱신(순수 liveness `update_probe`용), 운영 헬스는 `health_check` — 둘 분리(Q9=B, 롤백 루프 방지) | 예(부분) | services.md 7단계 "update_probe()는 health_check()와 분리 — AutoUpdater는 update_probe()만 소비". U6 `StatusService` 실제 메서드 |
| DEC-U8-14 | 사이클 결과 표면화 | 코디네이터가 드라이버 `CycleReport`/`UploadError`를 U0 `CycleOutcome`로 매핑해 `CriticalEventSink::report_cycle_result`(연속 실패 카운트 -> FR-19 케이스2 에스컬레이션) + `UploadHistoryStore.append` push; 성공 시 `record_sync_success` + `set_dirty(false)` + `set_operational(Idle)`. **실패 백오프는 `UploadError` 변형별(FIX4)**: `Transport(TransportError)`만 `RetryBackoffController` 분류 -> 백오프 재예약(AuthFailed 클래스는 제외); `HashMismatch`/`OverLimit`/`Aborted`는 백오프 없이 사이클 종료(다음 트리거 재스냅샷) | 아니오 | services.md 11/12단계. `UploadError` 4변형 중 `Transport`만 `TransportError` 보유(upload-client/src/error.rs). U6 실제 API |
| DEC-U8-15 | consent 게이트 위치 | 코디네이터가 드라이버 실행 **직전** `ConsentGate::is_upload_permitted() -> ConsentDecision`; `Blocked(reason)`이면 해당 조건 `raise_condition` + 업로드 skip(감시·상태 유지), `Permitted`면 드라이버 실행 | 아니오 | services.md 8단계. U5 `is_upload_permitted` 실제 API |
| DEC-U8-16 | 시작 스캔 순서 | `run_startup_scan(now)` 트리거를 **가장 먼저** 채널에 enqueue한 뒤 `FilesystemWatcher::start`로 정상 감시 진입 — 라이브 감시 전에 전체 재조정 1회 보장 | 아니오 | services.md 5단계 "run_startup_scan 로 시작 스캔 사이클을 먼저 돌린 뒤 start". NFR-03 백스톱 |

> **핵심 finding(동결 U3 흡수)**: services.md 사이클 6·7·10·11단계(no-op digest 조기종료·preflight limits·have-want-transfer-재검증·`commit_manifest`)는 **동결 `UploadProtocolDriver` 내부**에서 이미 수행된다(src `driver::run` 확인). 따라서 코디네이터는 이를 **중복하지 않고** 드라이버에 위임하며, 코디네이터 고유 책임은 트리거 직렬화·paused-skip·vault-availability·매니페스트 재계산·guard_diff·consent 게이트·mark_dirty·결과 push로 확정한다(§business-logic-model §0).

---

## 4. MANDATORY 카테고리 N/A + 확장 컴플라이언스

### 4.1 MANDATORY 프로세스 항목
- **Welcome Message / Rule Loading**: 워크플로 시작 시 1회 처리 완료(재로딩 안 함).
- **Question Format**: AUTOPILOT 모드 — 사용자 질문 없음(게이트 waived). N/A.
- **Content Validation**: 적용 — 산출물 박스드로잉 0건, ASCII 화살표(`->`)만, 한국어 산문, 백틱 Rust 타입.

### 4.2 확장 컴플라이언스

| 확장 | 활성 | 이 단계 적용 | 계획/판정 |
|---|---|---|---|
| **Resiliency Baseline** | ON | 적용 | RESILIENCY-01: `watcher-bin`이 전 크레이트를 링크한 **단일 배포 바이너리**(§1, `unit-of-work.md` §3.3). 회복력 오케스트레이션: `SyncStateStore::open_and_recover` 크래시 복구 배선(NFR-03), 사이클 실패 시 `RetryBackoffController` 백오프 재예약 + dirty 유지(무손실 재스냅샷), graceful 종료가 in-flight 사이클 drain + `LockGuard` 정리, `SingleInstanceLock` stale 회수(FR-23). RPO/RTO·HA/DR 세부는 Infra/Ops 이월 -> 부분 N/A |
| **Property-Based Testing** | ON (Full) | **강제(PBT-01)** | 각 산출물에 "Testable Properties"(`PROP-U8-*`) + 제너레이터(임의 트리거 시퀀스, 임의 lock 상태 live/stale/free, 임의 config 키맵 raw-parse 라운드트립, in-memory fake `WatchBackend`/fake `ControlClient`/기록 fake sink) 명시. 미준수 시 blocking. 프레임워크(proptest)는 code-gen 이월(PBT-09) |
| **Security Baseline** | OFF | N/A | 미로딩·미강제. config 평문 토큰(RISK-01)·로컬 평문 산출물은 문서화된 수용 위험 |

---

## 5. Drop-list (이미 확정 — 재개봉 금지)

| 항목 | 확정 내용 | 출처 |
|---|---|---|
| RESILIENCY-01 단일 바이너리 | `watcher-bin`이 전 lib 크레이트 링크 -> 단일 실행파일 | requirements / `unit-of-work.md` §3.3 |
| Q2=B 트리거당 1 직렬 사이클 | 생산자/소비자·별도 배출 루프 없음 | services.md |
| FR-23 단일 인스턴스 | 활성 PID/owner 보고 + 비정상 종료; stale 회수 | stories / `component-methods` |
| 하향 주입 / 의존성 역전 | U8이 U0 `ConfigProvider` + U6 싱크 구축·하향 주입; 하위는 U0 트레이트만 의존 | `unit-of-work.md` §4.2 |
| federated-config | U8 raw-parse + typed 주입; U0(core 6필드)·동결 크레이트 편집 없음 | task / U5·U6 lib.rs 주석 |
| Q9=B 2축 상태 | 운영 라이프사이클 축 + 활성 조건 축; `update_probe` 분리 | requirements / services.md |
| consent 게이트 | 유효 동의 없으면 업로드/커밋 차단(감시·상태 유지) | US-E4-04/05/06 |
| 토큰 저장 / TLS | 1차=config 평문 토큰(+env), TLS-only | requirements §13 / NFR-06 |
| U0 CoreTypes/codec/traits | 값 타입·CBOR 코덱·싱크 트레이트·오류 분류 동결 | U0 Functional Design |
| W0..W4 크레이트 API 동결 | 9개 lib 크레이트 공개 API 고정 — 편집 금지, 소비만 | WAVE CONTEXT |

---

## 6. 다음 단계
U8 Functional Design 산출물 3종 생성 완료 -> **U8 Code Generation**(NFR Requirements/Design 단계는 사용자 지시로 SKIP — 이 FD가 code-gen 직전 유일 설계 산출물) -> 이후 **Build-and-Test**(전 단위 조립 후). code-gen 인라인 확정: proptest 프레임워크 배선, 신호 크레이트(ctrlc vs self-pipe), `serde_json` raw-parse 세부, timer 스레드 tick 그래뉼래리티, `File::lock` MSRV 확인, Windows named-pipe/서비스 스텁 경로.

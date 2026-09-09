# U8 Orchestration — Domain Entities (도메인 엔티티 / 값 타입 / 배선 그래프)

**단계**: CONSTRUCTION -> U8 Orchestration -> Functional Design (산출물 1/3)
**작성일**: 2026-09-08 · **크레이트**: `watcher-bin` (binary)
**범위**: 조립 루트 U8이 **도입/특화**하는 값 타입·어댑터·배선 그래프만 정의한다. U0..U7b 타입은 재정의하지 않고 **이름 참조만** 한다(§0). 시그니처는 개념 형상(참고용)이며 기술중립이다.

> 규약: 한국어 산문, ASCII 화살표(`A -> B`)만, 박스드로잉/유니코드 화살표 금지, Rust 타입/식별자는 백틱.

---

## 0. 소비하는 타입 (재정의 금지 — 이름 참조만)

| 원 소유 | 타입/생성자 | U8 사용처 |
|---|---|---|
| U0 `foundation` | `ConfigProvider`, `WatcherConfig`, `ConfigSnapshot`, `FOUNDATION_CONFIG_KEYS`, `Manifest`, `ChangeSet`, `CycleOutcome`, `Version`, `Timestamp`, `OperationalState`, `ActiveCondition`, `LivenessSignal`, `RelativePath`, 트레이트 `Logger`/`StatusSink`/`HistorySink`/`CriticalEventSink`/`ReadJudgment`/`ConfigReloadObserver` | config 로드·주입, 사이클 값, 싱크 핸들 타입 |
| U1 `content-core` | `VaultScanner`, `ManifestBuilder`, `ManifestDiffer`, `SafetyLimitsValidator` | 매니페스트 재계산·diff |
| U2 `change-detect` | `TriggerSignal`, `TriggerKind`, `TriggerStream`(`=mpsc::Receiver<TriggerSignal>`), `FilesystemWatcher`, `NotifyBackend`, `ReconciliationScheduler`, `VaultAvailabilityGuard`, `Availability`, `GuardVerdict` | 트리거 소스·가드 판정 |
| U3 `upload-client` | `UploadProtocolDriver`, `BlobSource`, `SyncStore`, `CycleReport`, `UploadError`, `CommitOutcome` | 사이클 실행·seam 구현 |
| U4 `sync-state` | `SyncStateStore`, `StateConfig`, `RetryBackoffController`, `BackoffConfig` | 상태 복구·재시도 |
| U5 `auth-consent` | `AuthTransport`, `CredentialProvider`, `UreqAdapter`, `ConsentGate`, `ConsentDecision`, `BlockReason`, `ConfigSource`, `validate_request_timeout_s`, `AUTH_CONSENT_CONFIG_KEYS` | 전송·자격·동의 |
| U6 `observability` | `StructuredLogger`, `StatusService`, `UploadHistoryStore`, `CriticalErrorNotifier`, `TrayIndicator`, `ObservabilityConfig`, `LoggerConfig`, `OBSERVABILITY_CONFIG_KEYS`, `SystemClock` | 싱크 구축 |
| U7a `lifecycle-deploy` | `ServiceManager`, `native_controller`, `AutoUpdater`, `UpdateChannel`, `GateConfig`, `Uninstaller` | 배포/업데이트 |
| U7b `ops-control` | `RunStateController`, `RunState`, `StopMode`, `WatcherHandlers`, `ControlPlane`, `bind_native`, `OperatorCli`, `CliArgs`, `IpcControlClient`, `native_connector`, `NativeServiceOps` | 운영 제어 배선 |

> **이름 충돌 주의**: `CycleOutcome`은 U0(권위, 사이클 결과)와 U2(scheduler recon 결과)에 동명 존재; `HoldReason`은 U2 `guard`와 U7a `update`에 동명 존재. `watcher-bin`에서는 크레이트 경로를 명시해 소비한다(`foundation::CycleOutcome` 등).

---

## 1. 트리거 합류 모델 (SyncCycleCoordinator 입력, U8 소유)

세 트리거 소스를 하나의 직렬 스트림으로 합류시키기 위한 U8 값 타입(DEC-U8-01/02).

### 1.1 `CycleTrigger` (합류된 트리거 원인)

```rust
enum CycleTrigger {
    Filesystem(TriggerSignal),   // U2 FilesystemWatcher 디바운스 버스트 (mpsc 재전송)
    Reconcile(TriggerSignal),    // U2 ReconciliationScheduler startup/periodic tick
    SyncNow,                     // ControlPlane sync-now — U8 DaemonControlHandlers.sync_now가 채널에 직접 push(엣지 이벤트, run_state 폴링 아님)
}
```
- 원 `TriggerSignal.cause_summary`/`kind`/`observed_at`을 보존한다(US-E1-01 트리거 로그 원인 요약에 사용). `SyncNow`는 U2 타입이 아니므로 별도 변형으로 표현한다.

### 1.2 합류 채널(개념)

- **단일 `mpsc::channel::<CycleTrigger>()`**: `trigger_tx`(생산자) / `trigger_rx`(코디네이터 소비자, 유일). 소비자 1개 == 암묵적 단일-사이클 잠금(DEC-U8-02).
- 생산자: (a) forwarder 스레드가 `FilesystemWatcher`의 `TriggerStream`(rx)을 drain해 `Filesystem`으로 재전송, (b) timer 스레드가 `scheduler.tick`/`run_startup_scan` 결과를 `Reconcile`로 전송, (c) `ControlPlane` serve 스레드의 `DaemonControlHandlers.sync_now`가 `SyncNow`를 `trigger_tx`로 직접 push(엣지 이벤트, FIX1 — `run_state.sync_requested` 폴링 제거). 생산자 측 sender는 `ControlHandlers: Send + Sync` 계약을 만족하도록 `Sync`한 형태로 보유한다(std `Sender`는 `!Sync`이므로 `Mutex<Sender>` 또는 `SyncSender`, code-gen 확정).
- `cycle_in_progress: AtomicBool` — timer가 `tick(now, busy)`에 넘길 busy 신호(recon 중복 억제).

---

## 2. Shutdown 신호 (WatcherDaemon 소유)

```rust
struct ShutdownFlag(Arc<AtomicBool>);   // 신호 핸들러가 set, 루프들이 관측
```
- 신호(SIGTERM/SIGINT) 핸들러 또는 `RunStateController.stop_requested`가 `true`로 세팅(DEC-U8-10). 코디네이터 소비 루프·forwarder·timer·`ControlPlane` 스레드가 각 반복에서 관측해 정지한다. 신호 크레이트(ctrlc/self-pipe)는 code-gen 인라인 확정 — 이 타입은 메커니즘 중립이다.

---

## 3. SingleInstanceLock 타입 (U8 소유, FR-23)

### 3.1 `LockRecord` (락파일 내용 — PID/owner)

```rust
struct LockRecord { pid: u32, owner: String, started_at: Timestamp }
struct InstanceInfo { pid: u32, owner: String }   // 보고용 조회 결과
```
- 락파일(data-dir 하위)에 직렬화 저장한다. `owner`는 OS 사용자명. FR-23 보고 문구는 이 레코드에서 렌더한다.

### 3.2 `LockGuard` / `LockError`

```rust
struct LockGuard { /* 열린 락파일 핸들 + advisory lock 보유 */ }   // Drop 시 락 해제
enum   LockError {
    AlreadyRunning { pid: u32, owner: String },   // live holder -> 비정상 종료
    Io(/* source */),
}
// acquire(cfg: &LockConfig) -> Result<LockGuard, LockError>
// holder_info(cfg: &LockConfig) -> Option<InstanceInfo>
```
- 구현(DEC-U8-03): std `File::lock`/`try_lock` advisory 락 + PID/owner 기록. `try_lock` 실패 = live holder -> 레코드 읽어 `AlreadyRunning`. 성공 = free 또는 **stale 자동 회수**(dead holder면 OS가 락 해제한 상태) -> 레코드 덮어쓰기. `LockGuard` drop이 유일한 해제 경로.

---

## 4. 사이클 컨텍스트 / 결과 (SyncCycleCoordinator 소유)

### 4.1 `CycleContext`

```rust
struct CycleContext {
    trigger: CycleTrigger,
    cycle_id: CycleId,           // U0 상관관계 id (로그/히스토리 stamping)
    started_at: Timestamp,
}
```

### 4.2 `CoordinatorOutcome` (사이클 종결 사유 — U8 내부 분류)

```rust
enum CoordinatorOutcome {
    SkippedPaused,               // R-U8-06 run-state paused
    HeldVaultUnavailable,        // R-U8-07 guard check_reachable != Reachable
    HeldDestructiveEmpty,        // R-U8-07 guard_diff HoldDestructiveEmpty
    NoOp,                        // diff 비었음 (idle)
    BlockedConsent(BlockReason), // R-U8-08 consent Blocked
    Uploaded(CommitOutcome),     // 드라이버 성공
    Failed(UploadError),         // 드라이버 실패 -> Transport 변형만 백오프 재예약; 그 외(HashMismatch/OverLimit/Aborted) 사이클 종료(R-U8-09, FIX4)
}
```
- 이 enum은 U6 push 대상(`report_cycle_result`용 U0 `CycleOutcome` 매핑, status 조건, 히스토리 append)을 결정하는 U8 내부 분기 타입이다. 드라이버가 소유하는 결과(`CommitOutcome`/`UploadError`)는 그대로 승계한다.

---

## 5. U8 소유 seam 어댑터 (wiring 전용, 신규 로직 아님)

### 5.1 `SharedSyncStore` (DEC-U8-04)

```rust
struct SharedSyncStore(Arc<Mutex<SyncStateStore>>);
impl SyncStore for SharedSyncStore { /* 락 뒤에서 U4 SyncStateStore 메서드로 위임 */ }
```
- `UploadProtocolDriver::new`가 요구하는 `Box<dyn SyncStore>`를 공유 핸들로 충족한다. 코디네이터는 동일 `Arc<Mutex<SyncStateStore>>`를 보유해 `last_committed_manifest`(guard/diff)·`mark_dirty`를 직접 호출한다. 직렬 사이클이라 락 경합 없음.

### 5.2 `VaultBlobSource` (DEC-U8-05)

```rust
struct VaultBlobSource(VaultScanner);   // 볼트 루트로 생성
impl BlobSource for VaultBlobSource {
    fn open(&self, path: &RelativePath) -> Result<Box<dyn Read>, BlobSourceError>;
    // -> VaultScanner::open_reader(path), ScanError -> BlobSourceError 매핑
}
```
- U3가 `BlobSource` 트레이트만 정의하므로 U8이 U1 `VaultScanner::open_reader` 위에 얇게 구현한다(신규 파일 I/O 없음).

### 5.3 `DaemonControlHandlers` (DEC-U8-12, sync-now 엣지 + reload 체이닝 — FIX1/FIX2)

```rust
struct DaemonControlHandlers {
    inner: WatcherHandlers,                  // 동결 배선(status/health/history/pause/resume/stop/consent 위임)
    trigger_tx: Mutex<Sender<CycleTrigger>>, // sync-now 엣지 push (ControlHandlers:Send+Sync 위해 Sync한 형태)
    logger: Arc<StructuredLogger>,           // reload 시 레벨 재적용 체이닝
    config: Arc<ConfigProvider>,             // reload 시 config.reload()
}
impl ControlHandlers for DaemonControlHandlers {
    // status/health/history/pause/resume/stop/consent_view/grant/withdraw/acknowledge -> inner 위임
    fn sync_now(&self) { let _ = self.trigger_tx.lock()..send(CycleTrigger::SyncNow); }  // FIX1: run_state 폴링 대체
    fn reload(&self) -> Result<(), ControlError> {                                       // FIX2: 로거 레벨 재적용
        self.config.reload().map_err(..)?; let _ = self.logger.reload(); Ok(())
    }
}
```
- U8은 컴포지션 루트로서 `ControlPlane`의 핸들러를 소유한다. `ControlPlane<H: ControlHandlers>`가 제네릭(`new(Arc<H>)`, control_plane.rs)이라 동결 `WatcherHandlers` 대신 U8 고유 `DaemonControlHandlers`를 그대로 주입할 수 있다(신규 로직 아님, 위임+엣지 push+reload 체이닝만).
- **FIX1**: `sync_now`가 `CycleTrigger::SyncNow`를 `trigger_tx`로 직접 push(엣지 이벤트). 동결 `RunStateController`에 `sync_requested` clear/take API가 없어 폴링-소비가 불가능하기 때문(run_state.rs `request_sync_now`만 존재). `run_state.request_sync_now`는 U8이 호출하지 않는다.
- **FIX2**: `reload`가 `ConfigProvider::reload()` 성공 후 `StructuredLogger::reload()`를 체이닝해 캐시된 로그레벨을 재적용한다(logger.rs `reload`는 `provider.current()` 재조회). 로거를 `ConfigProvider::subscribe`로 관찰자 등록하지 않는다(참조 순환 -> 컴파일 불가, R-U8-04).
- `Sender`는 `Send`이나 `Sync`가 아닐 수 있어(std mpsc) `Mutex<Sender>` 또는 `SyncSender`로 감싸 `ControlHandlers: Send + Sync`를 만족시킨다(code-gen 확정, 직렬 사이클이라 경합 없음).

---

## 6. 데몬 배선 그래프 (WatcherDaemon 조립 — 실제 생성자명 참조)

구축 순서는 위상 정렬(하위 -> 상위 주입 대상). 화살표는 "주입/보유"를 뜻한다.

| 단계 | 구축 대상 | 실제 생성자(동결 API) | 주입 입력(출처) |
|---|---|---|---|
| B1 | `ConfigProvider` | `ConfigProvider::new(known_keys)` + `load(cli_path)` | known_keys union(DEC-U8-07); cli config 경로 |
| B2 | federated typed 값 | raw JSON 파싱(DEC-U8-06) | `request_timeout_s`->`Duration`, obs 경로->`PathBuf`, chunk/T_* 등 |
| B3 | `SingleInstanceLock` | `acquire(&LockConfig)` | data-dir 락파일 경로 |
| B4 | `SyncStateStore` | `open_and_recover(&StateConfig)` -> `Arc<Mutex<_>>` | data-dir 상태 경로 |
| B5 | U6 싱크(먼저) | `StatusService::new()`; `StructuredLogger::new(LoggerConfig, Arc<SystemClock>, Arc<ConfigProvider>)`; `UploadHistoryStore::new(history_path, Some(logger))`; `TrayIndicator::new()`; `CriticalErrorNotifier::new(logger, Arc<StatusService>, Arc<TrayIndicator>, threshold)` | LoggerConfig/ObservabilityConfig(B2); `notify_consecutive_failures`(core) |
| B6 | U5 | `CredentialProvider::with_defaults(Arc<ConfigProvider> as ConfigSource)`; `AuthTransport::new(cred, Arc<UreqAdapter>, config, timeout)`; `ConsentGate::open_with_system_clock(consent_path, Some(status_sink))` | timeout(B2); status_sink(B5) |
| B7 | U4 | `RetryBackoffController::new(&BackoffConfig, 0, seed)` | BackoffConfig(B2 기본) |
| B8 | U1 | `VaultScanner::new(root)`(스캔용 + blob용 2인스턴스); `ManifestBuilder`/`ManifestDiffer`/`SafetyLimitsValidator` | vault_root(core `vault_path`) |
| B9 | U2 | `VaultAvailabilityGuard::new(confirm_empty)`; `ReconciliationScheduler::new(t_recon)`; `FilesystemWatcher::start(root, &NotifyBackend, t_debounce)` | t_recon/t_debounce(B2) |
| B10 | U3 | `UploadProtocolDriver::new(AuthTransport(B6), Box<VaultBlobSource>, Box<SharedSyncStore(B4)>, status_sink(B5), chunk_threshold, chunk_size)` | chunk 값(B2) |
| B11 | U7a | **AutoUpdater MVP 미배선(FIX3)** — 데몬은 `AutoUpdater::new`(요구: `Box<dyn UpdateSource>` seam)를 구성하지 않는다(기본 `UpdateSource` 없음, auto-update opt-in/post-MVP). `ServiceManager::new(native_controller())`/`Uninstaller`는 CLI install/uninstall 경로(§main.rs)에서만 구성 — 데몬 run 그래프에는 U7a 노드 없음 | (없음) |
| B12 | U7b | `RunStateController::new(run_state_path, status_sink(B5))`; `WatcherHandlers::new(status, history, consent, config, run_state)`를 U8 고유 `DaemonControlHandlers`(§5.3, +`trigger_tx`/`logger`/`config`)로 감싸 `ControlPlane::new(Arc<DaemonControlHandlers>)`(제네릭 -> U8 핸들러 주입) + `bind_native(endpoint)` | socket endpoint(B2); trigger_tx(U8 합류 채널 §1.2); logger(B5) |
| B13 | `SyncCycleCoordinator` | U8 소유 — B4/B5/B8/B9/B10/B12 핸들 주입 | 위 전부 |

- 모든 화살표는 하위 -> 상위 방향(주입)이며 역엣지 0(acyclic, DEC-U8-08). U6 싱크(B5)가 U0 트레이트 객체로 하위(B6/B10/B11/B12)에 흐른다 -> 5개 U3/U5->U6 엣지가 파운데이션-계약 의존으로 성립.

---

## 7. 컴포넌트 -> 규칙 -> 속성 매핑 (추적성)

| 컴포넌트 | 소유 타입 | 규칙 | 속성 |
|---|---|---|---|
| `SingleInstanceLock` | `LockRecord`/`LockGuard`/`LockError`/`InstanceInfo` | R-U8-01/02 | PROP-U8-01 |
| `WatcherDaemon` | `ShutdownFlag`, `DaemonControlHandlers`(§5.3), 배선 그래프(§6) | R-U8-03/04/05/06/09 | PROP-U8-02/03/07 |
| `SyncCycleCoordinator` | `CycleTrigger`/`CycleContext`/`CoordinatorOutcome`, `SharedSyncStore`/`VaultBlobSource` | R-U8-06/07/08/10 | PROP-U8-04/05/06 |

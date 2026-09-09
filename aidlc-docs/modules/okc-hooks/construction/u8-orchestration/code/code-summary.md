# U8 orchestration — Code Summary (구현 요약)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U8 Orchestration** -> Code Generation
**크레이트**: `watcher-bin` (bin + lib) · **소속 컴포넌트**: `WatcherDaemon`, `SyncCycleCoordinator`, `SingleInstanceLock`
**근거 산출물**: FD 3종(`domain-entities.md` / `business-rules.md` / `business-logic-model.md`)
**역할**: 하위 9개 라이브러리 크레이트를 한 방향으로 조립하는 **단일 배포 바이너리**(RESILIENCY-01). 신규 도메인 로직 없이 조립·오케스트레이션만 소유한다.
**NFR 참고**: NFR 단계는 사용자 지시로 **SKIP** -> 코드는 FD + 하위 9개 크레이트(U0..U7b)의 실제 공개 API 에 직접 근거한다.

---

## 1. 구현된 컴포넌트 + 공개 API 표면

세 개의 응집 컴포넌트를 노출한다. 스레드/소켓/파일시스템/신호 I/O 는 `daemon` 에 격리하고, 사이클 판정
로직은 순수 함수(`decide_cycle_gate`/`coalesce_to_latest`/`wiring_is_acyclic`)로 분리해 스레드·네트워크
없이 결정적으로 property 검증한다. 코디네이터는 하위 크레이트 실체 타입이 아니라 6개 port 트레이트에만
의존한다(seam 격리, 하위->상위 단방향 주입).

- **`WatcherDaemon`**(모듈 `daemon`): 조립 루트. `run(Option<PathBuf>)` 이 기동 순서 S1~S9 를 고정 실행한다
  (R-U8-05): config 로드·federated 해소 -> `SingleInstanceLock::acquire` -> `SyncStateStore::open_and_recover`
  -> U6 싱크 선구축 -> 하위 단위 생성·하향 주입 -> `ControlPlane` 전용 스레드 배선 -> `SyncCycleCoordinator`
  조립 -> 시작 스캔 트리거 enqueue -> `FilesystemWatcher::start` + 디바운스 forwarder/주기 timer 스레드 ->
  소비자 루프(현 스레드) -> graceful 종료(감시 정지 -> 트리거 스레드 join -> 락 drop). SIGINT/SIGTERM 은
  `ctrlc` 로 `ShutdownFlag` 를 set 한다. 공개: `WatcherDaemon`/`DaemonError`.
- **`SyncCycleCoordinator`**(모듈 `coordinator`): 세 트리거 소스(U2 `FilesystemWatcher` 디바운스 / U2
  `ReconciliationScheduler` 백스톱 / U7b `ControlPlane` sync-now)를 단일 `mpsc` 스트림으로 받아 한 번에
  하나씩 처리한다 — 단일 소비자 == 암묵적 사이클 잠금. 각 사이클은 paused-skip -> vault 가용성 -> 매니페스트
  재계산 -> guard_diff -> diff -> consent 게이트 -> `UploadProtocolDriver` 위임 -> 결과 push 로 진행하고
  정확히 하나의 배타적 `CoordinatorOutcome` 으로 종결한다(R-U8-06/07/08/09). 공개: `SyncCycleCoordinator`/
  `CoordinatorPorts`/`CoordinatorSinks`/`CycleTrigger`/`CycleGate`/`CoordinatorOutcome`/`ShutdownFlag`/
  `WiringEdge`/`WIRING_EDGES`/`decide_cycle_gate`/`coalesce_to_latest`/`wiring_is_acyclic`.
- **`SingleInstanceLock`**(모듈 `instance_lock`): OS advisory 파일 잠금(std `File::try_lock`, MSRV 1.89 근거)
  기반 단일 인스턴스 보호. free/stale 파일은 현재 `pid`/`owner`/`started_at` 레코드로 덮고, live holder 는
  락파일 `LockRecord` 를 읽어 `AlreadyRunning{pid, owner}` 로 report-and-exit 한다(R-U8-01/02). `LockGuard`
  는 drop 시에만 락을 해제한다(graceful/비정상 종료 양쪽 정리). 공개: `SingleInstanceLock`/`LockConfig`/
  `LockRecord`/`LockGuard`/`LockError`/`InstanceInfo`.
- config seam(모듈 `config`): U0 `ConfigProvider`(core 6필드) 와 **병행**해 같은 JSON 의 비-core federated
  키를 각 하위 생성자 타입으로 raw-parse + typed 해소(R-U8-03). 공개: `load_runtime_config`/`known_config_keys`/
  `U8_CONFIG_KEYS`/`FederatedConfig`/`LoadedRuntimeConfig`/`RawFederatedConfig`(+`RawBackoffConfig`/
  `RawUpdateGateConfig`)/`UpdateRuntimeConfig`/`FederatedConfigError`/`RuntimeConfigError`.
- adapters seam(모듈 `adapters`): 코디네이터가 의존하는 6개 port 트레이트(`RunStatePort`/`AvailabilityGuard`/
  `ManifestSource`/`ConsentPort`/`CycleDriver`/`CoordinatorStore`) + 프로덕션 배선 3종(`SharedSyncStore`/
  `VaultBlobSource`/`VaultManifestSource`). 하위 동결 크레이트 실체 타입에 port 를 얇게 구현(로컬 트레이트,
  orphan-rule OK), property 는 동일 port 를 기록 fake 로 구현한다.
- control seam(모듈 `control`): `DaemonControlHandlers`(동결 U7b `WatcherHandlers` 위임 + sync-now/reload
  특화) + `Sync` 한 `TriggerSender`(std `mpsc::Sender` 를 `Mutex` 로 감싸 엣지 push).

## 2. 모듈 레이아웃

- `main.rs` — 바이너리 진입점. 첫 인자가 `run`(또는 없음)이면 `WatcherDaemon::run` 으로 데몬 전경 실행, 그 외
  서브커맨드는 U7b `dispatch_args` 로 IPC/서비스 조작 라우팅(DEC-U8-09). 조립 외 도메인 로직 없음.
- `lib.rs` — 6개 공개 모듈 재노출(예제/property 테스트가 동일 로직 재사용). `#![deny(missing_docs)]`.
- `config.rs` — federated raw JSON 투영(`RawFederatedConfig`) + per-field 검증 + 플랫폼별 기본 data-dir 해소.
- `instance_lock.rs` — advisory 락 + `LockGuard` 수명 + best-effort `holder_info`.
- `adapters.rs` — 6 port 트레이트 + 프로덕션 구현. panic-free lint-gate.
- `control.rs` — `DaemonControlHandlers`/`TriggerSender`. panic-free lint-gate.
- `coordinator.rs` — 사이클 루프 + 순수 판정 함수 + `WIRING_EDGES`. panic-free lint-gate.
- `daemon.rs` — 전 컴포넌트 조립 + 트리거 스레드 + 종료 수명주기. 조립/I-O 모듈이라 순수 lint-gate 는 없으나
  어떤 경로에서도 `unwrap`/`expect`/`panic` 을 쓰지 않는다.
- `proptest_support.rs` — U8 고유 제너레이터(`arb_cycle_trigger`/`arb_availability`/`arb_guard_verdict`/
  `arb_consent_decision`). 비기본 `proptest-support` feature 뒤에서만 노출(런타임 그래프 밖).

## 3. 의존성

**신규 외부 크레이트 1건: `ctrlc`**(SIGINT/SIGTERM -> `ShutdownFlag`). 하위 9개 라이브러리 전부에 의존하는
유일한 크레이트다: `foundation`(U0)/`content-core`(U1)/`change-detect`(U2)/`upload-client`(U3)/`sync-state`(U4)/
`auth-consent`(U5)/`observability`(U6)/`lifecycle-deploy`(U7a)/`ops-control`(U7b). 모든 배선은 하위 -> 상위
방향이며 역엣지가 없다(비순환, `WIRING_EDGES` 35간선을 `wiring_is_acyclic` 로 검증). 하위 단위는
`watcher-bin`/`observability` 구체 타입을 역참조하지 않고 U0 계약 트레이트(`Logger`/`StatusSink`/`HistorySink`/
`CriticalEventSink`)로만 주입받는다(R-U8-04). CLI 파싱은 U7b 소유(std 손수 파서)라 **`clap` 미도입**.
`serde`/`serde_json`(federated raw-parse + 락 레코드), `thiserror`(오류 taxonomy), `proptest`(optional). `proptest`
는 비-default feature(`proptest-support` = `dep:proptest` + `foundation/proptest-support`) 게이트라 프로덕션
빌드 그래프에 유입되지 않는다(PBT-07).

## 4. MVP 트림 (FD 대비 편차 — 코드 주석에 명시)

- **AutoUpdater 실행 루프 이연**: `resolve_update` 가 `UpdateRuntimeConfig{channel, gate}` 를 해소해 설정 의미는
  보존하나, MVP 데몬은 `UpdateSource` 가 없어 U7a `AutoUpdater` 백그라운드 폴링을 배선하지 않는다(설정만 보존).
- **Windows 명명 파이프 이연**: `config` 는 windows 에서 `IpcEndpoint::NamedPipe` 로 해소하나 실제 IPC 검증
  경로는 Unix 도메인 소켓(U7b 소유 seam 뒤 스텁). non-unix arm 은 컴파일만 확인(검증은 macOS aarch64).
- **제어면 bind 실패 = 계속 진행**: `ControlPlane` 은 non-`Send` 리스너라 전용 스레드 안에서 bind + serve 한다.
  bind 실패는 데몬을 중단시키지 않고 `control_plane.bind_failed` 로그만 남긴 뒤 동기화를 계속한다(제어 소켓 부재가
  동기화를 막지 않음, RESILIENCY).
- **destructive-empty hold 의 status 조건**: 동결 U0 `ActiveCondition` 에 별도 변형이 없어, `raise_hold` 가
  vault-unavailable 계열과 destructive-empty 보류를 모두 `ActiveCondition::VaultUnavailable` 로 표면화한다
  (로그 이벤트명은 `cycle.hold.destructive_empty` 로 구분).
- **sync-now = 엣지 이벤트(FIX1)**: 동결 `RunStateController` 에 `sync_requested` clear/take API 가 없어 폴링은
  폭주/fire-once 가 되므로, `DaemonControlHandlers.sync_now` 가 `CycleTrigger::SyncNow` 를 합류 채널에 직접 push.
- **reload = 명시 체이닝(FIX2)**: 로거를 `ConfigProvider::subscribe` 관찰자로 등록하면 참조 순환(컴파일 불가)이라,
  `reload` 가 `ConfigProvider::reload()` 성공 후 `StructuredLogger::reload()` 를 체이닝해 캐시 로그레벨을 재적용.
- `TrayIndicator` 는 no-op 으로 구축해 `CriticalErrorNotifier` 에 주입(headless MVP, 로그+헬스+CLI status 로 표면화).
- scan/hash 재계산 실패(`ManifestSource::rebuild` Err)는 `HeldVaultUnavailable` 로 흡수(별도 outcome 미도입).

## 5. 테스트 커버리지 (총 24개 통과, 0 실패)

- **예제/단위 테스트 15개**(기본 `cargo test`):
  - `tests/coordinator_cycle.rs`(7): 사이클 배타 종결 — paused-skip / vault-unreachable hold(+조건 raise) /
    destructive-empty hold / 빈 diff no-op / consent 차단 / permitted 시 드라이버 1회+성공 표면화 / 드라이버 실패 표면화.
  - `tests/config_keys.rs`(4): known-key union(U0/U5/U6/U8 중복 없는 합집합) / 기본 data-dir 관례 경로 파생 /
    비양수 `request_timeout_s` 거부(U5 규칙) / raw federated JSON round-trip.
  - `tests/instance_lock.rs`(4): free 획득+pid 기록 / live holder 배제 후 drop 재획득 / stale 파일 회수 / `holder_info`.
- **property 테스트 9개**(`proptest-support` 게이트, `tests/property_tests.rs`):
  PROP-U8-01(단일 인스턴스 상호배제), PROP-U8-03(실 배선 비순환 + 역엣지 주입 검출 — 탐지기 건전성 2건),
  PROP-U8-04(배타적 게이트 총-함수 + permitted 시 드라이버 정확히 1회 2건), PROP-U8-05(drain-to-latest 합류),
  PROP-U8-06(hold 시 조건 1회 raise + 성공 시 `record_sync_success` 1회 2건), PROP-U8-07(known-key union +
  federated round-trip + timeout 규칙). 스레드/네트워크/파일시스템 없이 in-memory 기록 fake seam + 순수 판정으로 검증.
- **PROP-U8-02 정직한 상태**: FD §5 의 graceful-shutdown drain 무손상 property 는 실 스레드 종료 시퀀스를 요구해
  전용 in-memory proptest 로 분리하지 못했다. `daemon.rs` S9 종료 순서(감시 정지 -> 트리거 스레드 join -> 락 drop)와
  `run_loop` 의 종료-중 신규-트리거-미수용 로직으로 설계 수준에서 성립하나, 독립 자동 검증은 **이연** 항목이다.

## 6. 검증 사실 (확인됨)

- `cargo build --workspace` 성공 — 10개 크레이트 전부 컴파일.
- watcher-bin 테스트 24개(예제 15 + property 9) 전부 통과, 0 실패
  (`cargo test -p watcher-bin --features proptest-support`).
- `cargo clippy --workspace --all-targets --features proptest-support -- -D warnings` CLEAN(0 error).
- 툴체인: cargo/rustc 1.97.1, edition 2024, 워크스페이스 MSRV 1.89(`File::try_lock` 안정화 근거).

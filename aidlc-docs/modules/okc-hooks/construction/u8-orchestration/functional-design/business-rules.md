# U8 Orchestration — Business Rules (결정 규칙 / 조립 제약 / 오케스트레이션 순서)

**단계**: CONSTRUCTION -> U8 Orchestration -> Functional Design (산출물 2/3)
**작성일**: 2026-09-08 · **크레이트**: `watcher-bin` (binary)
**범위**: 조립 루트 U8의 `R-U8-*` 규칙(단일 인스턴스·federated 주입·하향 주입·직렬 사이클·paused-skip·vault-hold·consent·graceful 종료) + `PROP-U8-*` Testable Properties.

> 규약: 한국어 산문, ASCII 화살표(`A -> B`)만, 박스드로잉/유니코드 화살표 금지, Rust 타입/식별자는 백틱.

---

## 1. SingleInstanceLock 규칙 (FR-23)

**규칙 R-U8-01 (단일 인스턴스 획득 — report-and-exit)**: `WatcherDaemon`은 기동 2단계에서 `SingleInstanceLock::acquire(&LockConfig)`를 호출한다(services.md 2단계). advisory 락(std `File::lock`/`try_lock`, DEC-U8-03)이 **live holder에 의해 점유**되어 있으면, 락파일 `LockRecord`에서 활성 `pid`/`owner`를 읽어 `LockError::AlreadyRunning{pid, owner}`로 반환하고, 데몬은 그 정보를 명확한 로그/CLI 오류로 보고한 뒤 **비정상 종료(non-zero exit)** 한다. 동시에 두 인스턴스가 사이클을 돌지 못한다(FR-23, PROP-U8-01).

**규칙 R-U8-02 (stale 락 자동 회수)**: 이전 인스턴스가 비정상 종료해 프로세스는 죽었으나 락파일이 잔존하는 경우, advisory 락은 OS가 프로세스 종료 시 해제하므로 `try_lock`이 **성공**한다 — 이때 잔존 `LockRecord`를 현재 `pid`/`owner`/`started_at`으로 덮어쓰고 진행한다(services.md 2단계 "stale이면 회수 후 진행"). 회수는 볼트나 다른 산출물을 건드리지 않는다. 획득한 `LockGuard`는 **drop 시에만** 락을 해제한다(graceful/비정상 종료 양쪽에서 정리, PROP-U8-01).

---

## 2. WatcherDaemon 조립/기동 규칙

**규칙 R-U8-03 (federated raw-parse + typed 주입)**: `WatcherDaemon`은 U0 `ConfigProvider::load`로 core 6필드(`FOUNDATION_CONFIG_KEYS`)를 검증·노출받는 것과 **병행하여**, 원본 config 파일을 raw(`serde_json::Value`)로 파싱해 비-core(federated) 키를 TYPED 값으로 해소한다(DEC-U8-06): `request_timeout_s` -> U5 `validate_request_timeout_s(i64) -> Duration`(검증 규칙은 U5 소유); obs 키(`log_file`/`history_file`/... ) -> `ObservabilityConfig`/`LoggerConfig`(부재 경로는 플랫폼 기본으로 해소한 `PathBuf`); `chunk_threshold_bytes`/`chunk_size_bytes` -> `u64`(U3); `t_debounce_s`/`t_recon_s` -> `Duration`(U2); update 채널/gate -> U7a; `data_dir`/`socket_path` -> U8. 해소한 값은 각 단위 생성자에 하향 주입하며, `ConfigProvider`(core만 노출)나 동결 크레이트를 편집하지 않는다. `ConfigProvider::new(known_keys)`의 `known_keys`는 `FOUNDATION_CONFIG_KEYS ∪ AUTH_CONSENT_CONFIG_KEYS ∪ OBSERVABILITY_CONFIG_KEYS ∪ U8-소유 키`의 union이다(DEC-U8-07; 미지-키 수용 허용 목록, PROP-U8-07).

**규칙 R-U8-04 (하향 주입 acyclic 배선)**: 컴포넌트 구축은 하위 -> 상위 위상 순서(domain-entities §6)로 진행하며, **U6 싱크를 먼저** 구축해 `Arc<dyn Logger>`/`Arc<dyn StatusSink>`/`Arc<dyn HistorySink>`/`Arc<dyn CriticalEventSink>`/`Arc<dyn ReadJudgment>` 핸들을 확보한 뒤 U1/U4/U5/U3/U2/U7a/U7b에 **U0 계약 트레이트 타입으로** 주입한다(DEC-U8-08). 어떤 하위 단위도 `watcher-bin`/`observability` 구체를 역참조하지 않으며, 배선 그래프에 역엣지가 없다(비순환, PROP-U8-03). `TrayIndicator`는 no-op으로 구축해 `CriticalErrorNotifier`에 주입하므로 트레이 부재에도 로그+헬스+CLI status 표면화가 성립한다(§0 수정2, US-E5-04). **로그레벨 live-reload는 `ConfigProvider::subscribe`로 로거를 관찰자 등록하지 않는다(FIX2)**: `subscribe(&mut self, Arc<dyn ConfigReloadObserver>)`(store.rs/observer.rs)는 가변 참조를 요구하는데, `StructuredLogger`가 생성자에서 `Arc<ConfigProvider>`를 보유하므로(logger.rs `new(.., Arc<ConfigProvider>)`) 로거를 관찰자로 등록하면 provider->logger->provider 참조 순환이 되어 `Arc::get_mut`이 `None`을 반환, 컴파일 불가다. 또한 `StructuredLogger::emit`은 캐시된 `LoggerState.level`을 읽고(매 방출 lazy `current()` 재조회가 아님, logger.rs 확인) 이 캐시는 `reload()` 호출 시에만 `provider.current()`로 갱신된다. 따라서 U8은 로거를 관찰자로 등록하는 대신, 고유 `DaemonControlHandlers::reload`(§domain-entities §5.3)가 `ConfigProvider::reload()` 성공 후 `StructuredLogger::reload()`를 명시적으로 체이닝해 캐시 레벨을 재적용한다(R-LOG-06 충족, 관찰자 등록 0). `Arc<ConfigProvider>`를 보유하지 않는 다른 관찰자만 순환 없이 등록 가능하나 본 설계에는 그런 관찰자가 없다.

**규칙 R-U8-05 (기동 오케스트레이션 순서)**: 기동은 `config 로드(+federated 해소) -> SingleInstanceLock::acquire -> SyncStateStore::open_and_recover(NFR-03 부분쓰기 롤백/복구) -> 전 컴포넌트 생성·주입 -> run_startup_scan 트리거 enqueue -> FilesystemWatcher::start -> 트리거 소스 배선 -> ControlPlane serve 스레드 spawn` 순으로 **고정**된다(services.md 1~6단계, DEC-U8-16). 시작 스캔은 라이브 감시 진입 **전에** enqueue되어 첫 사이클이 전체 재조정을 수행한다(NFR-03 백스톱). config 무효 시 acquire 이전에 명확한 오류로 종료한다.

---

## 3. SyncCycleCoordinator 규칙 (Q2=B 단일 직렬 사이클)

> **동결 U3 흡수**: services.md 사이클 6·7·10·11단계(no-op digest 조기종료·preflight `SafetyLimits`·have/want·전송·전송 직전 재-읽기+재-해시 재검증·`commit_manifest`)는 동결 `UploadProtocolDriver::run` 내부에서 수행된다(src 확인). 코디네이터는 이를 **중복하지 않고** 위임한다. 아래 규칙은 코디네이터 고유 책임만 규정한다.

**규칙 R-U8-06 (트리거당 직렬 사이클 + paused-skip)**: 코디네이터 소비 루프는 합류 채널(`mpsc::Receiver<CycleTrigger>`)에서 트리거를 **한 번에 하나** 수신해 사이클을 처음부터 끝까지 실행한다 — 단일 소비자 == 암묵적 단일-사이클 잠금(DEC-U8-02, PROP-U8-05). 각 사이클 시작 시 `cycle_in_progress`를 set하고 채널을 **drain-to-latest**로 합쳐 폭주를 흡수한다(busy면 최신만 유효). 사이클 진입 직후 `RunStateController::current()`를 읽어 `mode == Paused`이면 사이클을 `SkippedPaused`로 조기 종료한다(감시·상태는 유지, US-E6-03). 코디네이터는 `current()`를 **읽기 전용**으로만 소비하며 `RunStateController`를 역참조 호출하지 않는다(no back-reference). **sync-now는 `sync_requested` 폴링이 아니라 채널 엣지 이벤트로 전달된다(FIX1)**: 동결 `RunStateController`는 `sync_requested`를 clear/take하는 API가 없어(`new`=false / `request_sync_now`=true / `current`=clone만, run_state.rs) 폴링-소비 설계는 사이클 폭주 또는 fire-once-ever가 되어 Q2=B를 위반한다. 대신 U8은 컴포지션 루트로서 `ControlPlane`의 핸들러를 소유하므로(`ControlPlane<H: ControlHandlers>` 제네릭 -> U8 고유 `DaemonControlHandlers` 주입, control_plane.rs `new(Arc<H>)`), 그 `sync_now` 분기가 `CycleTrigger::SyncNow`를 U8의 `trigger_tx`(mpsc)로 직접 push한다. `RunStateController`는 pause/resume/stop만의 권위 보유자로 남고 `sync_requested`/`request_sync_now`는 U8이 소비하지 않는다(동결·무해).

**규칙 R-U8-07 (vault-availability hold + push 소유)**: (a) `VaultAvailabilityGuard::check_reachable(root)`가 `Reachable`이 아니면(RootMissing/Unmounted/Inaccessible) vault-unavailable 조건을 `StatusService.raise_condition(ActiveCondition::VaultUnavailable)` + `StructuredLogger`로 push하고 사이클을 `HeldVaultUnavailable`로 보류한다(다음 사이클/백오프 예약). (b) `VaultScanner::scan` + `ManifestBuilder::build`로 현재 매니페스트를 재계산한 뒤 `VaultAvailabilityGuard::guard_diff(new, last_committed, availability)`가 `HoldDestructiveEmpty`이면 파괴적-빈-커밋을 보류하고 사유를 status/log로 push한다(US-E1-06). **US-E1-01/06의 실제 push는 코디네이터가 소유**한다 — U2 검사기는 판정만 반환한다(services.md 노트2, PROP-U8-06). `last_committed`는 공유 `SyncStateStore`에서 읽는다.

**규칙 R-U8-08 (no-op diff + consent 게이트)**: `ManifestDiffer::diff(last, current)`가 빈 `ChangeSet`이면 사이클을 `NoOp`로 종결하고 idle status를 push한다(불필요한 업로드 억제; 드라이버 digest no-op과 이중 방어). diff가 비어있지 않으면 드라이버 실행 **직전** `ConsentGate::is_upload_permitted() -> ConsentDecision`를 평가한다: `Blocked(reason)`이면 해당 조건을 `raise_condition`으로 push하고 업로드를 차단하되 감시·상태는 유지한다(`BlockedConsent`, US-E4-04/05/06); `Permitted`이면 `SyncStateStore::mark_dirty()`(커밋 전 유실 방지) 후 `UploadProtocolDriver::execute_cycle_traced(&current)`를 호출한다(PROP-U8-04).

**규칙 R-U8-09 (사이클 결과 표면화 + 백오프)**: 드라이버 결과를 U6 싱크로 push한다(services.md 11/12단계, DEC-U8-14): 성공(`CommitOutcome`) -> `StatusService.record_sync_success(now)` + `set_dirty(false)` + `set_operational(Idle)`, `UploadHistoryStore.append(success 레코드)`, `RetryBackoffController.on_success`(성공 리셋). 실패(`UploadError`) -> 공통으로 dirty 유지, offline 조건 push, `UploadHistoryStore.append(failure/partial)`, `CriticalEventSink.report_cycle_result(CycleOutcome)`(연속 실패 -> FR-19 케이스2 에스컬레이션 카운트)를 수행한 뒤, **백오프 재예약은 `UploadError` 변형별로만** 결정한다(FIX4, upload-client/src/error.rs 4개 변형): (a) `Transport(TransportError)` -> `RetryBackoffController`가 `TransportError`(유일한 분류 가능 값)를 분류해 재시도 가능 클래스면 다음 재시도 시각 계산 + **전체 사이클 백오프 재예약**(재예약 사이 편집은 다음 재스냅샷이 자동 흡수, 무손실 US-E3-04); `AuthFailed` 클래스면 재시도에서 제외해 U5 인증 흐름에 위임하고 백오프 없이 사이클 종료(U4 분류 승계); (b) `HashMismatch{..}`(TOCTOU) / `OverLimit(_)`(한도 초과 halt) / `Aborted`(I/O·프로토콜 본문 코덱 실패) -> `RetryBackoffController`의 분류 대상이 아니므로(비-`TransportError`) **백오프 재예약 없이 사이클을 종료**하며(로그만), 다음 사이클 트리거가 재스냅샷으로 자동 흡수한다. 결과는 `CoordinatorOutcome::Failed(UploadError)`로 분류한다.

---

## 4. 종료 규칙

**규칙 R-U8-10 (graceful 종료가 in-flight 사이클 drain)**: `ShutdownFlag`(신호 SIGTERM/SIGINT 또는 `RunStateController.stop_requested`)가 set되면(DEC-U8-10), 종료 순서는 `FilesystemWatcher 정지(drop -> 감시 스레드 종료) -> forwarder/timer 스레드 정지 -> 진행 중 사이클을 안전하게 완주(drain)하거나 SyncStateStore에 dirty/재개 오프셋으로 보존 -> ControlPlane serve 스레드 종료 -> LockGuard drop(락 해제)` 이다(services.md 8단계, PROP-U8-02). 종료 중 새 트리거는 수용하지 않으며, in-flight 사이클을 중도 절단해 상태를 손상시키지 않는다(NFR-03 회복력). `StopMode`(graceful vs 즉시)에 따라 drain 여부를 결정한다.

---

## 5. Testable Properties (PBT-01 강제, 확장 ON Full)

각 속성은 in-memory fake seam으로 네트워크/파일시스템/실제 스레드 없이 검증한다. 제너레이터·프레임워크(proptest)는 code-gen 이월(PBT-09).

| ID | 대상 | 속성 | 제너레이터 / fake | 근거 |
|---|---|---|---|---|
| **PROP-U8-01** | SingleInstanceLock | 임의의 lock 상태(free / stale-잔존레코드 / live-holder)에 대해: free/stale -> `acquire`가 `Ok(LockGuard)`이고 레코드가 현재 pid로 갱신됨; live-holder -> `Err(AlreadyRunning{pid, owner})`이고 두 번째 `acquire`는 성공하지 못함. `LockGuard` drop 후 재획득 성공(정리 보장) | temp-dir 락파일 + 임의 잔존 `LockRecord` + fake liveness 플래그 | R-U8-01/02, FR-23 |
| **PROP-U8-02** | WatcherDaemon | 임의 시점에 `ShutdownFlag`를 set하면: 새 트리거는 무시되고, 관측된 마지막 in-flight 사이클은 완주 또는 dirty 보존으로 끝나며(중도 손상 없음), 종료 후 `LockGuard`가 해제된다(패닉 없음) | 임의 트리거 시퀀스 + 임의 shutdown 시점 + fake store(기록) | R-U8-10, NFR-03 |
| **PROP-U8-03** | WatcherDaemon | 임의 known_keys/federated 키맵에 대해: 배선 그래프(domain-entities §6)에 역엣지가 없고(하위->상위 주입만), U6 싱크가 하위 단위에 U0 트레이트로 주입된다. 어떤 하위 fake도 U8/U6 구체 타입을 호출하지 않음(기록 fake 0회) | 임의 config 키맵 + 기록 fake 하위 유닛 | R-U8-04, DEC-U8-08 |
| **PROP-U8-04** | SyncCycleCoordinator | 임의 (availability, last_committed, current_manifest, consent_state) 조합에 대해, 사이클은 정확히 하나의 `CoordinatorOutcome`으로 종결하고 그 분기가 배타적이다: unreachable->`HeldVaultUnavailable`(드라이버 미호출); 빈 diff->`NoOp`(미호출); consent `Blocked`->`BlockedConsent`(미호출); 그 외 diff 있고 `Permitted`이면 드라이버 정확히 1회 호출. 드라이버 미호출 분기에서 `execute_cycle` 호출 0회(기록 fake 확인) | 임의 `Availability`/매니페스트 쌍/`ConsentDecision` + 기록 fake 드라이버 | R-U8-06/07/08, services.md |
| **PROP-U8-05** | SyncCycleCoordinator | 임의 트리거 도착 시퀀스(동시 다수 포함)에 대해: 사이클은 결코 중첩 실행되지 않고(단일 소비자 직렬), 폭주 시 drain-to-latest로 최신 트리거만 새 사이클을 유발한다 | 임의 `CycleTrigger` 버스트 + 결정적 사이클 실행 카운터 | R-U8-06, Q2=B |
| **PROP-U8-06** | SyncCycleCoordinator | 임의 U2 판정(`Availability`/`GuardVerdict`)에 대해: vault-unavailable / HoldDestructiveEmpty 각각에서 정확히 대응하는 status 조건 raise + 로그 push가 1회 발생한다(U2는 판정만, push는 코디네이터). 성공 사이클은 `record_sync_success` 1회 | 임의 판정 + 기록 fake `StatusSink`/`Logger` | R-U8-07, services.md 노트2 |
| **PROP-U8-07** | WatcherDaemon | 임의 config 키 집합에 대해: `known_keys` union이 4개 소스(U0/U5/U6/U8)의 합집합과 정확히 일치하고, federated 값 해소가 라운드트립(raw->typed->동일 의미)하며, 미검증 `request_timeout_s`(0/음수)는 U5 규칙대로 거부된다 | 임의 키맵 + 임의 `request_timeout_s`(양/0/음) | R-U8-03, DEC-U8-07 |

---

## 6. 확장 컴플라이언스 요약

| 확장 | 활성 | 판정 |
|---|---|---|
| **Resiliency Baseline** | ON | RESILIENCY-01 단일 바이너리(전 크레이트 링크). 회복력이 조립 곳곳: R-U8-01/02(단일 인스턴스 + stale 회수 + guard drop 정리), R-U8-05(open_and_recover 크래시 복구 배선, NFR-03), R-U8-09(실패 백오프 재예약 + dirty 유지 무손실), R-U8-10(graceful drain, in-flight 비손상). RPO/RTO·HA/DR 세부는 Infra/Ops 이월 -> 부분 N/A |
| **Property-Based Testing** | ON (Full) | §5 PROP-U8-01..07 + 제너레이터/기록 fake seam 명시. 미준수 시 blocking. 프레임워크(proptest)는 code-gen 이월(PBT-09) |
| **Security Baseline** | OFF | N/A. 락파일 PID/owner·config 평문 토큰(RISK-01)은 정합성/회복력 설계이자 문서화된 수용 위험 |

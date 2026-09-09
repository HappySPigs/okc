## 서비스 (오케스트레이션) — okc-hooks "Watcher"

**단계**: INCEPTION → Application Design
**작성일**: 2026-09-08
**범위**: 컴포넌트를 조립·구동하는 **서비스(오케스트레이션) 계층**만 정의한다. 단일 배포 바이너리 내부에 서비스는 정확히 2개 — 조립 루트 `WatcherDaemon` 1개 + 단일 직렬 사이클 `SyncCycleCoordinator` 1개(Q2=B). 초안의 `UploadDrainWorker`/소비자 배출 루프·`DaemonSupervisor`는 제거/병합(§ `component-dependency.md` 네이밍 레지스트리 참조).

> **크리틱 non-blocking 노트 반영**:
> - **(노트2)** U2 검사기(`FilesystemWatcher`/`VaultAvailabilityGuard`/`ReconciliationScheduler`)는 의도적으로 `StatusService`/`StructuredLogger`를 의존하지 않는다. US-E1-01(트리거 로그)·US-E1-06(vault-unavailable을 로그/status/헬스로 표면화)의 실제 push는 **`SyncCycleCoordinator`가 소유**한다(코디네이터가 U2 검사기와 U6 싱크를 모두 의존) — 아래 사이클 단계 2·6·7·8·11·12에 명시.
> - Q9=B `update_probe()`(순수 liveness)와 `health_check()`(운영 헬스) 분리는 `WatcherDaemon` 7단계에 명시.

---

## WatcherDaemon (조립 루트 / Composition Root)
**단위**: Orchestration
**목적**: 단일 조립 루트. 의존성 주입, 단일 인스턴스 잠금 획득, 지속 상태 복구, 실행/graceful 종료, 상태 집계를 담당한다. (`DaemonSupervisor` 등 중복 없음.)
**공개 진입점**: `run(config_path) -> Result<(), DaemonError>`, `shutdown()`.
**오케스트레이션 단계**:
1. `ConfigProvider`(공유 파운데이션)로 단일 JSON config를 로드·검증한다. 유효하지 않으면 명확한 로그/CLI 오류로 종료.
2. `SingleInstanceLock::acquire()` — 이미 실행 중이면 활성 PID/소유자를 보고하고 비정상 종료. stale이면 회수 후 진행.
3. `SyncStateStore::open_and_recover()` — 부분쓰기 롤백 및 마지막 커밋 매니페스트/ dirty/ 재개 오프셋 복구(NFR-03).
4. 전 단위 컴포넌트를 생성·주입(DI)한다: U1(ContentAddressing, VaultScanner, ManifestBuilder, ManifestDiffer, SafetyLimitsValidator), U4(SyncStateStore, RetryBackoffController), U5(CredentialProvider, AuthTransport, ConsentGate), U2(FilesystemWatcher, ReconciliationScheduler, VaultAvailabilityGuard), U3(UploadProtocolDriver), U6(StructuredLogger, StatusService, UploadHistoryStore, CriticalErrorNotifier, TrayIndicator), U7(ServiceManager, AutoUpdater, RunStateController, OperatorCli, ControlPlane), 그리고 `SyncCycleCoordinator`. 상태·로그 싱크는 파운데이션 추상을 통해 하향 주입(push-only, U6 역참조 없음).
   - **(§0 수정1)** 로깅·상태-push 계약(`Logger`/`StatusSink`/`HistorySink`/`CriticalEventSink`)과 Q9 2축 상태 값 타입은 파운데이션 `CoreTypes` 소유이며, U6 컴포넌트는 그 구현만 제공한다. 조립 루트가 U6 구현을 생성해 하위 단위(U3/U5 등)에 파운데이션 계약 타입으로 주입 → 5개 U3/U5→U6 엣지는 파운데이션-계약 의존으로 성립(순서 역전 해소).
   - **(§0 수정2)** `TrayIndicator`는 nullable no-op 싱크로 생성해 `CriticalErrorNotifier`에 주입 — 헤드리스/트레이 부재 시에도 로그+헬스+CLI status 표면화(US-E5-04)가 깨지지 않는다.
5. `ReconciliationScheduler::run_startup_scan()`로 시작 스캔 사이클을 먼저 돌린 뒤 `FilesystemWatcher::start()`로 정상 감시에 진입.
6. 트리거 소스(FilesystemWatcher 디바운스, ReconciliationScheduler 주기, ControlPlane `sync-now`)를 `SyncCycleCoordinator`에 배선.
7. 운영 라이프사이클 + 활성 조건 집합을 `StatusService`로 집계 push(Q9=B 2축 모델). 업데이트용 순수 liveness `update_probe()`는 운영 `health_check()`와 분리해 노출(롤백 루프 방지 — AutoUpdater는 `update_probe()`만 소비).
8. graceful 종료: 감시 중지 → 진행 중 사이클을 안전하게 마무리하거나 `SyncStateStore`에 dirty/재개 오프셋으로 보존 → 잠금 해제.
**협력자**: `ConfigProvider`, `SingleInstanceLock`, `SyncStateStore`, `SyncCycleCoordinator`, `FilesystemWatcher`, `ReconciliationScheduler`, `StatusService`, `StructuredLogger`, `ControlPlane`(+조립 루트로서 U1..U7 전 컴포넌트 생성·주입).
**커버**: FR-21(장기 실행 데몬), FR-23(잠금 획득 배선), NFR-03(복구 오케스트레이션), NFR-05(3-OS 조립)

## SyncCycleCoordinator (단일 직렬 사이클)
**단위**: Orchestration
**목적**: Q2=B 단일 직렬 사이클. 트리거마다 감지→업로드→커밋을 끝까지 1회 수행하고, 실패/오프라인 시 전체 사이클을 백오프 재시도한다. 별도 생산자/소비자 루프·UploadDrainWorker 없음.
**공개 진입점**: `trigger(signal: TriggerSignal)`(감시/재조정/`sync-now` 인바운드; 단일-사이클 잠금으로 직렬화 — busy면 최신 트리거만 유효), `run_cycle() -> CycleOutcome`(내부 실행).
**오케스트레이션 단계(FQ-2=A 최신 상태 대체 + Q8=A 재검증 반영)**:
1. 트리거 수신 → 단일-사이클 잠금 획득(진행 중이면 새 트리거는 대기/합침; 재조정도 이 잠금으로 직렬화). **(노트2)** 트리거 원인 요약을 `StructuredLogger`로 로그 push(US-E1-01 — U2가 아니라 코디네이터가 표면화).
2. `VaultAvailabilityGuard::check_reachable(root)` — 도달 불가면 **(노트2)** vault-unavailable 조건을 `StatusService`에 raise + `StructuredLogger` 로그(US-E1-06 표면화는 코디네이터 소유) 후 보류, 다음 사이클/백오프 예약.
3. `VaultScanner`(U1) 폴더 재스냅샷 → `ManifestBuilder`(U1)로 매니페스트 계산(폴더가 진실의 원천 — 이벤트 큐 없음).
4. `ManifestDiffer`(U1)로 `SyncStateStore.last_committed_manifest()` 대비 diff.
5. `VaultAvailabilityGuard::guard_diff(...)` — 0-파일 전부-삭제이고 confirm-empty 아니면 `HoldDestructiveEmpty`로 커밋 보류(US-E1-06). **(노트2)** 보류 사유를 status/log로 push.
6. diff 비었으면 no-op — 필요 시 dirty 클리어, idle status push 후 종료.
7. `SafetyLimitsValidator`(U1) 프리플라이트 — 초과 시 halt, over-limit 조건 push + `CriticalErrorNotifier`(FR-19 케이스 3), 마지막 정상 커밋 보존(US-E2-08).
8. `ConsentGate`(U5) 게이트 — 토큰 부재/불일치 또는 동의 미부여/철회면 업로드 차단, 해당 조건 push(감시·상태는 유지).
9. `SyncStateStore.mark_dirty()`(커밋 전 유실 방지).
10. `UploadProtocolDriver`(U3) 실행 — `AuthTransport`(U5)를 통해 have/want(Q6=C) → 재개 청크 전송(전송 시 각 blob **재-읽기·재-해시**로 매니페스트 해시 재검증; 불일치면 그 커밋 중단하고 다음 사이클이 새 상태 재스냅샷 — Q8=A) → 커밋. 진행 중 오프셋은 `SyncStateStore`에 지속.
11. 성공: `SyncStateStore.commit_manifest(new)` + dirty/재개 오프셋 클리어, `UploadHistoryStore`(U6) append(success), `RetryBackoffController.on_success()`, idle status push.
12. 실패/오프라인: `RetryBackoffController.on_failure()` → 전체 사이클 백오프 재시도 예약, dirty 유지, offline/backoff status push, `UploadHistoryStore` append(failure/partial). 그 사이 편집은 다음 재스냅샷이 자동 흡수(무손실; US-E3-04 재해석 — 별도 배출 루프 없이 다음 사이클이 곧 drain).
**협력자**: `VaultAvailabilityGuard`, `VaultScanner`, `ManifestBuilder`, `ManifestDiffer`, `SafetyLimitsValidator`, `ConsentGate`, `AuthTransport`, `UploadProtocolDriver`, `SyncStateStore`, `RetryBackoffController`, `StatusService`, `UploadHistoryStore`, `CriticalErrorNotifier`, `FilesystemWatcher`, `ReconciliationScheduler`(트리거 소스).
**커버**: US-E1-01(사이클 인계), US-E3-02·US-E3-04(재스냅샷 최신 상태·재연결 drain), US-E7-04(latest-state-wins 구조적), US-E7-09(zero-loss 오케스트레이션), US-E7-11(회복탄력성 전략 — 상세 NFR Design 이월); FR-01, FR-04, FR-11, FR-12, NFR-03, NFR-11

---

> 컴포넌트 계약은 `components.md`, 메서드 시그니처는 `component-methods.md`, 의존성 그래프·데이터 흐름은 `component-dependency.md` 참조.

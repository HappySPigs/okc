# U2 Change Detect — Logical Components (논리 컴포넌트 분해 + 추적성 맵)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U2 `change-detect`** -> NFR Design -> 산출물 2/2 (`logical-components.md`)
**작성일**: 2026-09-08
**크레이트**: `change-detect` (lib) · **소속 컴포넌트(공개 애그리게이트)**: `FilesystemWatcher`, `ReconciliationScheduler`, `VaultAvailabilityGuard`
**입력 아티팩트**: 자매 산출물 `nfr-design/nfr-design-patterns.md`(§12 패턴 -> 논리 컴포넌트 안착 맵이 이 문서의 권위 근거) · `functional-design/`(domain-entities §1~§7 · business-rules R-DEB/R-RECON/R-GUARD/R-PURE · business-logic-model §0~§6) · `nfr-requirements/tech-stack-decisions.md`(T1~T10) · `u0-foundation/nfr-design/logical-components.md`(스타일 템플릿) · 활성 확장 `property-based-testing.md`(ON, Full)·`resiliency-baseline.md`(ON) · Security Baseline **OFF**
**규칙**: `construction/nfr-design.md` Step 6 · `common/content-validation.md`(ASCII 화살표 `A -> B`, 유니코드 화살표·박스/선-그리기 문자 미사용, 상한 `<=`, Rust 제네릭/타입 백틱) · `common/ascii-diagram-standards.md`

> **문서 성격 (입도 가드)**: 이 문서는 Application Design/FD가 확정한 **공개 3 애그리게이트**(`FilesystemWatcher`·`ReconciliationScheduler`·`VaultAvailabilityGuard`)를 상위 애그리게이트로 유지한 채, 그 내부의 **이미 확정된 기능(FD 타입·규칙·흐름)을 명명된 논리 컴포넌트로 전개**한다. 이는 발명이 아니라 확정 기능의 명명·정렬이며 FD 흐름에서 near-free로 파생된다. 각 논리 컴포넌트는 (a) 책임, (b) 공개 인터페이스 표면, (c) 안착하는 확정 NFR 패턴(자매 산출물 `nfr-design-patterns.md` §12 안착 맵과 1:1)으로 기술한다.
>
> **이것은 추적성 문서 맵이며 물리 모듈/크레이트 증식 강제가 아니다.** 논리 컴포넌트 -> 물리 파일/모듈 레이아웃 매핑은 **Code Generation에서 최소로** 결정된다(다수 논리 컴포넌트가 한 모듈에 상주할 수 있음). 목적은 NFR·규칙·PBT 속성 -> 명명 단위 추적성 극대화와 크레이트 의존 엣지(`foundation`만 의존, R-PURE-02) 강화다.
>
> **표기 규약**: 화살표는 ASCII `A -> B`만 사용(유니코드 화살표 금지). 박스/선-그리기 문자 미사용. Rust 제네릭/타입/식별자(예: `Duration`, `Instant`, `Option<&Manifest>`, `dyn WatchBackend`, `std::sync::mpsc`, `Result<T, WatchError>`)는 백틱으로 감싼다.

---

## 1. 논리 컴포넌트 전개 개요

| 공개 애그리게이트 | 성격 | 논리 컴포넌트(확정 명명) |
|---|---|---|
| `FilesystemWatcher` | 백엔드 구독 + 디바운스 타이머(단일 소유 스레드) + 순수 정규화 + 조회 표면 | WatchBackend(trait) · per-OS Adapters(`FsEventsBackend`/`INotifyBackend`/`RdcwBackend`) · DegradedBackend · EventNormalizer · DebounceTimer · TriggerEmitter · WatchStateView |
| `ReconciliationScheduler` | 순수 `tick(now)` + in-memory 스케줄 상태 + 조회 표면 | ReconTick · ReconStateStore · ReconResultView |
| `VaultAvailabilityGuard` | 파일시스템 stat 사전점검 + 순수 판정 표 | ReachabilityChecker · DiffGuard |
| (공용 값/전송) | 트리거 규약(두 소스 공용) + 오류 표면 | TriggerModel(`TriggerSignal`/`TriggerKind`/`TriggerStream`) · ConfigViews · ErrorSurface(`WatchError`) |
| (런타임 그래프 밖) | 테스트 전용 — 비-default `proptest-support` feature | ProptestGenerators (test-support 논리 단위, §5.2) |

---

## 2. `FilesystemWatcher` 애그리게이트 — 논리 컴포넌트

> `FilesystemWatcher`는 볼트 루트 이벤트를 감지해 버스트당 1 트리거로 변환하는 애그리게이트다. OS I/O 경계(어댑터)와 순수 로직(정규화·디바운스)이 명확히 분리된다.

### 2.1 WatchBackend (trait) + per-OS Adapters
- **책임**: OS별 네이티브 감시 API(FSEvents/inotify/ReadDirectoryChangesW)를 **정규화된 이벤트 소스**로 추상화하는 트레이트(NFR-05 이식성의 트레이트 경계). 각 어댑터(`FsEventsBackend`/`INotifyBackend`/`RdcwBackend`)는 크로스플랫폼 크레이트 `notify`(major `6`, T1)를 뒤에 두고 볼트 루트 하위 create/modify/delete/rename을 구독한다. rename은 delete+create로(D2), 백엔드 오버플로는 `Overflow`로(D6) 정규화.
- **공개 인터페이스 표면**: `trait WatchBackend`(참고 형상 `subscribe(root) -> raw event source`) · `enum RawFsEvent { Created(RelativePath), Modified(RelativePath), Deleted(RelativePath), Overflow }`(`domain-entities.md` §2.3). `notify` 타입은 트레이트 뒤 캡슐화되어 공개 API에 새지 않는다.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §4 이식성 어댑터(**U2-NFR-PORT-01** [KEY], PROP-U2-05/PROP-DE-U2-02) + §6 I/O 격리(**U2-NFR-REL-05**, 백엔드 실패 -> `WatchError`).

### 2.2 DegradedBackend
- **책임**: 네이티브 어댑터가 MVP 초과이거나 `WatchError::Unsupported`일 때의 **무이벤트 폴백 백엔드**(D3, T7). 이벤트를 전혀 방출하지 않으므로 디바운스 트리거가 0이고 트리거는 재조정 소스에서만 나온다 — 정확성 바닥은 재조정 백스톱(`<= T_recon`)이 보장.
- **공개 인터페이스 표면**: `WatchBackend` 구현(참고: `subscribe`가 빈 소스 반환).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §4 degraded 폴백(**U2-NFR-PORT-01** MVP 트림) — 정확성은 §2 `ReconTick` 백스톱(**U2-NFR-REL-02**)이 커버.

### 2.3 EventNormalizer
- **책임**: 어댑터가 방출한 raw 이벤트를 공통 `RawFsEvent` 어휘로 정규화하는 순수 로직. `RawFsEvent` -> `RelativePath` 정규화 실패(U0 fallible normalize)를 패닉이 아니라 스킵/진단 위임으로 흡수. rename -> delete+create, overflow -> `Overflow` 정규화 규칙이 전 백엔드 공통(behavioral equivalence).
- **공개 인터페이스 표면**: 순수 정규화 함수(어댑터 내부에서 호출). 백엔드 무관 동일 규칙.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §4 정규화 계약(**U2-NFR-PORT-01**, PROP-DE-U2-02) + §6 패닉프리(**U2-NFR-REL-05**, 순수 표면 clippy lint-gate 대상).

### 2.4 DebounceTimer
- **책임**: 볼트-전체 단일 정적-구간 타이머(D1). 정규화된 change 도착마다 std `recv_timeout(T_debounce)` 대기 창 리셋, 무이벤트 `T_debounce` 경과 시 트리거 1개(R-DEB-01). `Overflow`도 합성 change로 동일 투입(R-DEB-04). `WatchState::Paused` 중 이벤트 폐기(R-DEB-05, 정확성은 recon). 단일 소유 스레드가 관리하는 in-memory 상태(버스트 카운트·타이머 데드라인), 지속하지 않음. `notify-debouncer-*` 미사용(hand-rolled, T2).
- **공개 인터페이스 표면**: 내부 타이머 루프(생성자 주입 `T_debounce: Duration` 소비). 만료 시 `TriggerEmitter`로 트리거 위임.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §1 디바운스 단일-타이머(**U2-NFR-REL-01/REL-03/PERF-01**, R-DEB-01/02/04/05; PROP-U2-01/02/03) + §6 순수 정규화 no-panic.

### 2.5 TriggerEmitter
- **책임**: 디바운스 만료 시 `TriggerSignal { kind: Debounced, cause_summary, observed_at }` 1개를 `TriggerStream`(`std::sync::mpsc`)에 순서 보존 push(T4, D14). `WatchState` 전이(`Triggered -> Idle`) 반영. 페이로드 없음(FQ-2 — ChangeSet 미탑재).
- **공개 인터페이스 표면**: `start() -> (JoinHandle, TriggerStream)`(참고 형상). `TriggerStream` 수신단은 U8 유일 소비자.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §3 std-only 동시성 + `mpsc` 스트림(**U2-NFR-PERF-01/REL-02**, D14).

### 2.6 WatchStateView
- **책임**: coordinator/CLI가 `watcher status`를 구성하기 위한 **조회 전용 표면**(push 아님, R-PURE-01/R-DEB-06). `WatchState` 라벨과 마지막 관측 이벤트 단조 시점 노출. `Paused` 중 폐기된 이벤트는 갱신하지 않음.
- **공개 인터페이스 표면**: `fn watch_state(&self) -> WatchState`(`Watching`/`Triggered`/`Idle`/`Paused`) · `fn last_event_at(&self) -> Option<Instant>`(단조 시점) · `pause()`/`resume()`/`stop(self)`.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §6 순수성(**U2-NFR-REL-05**, R-PURE-01 조회 read-only, 부수효과 없음).

---

## 3. `ReconciliationScheduler` 애그리게이트 — 논리 컴포넌트

> `ReconciliationScheduler`는 미관측 변경의 검출 지연을 `<= T_recon`으로 상한 짓는 백스톱 트리거 소스다. 자체 타이머·스레드를 갖지 않는 순수 `tick(now)` 판정자다.

### 3.1 ReconTick
- **책임**: 단조 시점을 인자로 받는 순수 판정 `tick(now: Instant) -> Option<TriggerSignal>`(D7). busy이면 `None`(R-RECON-06), `now < next_due`이면 `None`, 아니면 발행 -> `next_due = now + T_recon`(발행-시각 앵커, R-RECON-04) + `Some(Reconciliation(Periodic))`. 시작 재조정(`Reconciliation(Startup)`) 1회 발행 + `next_due` 초기화(R-RECON-01). 자체 타이머 없음 — 상위 데몬 스케줄러가 단조 `now` 공급, async 런타임 미도입(T3).
- **공개 인터페이스 표면**: `fn tick(&mut self, now: Instant) -> Option<TriggerSignal>` · `fn run_startup_scan(&mut self, now: Instant) -> TriggerSignal`(참고 형상). busy 관찰은 U8이 전달.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §2 발행-시각 앵커 순수 스케줄러(**U2-NFR-REL-02** [KEY], R-RECON-01/02/03/04/06; PROP-U2-04/06) + §2 async 없음(**U2-NFR-PERF-01**).

### 3.2 ReconStateStore
- **책임**: in-memory 스케줄 상태(`next_due: Instant`·`last_result: Option<ReconResult>`) 보유. `record_result(outcome)`은 `last_result`만 갱신하고 `next_due`를 완료 시각으로 재계산하지 않는다(NFR-03 상한 유지, R-RECON-04). 상수 크기, 지속하지 않음 — 재시작 시 시작 재조정이 초기화.
- **공개 인터페이스 표면**: `fn record_result(&mut self, outcome: CycleOutcome)`(`NoOp`/`Committed`/`Held`/`Failed` 소비).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §2 발행-시각 앵커(**U2-NFR-REL-02**, R-RECON-04) + §2 정성 리소스(상수 크기 상태, **U2-NFR-PERF-01**).

### 3.3 ReconResultView
- **책임**: 조회 전용 표면(push 아님, R-PURE-01). "다음 재조정까지 남은 시간 + 마지막 결과"(US-E1-04 AC) 표면화는 U8이 이 값을 읽어 수행. 스케줄 기준은 단조 시점, 사람 표면화용 시각은 `ReconResult.at`(U0 `Timestamp`, UTC) — 클록 분리.
- **공개 인터페이스 표면**: `fn next_recon_due(&self) -> Instant`(단조) · `fn last_result(&self) -> Option<ReconResult>`(`phase`/`outcome`/`at`).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §6 순수성(**U2-NFR-REL-05**, 조회 read-only).

---

## 4. `VaultAvailabilityGuard` 애그리게이트 — 논리 컴포넌트

> `VaultAvailabilityGuard`는 파괴적-빈-커밋 방지 데이터 무결성 안전 불변식을 total-function 판정 표로 실현하는 순수 판정자다.

### 4.1 ReachabilityChecker
- **책임**: 볼트 루트의 도달 가능성을 파일시스템 stat으로 사전 점검해 `Availability` 4-분류 산출(D9). 세 unavailable 변이(`RootMissing`/`Unmounted`/`Inaccessible`)는 동일 안전 동작(HOLD)을 유발하고 구분은 진단 라벨. 크로스 OS 구분 난이도 시 best-effort 분류(안전 동작 불변).
- **공개 인터페이스 표면**: `fn check_reachable(&self, root) -> Availability`(`Reachable`/`RootMissing`/`Unmounted`/`Inaccessible`).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §5 가드 판정(**U2-NFR-REL-04**, R-GUARD-01) + §6 I/O 경계(stat 실패는 unavailable 판정으로 흡수, 패닉프리).

### 4.2 DiffGuard
- **책임**: `guard_diff(new_manifest, last_committed, availability) -> GuardVerdict`의 total-function 판정 표(G1~G4). 안전 불변식 — `availability != Reachable` 또는 (`empty(new)` AND `last_committed` 비어있지 않음 AND `!confirm_empty`)이면 절대 `Proceed` 아님(R-GUARD-03). 무상태 판정 -> 복구 시 자동 재개(R-GUARD-04) + 멱등. `confirm_empty` 영속 플래그 소비(D10). 파일시스템 stat 외 컴포넌트 의존 없음(순수, R-GUARD-06).
- **공개 인터페이스 표면**: `fn guard_diff(&self, new_manifest: &Manifest, last_committed: Option<&Manifest>, availability: Availability) -> GuardVerdict`(`Proceed`/`HoldVaultUnavailable(HoldReason)`/`HoldDestructiveEmpty`). 생성자 주입 `confirm_empty: bool` 소비.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §5 파괴적-빈-커밋 total-function 가드(**U2-NFR-REL-04/REL-05**, R-GUARD-02/03/04/05/06; PROP-U2-07/08) + §6 순수 표면 clippy lint-gate.

---

## 5. 공용 값/전송 + test-support 논리 컴포넌트

### 5.1 TriggerModel · ConfigViews · ErrorSurface (공용 값)
- **TriggerModel** — 두 트리거 소스(watcher/scheduler) 공용 규약. `struct TriggerSignal { kind: TriggerKind, cause_summary: String, observed_at: Instant }` · `enum TriggerKind { Debounced, Reconciliation(ReconPhase) }`(3-변이 폐쇄, Overflow도 `Debounced`) · `TriggerStream`(순서 보존 단방향, 구체 전송 `std::sync::mpsc`, U8 유일 소비자). 안착: `nfr-design-patterns.md` §3(**U2-NFR-PERF-01/REL-02**, D14) · §1(오버플로 정규화, PROP-DE-U2-01 enum 전역성).
- **ConfigViews** — U8 조립 루트가 주입하는 타입드 값의 **개념 형상**(`WatchConfig`/`ReconConfig`/`VaultConfig`, `domain-entities.md` §5). U2는 `ConfigProvider`/`ConfigSnapshot`을 읽지 않고 생성자로 `Duration`/`bool`을 받는다(read-only). 안착: `nfr-design-patterns.md` §7 federated 생성자 주입(**U2-NFR-MNT-02**, PROP-DE-U2-03).
- **ErrorSurface** — `enum WatchError { Unsupported, RootMissing, OsWatchInit(BackendCause), Backend(BackendCause) }`, `thiserror` 파생(운영 오류, U0 Q4=A 상속). `source`는 로그 전용·지속 안 함(round-trip 대상 아님). 판정 값(`Availability`/`GuardVerdict`/`CycleOutcome`)과 구분(순수 enum, thiserror 미적용). 안착: `nfr-design-patterns.md` §6 `WatchError` I/O 격리(**U2-NFR-REL-05**).

### 5.2 ProptestGenerators — 런타임 그래프 밖 test-support 논리 단위 (PBT-07)
- **배치**: 도메인 제너레이터(`proptest`)는 **런타임 의존 그래프(§6) 밖**의 별도 test-support 논리 단위다. 비-default cargo feature `proptest-support`로 게이트되어 릴리스 런타임 바이너리에 포함되지 않는다(U0 미러, T9).
- **책임**: (a) `RawFsEvent` 이벤트 도착 타임라인(단일/밀집 버스트/경계 간격/다중 버스트/`Overflow` 삽입), (b) `(now 증가, busy/idle 전이, record_result)` 스케줄러 명령 시퀀스(긴 busy·조밀 tick·경계 `now == next_due`), (c) 가드 입력 조합(`Availability` x 매니페스트-공허성 x `last_committed`-유무 x `confirm_empty`), (d) `WatchBackend` 목(동일 타임라인을 여러 백엔드가 방출) 제공. U0 `Manifest`/`RelativePath`/`Timestamp` 제너레이터는 `foundation`의 `proptest-support`를 dev에서 켜 **재사용**(재작성 금지, 단일 출처).
- **의존 방향**: `ProptestGenerators -> {change-detect 런타임 타입, foundation ProptestGenerators}`. 런타임 컴포넌트는 이 단위에 의존하지 않으므로 §6 런타임 DAG를 오염시키지 않는다.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §8(**U2-NFR-MNT-01**, PBT-07/09). 이월: 구체 구현·shrinking·고정 시드·CI 통합(PBT-08)은 Code Generation / Build-and-Test.

---

## 6. 크레이트 의존 엣지 (비순환 + `foundation`만 의존 확인)

> 규약: `A -> B` = "A가 B의 타입/함수/계약을 사용". U2는 Wave-2 소비 lib으로 **`foundation`(U0)에만** 의존하고 U1~U8 어느 단위도 역참조하지 않는다(R-PURE-02).

### 6.1 크레이트-레벨 엣지

```
런타임 의존 (normal):

  change-detect  ->  foundation        (CoreTypes: Manifest / RelativePath / Timestamp 참조만)
  change-detect  ->  notify (핀 6)     (WatchBackend 어댑터 뒤 캡슐화, U2 내부 한정)
  change-detect  ->  thiserror         (WatchError 파생, 워크스페이스 상속)

  change-detect  ->  (U1~U8 어느 단위 크레이트)   = 없음  => R-PURE-02 순수성

dev 의존 (test-support):

  change-detect[proptest-support]  ->  proptest                       (프레임워크 상속)
  change-detect[proptest-support]  ->  foundation[proptest-support]   (U0 제너레이터 재사용)

미의존(의도적): ciborium / serde_json / url (U2 무지속·무전송) · notify-debouncer-* (hand-rolled) · tokio/async-std (async 없음)
```

### 6.2 U2 내부 논리 컴포넌트 의존 방향

```
FilesystemWatcher 계층:
  per-OS Adapters (notify 뒤)  ->  EventNormalizer  ->  DebounceTimer  ->  TriggerEmitter  ->  TriggerModel(TriggerStream)
  DegradedBackend (무이벤트)   ->  (동일 경로, 이벤트 0)
  WatchStateView               ->  (조회만, DebounceTimer 상태 read-only)

ReconciliationScheduler 계층:
  ReconTick  ->  ReconStateStore  ->  TriggerModel(TriggerSignal)
  ReconResultView  ->  (조회만, ReconStateStore read-only)

VaultAvailabilityGuard 계층:
  ReachabilityChecker  ->  Availability
  DiffGuard  ->  Availability + foundation(Manifest)  ->  GuardVerdict

공용:  세 애그리게이트  ->  TriggerModel / ConfigViews / ErrorSurface  ->  foundation(CoreTypes)

방향 요약:  change-detect.*  ->  foundation.*     (한 방향)
           foundation       ->  (없음 = DAG 루트)  => 사이클 없음
```

### 6.3 비순환 및 순수성 확인
- **비순환(acyclic)**: 모든 크로스-크레이트 엣지는 `change-detect -> foundation` 방향으로만 흐르고 역방향이 없다(component-dependency.md 빌드 순서 Foundation -> U1 -> U4 -> U5 -> U2 정합). U2 내부 엣지도 어댑터 -> 정규화 -> 디바운스 -> 전송의 단방향.
- **순수성(R-PURE-01/02)**: 세 컴포넌트 공개 시그니처에 `StatusSink`/`Logger`/`HistorySink`/`CriticalEventSink` 파라미터가 없고, U2 `[dependencies]`가 `foundation`(+ `notify`/`thiserror`) 외 워크스페이스 단위 크레이트를 포함하지 않는다(컴파일/구조 검증). `TriggerStream`/`GuardVerdict`는 U8에 **반환/전달**되고 back-reference가 없다.
- **config 취득(federated, MNT-02)**: U2는 `ConfigProvider`를 읽지 않고 U8 조립 루트가 타입드 값(`Duration`/`bool`)을 생성자 주입한다. non-core 라이브리로드 미지원(재시작 반영, MVP 트림) — U2는 `ConfigReloadObserver`를 구독하지 않는다.

---

## 7. PBT 타깃 정렬 (PBT-01 / 확장 Full)

> 이 단계는 신규 PBT 결정을 내리지 않는다(PBT-09 프레임워크 = NFR-Req에서 `proptest`로 SATISFIED). 확정 속성(FD의 PROP-U2-*/PROP-DE-U2-*)을 명명 논리 컴포넌트에 정렬해 추적성을 강화한다. NFR ID -> 속성 매핑은 `nfr-design-patterns.md` §11/§12와 일관된다.

### 7.1 컴포넌트 -> Testable Property 정렬표

| 논리 컴포넌트 | 정렬 속성(FD 소유) | 카테고리 | 실현 NFR / 규칙 |
|---|---|---|---|
| DebounceTimer | PROP-U2-01(버스트당 1 트리거) · PROP-U2-02(감지 지연 상한) | Induction / Invariant / Oracle | U2-NFR-REL-01(R-DEB-01/02) |
| DebounceTimer | PROP-U2-03(오버플로 -> 최소 1 트리거) | Invariant | U2-NFR-REL-03(R-DEB-04) |
| ReconTick + ReconStateStore | PROP-U2-04(연속 재스냅샷 간격 `<= T_recon`) · PROP-U2-06(busy 중 동시 재조정 없음) | Induction / Invariant | U2-NFR-REL-02(R-RECON-03/04/06) |
| WatchBackend + EventNormalizer | PROP-U2-05(백엔드 무관 트리거 등가) · PROP-DE-U2-02(정규화 어휘 계약) | Invariant / Commutativity / Oracle | U2-NFR-PORT-01(NFR-05) |
| DiffGuard | PROP-U2-07(파괴적-빈-커밋 불가, G1·G2 no `Proceed`) · PROP-U2-08(자동 재개 + 멱등) | Invariant / Idempotence / Easy verification | U2-NFR-REL-04(R-GUARD-02/03/04) |
| 세 컴포넌트 순수 표면 | PROP-U2-07/01/04 실행 중 no-panic 관찰 | Invariant | U2-NFR-REL-05(§6 lint-gate, U0 Q8=A) |
| TriggerModel · ConfigViews | PROP-DE-U2-01(enum 전역성) · PROP-DE-U2-03(config 뷰 검증 경계) | Invariant / Easy verification | U2-NFR-MNT-02(config 투영) |

> **속성 없음(No PBT properties identified)**: `TriggerSignal.cause_summary`(진단 자유형)·`WatchError`(오류 분류 래퍼, 예제 기반 단위테스트 적합)·`ReconResult.at`/`observed_at` 단조 시점(지속 안 함)·`TriggerStream` 순서 보존(전송 메커니즘 = 통합/계약 테스트, D14)은 독립 PBT 속성이 없다(domain-entities §7.1 · business-logic-model §6.2와 일관).

---

## 8. RESILIENCY-01 매핑 — U2 기여(Critical 전체 Watcher의 감지·백스톱 로직)

- **분류**: 전체 Watcher는 RESILIENCY-01에서 **Critical**(zero-loss 목표)이다. U2는 그 중 **감지·백스톱·데이터 무결성 가드의 로직 근거**를 기여하는 순수 lib이며, 상태를 지속하지 않는다(무지속, business-logic-model.md §5).
- **회복력 기여(순수 감지 lib 범위)**:
  - **재조정 백스톱(U2-NFR-REL-02, ReconTick/ReconStateStore)** — 미관측 변경 검출 지연 `<= T_recon`(RESILIENCY-02 무손실 검출 지연 상한의 로직 근거). 발행-시각 앵커가 상한을 구조적으로 보증.
  - **오버플로 회복(U2-NFR-REL-03, DebounceTimer)** — 백엔드 오버플로를 `T_debounce`로 회복(중복 안전, recon이 최후 안전망).
  - **파괴적-빈-커밋 가드(U2-NFR-REL-04, DiffGuard)** — 마운트 드롭/루트 삭제가 "전부 삭제"로 증폭되는 경로 차단(데이터 무결성).
  - **패닉프리 순수성(U2-NFR-REL-05)** — 세 판정자가 U8 사이클 핫 경로에서 패닉하면 감지·백스톱이 정지하므로 clippy lint-gate가 회복력을 지탱. 시작 재조정 + 무지속이 크래시/재시작 회복을 U4 `SyncStateStore`와 협력.
- **N/A 범위**: RTO/RPO 수치·DR/HA·서킷브레이커·auto-scaling·배포/롤백·multi-AZ·카오스(RESILIENCY-02/05~14)는 순수 감지 lib에 부적용. U2 신규 인프라 통제 없음(`nfr-design-patterns.md` §9와 일관).
- **추적 사슬**: 공개 3 애그리게이트를 명명 논리 컴포넌트로 전개함으로써 (NFR ID -> 규칙 ID -> **명명 컴포넌트** -> Testable Property)의 4단 추적이 성립한다(§7.1 정렬표 + `nfr-design-patterns.md` §12).

---

## 9. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 산출물 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **준수 — blocking 없음** | §7이 각 논리 컴포넌트를 확정 속성(PROP-U2-*/PROP-DE-U2-*)에 정렬(PBT-01), ProptestGenerators를 test-support 논리 단위로 문서화(§5.2, PBT-07). 신규 PBT 결정 없음(PBT-09는 NFR-Req 충족·상속). PBT-08은 Code Generation/Build-and-Test 이월 |
| **Resiliency Baseline** | ON | **준수(부분 적용 + 대체로 N/A) — blocking 없음** | §8이 U2 = Critical 전체 Watcher의 감지·백스톱·무결성 로직 기여를 컴포넌트 입도로 확정. REL-02~05 계약의 컴포넌트 안착 명시. RTO/RPO/DR/HA/서킷브레이커/카오스는 순수 lib에 N/A. 신규 resiliency 결정 없음 |
| **Security Baseline** | OFF | **N/A — 미로딩·미강제** | 확장 OFF, RISK-01/02 수용. U2는 시크릿·토큰·네트워크 미취급 — `notify`는 로컬 FS 감시만. 보안 통제 컴포넌트 배치 없음 |

**블로킹 판정**: 이 산출물에 blocking finding 없음. §2~§5 컴포넌트 안착은 `nfr-design-patterns.md` §12 맵과 1:1이며, §6 의존 그래프는 비순환(U2 -> `foundation` 한 방향, U1~U8 무-import, sink-free)이다.

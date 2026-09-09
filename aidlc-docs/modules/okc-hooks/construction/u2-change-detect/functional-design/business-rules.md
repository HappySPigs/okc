# U2 Change Detection & Trigger — Business Rules (결정 규칙 / 검증 로직 / 제약)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U2 `change-detect`** -> Functional Design -> 산출물 2/3 (`business-rules.md`)
**작성일**: 2026-09-08
**크레이트**: `change-detect` (lib) · **소속 컴포넌트**: `FilesystemWatcher`, `ReconciliationScheduler`, `VaultAvailabilityGuard`
**전제(확정 답변)**: FQ-2=A(최신 상태 대체, 폴더가 진실의 원천) · Q2(사이클)=B(트리거당 1회 직렬 사이클) · NFR-01(감지 지연=디바운스 상한) · NFR-03(미관측 변경 검출 지연 <= T_recon) · NFR-05(크로스 OS 이식성) · MVP 결정 D1~D14(`u2-change-detect-functional-design-plan.md` §3)

> **문서 성격**: U2가 소유하는 **결정 규칙·검증 로직·제약·엣지케이스 판정**을 정의한다. 타입 정의는 `domain-entities.md`가 소유하며 이 문서는 그 타입명·필드명을 그대로 재사용한다. 알고리즘·흐름·시퀀스는 `business-logic-model.md`가 소유한다(중복 회피).
>
> **표기 규약**: 기술중립 설계. Rust스러운 시그니처는 참고용. 다이어그램은 ASCII 박스 없이 **화살표 표기(A -> B)** 와 표/목록. Rust 타입은 백틱.

---

## 1. 디바운스 규칙 (FilesystemWatcher — FR-01 / NFR-01 / US-E1-01)

### 1.1 exactly-1-trigger-per-burst

**규칙 R-DEB-01 (버스트당 정확히 1 트리거 — 구속력 있는 불변식)**: 편집 **버스트(burst)** 를 "연속한 파일시스템 change 이벤트들로, 인접 이벤트 간 간격이 모두 `T_debounce` 미만인 최대 구간"으로 정의한다. 한 버스트에 대해 `FilesystemWatcher`는 **정확히 하나의 `TriggerSignal`(kind=`Debounced`)** 을 발행한다.

- **디바운스 타이머 규칙**: 임의 change 이벤트 도착 시 디바운스 타이머를 `T_debounce`로 **리셋**한다(D1: 볼트 전체 단일 타이머). 타이머가 리셋 없이 `T_debounce` 동안 만료되면(정적 구간 quiet period 도달) 트리거 1개를 발행하고 버스트를 종료한다.
- **버스트 합치기**: 버스트 진행 중 도착하는 추가 이벤트는 **새 트리거를 만들지 않고** 타이머만 리셋한다 — 다수 이벤트가 1개 트리거로 합쳐진다(US-E1-01 AC "버스트 합치기").
- **페이로드 없음(D12/FQ-2)**: 트리거는 어떤 파일이 바뀌었는지를 담지 않는다 — 코디네이터가 재스냅샷한다. 따라서 이벤트 상세를 축적/기억할 필요가 없다(폴더가 진실의 원천).

### 1.2 감지 지연 상한 (NFR-01)

**규칙 R-DEB-02 (지연 상한)**: 버스트의 **마지막 이벤트** 시각을 `t_last`라 할 때, 트리거는 `t_last + T_debounce` 시점(정적 구간 만료)에 발행된다. 즉 변경 감지 지연(마지막 이벤트 -> 사이클 시작 트리거)은 `T_debounce`로 **상한이 정해진다**(구현 스케줄링 오차 epsilon 제외). 이것이 NFR-01의 "감지 지연 = 설정 가능한 디바운스로 통제"의 형식이다.

**규칙 R-DEB-03 (T_debounce 검증)**: `T_debounce`(`WatchConfig.debounce_ms`)는 **양의 정수(> 0)** 여야 한다. 0/음수/비정수는 **시작 시 명확한 로그/CLI 오류로 거부**한다(US-E1-01 AC). 검증 실패는 U0 config 검증 계층(strict)에서 표면화되며 데몬은 기동 실패(abort, U0 R-RELOAD-04)한다.

### 1.3 오버플로 정규화 (D6 — 이벤트 손실 즉시 대응)

**규칙 R-DEB-04 (백엔드 오버플로 -> 합성 트리거)**: 백엔드가 `RawFsEvent::Overflow`(inotify `IN_Q_OVERFLOW` / FSEvents 병합 / RDCW 버퍼 오버플로 / 무이벤트 마운트 감지)를 방출하면, 이를 **최소 1개의 합성 change로 취급해 디바운스에 투입**한다 -> 정적 구간 후 트리거 1개 발행. 상세 유실 이벤트를 복원하려 시도하지 않는다(불가능) — 코디네이터가 재스냅샷하면 실제 변경이 diff로 정확히 드러난다(FQ-2).

- **근거**: 오버플로 시 검출 지연을 `T_recon`이 아닌 `T_debounce`로 회복(저비용·고가치). recon 백스톱(§2)은 최후 안전망으로 여전히 유효하다.

### 1.4 pause / resume (D13)

**규칙 R-DEB-05 (paused 중 이벤트 폐기)**: `WatchState::Paused`에서는 도착한 이벤트를 **폐기**한다(버퍼링 없음) — 디바운스 타이머 미가동, 트리거 미발행. `resume` 시 새 버스트부터 다시 감시한다. **정지 중 발생한 변경의 정확성은 재조정 백스톱(<= T_recon)과 resume 후 다음 스캔이 보장**한다(누락 없음, FQ-2 재스냅샷). pause/resume 호출자는 U7b `RunStateController`(감시만 억제, 데몬 상주).

### 1.5 조회 표면 (status 소싱 — US-E1-01 AC)

**규칙 R-DEB-06 (마지막 이벤트 조회 — 순수 조회)**: `FilesystemWatcher`는 상태 라벨 `watch_state() -> WatchState`에 더해, **마지막으로 관측한 `RawFsEvent`(change/overflow)의 단조 시점**을 `last_event_at() -> Option<Instant>`로 노출한다(아직 관측 이벤트가 없으면 `None`; `Paused` 중 폐기된 이벤트는 갱신하지 않음). `watcher status`의 "마지막 이벤트 이후 경과 시간"(US-E1-01 AC)은 U8 `SyncCycleCoordinator`가 이 값을 **읽어** `elapsed = now - last_event_at`로 계산·표시한다. 순수 read-only 조회이며 U2는 값을 push하지 않는다(R-PURE-01 정합). 벽시계가 아닌 단조 시점을 반환하므로 클록 점프에 영향받지 않는 경과 계산이 가능하다.

---

## 2. 재조정 스케줄링 규칙 (ReconciliationScheduler — FR-04 / NFR-03 / US-E1-03·04)

### 2.1 시작 재조정 (US-E1-03)

**규칙 R-RECON-01 (기동 1회 전체 재조정)**: 데몬 기동 시(부트 자동시작 또는 수동 `watcher start`), **정상 라이브 감시 진입 전에** `run_startup_scan()`이 `TriggerSignal(kind=Reconciliation(Startup))` 1개를 발행한다. 이는 데몬 정지 중 발생한 변경을 복구하기 위한 전체 재해시+diff 트리거다(실제 재해시·diff는 U1 위임).

### 2.2 주기 백스톱 + <= T_recon 상한 (NFR-03)

**규칙 R-RECON-02 (주기 트리거 조건)**: `tick(now)`는 다음을 **모두** 만족할 때만 `Some(TriggerSignal(kind=Reconciliation(Periodic)))` 를 반환한다:
1. `now >= next_due` (마지막 재조정 발행 후 `T_recon` 경과), **그리고**
2. 진행 중 사이클이 없음(idle) — §2.3.

그 외에는 `None`.

**규칙 R-RECON-03 (검출 지연 <= T_recon 불변식 — 구속력)**: 파일시스템 이벤트로 관측되지 못한 임의의 변경은 **`T_recon` 이내에 검출**된다. 형식: 임의 시점 `t`에서 발생한 미관측 변경은, `t` 이후 처음 완료되는 재조정(또는 이미 진행 중인 사이클의 재스냅샷)에 의해 검출되며, 그 시점은 `<= t + T_recon`이다. **근거**: `next_due`가 항상 `<= (마지막 재조정 발행) + T_recon`이고, 진행 중 사이클은 그 자체로 재스냅샷(=재조정 커버)이므로 연속 재스냅샷 간 최대 간격이 `T_recon`을 넘지 않는다.

**규칙 R-RECON-04 (스케줄 기준 — 발행 시각 앵커)**: `next_due = (마지막 재조정 발행 시각) + T_recon` (단조 시점 기준, D7). `next_due`는 재조정 트리거가 **발행되는 순간** 설정된다 — 시작 재조정(`run_startup_scan()`) 발행 시, 그리고 주기 `tick(now)`가 `Some`을 반환하는 순간 `next_due = now + T_recon`. 이는 **완료 시각(`record_result`) 기준이 아니다**. `record_result(outcome)`는 재조정 완료를 통지해 `last_result`만 갱신하며 `next_due`를 재계산하지 않는다 — 완료 기준으로 재계산하면 연속 재스냅샷 간격이 `사이클 시간 + T_recon`이 되어 R-RECON-03/NFR-03 상한을 위반하기 때문이다. 발행 시각 앵커 덕분에 `next_due`는 다음 발행까지 재상승하지 않아 재-발행이 자연히 억제된다(스택 없음). 사이클 종류(NoOp/Committed/Held/Failed)와 무관하게 **재스냅샷이 수행됐으면 재조정 커버로 계산**한다(§`domain-entities.md` CycleOutcome 불변식).

**규칙 R-RECON-05 (T_recon 검증)**: `T_recon`(`ReconConfig.reconciliation_interval_s`)은 **양의 정수(>= 1초)** 여야 한다. 위반 시 시작 검증 실패(U0 strict).

### 2.3 사이클 직렬화 (Q2=B / US-E1-04 AC)

**규칙 R-RECON-06 (진행 중 사이클과 직렬화)**: 재조정은 진행 중 사이클과 **동시 실행되지 않는다**. 직렬화 잠금은 **U8 `SyncCycleCoordinator`가 소유**하며, 스케줄러는 busy(진행 중 사이클) 상태에서 `tick`이 `None`을 반환해 트리거를 **건너뛴다**(다음 주기로 미룸, 스택 쌓지 않음). 진행 중 사이클이 이미 재스냅샷하므로 이 skip은 NFR-03을 위반하지 않는다(§2.2 R-RECON-03).

---

## 3. 볼트 가용성 가드 규칙 (VaultAvailabilityGuard — US-E1-06 / FR-09)

### 3.1 도달 가능성 분류

**규칙 R-GUARD-01 (사전 점검)**: `check_reachable(root)`는 스냅샷 전에 볼트 루트의 (a) 존재, (b) 마운트, (c) 접근 권한을 판정해 `Availability`를 반환한다. 4 명명 조건만(D9): `Reachable` / `RootMissing` / `Unmounted` / `Inaccessible`. 세 unavailable 변이는 **best-effort 진단 라벨**이며 안전 동작(HOLD)은 동일하다. 부분가용·심링크·권한 경계 세부는 미분류(MVP 트림).

### 3.2 파괴적-빈-커밋 방지 (핵심 안전 불변식)

**규칙 R-GUARD-02 (`guard_diff` 판정 표 — 구속력 있는 total function)**: `guard_diff(new_manifest, last_committed, availability)`는 아래 표로 **완전히 결정**된다(누락·중복 없음). `empty(new)`는 `new_manifest.entries.is_empty()`.

| # | 조건 | 판정 |
|---|---|---|
| G1 | `availability != Reachable` | `HoldVaultUnavailable(reason)` (reason = availability 유래) |
| G2 | `availability == Reachable` AND `empty(new)` AND `last_committed`가 Some이고 비어있지 않음 AND `!confirm_empty` | `HoldDestructiveEmpty` |
| G3 | `availability == Reachable` AND `empty(new)` AND (`last_committed`가 None **또는** 비어있음 **또는** `confirm_empty`) | `Proceed` |
| G4 | `availability == Reachable` AND `!empty(new)` | `Proceed` |

**규칙 R-GUARD-03 (파괴적 커밋 불가 불변식 — 구속력)**: 위 표에서 **`Proceed`는 G1·G2 조건에서 절대 반환되지 않는다**. 즉:
- 볼트 도달 불가(G1)이면 어떤 매니페스트든 진행하지 않는다.
- 0-파일 매니페스트가 비어있지 않은 마지막 커밋에 대해 **확인(confirm-empty) 없이는** 진행하지 않는다(G2) — "전부 삭제" diff가 커밋으로 이어지는 경로가 **존재하지 않는다**(US-E1-06, FR-09).

**규칙 R-GUARD-04 (자동 재개 — 무상태)**: 볼트가 파일과 함께 다시 도달 가능해지면(다음 사이클에서 `availability == Reachable` AND `!empty(new)`) G4에 의해 **확인 없이 정상 diff가 재개**된다(US-E1-06 AC). 가드는 상태를 보유하지 않으므로(순수 판정) 재개가 자연히 성립한다 — hold를 해제하는 별도 절차가 필요 없다.

**규칙 R-GUARD-05 (confirm-empty 소비, D10)**: `is_confirm_empty()`는 `VaultConfig.confirm_empty` **영속 불리언 플래그**를 반영한다(1회성 토큰 아님). `true`이면 G3에 의해 진짜 빈 볼트가 수용된다(US-E1-06 AC). **잔여 리스크**: 플래그가 켜져 있는 동안 파괴적-빈-커밋 가드가 상시 비활성이다 — 이는 문서화된 수용 동작이며, 진짜 빈 볼트 의도를 명시적으로 표현하는 최소 수단이다. 확인 수용은 로그로 남긴다(표면화는 U8).

### 3.3 순수성 · 표면화 경계

**규칙 R-GUARD-06 (판정만 반환, 표면화는 U8)**: 가드는 판정(`GuardVerdict`)을 **반환**하고, 구조화 로그·`watcher status`·헬스체크로의 vault-unavailable 표면화(US-E1-06 AC "조용히 무시하지 않음")는 **U8 `SyncCycleCoordinator`가 판정을 받아 수행**한다(components.md §0 노트2). 가드는 `StatusSink`/`Logger`를 의존하지 않는다(D5). 가드는 파일시스템 stat 외 어떤 컴포넌트도 의존하지 않는 **순수 분류기**다.

---

## 4. U2 순수성 불변식 (횡단 — components.md §0 노트2 / D5)

**규칙 R-PURE-01**: `FilesystemWatcher`·`ReconciliationScheduler`·`VaultAvailabilityGuard`는 `StatusSink`/`Logger`/`HistorySink`/`CriticalEventSink`를 **주입받지 않으며 호출하지 않는다**. 세 컴포넌트는 트리거(`TriggerSignal`)와 판정(`GuardVerdict`/`ReconResult`)을 **반환**할 뿐이다. 실제 관측 push(트리거 로그 US-E1-01, vault-unavailable 표면화 US-E1-06)는 U8 코디네이터가 소유한다.

**규칙 R-PURE-02 (역참조 없음)**: U2는 U1~U8 어느 단위 크레이트도 `[dependencies]`에 넣지 않는다(`foundation`만 의존). 실제 scan/hash/diff(U1), 마지막 커밋 매니페스트 지속(U4), 사이클 직렬화(U8)는 호출 시 인자 전달 또는 상위 배선으로 성립하며 U2가 역참조하지 않는다(비순환, unit-of-work.md §3.2).

---

## 5. Testable Properties (PBT-01) — 규칙(RULES) 계층

> **확장 강제(PBT-01, Full)**: 각 속성에 카테고리 라벨 {Round-trip, Invariant, Idempotence, Commutativity, Oracle, Induction, Easy verification}과 도메인 제너레이터(PBT-07) 요구를 기재한다. 프레임워크(proptest)·제너레이터 구체 구현은 NFR Requirements/Code-gen 이월. 타입 속성은 `domain-entities.md` §7, 로직/흐름 속성은 `business-logic-model.md` §6에서 다룬다(중복 회피).

### 5.1 디바운스 (NFR-01 / FR-01)

- **PROP-U2-01 — exactly-1-trigger-per-burst** (카테고리: **Induction**/상태 기반 + **Invariant**; NFR-01·US-E1-01)
  - **속성**: 임의의 이벤트 타임라인(도착 시각 시퀀스)에 대해, "인접 간격 < T_debounce"로 정의되는 각 버스트마다 **정확히 1개** 트리거가 발행된다. 버스트 사이(간격 >= T_debounce)마다 새 트리거. 총 트리거 수 = 버스트 수.
  - **오라클**: 타임라인을 버스트로 분할하는 참조 함수(간격 >= T_debounce에서 분할). 실제 발행 트리거 수·시점이 오라클과 일치.
  - **제너레이터(PBT-07)**: 이벤트 도착 시각 시퀀스 제너레이터 — 단일 이벤트, 밀집 버스트(간격 < T_debounce), 경계 간격(정확히 T_debounce), 다중 버스트, 빈 타임라인.

- **PROP-U2-02 — 디바운스 지연 상한** (카테고리: **Invariant** / **Oracle**; NFR-01)
  - **속성**: 버스트의 마지막 이벤트 `t_last`에 대해 트리거 발행 시각은 `t_last + T_debounce`(스케줄링 epsilon 이내). 감지 지연이 `T_debounce`로 상한.
  - **제너레이터(PBT-07)**: 위와 동일 타임라인 + 다양한 `T_debounce` 값.

- **PROP-U2-03 — 오버플로 -> 트리거 보장** (카테고리: **Invariant**; D6/FR-04 백스톱)
  - **속성**: 임의 위치에 `Overflow` 신호가 포함된 타임라인에서, 그 오버플로 이후 정적 구간이 도래하면 **최소 1개 트리거**가 발행된다(오버플로가 트리거 없이 삼켜지지 않음).
  - **제너레이터(PBT-07)**: change 이벤트 + `Overflow`를 임의 위치에 삽입한 타임라인.

### 5.2 재조정 백스톱 (NFR-03 / FR-04)

- **PROP-U2-04 — 검출 지연 <= T_recon** (카테고리: **Induction**/상태 기반 + **Invariant**; NFR-03·US-E1-04)
  - **속성**: `tick`/`record_result` 명령 시퀀스와 임의의 "이벤트 유실" 시나리오에 대해, **연속한 재스냅샷(재조정 트리거 또는 진행 중 사이클) 사이의 최대 시간 간격이 `T_recon`을 초과하지 않는다** — 따라서 미관측 변경의 검출 지연 <= T_recon.
  - **오라클**: `next_due <= (마지막 재조정 발행) + T_recon` 및 busy 구간이 재조정 커버로 계산됨을 검증하는 참조 모델.
  - **제너레이터(PBT-07)**: `(now 증가 시퀀스, busy/idle 상태 전이, record_result 통지)` 명령 시퀀스 제너레이터 — 긴 busy 구간, 조밀한 tick, 경계(now == next_due).

- **PROP-U2-05 — 백엔드 무관 트리거 등가 (NFR-05 이식성)** (카테고리: **Invariant** / **Commutativity(백엔드 치환)**)
  - **속성**: 동일한 정규화 이벤트 타임라인을 서로 다른 `WatchBackend`(FSEvents/inotify/RDCW/Degraded 목)가 방출해도, 상위 디바운스 로직이 발행하는 트리거 시퀀스는 **동일**하다 — 디바운스/재조정 동작은 OS 백엔드에 의존하지 않는다.
  - **오라클**: 백엔드를 목으로 치환해 동일 타임라인 재생; 트리거 시퀀스가 백엔드 치환에 불변.
  - **제너레이터(PBT-07)**: `RawFsEvent` 타임라인 + 백엔드 목 목록. rename -> delete+create 정규화 준수(교차: `domain-entities.md` PROP-DE-U2-02).

- **PROP-U2-06 — 재조정 직렬화** (카테고리: **Invariant**; US-E1-04 AC)
  - **속성**: 진행 중 사이클(busy)인 동안 `tick`은 절대 `Some`을 반환하지 않는다(동시 재조정 없음).
  - **제너레이터(PBT-07)**: busy 구간과 겹치는 tick 호출 시퀀스.

### 5.3 가드 (US-E1-06 / FR-09)

- **PROP-U2-07 — 파괴적-빈-커밋 불가 불변식** (카테고리: **Invariant**; US-E1-06·FR-09)
  - **속성**: 임의의 (`new_manifest`, `last_committed`, `availability`, `confirm_empty`) 조합에 대해, `availability != Reachable`이거나 (`empty(new)` AND `last_committed` 비어있지 않음 AND `!confirm_empty`)이면 `guard_diff`는 **절대 `Proceed`를 반환하지 않는다**(R-GUARD-03).
  - **오라클**: §3.2 판정 표(G1~G4)가 참조 오라클 — 실제 판정이 표와 일치(total function).
  - **제너레이터(PBT-07)**: `Availability` 전 변이 x 매니페스트(빈/비빈) x `last_committed`(None/빈/비빈) x `confirm_empty`(bool) 조합 — 유한 축소 도메인(전수/근사 전수 검증, Easy verification 성격).

- **PROP-U2-08 — 자동 재개 (무상태)** (카테고리: **Invariant** / **Idempotence**; US-E1-06 AC)
  - **속성**: `availability == Reachable` AND `!empty(new)`이면 이전 판정 이력과 무관하게 항상 `Proceed`(hold 잔존 상태 없음). 동일 입력에 대한 반복 호출은 동일 판정(순수·멱등).
  - **제너레이터(PBT-07)**: hold를 유발하는 입력 뒤에 복구 입력(reachable+non-empty)을 잇는 시퀀스.

### 5.4 속성 없음(No PBT properties identified) 판정

| 규칙/요소 | 판정 | 근거 |
|---|---|---|
| R-DEB-05 pause/resume | PROP-U2-04(recon 백스톱)에 흡수 | paused 중 폐기의 정확성은 recon <= T_recon 검출로 보증 — 독립 값 속성 없음 |
| R-GUARD-01 Availability 분류 | **예제 기반 단위테스트 적합** | best-effort OS 분류(파일시스템 stat) — 속성 대상 아님(D9). 안전 동작은 PROP-U2-07이 커버 |
| R-PURE-01/02 순수성·비순환 | **컴파일타임/구조 검증** | 크레이트 의존 그래프(비순환)와 트레이트 미주입은 컴파일·리뷰로 강제 — PBT 대상 아님 |
| T_debounce/T_recon 검증 | `domain-entities.md` PROP-DE-U2-03 | config 뷰 검증 속성으로 이관 |

---

## 6. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 문서 적용 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **준수** | §5에 규칙 계층 속성 식별(PROP-U2-01~08) — exactly-1-trigger-per-burst(NFR-01), <= T_recon 백스톱(NFR-03), 백엔드 무관 등가(NFR-05), 파괴적-빈-커밋 불변식(US-E1-06). 카테고리 라벨·오라클·제너레이터(PBT-07) 요구 기재, 속성 없는 요소는 §5.4 판정. |
| **Resiliency Baseline** | ON | **부분 적용** | R-RECON-03(검출 지연 <= T_recon)이 RESILIENCY-02(무손실 검출 지연 상한)의 규칙 근거. R-GUARD-03(파괴적 커밋 불가)이 데이터 무결성 보호. R-DEB-04 오버플로 회복이 이벤트 손실 복원력. RTO/HA/DR/배포·롤백은 U2에 **N/A**(상위/인프라 소관). |
| **Security Baseline** | OFF | **N/A** | 미로딩·미강제. U2는 시크릿·네트워크 미취급. RISK-01(로컬 평문)은 문서화된 수용 위험이며 U2 산출물과 무관. |

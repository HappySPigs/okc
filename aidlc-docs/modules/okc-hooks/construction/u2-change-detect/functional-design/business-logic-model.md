# U2 Change Detection & Trigger — Business Logic Model (핵심 로직 · 알고리즘 · 데이터 흐름)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U2 `change-detect`** -> Functional Design -> 산출물 3/3 (`business-logic-model.md`)
**작성일**: 2026-09-08
**크레이트**: `change-detect` (lib) · **소속 컴포넌트**: `FilesystemWatcher`, `ReconciliationScheduler`, `VaultAvailabilityGuard`
**전제(확정 답변)**: FQ-2=A(최신 상태 대체, 폴더가 진실의 원천) · Q2(사이클)=B(트리거당 1회 직렬 사이클) · NFR-01·NFR-03·NFR-05 · MVP 결정 D1~D14(`u2-change-detect-functional-design-plan.md` §3)

> **문서 성격**: `domain-entities.md`(타입)와 `business-rules.md`(규칙) 위에서 동작하는 **핵심 로직·알고리즘·데이터 흐름**을 기술한다. 타입/규칙은 두 자매 산출물이 소유하며 이 문서는 그것을 참조한다(중복 정의 회피).
>
> **표기 규약**: 기술중립 설계. Rust스러운 시그니처는 참고용. 다이어그램은 ASCII 박스 없이 **화살표 표기(A -> B)** 와 표/순서 목록. Rust 타입은 백틱.

---

## 0. 크로스 단위 경계 (무엇이 인자로 들어오고 무엇이 밖에서 소유되나)

U2를 이해하는 핵심은 **경계**다. U2 세 컴포넌트는 순수 판정자이며 아래 표의 "밖에서 소유" 항목을 **소유하지 않는다**.

| 관심사 | 소유 단위 | U2와의 연결 |
|---|---|---|
| 볼트 스캔·재해시·매니페스트 빌드·diff | **U1** (`VaultScanner`/`ContentAddressing`/`ManifestBuilder`/`ManifestDiffer`) | U2 트리거를 받은 U8이 U1을 구동. U2는 scan/diff를 호출하지 않음 |
| 마지막 커밋 매니페스트 지속(`last_committed`) | **U4** (`SyncStateStore`) | `guard_diff`에 **인자로 전달**됨. U2는 지속·읽기 안 함 |
| 방금 재스냅샷된 `new_manifest` | **U1** (`ManifestBuilder`) 산출, **U8**이 전달 | `guard_diff`에 **인자로 전달** |
| 사이클 직렬화 잠금(트리거당 1회) | **U8** (`SyncCycleCoordinator`) | 스케줄러는 busy 여부만 관찰해 tick skip |
| 관측 push(트리거 로그, vault-unavailable 표면화, cycle_id 부여) | **U8** (`SyncCycleCoordinator`) + U6 싱크 | U2는 판정/트리거만 **반환** (D5, 노트2) |
| pause/resume 명령 발원 | **U7b** (`RunStateController`) | `FilesystemWatcher::pause/resume` 호출자 |
| config 로드·검증·리로드 팬아웃 | **U0** (`ConfigProvider`) | U2 config 뷰(`WatchConfig`/`ReconConfig`/`VaultConfig`)로 주입 |

> **U2가 소유하는 것**: (1) 디바운스 타이머 루프 -> 버스트당 1 트리거, (2) 시작/주기 재조정 스케줄링 -> 백스톱 트리거, (3) 볼트 가용성·파괴적-빈-커밋 가드 판정. 이 셋은 모두 **판정/신호를 반환**하고 부수효과(로그·상태·네트워크·지속)를 내지 않는다.

---

## 1. FilesystemWatcher — 디바운스 타이머 루프 (FR-01 / NFR-01)

### 1.1 데이터 흐름 (백엔드 -> 정규화 -> 디바운스 -> 트리거)

```
볼트 루트(디스크)
  -> [WatchBackend 구독]  OS 네이티브 이벤트          (FSEvents / inotify / RDCW / Degraded, NFR-05)
  -> [정규화]  RawFsEvent { Created | Modified | Deleted | Overflow }   (rename -> delete+create, D2)
  -> [디바운스 타이머]  이벤트마다 타이머 리셋(T_debounce)               (볼트 전체 단일 타이머, D1)
  -> [정적 구간 만료]  quiet period 도달
  -> TriggerSignal { kind: Debounced, cause_summary, observed_at }
  -> [TriggerStream]  순서 보존 push
  -> U8 SyncCycleCoordinator (유일 소비자)
```

### 1.2 디바운스 알고리즘 (순서 목록)

1. `start()` -> 백엔드 구독 개시, `WatchState = Watching`, 디바운스 타이머 미가동.
2. `RawFsEvent` 도착(change: Created/Modified/Deleted):
   - `WatchState == Paused`이면 **이벤트 폐기**하고 종료(R-DEB-05, 정확성은 recon).
   - 아니면 디바운스 타이머를 `T_debounce`로 **리셋**(이미 가동 중이면 재설정 = 버스트 합치기). `cause_summary`에 버스트 카운트 누적(진단용).
3. `RawFsEvent::Overflow` 도착: **합성 change로 취급**해 2와 동일하게 타이머 리셋(R-DEB-04, D6).
4. 타이머가 리셋 없이 `T_debounce` 만료(정적 구간 도달):
   - `TriggerSignal(kind=Debounced)` 1개를 `TriggerStream`에 push, `WatchState = Triggered -> Idle`.
   - 버스트 종료(카운트 리셋).
5. `pause()` -> `WatchState = Paused`(이후 이벤트 폐기). `resume()` -> `Watching`. `stop(self)` -> 구독 해제.

- **exactly-1-trigger-per-burst(R-DEB-01)**: 한 버스트의 다수 이벤트는 타이머를 계속 리셋하므로 만료가 1회뿐 -> 트리거 1개. **지연 상한(R-DEB-02)**: 마지막 이벤트 + T_debounce에 발행 -> NFR-01.
- **페이로드 없음(FQ-2)**: 트리거는 무엇이 바뀌었는지 담지 않는다 — 코디네이터가 재스냅샷. 디바운스는 "언제 사이클을 돌릴지"만 결정하며 "무엇을 동기화할지"는 결정하지 않는다.

### 1.3 이식성 경계 (NFR-05)

`WatchBackend` 트레이트가 OS 차이를 흡수한다 — 상위 디바운스 로직(§1.2)은 백엔드 종류를 모른다. 각 어댑터는 네이티브 API를 `RawFsEvent` 공통 어휘로 정규화한다(rename -> delete+create, 오버플로 -> `Overflow`). 완전 네이티브 구현이 MVP 초과이면 `DegradedBackend`(무이벤트)로 폴백해도 **정확성은 재조정 백스톱이 보장**한다(D3, `business-rules.md` R-RECON-03). 이 등가성은 PROP-U2-05로 검증한다.

### 1.4 조회 표면 (push 아님)

`watch_state() -> WatchState`와 `last_event_at() -> Option<Instant>`(마지막으로 관측한 `RawFsEvent`의 단조 시점, 관측 이벤트가 없으면 `None`)는 **조회만** 제공한다. CLI `status`의 "마지막 이벤트 이후 경과 시간"(US-E1-01 AC) 표면화는 U8이 `last_event_at`을 **읽어** `elapsed = now - last_event_at`로 계산·표면화한다(U2 순수성, R-PURE-01 / `business-rules.md` R-DEB-06). `Paused` 중 폐기된 이벤트는 갱신하지 않으며, 단조 시점 반환이라 벽시계 점프에 영향받지 않는 경과 계산이 가능하다.

---

## 2. ReconciliationScheduler — 백스톱 스케줄링 (FR-04 / NFR-03)

### 2.1 시작 재조정 흐름 (US-E1-03)

```
데몬 기동 (부트 자동시작 / watcher start)
  -> [run_startup_scan()]  정상 라이브 감시 진입 전에
  -> TriggerSignal { kind: Reconciliation(Startup) }
  -> next_due = 발행 시각 + T_recon 초기화 (첫 주기 백스톱 기준점, 발행 시각 앵커)
  -> U8 코디네이터 -> U1 전체 재해시 + diff (정지 중 발생 변경 복구)
```

### 2.2 주기 백스톱 tick 흐름 (US-E1-04)

```
주기 루프 (단조 시점 now 공급)
  -> [tick(now)]
       if 진행 중 사이클(busy)          -> None        (직렬화, R-RECON-06 — 진행 사이클이 재조정 커버)
       else if now < next_due           -> None
       else (발행)                      -> next_due = now + T_recon (발행 시각 = now 앵커, R-RECON-04)
                                        -> Some(TriggerSignal { kind: Reconciliation(Periodic) })
  -> (Some인 경우) U8 코디네이터 -> U1 재해시 + diff
  -> 사이클 완료 -> [record_result(outcome)] -> last_result 갱신 (next_due 는 발행 시각 + T_recon 유지, 완료 기준 재계산 아님) (R-RECON-04)
```

- **<= T_recon 상한(R-RECON-03)**: `next_due`가 항상 `<= (마지막 재조정 발행) + T_recon`이고 busy 구간은 재스냅샷 커버이므로, 연속 재스냅샷 간 최대 간격이 `T_recon`을 넘지 않는다 -> 미관측 변경 검출 지연 <= T_recon(NFR-03). PROP-U2-04로 검증.
- **스케줄 기준(D7)**: `next_due`는 **재조정 트리거 발행 시각 기준**(완료 기준 아님)으로 잡는다 — `tick`이 `Some`을 반환하는 순간 `next_due = now(발행 시각) + T_recon`으로 설정하고, `record_result`는 `last_result` 갱신만 하며 `next_due`를 완료 시각으로 재계산하지 않는다(완료 앵커는 간격을 `사이클 시간 + T_recon`으로 늘려 상한을 깨뜨린다). 발행 시각 앵커가 다음 발행까지 재-발행을 자연히 억제하므로 busy skip은 스택을 쌓지 않는다(중복 재조정 방지).
- **직렬화 소유(R-RECON-06)**: 실제 단일-사이클 잠금은 U8이 소유하며, 스케줄러는 busy 여부만 관찰한다. 스케줄러는 재해시·diff를 직접 수행하지 않는다 — 트리거만 발행(U1 위임).

### 2.3 조회 표면 (push 아님)

`next_recon_due() -> Instant`(단조), `last_result() -> Option<ReconResult>`는 **조회만** 제공한다. CLI `status`의 "다음 재조정까지 남은 시간 + 마지막 결과"(US-E1-04 AC) 표면화는 U8이 이 값을 **읽어** 수행한다(U2 순수성, R-PURE-01).

---

## 3. VaultAvailabilityGuard — 가용성·파괴적-빈-커밋 가드 (US-E1-06)

### 3.1 판정 흐름 (check_reachable -> guard_diff)

```
사이클 시작(U8 코디네이터가 구동)
  -> [check_reachable(root)]  파일시스템 stat            -> Availability { Reachable | RootMissing | Unmounted | Inaccessible }
  -> (Reachable이면) U8이 U1로 볼트 재스냅샷 -> new_manifest       (U1 소유)
  -> U4 SyncStateStore가 last_committed 제공                        (U4 소유, 인자 전달)
  -> [guard_diff(new_manifest, last_committed, availability)]      (순수 판정, R-GUARD-02 표)
       G1: availability != Reachable                          -> HoldVaultUnavailable(reason)
       G2: empty(new) & last_committed 비어있지않음 & !confirm  -> HoldDestructiveEmpty
       G3: empty(new) & (last_committed None|빈 | confirm)     -> Proceed
       G4: !empty(new)                                        -> Proceed
  -> GuardVerdict 반환
  -> U8: Proceed면 사이클 계속(diff -> preflight -> upload); Hold면 표면화(VaultUnavailable 조건 raise, held/unhealthy) + 커밋 미발행(FR-09)
```

### 3.2 핵심 알고리즘 판정

- **파괴적 커밋 불가(R-GUARD-03)**: G1·G2에서 `Proceed`가 나올 수 없다 -> 0-파일 매니페스트가 확인 없이 "전부 삭제"로 커밋되는 경로가 존재하지 않는다(US-E1-06, FR-09). PROP-U2-07로 검증.
- **자동 재개 무상태(R-GUARD-04)**: 가드는 hold 상태를 보유하지 않는다 — 볼트가 reachable+non-empty로 복구되면 G4로 즉시 `Proceed`. 별도 해제 절차 없음. PROP-U2-08로 검증.
- **confirm-empty(R-GUARD-05, D10)**: `is_confirm_empty()`는 `VaultConfig.confirm_empty` 영속 플래그. `true`면 G3로 진짜 빈 볼트 수용. 잔여 리스크(상시 가드 비활성)는 문서화된 수용 동작.
- **순수성(R-GUARD-06)**: `guard_diff`는 파일시스템 stat(check_reachable) 외 컴포넌트 의존 없음. vault-unavailable 표면화는 U8이 판정을 받아 수행("조용히 무시하지 않음" US-E1-06 AC는 U8이 충족).

---

## 4. 컴포넌트 합성 — 전체 사이클에서 U2의 위치

U2 세 컴포넌트는 **트리거 발행**(watcher/scheduler)과 **가드 판정**(guard)의 두 시점에 사이클에 관여한다. 전체 사이클(watch/debounce -> snapshot+manifest -> preflight -> have/want -> transfer -> commit)에서 U2는 **양끝(트리거 시작 + 스냅샷 직후 가드)** 을 담당한다.

```
[트리거 발행 — U2 소유]
  FilesystemWatcher(디바운스)  --\
  ReconciliationScheduler(시작/주기) --+-- TriggerStream --> U8 SyncCycleCoordinator
  (ControlPlane sync-now — U7b)     --/                         |
                                                                v  (트리거당 1회 직렬 사이클, Q2=B)
[사이클 본체 — 타 단위 소유]                                     |
  U2 VaultAvailabilityGuard.check_reachable(root) ------------- 사전 점검 (U2 판정)
       |                                                        |
       v (Reachable)                                            v
  U1 VaultScanner.scan -> ManifestBuilder.build -> new_manifest (U1)
  U4 SyncStateStore.last_committed_manifest() -> last_committed (U4)
       |                                                        |
       v                                                        v
  U2 VaultAvailabilityGuard.guard_diff(new, last, avail) ------ GuardVerdict (U2 판정)
       |
       +-- Hold  -> U8: 표면화 + 커밋 미발행 (FR-09)
       +-- Proceed -> U1 ManifestDiffer.diff -> ChangeSet -> U1 SafetyLimitsValidator -> U3 업로드 -> commit
                                                                (diff/preflight=U1, upload/commit=U3, 재시도=U4, 동의=U5)
```

- **U2 -> U8 방향만 존재**: U2는 트리거·판정을 U8에 **반환/전달**하고, U8이 나머지(scan/diff/upload/push)를 오케스트레이션한다. U2는 U1/U3/U4/U5/U6를 역참조하지 않는다(R-PURE-02, 비순환).
- **FQ-2 정합**: 트리거는 ChangeSet를 담지 않고, 매 사이클이 볼트를 재스냅샷한다 -> "무엇이 바뀌었는지"를 U2가 기억할 필요가 없다(폴더가 진실의 원천). 이것이 디바운스/재조정을 순수하고 상태-경량으로 유지하는 근거다.

---

## 5. 상태·시점 취급 노트

- **단조 시점 vs 벽시계**: 디바운스 타이머·`next_recon_due`·`observed_at`은 **단조 시점(monotonic)** — 벽시계 점프(NTP/DST)에 영향받지 않는 스케줄링용이며 지속하지 않는다. 사람 표면화용 시각(`ReconResult.at`)만 U0 `Timestamp`(UTC)를 쓴다(U0 `Timestamp` 불변식 정합).
- **지속 상태 없음**: U2는 아무 상태도 디스크에 지속하지 않는다. 재조정 스케줄 상태(`next_due`/`last_result`)는 in-memory이며 재시작 시 시작 재조정(R-RECON-01)이 초기화한다 — 크래시/재시작 무손실은 U4 `SyncStateStore`(마지막 커밋 매니페스트 + dirty + 재개 오프셋)와 시작 재조정이 함께 보장한다.

---

## 6. Testable Properties (PBT-01) — 로직/알고리즘 계층

> **확장 강제(PBT-01, Full)**: 로직·흐름 계층 속성을 식별한다. 규칙 계층 속성(PROP-U2-01~08)은 `business-rules.md` §5, 타입 속성은 `domain-entities.md` §7이 소유한다. 이 절은 **컴포넌트별 실행 위치·흐름 관점**을 명시하고 중복 재확인한다.

### 6.1 컴포넌트별 Testable-Properties 노트

| 컴포넌트 | 속성(정본 위치) | 카테고리 | 실행 위치 | 오라클/제너레이터 |
|---|---|---|---|---|
| `FilesystemWatcher` | PROP-U2-01(버스트당 1 트리거), PROP-U2-02(지연 상한), PROP-U2-03(오버플로 트리거), PROP-U2-05(백엔드 무관 등가) | Induction/Invariant/Commutativity | U2 크레이트(디바운스 로직 + 백엔드 목) | 이벤트 타임라인 제너레이터 + 버스트 분할 오라클 + 백엔드 목 치환 |
| `ReconciliationScheduler` | PROP-U2-04(검출 지연 <= T_recon), PROP-U2-06(직렬화) | Induction/Invariant | U2 크레이트(tick/record_result 상태 기반) | `(now, busy/idle, record_result)` 명령 시퀀스 + `next_due` 상한 오라클 |
| `VaultAvailabilityGuard` | PROP-U2-07(파괴적-빈-커밋 불가), PROP-U2-08(자동 재개·멱등) | Invariant/Idempotence/Easy verification | U2 크레이트(순수 `guard_diff`) | §3 판정 표(G1~G4) 오라클 + `availability x manifest x last_committed x confirm` 조합 제너레이터 |

- **NFR-03 흐름 관점(PROP-U2-04)**: 스케줄러 명령 시퀀스에 대해 실제 tick 동작이 "연속 재스냅샷 간 <= T_recon" 참조 모델과 관측적으로 동일해야 한다. 긴 busy 구간(진행 사이클)이 재조정 커버로 계산되는지가 핵심 불변식.
- **NFR-05 흐름 관점(PROP-U2-05)**: 동일 타임라인을 여러 백엔드 목으로 재생해 트리거 시퀀스가 백엔드에 불변임을 검증 — 디바운스/재조정이 OS 이식성 계층 위에서 동일하게 동작함을 증명.

### 6.2 속성 없음(No PBT properties identified) 판정

| 로직/흐름 요소 | 판정 | 근거 |
|---|---|---|
| §4 컴포넌트 합성(U2 -> U8 방향) | **구조/컴파일 검증** | 비순환·역참조 없음은 크레이트 의존 그래프로 강제 — 값 속성 아님(R-PURE-02) |
| §5 단조 시점 취급 | **No PBT properties identified** | 스케줄링 세부(비지속) — 예제 기반 단위테스트 적합 |
| `TriggerStream` 순서 보존 | 통합/계약 테스트 | 전송 메커니즘은 NFR/Code-gen 이월(D14) — U2 로직 속성 아님 |

> **제너레이터(PBT-07) 총괄**: U2 속성은 (a) 이벤트 도착 타임라인, (b) 스케줄러 명령 시퀀스(now/busy/record_result), (c) 가드 입력 조합(availability x manifest-공허성 x last_committed-유무 x confirm), (d) `WatchBackend` 목을 요구한다. 구체 구현·shrinking·시드·CI(PBT-08)와 프레임워크(proptest, PBT-09)는 NFR Requirements/Code-gen/Build-and-Test 이월이며, 여기서는 요구 사실·대상을 명시한다.

---

## 7. 확장 컴플라이언스 요약 (이 산출물 범위)

| 확장 | 활성 | 이 산출물 적용 | 판정 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제·준수** | §6에 컴포넌트별 로직/흐름 속성 실행 위치·오라클·제너레이터 명시(PROP-U2-01~08 정본은 규칙/타입 산출물). 속성 없는 요소는 §6.2 판정. |
| **Resiliency Baseline** | ON | **부분 적용** | §2 재조정 백스톱(NFR-03 <= T_recon)·§1.3 오버플로 회복(D6)·§3 파괴적-빈-커밋 가드가 RESILIENCY-02(무손실 검출 지연/데이터 무결성)의 로직 근거. §5 무지속 + 시작 재조정이 크래시/재시작 회복 흐름(U4와 협력). RTO/HA/DR/배포·롤백은 U2에 **N/A**(상위/인프라). |
| **Security Baseline** | OFF | **N/A** | 미로딩·미강제. U2는 시크릿·네트워크 미취급. RISK-01은 문서화된 수용 위험. |

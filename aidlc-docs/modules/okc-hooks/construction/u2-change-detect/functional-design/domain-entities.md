# U2 Change Detection & Trigger — Domain Entities (도메인 엔티티 / 값 타입)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U2 `change-detect`** -> Functional Design -> 산출물 1/3 (`domain-entities.md`)
**작성일**: 2026-09-08
**크레이트**: `change-detect` (lib) · **소속 컴포넌트**: `FilesystemWatcher`, `ReconciliationScheduler`, `VaultAvailabilityGuard`
**전제(확정 답변)**: FQ-1=A(표준 SHA-256, 권위 `vault_content_id`는 서버) · FQ-2=A(최신 상태 대체, 폴더가 진실의 원천, 이벤트 큐 없음) · Q2(사이클)=B(트리거당 1회 직렬 사이클) · NFR-05(크로스 OS 이식성) · MVP 결정 D1~D14(`u2-change-detect-functional-design-plan.md` §3)

> **문서 성격**: 이 문서는 U2가 **도입/특화하는 값 타입**을 정의한다. U0 `CoreTypes` 타입(`Manifest`/`ManifestEntry`/`RelativePath`/`Timestamp` 등)은 **참조만** 하며 재정의하지 않는다 — U0가 소유·구현·테스트를 완료한 타입이다. 규칙(검증·불변식·판정)은 자매 산출물 `business-rules.md`, 알고리즘·흐름은 `business-logic-model.md`가 소유하며 이 문서의 타입명·필드명을 그대로 재사용한다(중복 정의 회피).
>
> **표기 규약**: 비즈니스 의미 중심의 **기술중립 설계**다. Rust스러운 타입 시그니처는 **참고용(reference-only)** 이며 인프라·스레딩·I/O 메커니즘이 아니라 개념 형상을 표현한다. 다이어그램은 ASCII 박스 없이 **화살표 표기(A -> B)** 와 표/목록으로 기술한다. English 식별자명은 원문 유지, Rust 타입은 백틱으로 감싼다.

---

## 0. U0에서 소비하는 타입 (재정의 금지 — 참조만)

U2는 아래 U0 `CoreTypes`/`ConfigProvider` 타입을 **입력·인자로 소비**한다. 형상·불변식은 U0 `domain-entities.md`가 정본이며 여기서는 U2에서의 **용법**만 기술한다.

| U0 타입 | U2에서의 용법 |
|---|---|
| `Manifest` | `VaultAvailabilityGuard::guard_diff`의 입력(방금 재스냅샷된 `new_manifest`, U1 `ManifestBuilder` 산출). 0-엔트리 매니페스트(빈 볼트)는 U0에서 유효값이며, 이를 "전부 삭제"로 해석할지 판정하는 것이 U2 가드의 책임 |
| `Manifest`(마지막 커밋) | `guard_diff`의 `last_committed: Option<&Manifest>` — **U4 `SyncStateStore`가 소유**하고 호출 시 인자로 전달. U2는 지속하지 않음 |
| `RelativePath` | 매니페스트 엔트리·삭제 경로 식별자(가드 판정에서 파일 존재 여부 판단의 도메인 값). U2는 새 경로 타입을 만들지 않음 |
| `Timestamp` | `ReconResult`의 재조정 시각. 벽시계 UTC(U0 정의) |
| `WatcherConfig` / `ConfigSnapshot` | U2 config 뷰(§4)의 원천. U2는 `debounce_ms`/`reconciliation_interval_s`/`confirm_empty` 섹션만 소비 |

> **관측 싱크 미소비(D5)**: components.md §0 노트2에 따라 U2는 `StatusSink`/`Logger` 등 U0 싱크 트레이트를 **주입받지 않는다**(순수성). `VaultUnavailable`(`ActiveCondition`)의 실제 raise는 U8 `SyncCycleCoordinator`가 U2 판정을 받아 수행한다.

---

## 1. 트리거 타입 (FilesystemWatcher · ReconciliationScheduler 공용)

두 트리거 소스(`FilesystemWatcher` 디바운스, `ReconciliationScheduler` 시작/주기)는 **동일한 트리거 규약**으로 U8 `SyncCycleCoordinator`에 전달된다. 소스는 `TriggerKind`로 구분된다.

### 1.1 TriggerKind

- **목적**: 트리거의 **발생 소스·의미**를 구분하는 태그. 코디네이터가 사이클 로그·상태 표면화 시 트리거 성격을 구별하는 데 쓰인다(표면화는 U8; U2는 태그만 부여).
- **변이(variants)**

  | 변이 | 의미 |
  |---|---|
  | `Debounced` | `FilesystemWatcher`가 편집 버스트의 정적 구간 도달로 발행 (US-E1-01) |
  | `Reconciliation(ReconPhase::Startup)` | 데몬 기동 시 1회 전체 재조정 (US-E1-03) |
  | `Reconciliation(ReconPhase::Periodic)` | 마지막 재조정 후 T_recon 경과로 발행한 백스톱 (US-E1-04) |

  참고 형상:
  ```
  enum ReconPhase { Startup, Periodic }
  enum TriggerKind { Debounced, Reconciliation(ReconPhase) }
  ```

- **불변식**: 변이 집합은 위 3개로 폐쇄(Overflow 유래 트리거도 `Debounced`로 정규화 — 별도 변이 없음, D6). 코디네이터의 사이클 동작은 kind와 무관하게 동일한 "재스냅샷 -> diff -> ..." 경로다(FQ-2=A) — kind는 진단/로그 구분용이다.

### 1.2 TriggerSignal

- **목적**: 트리거 1건. **ChangeSet를 탑재하지 않는다**(D12, FQ-2=A) — 코디네이터가 트리거를 받아 볼트를 재스냅샷하므로 "무엇이 바뀌었는지"를 트리거가 기억할 필요가 없다.
- **필드 스키마**

  | 필드 | 개념 타입 | 의미 |
  |---|---|---|
  | `kind` | `TriggerKind` | 발생 소스/의미(§1.1) |
  | `cause_summary` | 문자열(진단용) | 트리거 유발 요약(예: "N events in burst", "periodic recon due"). 표면화·로그 보조 — 동작 결정에 쓰지 않음 |
  | `observed_at` | 단조 시점(monotonic instant) | 트리거 발행 시각(디바운스 만료·tick 도래 시점) |

  참고 형상: `struct TriggerSignal { kind: TriggerKind, cause_summary: String, observed_at: Instant }`

- **불변식**: `cause_summary`는 **진단 전용**이며 사이클 로직의 입력이 아니다(FQ-2: 폴더가 진실의 원천). `cycle_id`는 **U8이 부여**하며 트리거에 담기지 않는다(U2 순수성). `observed_at`은 벽시계가 아니라 스케줄링용 단조 시점(지속 대상 아님).
- **동등성**: 값 동등성 없음(부수 신호). 순서만 의미(발행 순서 보존, §1.3).

### 1.3 TriggerStream (트리거 채널 계약)

- **목적**: 트리거 소스 -> 코디네이터로의 **순서 보존 단방향 트리거 전달 채널**. `FilesystemWatcher::start()`가 반환하고 코디네이터가 소비한다.
- **계약 형상(기술중립)**: 발행 순서를 보존하는 순차 스트림. 구체 전송 메커니즘(mpsc 채널/async stream/콜백)은 **NFR/Code-gen 이월**(D14). U2 FD는 "순서 보존 + 단방향 push + 코디네이터가 유일 소비자"라는 계약만 고정한다.
- **불변식**: 소비자는 1개(U8 코디네이터). 디바운스로 흡수된 다중 이벤트는 스트림에 **버스트당 1개 트리거**로만 나타난다(exactly-1-trigger-per-burst, `business-rules.md` R-DEB-01).

---

## 2. FilesystemWatcher 타입

### 2.1 WatchState

- **목적**: 감시자의 조회 가능한 **런타임 상태**(coordinator/CLI 진단용). U2는 이 값을 **노출(반환)만** 하고 push하지 않는다.
- **변이**

  | 변이 | 의미 |
  |---|---|
  | `Watching` | 백엔드 구독 활성, 이벤트 대기 중 |
  | `Triggered` | 디바운스 만료로 트리거 발행 직후(전이 상태) |
  | `Idle` | 활성이나 최근 이벤트/디바운스 없음 |
  | `Paused` | pause 요청으로 감시 억제(이벤트 폐기, D13) — 데몬은 상주 |

  참고 형상: `enum WatchState { Watching, Triggered, Idle, Paused }`

- **불변식**: 집합은 위 4개로 폐쇄. `Paused` 중에는 트리거를 발행하지 않으며 도착 이벤트를 폐기한다(정확성은 recon 백스톱이 보장).

### 2.2 WatchError (taxonomy)

- **목적**: 감시 시작/백엔드 초기화 실패의 분류. `new`/`start`가 반환한다.
- **변이**

  | 변이 | 의미 | 처리 방향 |
  |---|---|---|
  | `Unsupported` | 현재 OS에 네이티브 백엔드 없음 | degraded 폴백(무이벤트 백엔드)으로 전환 가능 — recon이 정확성 보장(D3) |
  | `RootMissing` | 감시 대상 볼트 루트 부재 | 감시 미개시 — `VaultAvailabilityGuard`가 vault-unavailable로 별도 판정 |
  | `OsWatchInit(source)` | OS 감시 API 초기화 실패 | 진단 상세 동반 |
  | `Backend(source)` | 백엔드 런타임 오류 | 진단 상세 동반 |

  참고 형상: `enum WatchError { Unsupported, RootMissing, OsWatchInit(BackendCause), Backend(BackendCause) }`

- **불변식**: `WatchError`는 **감시 계층 오류**이며 vault-availability 판정(§3)과 구분된다. `RootMissing`은 여기서 감시 실패로 표면화되지만, "빈-커밋 방지" 판정은 `VaultAvailabilityGuard`가 소유한다(관심사 분리). `source`(진단 상세)는 자유형이며 무손실 표현 대상 아님(지속 안 함 — 로그 전용).

### 2.3 WatchBackend (per-OS 어댑터 계약, NFR-05)

- **목적**: OS별 네이티브 감시 API를 **정규화된 이벤트 스트림**으로 추상화하는 트레이트. NFR-05 이식성을 **트레이트 경계**로 설계 충족한다.
- **계약(개념)**: 볼트 루트 하위의 create/modify/delete/rename를 구독하고, **정규화된 raw 이벤트**로 방출한다. rename은 **delete + create 두 이벤트로 정규화**(D2). 백엔드 오버플로(inotify `IN_Q_OVERFLOW` / FSEvents 병합 / RDCW 버퍼 오버플로 / 무이벤트 네트워크 마운트)는 **`Overflow` 신호로 정규화**(D6) — 상세 개별 이벤트를 복원하지 않는다.

  참고 형상:
  ```
  enum RawFsEvent { Created(RelativePath), Modified(RelativePath), Deleted(RelativePath), Overflow }
  trait WatchBackend { /* subscribe(root) -> raw event source; 정규화는 어댑터 책임 */ }
  ```

  대상 어댑터: `FsEventsBackend`(macOS) · `INotifyBackend`(Linux) · `RdcwBackend`(Windows) · `DegradedBackend`(무이벤트 폴백 — recon만으로 정확성 성립, D3).
- **불변식(이식성 계약)**: **모든 백엔드는 동일한 `RawFsEvent` 어휘로 정규화**한다 — 상위 디바운스 로직은 백엔드 종류를 모른다(behavioral equivalence, `business-rules.md` PROP-U2-05 대상). rename -> delete+create 정규화는 전 백엔드 공통. `Overflow`는 항상 최소 1개의 합성 change로 디바운스에 투입되어 트리거를 유발한다.
- **경계**: 실제 파일 콘텐츠 읽기·해시는 U1 소유. 백엔드는 **경로·이벤트 종류만** 방출하며 콘텐츠를 읽지 않는다(FQ-2: 코디네이터가 재스냅샷).

### 2.4 FilesystemWatcher 조회 표면 (조회 전용 — push 아님)

- **목적**: coordinator/CLI가 감시자 상태를 **읽어** `watcher status`를 구성하기 위한 조회 전용 표면. U2는 값을 노출(반환)만 하고 push하지 않는다(R-PURE-01).
- **표면**

  | 조회 | 개념 반환 | 의미 |
  |---|---|---|
  | `watch_state()` | `WatchState`(§2.1) | 현재 런타임 상태 라벨 |
  | `last_event_at()` | `Option<Instant>`(단조 시점) | 마지막으로 관측한 `RawFsEvent`의 단조 시점(관측 이벤트가 없으면 `None`). U8이 `elapsed = now - last_event_at`로 "마지막 이벤트 이후 경과"(US-E1-01 AC)를 계산 |

  참고 형상: `fn watch_state(&self) -> WatchState` · `fn last_event_at(&self) -> Option<Instant>`

- **불변식**: 두 조회 모두 **순수 read-only**이며 부수효과·push가 없다(`business-rules.md` R-DEB-06 / R-PURE-01). `last_event_at`은 벽시계가 아닌 **단조 시점**(스케줄링 클록 정합, §4.2 노트)이라 지속 대상이 아니며, `Paused` 중 폐기된 이벤트는 갱신하지 않는다.

---

## 3. VaultAvailabilityGuard 타입

### 3.1 Availability

- **목적**: 볼트 루트의 **도달 가능성 분류**(스냅샷 전 사전 점검). 4 명명 조건만(D9).
- **변이**

  | 변이 | 의미 | diff 판정 영향 |
  |---|---|---|
  | `Reachable` | 루트 존재·마운트·접근 권한 OK | 정상 diff 경로 후보 |
  | `RootMissing` | 루트 경로 부재(삭제/이동) | HOLD (vault-unavailable) |
  | `Unmounted` | 마운트 지점이나 볼륨 미마운트 | HOLD (vault-unavailable) |
  | `Inaccessible` | 존재하나 권한/오류로 접근 불가 | HOLD (vault-unavailable) |

  참고 형상: `enum Availability { Reachable, RootMissing, Unmounted, Inaccessible }`

- **불변식·MVP 판정(D9)**: 세 unavailable 변이(`RootMissing`/`Unmounted`/`Inaccessible`)는 **동일한 안전 동작(HOLD)** 을 유발한다 — 구분은 **진단 라벨**일 뿐이다. 크로스 OS에서 셋을 신뢰성 있게 구분하기 어려운 경우 best-effort로 분류하되, 안전 동작은 불변이다. 부분가용·심링크·권한 경계 세부는 **미분류**(MVP 트림).

### 3.2 GuardVerdict

- **목적**: 스냅샷/diff에 대한 **가드 판정**. `guard_diff`가 반환하며, 코디네이터가 이를 받아 진행/보류를 결정하고 표면화한다(US-E1-06).
- **변이**

  | 변이 | 의미 | 코디네이터 후속(U8) |
  |---|---|---|
  | `Proceed` | 정상 diff·업로드 진행 허용 | 사이클 계속(diff -> 프리플라이트 -> 업로드) |
  | `HoldVaultUnavailable(reason)` | 볼트 도달 불가 — 보류 | `VaultUnavailable` 조건 raise + unhealthy/held 표면화, 사이클 중단 |
  | `HoldDestructiveEmpty` | 0-파일 매니페스트인데 마지막 커밋엔 파일 존재 + confirm-empty 아님 — 파괴적 전부-삭제 방지 보류 | 보류 + 표면화, 커밋 미발행(FR-09) |

  참고 형상:
  ```
  struct HoldReason(String)   // 진단 문구(Availability 변이 유래)
  enum GuardVerdict { Proceed, HoldVaultUnavailable(HoldReason), HoldDestructiveEmpty }
  ```

- **불변식(핵심 안전 계약)**: `guard_diff`는 **다음 중 하나라도 성립하면 절대 `Proceed`를 반환하지 않는다** — (a) `availability != Reachable`, (b) `new_manifest` 0-파일 AND `last_committed`가 비어있지 않음 AND `!confirm_empty`. 즉 **0-파일 매니페스트가 비어있지 않은 마지막 커밋에 대해 확인 없이 diff되어 "전부 삭제"로 이어지는 경로가 존재하지 않는다**(US-E1-06, `business-rules.md` R-GUARD-*). 판정은 **순수**(파일시스템 stat 외 컴포넌트 의존 없음)하다.

### 3.3 (참조) 파괴적-빈-커밋 판정 입력

`guard_diff(new_manifest, last_committed, availability)`의 세 입력 중 `last_committed`는 **U4 `SyncStateStore` 소유**(호출 시 전달), `new_manifest`는 **U1 `ManifestBuilder` 산출**(호출 시 전달), `availability`는 U2 `check_reachable`의 산출이다. 가드는 이 셋을 결합해 판정만 낸다 — 어느 것도 지속하지 않는다.

---

## 4. ReconciliationScheduler 타입

### 4.1 CycleOutcome

- **목적**: 코디네이터가 완료한 사이클 결과를 스케줄러에 통지(`record_result`)하는 값. 스케줄러가 `last_result` 갱신에 소비한다(`next_due`는 발행 시각 앵커로 이미 설정됨 — 완료 시각으로 재계산하지 않음, `business-rules.md` R-RECON-04).
- **변이**

  | 변이 | 의미 |
  |---|---|
  | `NoOp` | diff가 비어 업로드 미발생(변경 없음) |
  | `Committed` | 업로드·커밋 성공 |
  | `Held(reason)` | 가드 보류(vault-unavailable / destructive-empty) |
  | `Failed(reason)` | 사이클 실패(재시도는 U4 소관) |

  참고 형상: `enum CycleOutcome { NoOp, Committed, Held(String), Failed(String) }`

- **불변식**: 어느 결과든 **재조정 커버로 간주**된다(재스냅샷이 수행됐으므로). `next_due`는 재조정 트리거 **발행 시각 기준**으로 이미 설정되어 있으며, `record_result`는 `last_result` 갱신만 수행하고 `next_due`를 완료 시각으로 재계산하지 않는다(NFR-03 상한 유지, `business-rules.md` R-RECON-04).

### 4.2 ReconResult

- **목적**: 마지막 재조정의 조회 가능한 요약(`last_result`; CLI `status`가 "다음 재조정까지 남은 시간 + 마지막 결과" 표면화 시 U8이 읽음).
- **필드 스키마**

  | 필드 | 개념 타입 | 의미 |
  |---|---|---|
  | `phase` | `ReconPhase` | Startup / Periodic |
  | `outcome` | `CycleOutcome` | 그 재조정 사이클의 결과 |
  | `at` | `Timestamp` | 재조정 완료 시각(UTC) |

  참고 형상: `struct ReconResult { phase: ReconPhase, outcome: CycleOutcome, at: Timestamp }`

- **불변식**: 진단·표면화 전용. `next_recon_due()`는 별도로 단조 시점을 반환(벽시계 아님) — `ReconResult.at`(UTC)은 사람 표면화용, 스케줄링 기준은 단조 시점(§`business-logic-model.md` §2).

---

## 5. U2 Config 뷰 (WatcherConfig의 U2 소비 섹션)

U0 `WatcherConfig`(단일 JSON)의 U2 소비 섹션을 타입드 뷰로 노출한다. **필드 스키마·기본값은 U2가 확정**(U0 §2 이월 규약), 알 수 없는 키 판정은 U0 전체 통합 스키마 기준(Q4=B strict). 컴포넌트 생성 시 조립 루트(U8)가 이 뷰를 주입한다.

| 뷰(개념 타입) | 소비 컴포넌트 | 필드 | 검증 규칙(형식·범위) | 기본값 |
|---|---|---|---|---|
| `WatchConfig` | `FilesystemWatcher` | `vault_path`(U0 공통) · `debounce_ms` | `debounce_ms`: 양의 정수(> 0). 0/음수/비정수 = 검증 실패(시작 시 명확한 로그/CLI 오류, US-E1-01 AC) | `debounce_ms` 기본값은 **NFR Requirements 이월**(예: 2000ms 후보) |
| `ReconConfig` | `ReconciliationScheduler` | `reconciliation_interval_s` (T_recon) | 양의 정수(>= 1초). 0/음수 = 검증 실패 | 기본값 **NFR Requirements 이월**(예: 900s 후보) |
| `VaultConfig` | `VaultAvailabilityGuard` | `vault_path`(U0 공통) · `confirm_empty` | `confirm_empty`: 불리언(기본 `false`). 비불리언 = 검증 실패 | `confirm_empty` = `false` |

참고 형상:
```
struct WatchConfig  { vault_path: String, debounce_ms: u64 }
struct ReconConfig  { reconciliation_interval_s: u64 }
struct VaultConfig  { vault_path: String, confirm_empty: bool }
```

- **불변식**: 세 뷰는 U0 `WatcherConfig`의 **읽기 투영(read-only projection)** 이며 U2가 config를 지속·변경하지 않는다. `debounce_ms`/`reconciliation_interval_s`는 D4에 따라 **고정 duration**(적응형 없음). `confirm_empty`는 D10에 따라 **영속 플래그**(1회성 토큰 아님) — 설정 시 파괴적-빈-커밋 가드가 상시 비활성이며 이는 문서화된 수용 동작이다.
- **리로드**: `confirm_empty`/`debounce_ms`/`reconciliation_interval_s` 변경 시 U0 관찰자 팬아웃으로 U8이 재주입(구체 재적용 흐름은 U8 소관; U2는 뷰 스키마만 정의).

---

## 6. 엔티티 관계 개요 (화살표 표기 — ASCII 박스 미사용)

**합성/참조 관계 ("A -> B" = A가 B를 필드로 포함/참조):**

- `TriggerSignal` -> `TriggerKind`(1) + `cause_summary:String` + `observed_at:Instant`
- `TriggerKind::Reconciliation` -> `ReconPhase`(1)
- `TriggerStream` -- (순서 보존 전달) --> `TriggerSignal`(다수) -- 소비 --> U8 `SyncCycleCoordinator`
- `WatchBackend` -- (정규화 방출) --> `RawFsEvent`(다수) -- 디바운스 흡수 --> `TriggerSignal`(버스트당 1)
- `GuardVerdict::HoldVaultUnavailable` -> `HoldReason`(1, `Availability` 유래)
- `guard_diff(new_manifest, last_committed, availability)` -- 결합 --> `GuardVerdict`
- `ReconResult` -> `ReconPhase` + `CycleOutcome` + `Timestamp`
- `WatchConfig`/`ReconConfig`/`VaultConfig` -- 투영 --> U0 `WatcherConfig`

**소유/경계 방향(크로스 단위):**

- U0 `CoreTypes`(`Manifest`/`RelativePath`/`Timestamp`) -- depends-on --> U2 (참조만)
- U4 `SyncStateStore` -- 제공(호출 시 인자) --> `last_committed: Option<&Manifest>` --> U2 `guard_diff`
- U1 `ManifestBuilder` -- 제공(호출 시 인자) --> `new_manifest: &Manifest` --> U2 `guard_diff`
- U2 `TriggerStream`/`GuardVerdict` -- 반환(push 아님) --> U8 `SyncCycleCoordinator` -- 표면화 --> U6 싱크(U0 계약 경유)

**텍스트 설명(다이어그램 대안)**: U2의 최상위 산출은 두 부류다 — (1) `TriggerSignal`(디바운스/재조정 소스가 `TriggerStream`으로 코디네이터에 전달, ChangeSet 미탑재), (2) `GuardVerdict`(가드가 `new_manifest`+`last_committed`+`availability`를 결합해 진행/보류 판정). 두 산출 모두 **판정/신호를 반환**할 뿐 상태·로그를 push하지 않는다(U2 순수성, D5). `WatchBackend`는 OS별 네이티브 API를 `RawFsEvent` 공통 어휘로 정규화해 이식성(NFR-05)을 트레이트 경계로 성립시키며, rename은 delete+create로, 오버플로는 합성 change로 정규화된다.

---

## 7. Testable Properties (PBT-01) — 엔티티/타입 계층

> **확장 강제(PBT-01, Full)**: 타입 계층 속성만 여기서 식별한다. 디바운스·재조정·가드의 **동작 속성**은 `business-rules.md`(규칙 계층)·`business-logic-model.md`(로직 계층)가 소유한다(중복 회피). 프레임워크(proptest)·제너레이터 구체 구현은 NFR Requirements/Code-gen 이월.

- **PROP-DE-U2-01 — `TriggerKind`/`WatchState`/`Availability`/`GuardVerdict`/`CycleOutcome` 유한 열거 전역성** (카테고리: **Invariant** + **Easy verification**)
  - **속성**: 각 enum은 유한 폐쇄 변이 집합이며, 이를 소비하는 판정 함수(`business-rules.md`의 가드 판정 등)는 모든 변이에 대해 정의된 total function이다(미정의 변이 없음).
  - **제너레이터(PBT-07)**: 각 enum 전 변이 열거(유한 도메인 — 전수 검증).

- **PROP-DE-U2-02 — `WatchBackend` 이벤트 정규화 계약** (카테고리: **Invariant** / **Oracle**)
  - **속성**: 임의 백엔드가 방출한 `RawFsEvent` 시퀀스에서 rename은 항상 (delete, create) 쌍으로 나타나고, `Overflow`는 항상 최소 1개의 change로 디바운스에 투입된다(어휘 준수). 오라클 = 백엔드 무관 동일 정규화 규칙.
  - **제너레이터(PBT-07)**: OS별 원시 이벤트 타임라인 목(mock) — 동일 타임라인을 여러 백엔드가 동일 `RawFsEvent`로 정규화하는지 대조.
  - **비고**: 이 계약이 NFR-05 이식성의 값-수준 근거이며, 동작 등가 속성(디바운스가 백엔드 무관 동일 트리거)은 `business-rules.md` PROP-U2-05에서 실행.

- **PROP-DE-U2-03 — U2 config 뷰 검증** (카테고리: **Invariant**)
  - **속성**: `debounce_ms > 0`, `reconciliation_interval_s >= 1`, `confirm_empty ∈ {true,false}`를 위반하는 config는 검증 실패로 거부된다(시작 시 명확한 오류). 유효 뷰는 U0 `WatcherConfig`의 read-only 투영으로 손실 없이 도출된다.
  - **제너레이터(PBT-07)**: 경계(0·1·대값 정수) + 무효(음수/비정수) 조합 제너레이터.

### 7.1 속성 없음(No PBT properties identified) 판정

| 타입 | 판정 | 근거 |
|---|---|---|
| `TriggerSignal.cause_summary` | **No PBT properties identified** | 진단 전용 자유형 문자열 — 동작에 영향 없음, 값 속성 아님 |
| `WatchError` | **No PBT properties identified** | 오류 분류 래퍼 — 지속 안 함(로그 전용), 예제 기반 단위테스트 적합 |
| `ReconResult`/`TriggerSignal.observed_at`(단조 시점) | round-trip 대상 아님 | 스케줄링/표면화 전용, 지속 안 함(단조 클록은 지속 관심사 아님, U0 `Timestamp` 정합) |

> **제너레이터(PBT-07) 총괄**: 위 속성은 U2 도메인 제너레이터(원시 이벤트 타임라인, `Availability`x매니페스트-공허성x`confirm_empty`x`last_committed`-유무 조합, config 경계)를 요구한다. 구체 구현·shrinking·시드·CI(PBT-08)는 Code-gen/Build-and-Test 이월.

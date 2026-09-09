# U2 Change Detect — NFR Requirements (비기능 요구사항 / 품질 속성)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U2 `change-detect`** -> NFR Requirements -> 산출물 1/2 (`nfr-requirements.md`)
**작성일**: 2026-09-08
**크레이트**: `change-detect` (lib) · **소속 컴포넌트**: `FilesystemWatcher`, `ReconciliationScheduler`, `VaultAvailabilityGuard`
**입력 아티팩트**: `functional-design/domain-entities.md`·`business-rules.md`·`business-logic-model.md`(U2 FD 산출물) · `plans/u2-change-detect-functional-design-plan.md`(§3 MVP 결정 D1~D14 · §4 N/A·확장 · §5 드롭리스트) · `inception/requirements/requirements.md`(§5 NFR-01/03/05·§6 RESILIENCY-01/02·§7 RISK-01/02·§9 "Concrete defaults -> Functional/NFR Design") · `u0-foundation/nfr-requirements/{nfr-requirements.md,tech-stack-decisions.md}`(스타일 템플릿 + 워크스페이스 스택 상속) · 활성 확장 `resiliency-baseline.md`·`property-based-testing.md`(Full)
**모드**: AUTOPILOT (게이트 면제, 2026-09-08 사용자 승인) — 열린 점은 **권장안 + MVP 편향**으로 저자가 직접 결정하고 MVP 트림을 명시한다. 사용자 질문 미발행. 이미 확정된 항목(U0 스택·FD 결정 D1~D14)은 재오픈하지 않는다.
**전제(재오픈 금지)**: FQ-2=A(최신 상태 대체 재스냅샷) · Q2(사이클)=B(트리거당 1회 직렬) · D1~D14(FD) · U0 워크스페이스 스택(Rust / Edition 2024 / 고정 MSRV 1.85 / `serde`·`ciborium`·`serde_json`·`url`·`thiserror`·`arc_swap`·`proptest`) · **federated-config 해소(wave-level, DEC-FEDERATED-KEYS Q6=A)**: 비코어 config 값(`debounce_ms`/`reconciliation_interval_s`/`confirm_empty`)은 U0 `ConfigSnapshot`(코어 6필드 전용)에서 읽지 않고, U8 조립 루트가 raw config를 파싱해 **해소된 타입드 값(`Duration`·`bool`)을 U2에 생성자 주입**한다(하향 주입). U0 FROZEN.

> **문서 성격**: 이 문서는 U2가 소유하는 타입(`domain-entities.md`)·규칙(`business-rules.md`)·흐름(`business-logic-model.md`) **위에 얹는 NFR(품질 속성) 계층**이다. FD 규칙을 재기술하지 않고 **규칙 ID(R-*)·속성 ID(PROP-U2-*)로 참조**하며, 각 NFR에 근거(아티팩트 + 섹션 + NFR/규칙/PBT ID)와 수용 기준을 붙인다. 구체 크레이트·툴체인 선택의 정본은 자매 산출물 `tech-stack-decisions.md`이며, 이 문서는 각 품질 속성을 **실현하는 메커니즘**으로 그 결정을 참조한다.
>
> **표기 규약**: 기술중립을 지향하되(품질 속성 중심), tech-stack-decisions.md가 위임한 곳에서만 구체 크레이트를 인용한다. 다이어그램은 ASCII 박스 없이 **화살표 표기(A -> B)** 와 표/목록. 상한 부등호는 `<=`로 표기(유니코드 기호 미사용). English 식별자명(크레이트·타입·config 키·규칙/NFR ID)은 원문 유지, Rust 제네릭/타입(예: `Option<&Manifest>`, `Duration`)은 백틱으로 감싼다.

---

## 1. NFR 개요 및 U2 특성

U2 `change-detect`는 볼트 변경을 감지해 **동기화 사이클 트리거를 발행**하고(디바운스 + 시작/주기 재조정), 런타임 이벤트 손실에 대한 **백스톱**과 파괴적 커밋에 대한 **가드**를 제공하는 **순수 판정 lib**이다. 세 컴포넌트는 관측 push·상태 지속·네트워크를 하지 않고 **트리거·판정을 반환만** 한다(D5, R-PURE-01). 따라서 U2 NFR의 핵심은 (a) **디바운스·재조정의 시간 계약**(NFR-01 디바운스 상한 / NFR-03 <= T_recon 백스톱), (b) **크로스 OS 이식성**(NFR-05, `WatchBackend` 어댑터 경계), (c) **파괴적-빈-커밋 방지 데이터 무결성**(US-E1-06), (d) **순수성·패닉프리 계약**이다. U2는 시크릿·전송을 취급하지 않으므로 보안 강제 통제는 N/A다(§7).

**본 문서가 확정하는 U2 NFR 카탈로그(카테고리별 ID)**:

| 카테고리 | NFR ID | 요약 |
|---|---|---|
| 신뢰성 | U2-NFR-REL-01 | 디바운스 exactly-1-trigger-per-burst + 감지 지연 상한(T_debounce) |
| 신뢰성 | U2-NFR-REL-02 | 재조정 백스톱 — 미관측 변경 검출 지연 <= T_recon |
| 신뢰성 | U2-NFR-REL-03 | 백엔드 오버플로 -> 합성 트리거(이벤트 손실 즉시 회복) |
| 신뢰성 | U2-NFR-REL-04 | 파괴적-빈-커밋 가드(데이터 무결성 안전 불변식) |
| 신뢰성 | U2-NFR-REL-05 | U2 순수성 + 적대적 FS 이벤트에 패닉프리 판정 |
| 성능/리소스 | U2-NFR-PERF-01 | 감지 지연 = T_debounce 상한(구체 기본값) · 비동기 런타임 없음(std 타이머) · 정성 리소스 계약 |
| 이식성 | U2-NFR-PORT-01 | 크로스 OS(macOS/Windows/Linux) — `WatchBackend` 어댑터 경계(NFR-05) |
| 유지보수성 | U2-NFR-MNT-01 | PBT 제너레이터 전략(U0 `proptest-support` 재사용 + U2 `proptest-support` 노출) |
| 유지보수성 | U2-NFR-MNT-02 | federated config 생성자 주입(비코어 값 = `Duration`/`bool`, 라이브리로드 MVP 제외) |

---

## 2. 신뢰성 (Reliability)

U2의 신뢰성은 "**변경이 누락 없이·유계 지연으로 사이클 트리거로 변환되고, 파괴적 커밋이 구조적으로 불가능하며, 판정이 어떤 입력에도 죽지 않는 순수 계약**"을 제공하는 것이다.

### 2.1 U2-NFR-REL-01 — 디바운스 exactly-1-trigger-per-burst + 감지 지연 상한 (NFR-01)

- **요구**: `FilesystemWatcher`는 편집 **버스트**(인접 이벤트 간격이 모두 `T_debounce` 미만인 최대 구간)당 **정확히 하나**의 `TriggerSignal(kind=Debounced)`을 발행한다. 버스트의 마지막 이벤트 `t_last`에 대해 트리거는 `t_last + T_debounce`(정적 구간 만료) 시점에 발행되어 **변경 감지 지연이 `T_debounce`로 상한**이 정해진다(스케줄링 epsilon 제외). 트리거는 ChangeSet를 탑재하지 않는다(FQ-2 — 코디네이터가 재스냅샷).
- **근거**: `business-rules.md` §1 R-DEB-01(버스트당 1 트리거 불변식)·R-DEB-02(지연 상한)·R-DEB-05(paused 폐기) · `business-logic-model.md` §1.2(디바운스 알고리즘) · requirements.md §5 NFR-01(감지 지연 = 설정 가능한 디바운스로 통제) · FR-01 · US-E1-01. 실현 메커니즘(단일 볼트-전체 정적 구간 타이머, D1)의 구체 타이머·채널 선택은 정본 tech-stack-decisions.md(std `Instant`/`Duration` + `recv_timeout`, 비동기 런타임 없음).
- **수용 기준**:
  - (AC-1) 임의 이벤트 도착 타임라인에 대해 발행 트리거 수 = 버스트 수임을 참조 오라클(간격 >= T_debounce에서 분할)로 검증한다(PROP-U2-01, 카테고리 Induction/Invariant).
  - (AC-2) 버스트 마지막 이벤트 + `T_debounce` 시점에 트리거가 발행됨을 다양한 `T_debounce` 값으로 검증한다(PROP-U2-02, 카테고리 Invariant/Oracle).
  - (AC-3) `WatchState::Paused` 중 도착 이벤트는 폐기되어 트리거를 발행하지 않으며, 그 손실은 U2-NFR-REL-02(재조정)로 회복된다(R-DEB-05).
- **비고**: 구체 기본값 `T_debounce`(`debounce_ms`)는 requirements §9 "Concrete defaults -> Functional/NFR Design"에 따라 이 단계에서 확정한다 — tech-stack-decisions.md §2(기본 2000ms, MVP 고정 duration, 적응형 없음 D4).

### 2.2 U2-NFR-REL-02 — 재조정 백스톱: 미관측 변경 검출 지연 <= T_recon (NFR-03) [KEY]

- **요구**: 파일시스템 이벤트로 관측되지 못한 임의의 변경(inotify `IN_Q_OVERFLOW`·FSEvents 병합/볼륨 드롭·RDCW 버퍼 오버플로·무이벤트 네트워크 마운트·paused 중 폐기·degraded 백엔드)은 **`T_recon` 이내에 검출**된다. 형식: 임의 시점 `t`의 미관측 변경은 `t` 이후 처음 완료되는 재조정 또는 이미 진행 중인 사이클의 재스냅샷에 의해 검출되며 그 시점은 `<= t + T_recon`이다. `next_due`는 **재조정 발행 시각 앵커**(`next_due = 발행 시각 + T_recon`)로 설정되고, `record_result`는 `last_result`만 갱신하며 `next_due`를 완료 시각으로 재계산하지 않는다(완료 앵커는 간격을 `사이클 시간 + T_recon`으로 늘려 상한을 위반).
- **근거**: `business-rules.md` §2 R-RECON-01(시작 1회 전체 재조정)·R-RECON-02(주기 트리거 조건)·R-RECON-03(<= T_recon 불변식, 구속력)·R-RECON-04(발행 시각 앵커)·R-RECON-06(직렬화) · `business-logic-model.md` §2 · requirements.md §5 NFR-03(미관측 변경 검출 지연 <= T_recon)·FR-04(백스톱)·§6 RESILIENCY-02(RPO/무손실 검출 지연) · US-E1-03/04.
- **선정 근거(왜 KEY NFR인가)**: NFR-03의 zero-loss 의도에서 **event-watching이 놓친 변경의 정확성 바닥**을 U2 재조정이 단독으로 보장한다 — 이 상한이 깨지면(예: 완료 앵커 실수) FQ-2 재스냅샷 모델 전체의 무손실 보증이 무너진다. degraded 백엔드 폴백(D3)이 정당화되는 근거도 이 백스톱이다.
- **수용 기준**:
  - (AC-1) `(now 증가, busy/idle 전이, record_result)` 명령 시퀀스에 대해 **연속 재스냅샷 간 최대 간격이 `T_recon`을 초과하지 않음**을 참조 모델(`next_due <= 마지막 발행 + T_recon`, busy 구간을 재조정 커버로 계산)로 검증한다(PROP-U2-04, 카테고리 Induction/Invariant).
  - (AC-2) 진행 중 사이클(busy) 동안 `tick`은 절대 `Some`을 반환하지 않는다 — 동시 재조정 없음(PROP-U2-06, 카테고리 Invariant; 직렬화 잠금은 U8 소유, 스케줄러는 busy만 관찰).
  - (AC-3) 시작 재조정(`kind=Reconciliation(Startup)`)이 정상 라이브 감시 진입 전 1회 발행되어 정지 중 발생 변경을 복구하고 첫 주기 백스톱 앵커를 초기화한다(R-RECON-01).
- **비고**: 구체 기본값 `T_recon`(`reconciliation_interval_s`)은 이 단계에서 확정한다 — tech-stack-decisions.md §2(기본 900s = 15분, MVP 고정 duration D4). 실제 재해시·diff는 U1 위임(U2는 트리거만).

### 2.3 U2-NFR-REL-03 — 백엔드 오버플로 -> 합성 트리거(이벤트 손실 즉시 회복) (D6)

- **요구**: 백엔드가 `RawFsEvent::Overflow`를 방출하면 이를 **최소 1개의 합성 change로 취급해 디바운스에 투입** -> 정적 구간 후 트리거 1개를 발행한다. 오버플로가 트리거 없이 삼켜지지 않는다. 상세 유실 이벤트를 복원하려 시도하지 않는다(불가능 — 코디네이터 재스냅샷이 diff로 정확히 드러냄).
- **근거**: `business-rules.md` §1 R-DEB-04(오버플로 -> 합성 트리거) · `business-logic-model.md` §1.1/§1.3 · requirements.md FR-04(백스톱 근거 — inotify overflow/FSEvents coalescing/RDCW overflow/무이벤트 마운트) · plans §3 D6.
- **선정 근거**: 저비용·고가치 — 오버플로 시 검출 지연을 `T_recon`이 아닌 `T_debounce`로 회복한다. 재조정 백스톱(U2-NFR-REL-02)은 최후 안전망으로 여전히 유효하다(중복 안전).
- **수용 기준**:
  - (AC-1) 임의 위치에 `Overflow`가 삽입된 타임라인에서 그 이후 정적 구간이 도래하면 **최소 1개 트리거**가 발행된다(PROP-U2-03, 카테고리 Invariant).
  - (AC-2) `Overflow` 유래 트리거는 별도 변이 없이 `Debounced`로 정규화된다(`TriggerKind` 3-변이 폐쇄, `domain-entities.md` §1.1 불변식).

### 2.4 U2-NFR-REL-04 — 파괴적-빈-커밋 가드(데이터 무결성 안전 불변식) (US-E1-06 / FR-09)

- **요구**: `guard_diff(new_manifest, last_committed, availability)`는 §3.2 판정 표(G1~G4)로 완전히 결정되는 total function이며, 다음 중 하나라도 성립하면 **절대 `Proceed`를 반환하지 않는다** — (a) `availability != Reachable`, (b) `empty(new)` AND `last_committed` 비어있지 않음 AND `!confirm_empty`. 즉 0-파일 매니페스트가 비어있지 않은 마지막 커밋에 대해 확인 없이 "전부 삭제"로 커밋되는 경로가 **구조적으로 존재하지 않는다**. 볼트 복구 시(Reachable + non-empty) 별도 해제 절차 없이 무상태로 자동 재개된다(G4).
- **근거**: `business-rules.md` §3 R-GUARD-01(도달성 4-분류 D9)·R-GUARD-02(판정 표)·R-GUARD-03(파괴적 커밋 불가, 구속력)·R-GUARD-04(자동 재개 무상태)·R-GUARD-05(confirm-empty D10) · `business-logic-model.md` §3 · requirements.md §5 NFR-03·FR-09 · §6 RESILIENCY-02(데이터 무결성) · US-E1-06.
- **선정 근거(Resiliency 근거)**: 이 가드는 RESILIENCY-02의 데이터 무결성 보호의 U2 로직 근거다 — 마운트 드롭/루트 삭제가 "전부 삭제" 동기화로 증폭되는 경로를 차단한다. `confirm_empty=true`의 잔여 리스크(상시 가드 비활성)는 문서화된 수용 동작(D10).
- **수용 기준**:
  - (AC-1) `Availability` 전 변이 x 매니페스트(빈/비빈) x `last_committed`(None/빈/비빈) x `confirm_empty`(bool) **유한 조합**에 대해 실제 판정이 §3.2 표와 일치하고 G1·G2에서 `Proceed`가 나오지 않음을 검증한다(PROP-U2-07, 카테고리 Invariant/Easy verification, 전수/근사 전수).
  - (AC-2) hold 유발 입력 뒤 복구 입력(Reachable + non-empty)을 잇는 시퀀스에서 이력과 무관하게 `Proceed`가 나오고, 동일 입력 반복 호출은 동일 판정임을 검증한다(PROP-U2-08, 카테고리 Invariant/Idempotence).
- **비고**: vault-unavailable 표면화("조용히 무시하지 않음" US-E1-06 AC)는 U8이 판정을 받아 수행한다 — U2는 판정만 반환(R-GUARD-06, U2-NFR-REL-05).

### 2.5 U2-NFR-REL-05 — U2 순수성 + 적대적 FS 이벤트에 패닉프리 판정 (R-PURE-01/02, Q8=A 상속)

- **요구**: (a) **순수성**: `FilesystemWatcher`·`ReconciliationScheduler`·`VaultAvailabilityGuard`는 `StatusSink`/`Logger`/`HistorySink`/`CriticalEventSink`를 주입받지 않고 호출하지 않으며, U1~U8 어느 단위 크레이트도 역참조하지 않는다(`foundation`만 의존) — 트리거·판정을 **반환만** 한다. (b) **패닉프리**: 순수 판정 표면(`guard_diff`·`tick`·디바운스 정규화)은 임의의 백엔드 이벤트 입력(무순서·중복·경계 시각·적대적 경로)에 대해 패닉 없이 결정적으로 판정/트리거를 산출한다. `RawFsEvent` 정규화는 `RelativePath` 정규화 실패(U0 fallible normalize)를 패닉이 아니라 진단 처리(스킵/로그 위임)로 흡수한다.
- **근거**: `business-rules.md` §4 R-PURE-01(싱크 미주입)·R-PURE-02(역참조 없음) · `business-logic-model.md` §0·§4·§5.2 · plans §3 D5 · U0 Q8=A(순수 표면 panic-free total, 상속 규율). U0 `RelativePath::normalize`의 fallible 계약(U0-NFR-REL-02)에 정합.
- **선정 근거**: U2 세 판정자는 U8 사이클의 핫 경로에 놓인다 — 여기서 패닉하면 데몬 스레드가 죽어 감지·백스톱이 정지하고 NFR-03 무손실 목적이 무력화된다. 순수성은 PBT 테스트 가능성(백엔드 목 치환·명령 시퀀스 재생)의 전제이기도 하다.
- **수용 기준**:
  - (AC-1) 세 컴포넌트의 공개 시그니처에 `StatusSink`/`Logger`/`HistorySink`/`CriticalEventSink` 파라미터가 없고, `change-detect`의 `[dependencies]`가 `foundation`(+ FS-watch 백엔드 크레이트) 외 워크스페이스 단위 크레이트를 포함하지 않는다(컴파일/구조 검증, R-PURE-02).
  - (AC-2) `guard_diff`는 모든 입력 조합에 대해 정의된 total function으로 패닉하지 않는다(PROP-U2-07에 흡수). 디바운스·스케줄러 순수 로직은 임의 명령/이벤트 시퀀스에 대해 패닉 없이 종결한다(PROP-U2-01/04 실행 중 no-panic 관찰).
- **비고**: OS 네이티브 백엔드 어댑터의 I/O 실패는 `WatchError`(로그 전용, 지속 안 함)로 표면화되며 순수 로직 계층을 오염시키지 않는다. panic-free no-panic 속성의 케이스 수·shrinking·시드·CI(PBT-08)는 Code Generation / Build-and-Test 이월.

---

## 3. 성능 / 리소스 (Performance)

### 3.1 U2-NFR-PERF-01 — 감지 지연 = T_debounce 상한(구체 기본값) · 비동기 런타임 없음 · 정성 리소스 계약

- **요구(시간 계약 + 정성 리소스)**:
  - **감지 지연 상한 = `T_debounce`**(U2-NFR-REL-01과 결합) — 이것이 U2의 유일한 수치성 성능 계약이며, requirements §9에 따라 이 단계에서 **구체 기본값을 확정**한다: `T_debounce = 2000ms`, `T_recon = 900s`(tech-stack-decisions.md §2, MVP 고정 duration D4).
  - **비동기 런타임 없음**: U2 lib은 tokio/async-std 등 async 런타임을 도입하지 않는다. 디바운스 타이머·주기 스케줄은 std `Instant`/`Duration` + 데몬 스케줄러(상위)로 실현한다(tech-stack-decisions.md §3).
  - **정성 리소스 계약(수치 게이트 없음)**: U2는 이벤트 상세를 축적/큐잉하지 않는다(FQ-2 — 페이로드 없는 트리거, 버스트당 1). 메모리 사용은 감시 대상 트리 크기·이벤트 버스트 규모에 대해 유계이며, 스케줄러 상태(`next_due`/`last_result`)는 상수 크기다. 볼트 스캔·해시의 메모리 바운드(NFR-02, 100k 파일 / 20 GiB)는 **U1 소관**이지 U2 트리거 로직의 관심사가 아니다.
- **명시적 결정 — throughput/peak-memory 수치 게이트 없음(근거)**: (a) U2의 유일한 도메인 성능 축은 감지 지연이며 그것은 `T_debounce`로 이미 상한이 정해진다. (b) U2는 콘텐츠를 읽지 않고 경로·이벤트 종류만 다루므로 처리량 게이트를 정의할 도메인 근거가 없다. (c) 대규모 스캔·해시 처리량/메모리는 U1 소관.
- **근거**: requirements.md §5 NFR-01(디바운스)·NFR-02(스트리밍 해시 = U1)·§9(concrete defaults -> Functional/NFR Design) · `business-logic-model.md` §5(무지속·단조 시점 스케줄링) · plans §3 D1/D4/D14.
- **수용 기준**:
  - (AC-1) 감지 지연 상한 = `T_debounce`가 PROP-U2-02로 검증된다(U2-NFR-REL-01 AC-2 재사용). 기본값 2000ms/900s가 config 뷰 기본값으로 문서화된다(`domain-entities.md` §5).
  - (AC-2) U2 lib 의존 그래프에 async 런타임 크레이트가 없음을 구조 검증한다(tech-stack-decisions.md 의존성 요약표).
  - (AC-3) 어떤 절대 처리량/피크메모리 임계도 U2 게이트로 설정하지 않는다(수치 게이트 부재를 명시).

---

## 4. 이식성 (Portability — NFR-05) [KEY]

### 4.1 U2-NFR-PORT-01 — 크로스 OS via `WatchBackend` 어댑터 경계

- **요구**: U2는 macOS·Windows·Linux에서 **플랫폼 네이티브 파일시스템 감시**(FSEvents / ReadDirectoryChangesW / inotify)로 동작한다. OS 차이는 `WatchBackend` **트레이트 1개 + per-OS 어댑터**가 흡수하며, 각 어댑터는 네이티브 API를 공통 `RawFsEvent` 어휘(`Created`/`Modified`/`Deleted`/`Overflow`)로 정규화한다(rename -> delete+create D2, 오버플로 -> `Overflow` D6). 상위 디바운스·재조정 로직은 백엔드 종류를 모른다(behavioral equivalence). 완전 네이티브 어댑터가 MVP 초과이면 `DegradedBackend`(무이벤트) 폴백을 허용하며, 이 경우 **정확성 바닥은 재조정 백스톱(U2-NFR-REL-02, <= T_recon)이 보장**한다(D3).
- **근거**: `domain-entities.md` §2.3 `WatchBackend`(per-OS 어댑터 계약, NFR-05)·PROP-DE-U2-02(정규화 계약) · `business-rules.md` §5.2 PROP-U2-05(백엔드 무관 등가) · `business-logic-model.md` §1.3(이식성 경계) · requirements.md §5 NFR-05(플랫폼 네이티브 감시). 실현 크레이트(`notify`) 및 degraded 폴백 정책 정본 = tech-stack-decisions.md §3.
- **선정 근거(왜 KEY NFR인가)**: NFR-05 이식성은 **트레이트 경계로 설계 충족**한다 — 네이티브 어댑터는 best-effort이고 정확성은 recon 상한이 보증하므로, degraded 폴백이 정당한 MVP 트림이 된다(3-OS 네이티브 완비를 코드젠 이월 가능). 이 분리가 U2를 OS 무관하게 테스트 가능(백엔드 목 치환)하게 만든다.
- **수용 기준**:
  - (AC-1) 동일 정규화 이벤트 타임라인을 서로 다른 `WatchBackend` 목(FSEvents/inotify/RDCW/Degraded)이 방출해도 상위 로직이 발행하는 트리거 시퀀스가 **동일**함을 검증한다(PROP-U2-05, 카테고리 Invariant/Commutativity).
  - (AC-2) 모든 백엔드가 동일 `RawFsEvent` 어휘로 정규화하고(rename -> delete+create, overflow -> `Overflow`) 그 계약을 백엔드 목으로 대조한다(PROP-DE-U2-02).
  - (AC-3) `DegradedBackend`(무이벤트) 하에서도 재조정 백스톱만으로 미관측 변경이 <= T_recon에 검출됨이 U2-NFR-REL-02(AC-1)로 보증된다(degraded 폴백의 정확성 바닥).
- **비고**: 크로스플랫폼 **재현 빌드**(Edition 2024 + 고정 MSRV 1.85)는 U0가 워크스페이스 상속으로 전파하며 U2는 그 스택을 물려받는다(별도 NFR 신설 없음). `TriggerStream` 구체 전송(D14)은 tech-stack-decisions.md §3에서 std 채널로 확정한다.

---

## 5. 유지보수성 (Maintainability)

### 5.1 U2-NFR-MNT-01 — PBT 제너레이터 전략(U0 재사용 + U2 노출) (PBT-07/09)

- **요구**: PBT 프레임워크는 워크스페이스 상속으로 **`proptest`**(U0 PBT-09 확정)를 사용한다. U2 속성(PROP-U2-01~08, PROP-DE-U2-01~03)이 요구하는 제너레이터 — (a) `RawFsEvent` 이벤트 도착 타임라인, (b) `(now, busy/idle, record_result)` 스케줄러 명령 시퀀스, (c) 가드 입력 조합(`Availability` x 매니페스트-공허성 x `last_committed`-유무 x `confirm_empty`), (d) `WatchBackend` 목 — 을 `change-detect`의 **비기본 cargo feature `proptest-support`** 뒤에 노출한다(U0 미러링). U0 도메인 타입 제너레이터(`Manifest`·`RelativePath`·`Timestamp`)는 `foundation`의 `proptest-support` feature를 dev에서 켜 **재사용**한다(재작성 금지).
- **근거**: `business-rules.md` §5·`business-logic-model.md` §6·`domain-entities.md` §7(제너레이터 요구 총괄) · property-based-testing.md PBT-07(제너레이터 재사용성)·PBT-09(프레임워크 선택, U0 확정) · U0 `nfr-requirements.md` §4.2(U0-NFR-MNT-02) / `tech-stack-decisions.md` §7. feature 이름·게이팅 정본 = tech-stack-decisions.md.
- **수용 기준**:
  - (AC-1) `change-detect`의 제너레이터 모듈은 non-default `proptest-support` feature 뒤에 위치하며 기본/프로덕션 빌드에 `proptest`가 유출되지 않는다.
  - (AC-2) `foundation`의 `Manifest`/`RelativePath` 제너레이터를 dev-dependency의 `proptest-support` feature로 재사용하고 U2에서 재정의하지 않는다(단일 출처, 드리프트 방지).

### 5.2 U2-NFR-MNT-02 — federated config 생성자 주입(라이브리로드 MVP 제외) (DEC-FEDERATED-KEYS Q6=A)

- **요구**: U2 비코어 config 값(`debounce_ms`/`reconciliation_interval_s`/`confirm_empty`)은 U0 `ConfigSnapshot`(코어 6필드 전용)에서 읽지 않는다. U8 조립 루트가 raw config 파일을 파싱해 **해소된 타입드 값(`Duration` T_debounce·`Duration` T_recon·`bool` confirm_empty)을 U2 컴포넌트 생성자에 하향 주입**한다. U2는 `ConfigProvider`를 직접 읽지 않으며 config를 지속·변경하지 않는다(read-only 소비). config 값 검증(`debounce_ms > 0`, `reconciliation_interval_s >= 1`)은 U0 strict 검증 계층 + U8 해소 시점에서 수행되고 위반 시 시작 실패(abort, U0 R-RELOAD-04)한다.
- **선정 근거(federated 해소 정합)**: wave-level DEC-FEDERATED-KEYS Q6=A를 U2에 일관 적용한다 — U0 FROZEN(코어 필드만 타입드 노출)을 유지하면서 비코어 파라미터를 조립 루트가 주입한다. FD `domain-entities.md` §5의 config 뷰(`WatchConfig`/`ReconConfig`/`VaultConfig`)는 **주입되는 타입드 값의 개념 형상**으로 재해석되며, U2는 `WatcherConfig` 전체를 읽지 않는다.
- **MVP 트림**: federated 파라미터의 **라이브 리로드는 MVP 범위 밖**이다 — `debounce_ms`/`reconciliation_interval_s`/`confirm_empty` 변경은 **재시작으로 반영**한다. U0 코어 필드(토큰 등) 리로드만 U0 관찰자 팬아웃으로 전파되며, U2 비코어 값은 생성자 주입 시점에 고정된다. 이는 스코프를 크게 줄이는 문서화된 트림이다(재조정 백스톱이 정확성에 영향 없음 — 값 변경은 다음 기동부터 적용).
- **근거**: `domain-entities.md` §5(U2 config 뷰) · plans §3 D4/D10 · wave-level DEC-FEDERATED-KEYS Q6=A(다운워드 주입) · U0 `tech-stack-decisions.md` §3.2(federated known-key-set — U2가 자기 섹션 키 등록).
- **수용 기준**:
  - (AC-1) U2 세 컴포넌트 생성자는 해소된 타입드 값(`Duration`/`bool`)을 인자로 받고 `ConfigProvider`/`ConfigSnapshot`을 파라미터로 갖지 않는다(생성자 주입 검증).
  - (AC-2) config 뷰 검증 속성(PROP-DE-U2-03: `debounce_ms > 0`·`reconciliation_interval_s >= 1`·`confirm_empty`는 불리언)이 유지되며 위반은 시작 시 거부된다.
  - (AC-3) 라이브 리로드 미지원이 명시되고(재시작 반영), U2는 non-core 값에 대해 U0 관찰자를 구독하지 않는다.

---

## 6. 보안 (Security)

**전제**: Security Baseline 확장 = **OFF**. U2는 **시크릿·토큰·네트워크 전송을 취급하지 않는다** — 볼트 루트의 파일시스템 이벤트/stat과 매니페스트 값(경로·다이제스트)만 다룬다. 따라서 U2에는 보안 강제 통제도, 잔존 통제(TLS·토큰 위생)도 신설하지 않는다(§7 N/A 표). RISK-01(로컬 평문 산출물)·RISK-02(재승인 thrash)는 문서화된 수용 위험이며 U2 산출물과 직접 관련이 없다(토큰·config 평문은 U0/U5/U6 소관, 업로드 빈도는 U3/U8 소관). U2가 기여하는 간접 완화는 디바운스(FR-01)·재조정 커버로 업로드 트리거 빈도를 통제해 RISK-02를 부분 완화하는 것이며(requirements §7 "partial mitigations already in scope"), 이는 신규 보안 통제가 아니라 U2-NFR-REL-01/02의 기능적 부수효과다.

---

## 7. N/A 카테고리 근거표

아래 카테고리는 U2에 요구를 신설하지 않는다(발명 금지). 각 판정은 확정 세트(plans §4·requirements RESILIENCY-02/§6/§7)에서 온 것이다.

| 카테고리 | 판정 | 근거 |
|---|---|---|
| **확장성(Scalability)** | N/A | U2는 이벤트 상세를 큐잉/축적하지 않는 순수 트리거·판정 lib(FQ-2 페이로드 없는 트리거, 버스트당 1). 자체 처리량/샤딩 축 없음. 100k 파일 / 20 GiB 스캔 스케일(NFR-02)은 U1 소관. (plans §4.1) |
| **가용성(Availability)** | N/A | lib 크레이트라 SLA 없음. 전체 Watcher는 단일 사용자·사용자 재시작 로컬 프로세스라 RTO/availability-SLA가 requirements RESILIENCY-02(§6)에서 이미 N/A 확정. (requirements §6) |
| **성능 수치 목표(throughput/peak-memory)** | N/A(정성 계약 + 지연 상한만) | U2의 유일 수치 축은 감지 지연이며 `T_debounce`로 상한. 처리량/피크메모리 게이트 근거 없음(콘텐츠 미판독). 스캔·해시 메모리 바운드(NFR-02)는 U1. (§3.1) |
| **Resiliency DR / RTO / HA / 배포·롤백 / multi-AZ** | N/A | RESILIENCY-02에서 RTO/availability = N/A(단일 사용자 로컬), DR/HA/autoscale/failover(RESILIENCY-07~13)는 클라우드 워크로드 대상 N/A. U2는 순수 감지 로직이라 인프라 관심사 없음. (requirements §6) |
| **Resiliency RPO(무손실 지속)** | N/A for U2 실행(U2는 기여만) | RPO=0 zero-loss는 **U4 durable state**가 전달한다. U2는 재조정 백스톱(<= T_recon, U2-NFR-REL-02)과 시작 재조정으로 검출 지연 상한을 기여하나 상태를 지속하지 않는다(무지속, `business-logic-model.md` §5). (requirements §6, RESILIENCY-02) |
| **사용성(Usability)** | N/A | U2는 UI/CLI/트레이 표면 없는 lib. `watcher status` 표면화는 U8이 U2 조회 표면(`watch_state`/`last_event_at`/`next_recon_due`/`last_result`)을 **읽어** 수행. config 검증 오류 메시지는 U0 소관. (plans §4.1) |
| **보안 강제 통제 + 잔존 통제(Security)** | N/A | Security Baseline OFF. U2는 시크릿·토큰·네트워크 미취급 — TLS(U0-NFR-SEC-01)·토큰 위생(U0-NFR-SEC-02)이 U2에 적용될 표면 자체가 없음. RISK-01/02 수용. (§6, requirements §2.3/§7) |

---

## 8. NFR to 요구사항 추적표

각 U2 NFR을 소스 NFR ID(requirements §5) + 규칙 ID(business-rules) + 속성 ID(PBT)로 매핑한다.

| U2 NFR ID | 요약 | 소스 NFR/FR | 규칙 ID | 속성 ID(PBT) |
|---|---|---|---|---|
| U2-NFR-REL-01 | 디바운스 exactly-1-per-burst + 지연 상한 | NFR-01, FR-01, US-E1-01 | R-DEB-01/02/05 | PROP-U2-01/02 |
| U2-NFR-REL-02 | 재조정 백스톱 <= T_recon | NFR-03, FR-04, US-E1-03/04, RESILIENCY-02 | R-RECON-01/02/03/04/06 | PROP-U2-04/06 |
| U2-NFR-REL-03 | 오버플로 -> 합성 트리거 회복 | FR-04(백스톱), D6 | R-DEB-04 | PROP-U2-03 |
| U2-NFR-REL-04 | 파괴적-빈-커밋 가드(데이터 무결성) | NFR-03, FR-09, US-E1-06, RESILIENCY-02 | R-GUARD-01/02/03/04/05 | PROP-U2-07/08 |
| U2-NFR-REL-05 | U2 순수성 + 패닉프리 판정 | NFR-03(핫경로 기반) | R-PURE-01/02, R-GUARD-06 | PROP-U2-07(no-panic 흡수), U0 Q8=A 상속 |
| U2-NFR-PERF-01 | 감지 지연=T_debounce·async 없음·정성 | NFR-01, NFR-02(U1 위임), §9 defaults | R-DEB-02 | PROP-U2-02(간접) |
| U2-NFR-PORT-01 | 크로스 OS `WatchBackend` 어댑터 | NFR-05 | (WatchBackend 정규화 계약) | PROP-U2-05, PROP-DE-U2-02 |
| U2-NFR-MNT-01 | PBT 제너레이터 전략(U0 재사용 + U2 노출) | NFR-17(기반) | (제너레이터 계약) | PBT-07/PBT-09 |
| U2-NFR-MNT-02 | federated config 생성자 주입(라이브리로드 MVP 제외) | (federated 해소) | (config 뷰 read-only 투영) | PROP-DE-U2-03 |

---

## 9. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 단계 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제·준수 — PBT-09 상속 충족** | PBT-09(프레임워크)는 U0에서 `proptest`로 확정됨 -> U2는 워크스페이스 상속으로 재사용(재선택 아님, tech-stack-decisions.md §4). PBT-07(제너레이터 재사용)은 U2-NFR-MNT-01(U2 `proptest-support` feature 노출 + U0 제너레이터 재사용)로 실현. 규칙/타입/로직 계층 속성(PROP-U2-01~08, PROP-DE-U2-01~03)은 FD에서 식별 완료 -> 각 NFR 수용 기준에 매핑(§2·§4). no-panic 속성(U2-NFR-REL-05)은 PROP-U2-07/01/04 실행 중 관찰. 케이스 수·shrinking·시드·CI(PBT-08)는 Code Generation / Build-and-Test 이월. **blocking 없음.** |
| **Resiliency Baseline** | ON | **부분 적용** | RESILIENCY-01: 전체 Watcher = Critical(zero-loss 목표). U2 기여: 재조정 백스톱(U2-NFR-REL-02, <= T_recon)이 RESILIENCY-02 무손실 검출 지연 상한의 로직 근거, 파괴적-빈-커밋 가드(U2-NFR-REL-04)가 데이터 무결성 보호, 오버플로 회복(U2-NFR-REL-03)이 이벤트 손실 복원력, 시작 재조정 + 무지속(U2-NFR-REL-05)이 크래시/재시작 회복을 U4와 협력. **RTO/RPO 수치·DR·HA·배포/롤백·multi-AZ는 U2(순수 감지 lib)에 N/A**(RESILIENCY-02, §7). 신규 U2 Resiliency 인프라 결정 없음. |
| **Security Baseline** | OFF | **N/A** | 미로딩·미강제. U2는 시크릿·토큰·네트워크 미취급 — 강제/잔존 통제가 적용될 표면 없음(§6). RISK-01/02 수용. 디바운스·재조정의 업로드 빈도 통제는 RISK-02 부분 완화(기능적 부수효과, 신규 통제 아님). |

**블로킹 판정**: PBT-09가 U0에서 확정되어 U2가 상속하고 제너레이터 노출(PBT-07)이 U2-NFR-MNT-01로 실현되므로 Property-Based Testing 확장의 blocking finding은 없다. Resiliency는 부분 적용/N/A, Security는 N/A로 blocking 없음.

---

## 10. 후속 단계 이월 항목 (참고)

- PBT 케이스 수·shrinking·고정 시드·CI 통합(PBT-08) -> Code Generation / Build-and-Test.
- `TriggerStream` 구체 전송(std 채널)·디바운스 타이머 구현 세부 -> Code Generation(계약·기본값은 tech-stack-decisions.md 확정).
- 3-OS 네이티브 백엔드 어댑터 완비(FSEvents/inotify/RDCW) — MVP는 degraded 폴백 허용, 네이티브 구현은 Code Generation 강화 대상(정확성 바닥은 recon).
- patch 버전 핀 + MSRV(Edition 2024, Rust 1.85) 대비 CI 검증 -> Build-and-Test.
- 구체 크레이트/툴체인 결정(FS-watch=`notify`·std 타이머·`proptest-support` feature·기본값 T_debounce/T_recon)의 정본 -> 자매 산출물 `tech-stack-decisions.md`.

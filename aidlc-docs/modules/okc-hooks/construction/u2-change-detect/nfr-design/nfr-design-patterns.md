# U2 Change Detect — NFR Design Patterns (NFR 실현 설계 패턴)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U2 `change-detect`** -> NFR Design -> 산출물 1/2 (`nfr-design-patterns.md`)
**작성일**: 2026-09-08
**크레이트**: `change-detect` (lib) · **소속 컴포넌트(공개 애그리게이트)**: `FilesystemWatcher`, `ReconciliationScheduler`, `VaultAvailabilityGuard`
**입력 아티팩트**: `nfr-requirements/nfr-requirements.md`·`nfr-requirements/tech-stack-decisions.md`(U2 NFR Requirements 산출물, AUTOPILOT 결정 T1~T10) · `functional-design/`(domain-entities §1~§7 · business-rules R-DEB/R-RECON/R-GUARD/R-PURE · business-logic-model §0~§6) · `plans/u2-change-detect-functional-design-plan.md` §3(MVP 결정 D1~D14) · `u0-foundation/nfr-design/{nfr-design-patterns.md,logical-components.md}`(스타일 템플릿 + 상속 계약) · `inception/requirements/requirements.md`(§5 NFR-01/03/05·§6 RESILIENCY-02) · 활성 확장 `property-based-testing.md`(ON, Full)·`resiliency-baseline.md`(ON) · Security Baseline **OFF**
**규칙**: `construction/nfr-design.md` Step 6 · `common/content-validation.md`(ASCII 화살표 `A -> B`, 유니코드 화살표·박스/선-그리기 문자 미사용, 상한 `<=`, Rust 제네릭/타입 백틱) · `common/ascii-diagram-standards.md`
**모드**: AUTOPILOT (게이트 면제, 2026-09-08 사용자 승인) — 열린 점은 **권장안 + MVP 편향**으로 저자가 직접 결정하고 MVP 트림을 명시한다. 이미 확정된 항목(U0 스택·FD 결정 D1~D14·NFR-Req T1~T10)은 재오픈하지 않는다.

> **문서 성격**: 이 문서는 U2가 확정한 **품질 속성(NFR Requirements)** 을 **어떤 설계 패턴으로 실현하는가** 를 기록한다. 여기서 새 크레이트·기본값을 결정하지 않는다 — NFR Requirements(카테고리별 U2-NFR-*)·tech-stack-decisions(구체 크레이트 T1~T10)·Functional Design(규칙 R-*, 속성 PROP-U2-*)을 **설계 패턴으로 실현**한다. 각 패턴은 (a) 패턴/결정 진술, (b) 실현하는 NFR·규칙·속성 근거, (c) 구조 노트/불변식, (d) 명시적 트레이드오프로 기술한다. 논리 컴포넌트 분해 상세 맵은 자매 산출물 `logical-components.md`가 소유하며, 이 문서는 각 패턴이 어느 논리 컴포넌트에 안착하는지만 §9 추적표에서 참조한다.
>
> **표기 규약**: 화살표는 ASCII `A -> B`만 사용한다(유니코드 화살표 금지). 박스/선-그리기 문자를 쓰지 않는다. 상한 부등호는 `<=`로 표기한다. Rust 제네릭/타입/식별자(예: `Duration`, `Instant`, `Option<&Manifest>`, `dyn WatchBackend`, `std::sync::mpsc`, `Result<T, WatchError>`)는 백틱으로 감싼다. English 식별자명(크레이트·config 키·규칙/NFR/속성 ID)은 원문 유지.

---

## 1. 디바운스 단일-타이머 정적-구간 패턴 (REL-01 / REL-03) — 볼트-전체 `recv_timeout` 타이머 + 오버플로 합성 change [T2/D1/D6]

**(a) 패턴/결정**: `FilesystemWatcher`의 디바운스를 **볼트-전체 단일 정적-구간 타이머**로 실현한다. 정규화된 `RawFsEvent`는 std `std::sync::mpsc` 수신 루프로 흡수하고, 각 이벤트 도착마다 `recv_timeout(T_debounce)`의 대기 창을 리셋한다 — 무이벤트로 `T_debounce`가 경과(정적 구간 도달)하면 `TriggerSignal { kind: Debounced }` **정확히 1개**를 `TriggerStream`에 push한다. `notify-debouncer-*` 등 외부 디바운서 크레이트는 미채택(hand-rolled, T2). 파일별 디바운스·rename 추적은 없다(D2). `RawFsEvent::Overflow`는 **최소 1개 합성 change**로 동일 타이머 리셋 경로에 투입되어 오버플로가 트리거 없이 삼켜지지 않는다(D6).

**(b) NFR/규칙/속성 실현**:
- **U2-NFR-REL-01**(exactly-1-trigger-per-burst + 감지 지연 상한): 버스트의 다수 이벤트가 타이머를 계속 리셋하므로 만료는 1회뿐 -> 트리거 1개(R-DEB-01), 마지막 이벤트 `t_last` + `T_debounce`에 발행 -> 감지 지연 <= `T_debounce`(R-DEB-02). 검증 PROP-U2-01(버스트당 1)·PROP-U2-02(지연 상한).
- **U2-NFR-REL-03**(오버플로 -> 합성 트리거): `Overflow`를 합성 change로 투입해 검출 지연을 `T_recon`이 아닌 `T_debounce`로 회복(R-DEB-04). `TriggerKind` 3-변이 폐쇄 유지(`Debounced`로 정규화, 별도 변이 없음). 검증 PROP-U2-03.
- **U2-NFR-PERF-01**(감지 지연 = `T_debounce` 상한): 이 패턴이 U2의 유일한 수치성 성능 계약을 실현. 기본값 `T_debounce = 2000ms`(tech-stack-decisions.md §2, D4 고정 duration).
- **R-DEB-05**(paused 폐기): `WatchState::Paused` 중 도착 이벤트는 타이머에 투입하지 않고 폐기하며(트리거 미발행), 그 손실은 §2 재조정 백스톱이 회복한다.

**(c) 구조 노트/불변식**: 불변식 — 임의 이벤트 도착 타임라인에서 발행 트리거 수 = 버스트 수(간격 `>= T_debounce`에서 분할). 페이로드 없음(FQ-2): 트리거는 ChangeSet를 담지 않고 "언제 사이클을 돌릴지"만 결정한다. 타이머·`observed_at`은 **단조 시점(`Instant`)** 이며 벽시계 점프(NTP/DST)에 영향받지 않는다(business-logic-model.md §5). 디바운스 상태(현재 버스트 카운트·타이머 데드라인)는 in-memory이며 지속하지 않는다.

**(d) 트레이드오프**: (1) `notify-debouncer-full`/`notify-debouncer-mini` 채택은 **미채택(T2)** — U2 디바운스 의미(볼트-전체 단일 타이머·버스트당 1·오버플로 정규화)가 단순·고유해 외부 디바운서보다 hand-rolled가 작고, 순수 로직으로 백엔드 목 치환 PBT(PROP-U2-01~05)가 가능하다. (2) 파일별/경로별 세분 디바운스는 미채택 — FQ-2 재스냅샷 모델에서 "무엇이 바뀌었는가"는 무의미하므로 볼트-전체 1 타이머로 충분하며 상태가 상수 크기로 유지된다. (3) 채택안의 대가는 std 타이머 루프를 직접 관리하는 코드 규율뿐이며 런타임/의존 비용은 크레이트 0이다.

---

## 2. 재조정 백스톱 = 발행-시각 앵커 순수 `tick(now)` 스케줄러 (REL-02) [KEY, T3/D7]

**(a) 패턴/결정**: `ReconciliationScheduler`를 **단조 시점을 인자로 받는 순수 함수 `tick(now: Instant) -> Option<TriggerSignal>`** 로 실현한다. 자체 타이머·스레드를 갖지 않고, 주기 구동은 상위 데몬 스케줄러(U8)가 단조 `now`를 공급한다. `next_due`는 **재조정 트리거 발행 시각 앵커**로 설정한다 — `tick`이 `Some`을 반환하는 순간 `next_due = now + T_recon`(R-RECON-04). 사이클 완료 통지 `record_result(outcome)`은 `last_result`만 갱신하고 `next_due`를 완료 시각으로 재계산하지 않는다(완료 앵커는 간격을 `사이클 시간 + T_recon`으로 늘려 상한을 위반). 진행 중 사이클(busy) 동안 `tick`은 절대 `Some`을 반환하지 않는다(R-RECON-06 직렬화 — 실제 잠금은 U8 소유, 스케줄러는 busy만 관찰). 데몬 기동 시 라이브 감시 진입 전 시작 재조정(`kind=Reconciliation(Startup)`) 1회 발행 + `next_due` 초기화(R-RECON-01).

**(b) NFR/규칙/속성 실현**:
- **U2-NFR-REL-02**(미관측 변경 검출 지연 `<= T_recon`) [KEY]: 발행-시각 앵커로 `next_due <= (마지막 발행) + T_recon`이 항상 성립하고 busy 구간은 재스냅샷 커버로 계산되므로 연속 재스냅샷 간 최대 간격 `<= T_recon`(R-RECON-03). 이것이 event-watching이 놓친 변경(inotify overflow·paused 폐기·degraded 백엔드)의 **정확성 바닥**이다. 검증 PROP-U2-04(간격 상한)·PROP-U2-06(busy 중 동시 재조정 없음).
- **U2-NFR-PERF-01**(비동기 런타임 없음): `tick(now)`는 단조 시점 주입 순수 함수라 async 축이 없다 — tokio/async-std 미도입(T3). 스케줄러 상태(`next_due`/`last_result`)는 상수 크기(정성 리소스 계약).
- **RESILIENCY-02**(무손실 검출 지연 상한)의 U2 로직 근거이며, §5 degraded 백엔드 폴백이 정당한 MVP 트림이 되는 근거다(정확성이 recon 상한으로 보증되므로).

**(c) 구조 노트/불변식**: 불변식 — (1) `next_due`는 발행 시각 앵커라 다음 발행까지 재-발행을 자연히 억제한다(busy skip이 스택을 쌓지 않음, 중복 재조정 방지). (2) 어느 `CycleOutcome`(`NoOp`/`Committed`/`Held`/`Failed`)이든 재스냅샷이 수행됐으므로 재조정 커버로 간주한다. (3) 스케줄 기준은 단조 시점(`next_recon_due() -> Instant`), 사람 표면화용 시각은 `ReconResult.at`(U0 `Timestamp`, UTC) — 클록 분리. (4) 스케줄러는 재해시·diff를 직접 수행하지 않고 트리거만 발행한다(U1 위임).

**(d) 트레이드오프**: (1) 완료-시각 앵커(`record_result`에서 `next_due` 재계산)는 **미채택** — 간격을 `사이클 시간 + T_recon`으로 늘려 NFR-03 상한을 구조적으로 위반한다(D7 명시). (2) 스케줄러 내부 타이머 스레드·async 런타임 소유는 미채택(T3) — 순수 `tick(now)`가 PBT 명령-시퀀스 재생(임의 `now` 주입)과 U8 단일 스케줄 스레드 정합을 동시에 만족하며 스코프가 작다. (3) 채택안의 대가는 상위 데몬이 단조 시점 공급·직렬화 잠금을 소유해야 한다는 U8 계약 의존뿐이다.

---

## 3. 동시성/전송 모델 = 순수 판정 + `std::sync::mpsc` 단방향 스트림 (PERF-01 / REL-02) [T3/T4/D14]

**(a) 패턴/결정**: U2는 **비동기 런타임 없는 std-only 동시성 모델**을 채택한다. `TriggerStream`(디바운스·재조정 공용)의 구체 전송은 **`std::sync::mpsc`**(순서 보존·단방향·단일 소비자 U8)로 실현한다(T4, D14 계약). 사이클 직렬화 잠금은 **U2가 소유하지 않는다** — U8 `SyncCycleCoordinator`가 소유하고 스케줄러는 busy 여부만 관찰한다. `guard_diff`·`check_reachable`·`tick`은 상태 공유 없는 순수 판정 표면이라 자체 동기화 프리미티브가 불필요하며, 유일한 가변 상태는 디바운스 타이머 루프(단일 소유 스레드)와 스케줄러 in-memory 상태(호출자 순차 접근)다.

**(b) NFR/규칙/속성 실현**:
- **U2-NFR-PERF-01**(async 런타임 없음 · 정성 리소스): 의존 그래프에 tokio/async-std가 없음을 구조 검증(AC-2). `mpsc`는 std라 신규 크레이트 0. 이벤트 상세를 큐잉/축적하지 않아(FQ-2 페이로드 없는 트리거) 메모리가 감시 트리·버스트 규모에 유계.
- **U2-NFR-REL-02**(직렬화 정합): 단일 소비자(U8) + 순서 보존 스트림이 "트리거당 1회 직렬 사이클"(Q2=B)과 정합. `mpsc`가 발행 순서를 보존하므로 디바운스로 흡수된 다중 이벤트가 버스트당 1 트리거로만 스트림에 나타난다(R-DEB-01).
- **D14 계약**(순서 보존 + 단방향 push + 코디네이터 유일 소비자)을 std로 충족.

**(c) 구조 노트/불변식**: 불변식 — `TriggerStream` 소비자는 정확히 1개(U8). U2 -> U8 방향 채널만 존재하고 back-channel이 없다(U2 순수성 §6, R-PURE-02). 디바운스 타이머 루프가 `TriggerSignal`을 send하고 스케줄러 `tick`이 `Some(TriggerSignal)`을 반환하는 두 트리거 소스는 동일 채널 규약으로 U8에 도달한다(`domain-entities.md` §1.3).

**(d) 트레이드오프**: (1) async stream(`futures::Stream`/tokio channel) 전송은 **미채택(T4)** — 이 lib에 async 축이 없어 무이득이며 런타임 크레이트를 끌어온다. (2) 다중 소비자/브로드캐스트 채널은 미채택 — 소비자는 U8 단일이므로 SPSC-형 `mpsc`로 충분. (3) U2 내부 사이클 잠금 소유는 미채택 — 직렬화는 U8이 소유해야 트리거 소스(watcher/scheduler/control-plane)를 단일 지점에서 조정할 수 있고 U2 순수성이 보존된다. 대가는 U8 스케줄/잠금 계약 의존.

---

## 4. 크로스 OS 이식성 = `WatchBackend` 트레이트 경계 + per-OS 어댑터 + degraded 폴백 (PORT-01 / REL-05) [KEY, T1/T7/D2/D3]

**(a) 패턴/결정**: NFR-05 이식성을 **`WatchBackend` 트레이트 1개 + per-OS 어댑터**로 설계 충족한다(어댑터 패턴). 크로스플랫폼 감시 크레이트 `notify`(major `6` 핀, T1)를 어댑터 뒤에 감추고 U2는 이를 **직접 노출하지 않는다**. 각 어댑터(`FsEventsBackend`/`INotifyBackend`/`RdcwBackend`)는 네이티브 API(FSEvents/inotify/ReadDirectoryChangesW)를 공통 `RawFsEvent`(`Created`/`Modified`/`Deleted`/`Overflow`) 어휘로 정규화한다 — rename은 delete+create로(D2), 백엔드 오버플로는 `Overflow`로(D6). 상위 디바운스·재조정 로직은 백엔드 종류를 모른다(behavioral equivalence). 완전 네이티브 어댑터가 MVP 초과이면 **`DegradedBackend`(무이벤트) 폴백**을 허용하며, 이 경우 정확성 바닥은 §2 재조정 백스톱(`<= T_recon`)이 보장한다(D3, T7).

**(b) NFR/규칙/속성 실현**:
- **U2-NFR-PORT-01**(크로스 OS via `WatchBackend`): 이식성을 값이 아니라 **트레이트 경계로 실현** — 동일 정규화 타임라인을 서로 다른 백엔드 목이 방출해도 상위 트리거 시퀀스가 동일함을 검증(PROP-U2-05 백엔드 무관 등가). 어휘 정규화 계약은 PROP-DE-U2-02.
- **U2-NFR-REL-05**(패닉프리·순수성): 트레이트 경계가 OS 네이티브 I/O 실패를 순수 로직에서 격리 — 백엔드 초기화/런타임 실패는 `WatchError`(§6)로 표면화되고 디바운스 순수 계층을 오염시키지 않는다. `RawFsEvent` -> `RelativePath` 정규화 실패(U0 fallible normalize)는 패닉이 아니라 스킵/진단 위임으로 흡수.
- **RESILIENCY-02**: degraded 폴백이 이벤트 손실 복원력을 제공하되 정확성은 recon 상한이 보증(중복 안전).

**(c) 구조 노트/불변식**: 불변식(이식성 계약) — 모든 백엔드는 동일 `RawFsEvent` 어휘로 정규화하고, `Overflow`는 항상 최소 1개 합성 change로 디바운스에 투입된다(§1). rename -> delete+create 정규화는 전 백엔드 공통. `notify` 의존과 어댑터는 **U2 내부 한정**이며 공개 API에 `notify` 타입이 새지 않는다(트레이트 뒤 캡슐화). `DegradedBackend`는 이벤트를 전혀 방출하지 않으므로 디바운스 트리거가 0이고 트리거는 전적으로 재조정 소스에서만 나온다.

**(d) 트레이드오프**: (1) 3-OS 직접 FFI(`fsevent-sys`/`inotify`/winapi)는 **미채택(T1)** — 3배 표면·플랫폼 조건부 컴파일 폭증(MVP 초과). 검증된 `notify`에 위임. (2) polling 전용 자체 구현은 미채택 — NFR-05 "platform-native" 위반(단, degraded 폴백으로만 T7). (3) 3-OS 네이티브 어댑터 완비는 **Code Generation 강화로 이월** — MVP는 degraded 폴백 허용, 정확성 바닥이 recon(`<= T_recon`)이라 이 트림이 무손실 목적을 깨지 않는다(**MVP 트림**). 대가는 degraded 모드에서 검출 지연이 `T_debounce`가 아닌 최대 `T_recon`으로 늘어나는 것(문서화된 수용 동작).

---

## 5. 파괴적-빈-커밋 가드 = total-function 판정 표 안전 불변식 (REL-04) [D9/D10]

**(a) 패턴/결정**: `guard_diff(new_manifest: &Manifest, last_committed: Option<&Manifest>, availability: Availability) -> GuardVerdict`를 **입력 조합에 대해 완전히 정의된 total function**으로 실현하고, 판정 표(G1~G4, business-logic-model.md §3.2)로 결정한다. 안전 불변식: 다음 중 하나라도 성립하면 **절대 `Proceed`를 반환하지 않는다** — (a) `availability != Reachable`(G1 -> `HoldVaultUnavailable`), (b) `empty(new)` AND `last_committed` 비어있지 않음 AND `!confirm_empty`(G2 -> `HoldDestructiveEmpty`). 가드는 hold 상태를 보유하지 않는 **무상태 판정자** — 볼트가 `Reachable` + non-empty로 복구되면 G4로 즉시 `Proceed`(별도 해제 절차 없음, R-GUARD-04). `check_reachable(root)`은 파일시스템 stat만으로 `Availability` 4-분류(D9)를 산출하며, 세 unavailable 변이는 동일 안전 동작(HOLD)을 유발하고 구분은 진단 라벨일 뿐이다.

**(b) NFR/규칙/속성 실현**:
- **U2-NFR-REL-04**(데이터 무결성 안전 불변식): 0-파일 매니페스트가 비어있지 않은 마지막 커밋에 대해 확인 없이 "전부 삭제"로 커밋되는 경로가 **구조적으로 부재**(R-GUARD-03). 마운트 드롭/루트 삭제가 "전부 삭제" 동기화로 증폭되는 경로를 차단한다(RESILIENCY-02 데이터 무결성의 U2 로직 근거). 검증 PROP-U2-07(유한 조합 전수/근사 전수, G1·G2에서 `Proceed` 부재)·PROP-U2-08(자동 재개 + 멱등).
- **U2-NFR-REL-05**(패닉프리): 모든 입력 조합에 대해 정의된 total function이라 패닉 경로가 부재(no-panic이 PROP-U2-07에 흡수). `confirm_empty`는 영속 boolean 플래그(D10, 1회성 토큰 아님) — `true`면 G3로 진짜 빈 볼트 수용, 잔여 리스크(상시 가드 비활성)는 문서화된 수용 동작(R-GUARD-05).

**(c) 구조 노트/불변식**: 불변식 — `guard_diff`는 파일시스템 stat(`check_reachable`) 외 컴포넌트 의존이 없고(순수, R-GUARD-06) 어떤 입력도 지속하지 않는다. `last_committed`는 U4 `SyncStateStore` 소유(인자 전달), `new_manifest`는 U1 `ManifestBuilder` 산출(인자 전달) — 가드는 셋을 결합해 판정만 낸다. vault-unavailable 표면화("조용히 무시하지 않음" US-E1-06 AC)는 U8이 판정을 받아 수행한다(U2는 판정만 반환).

**(d) 트레이드오프**: (1) hold-상태 보유형 가드(락/플래그 유지 후 명시적 해제)는 **미채택** — 무상태 판정이 자동 재개(R-GUARD-04)를 별도 절차 없이 만족하고 멱등(동일 입력 = 동일 판정)이라 PBT가 단순. (2) `Availability` 세 unavailable 변이의 세분 동작(예: `Unmounted`만 재시도, `Inaccessible`만 알림)은 미채택 — 크로스 OS에서 신뢰성 있게 구분하기 어렵고 안전 동작은 HOLD로 불변(**MVP 트림**, D9). 부분가용·심링크·권한 경계 세부는 미분류. 대가는 진단 라벨 정확도가 best-effort라는 점(안전성에는 영향 없음).

---

## 6. U2 순수성 + 패닉프리 = sink-free 시그니처 + 순수 표면 clippy lint-gate + `WatchError` I/O 격리 (REL-05) [D5, U0 Q8=A 상속]

**(a) 패턴/결정**: U2 순수성·패닉프리를 세 겹으로 실현한다. (1) **sink-free 시그니처**: 세 컴포넌트 공개 시그니처에 `StatusSink`/`Logger`/`HistorySink`/`CriticalEventSink` 파라미터를 두지 않고 호출하지 않으며, U1~U8 어느 단위 크레이트도 역참조하지 않는다(`foundation`만 의존, R-PURE-01/02). (2) **순수 표면 clippy lint-gate**(U0 §1 미러): 순수 판정 표면(`guard_diff`·`check_reachable` 결합 로직·`tick`·디바운스 정규화)이 위치한 순수 모듈 상단에 `deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)`을 걸어 패닉프리를 컴파일타임 강제(U0 Q8=A 규율 상속, 런타임 방어 래퍼 없음). (3) **`WatchError` I/O 격리**: OS 네이티브 백엔드 어댑터의 I/O 실패는 `WatchError`(§7-오류처리, `thiserror`)로만 표면화되고 순수 로직 계층에 침투하지 않는다.

**(b) NFR/규칙/속성 실현**:
- **U2-NFR-REL-05**(순수성 + 패닉프리): 세 컴포넌트가 판정/트리거를 **반환만** 하고 부수효과(로그·상태·네트워크·지속)를 내지 않는다(R-PURE-01, D5). 순수 판정 표면은 임의의 백엔드 이벤트(무순서·중복·경계 시각·적대적 경로)에 대해 패닉 없이 결정적으로 종결(PROP-U2-01/04/07 실행 중 no-panic 관찰). 검증 AC-1(sink 파라미터 부재 + 의존 그래프 구조 검증)·AC-2(total function no-panic).
- **U2-NFR-REL-02**(핫경로 생존성): 세 판정자는 U8 사이클 핫 경로에 놓여 여기서 패닉하면 데몬 스레드가 죽어 감지·백스톱이 정지하고 NFR-03 무손실 목적이 무력화된다 — lint-gate가 이 회복력을 지탱(U0-NFR-REL-02 상속 정합).
- **U0 `RelativePath::normalize` fallible 계약**(U0-NFR-REL-02)에 정합: 정규화 실패를 패닉이 아니라 스킵/진단 위임으로 흡수.

**(c) 구조 노트/불변식**: lint-gate는 **순수 판정 모듈에만** 적용한다(가드/스케줄러/디바운스 정규화). std 타이머 루프·`WatchBackend` 어댑터의 I/O 경계는 순수 표면이 아니므로 게이트 대상이 아니며, 그 실패는 `Result<_, WatchError>`로 표면화된다. 불변식 — 순수 표면의 모든 실패 경로는 `Result::Err`(또는 total 판정 값)로만 나가며 `panic!`/`unwrap`/인덱싱 패닉 경로가 타입·lint 수준에서 부재한다. 순수성은 PBT 테스트 가능성(백엔드 목 치환·명령 시퀀스 재생)의 전제이기도 하다.

**(d) 트레이드오프**: (1) `decode`/이벤트 경계 런타임 `catch_unwind` 방어 래퍼는 **미채택** — total 순수 함수에 unwind-catch는 panic-free 계약과 상충하고 버그를 은폐한다(U0 §1 트레이드오프 상속). (2) lint 없이 코드 규율 + PBT만은 미채택 — 구조적 강제가 약해 하위 기여자가 `unwrap`/인덱싱을 재도입할 여지. (3) 채택안의 비용은 기여자가 명시적 `Result` 처리를 써야 하는 규율 부담뿐이며 런타임 비용은 0이다.

---

## 7. federated config = U8 조립 루트 타입드-값 생성자 주입 (MNT-02) [T8, DEC-FEDERATED-KEYS Q6=A]

**(a) 패턴/결정 — federated-config 해소 적용**: U2 비코어 config 값(`debounce_ms`/`reconciliation_interval_s`/`confirm_empty`)은 **U0 `ConfigSnapshot`(코어 6필드 전용)에서 읽지 않는다**. **U8 조립 루트(`watcher-bin`)가 raw config 파일을 파싱·검증·해소해 타입드 값(`Duration` T_debounce·`Duration` T_recon·`bool` confirm_empty)을 U2 세 컴포넌트 생성자에 하향 주입**한다(downward injection). U2는 `ConfigProvider`/`ConfigSnapshot`을 **직접 읽지 않으며** config를 지속·변경하지 않는다(read-only 소비). U2는 자기 섹션 키(`debounce_ms`/`reconciliation_interval_s`/`confirm_empty`)를 컴파일타임 `const &[&str]`로 노출해 U0 federated known-key-set에 등록한다(U0 nfr-design-patterns §5 계약과 대칭). config 값 검증(`debounce_ms > 0`, `reconciliation_interval_s >= 1`, `confirm_empty`는 불리언 true/false)은 U0 strict 검증 계층 + U8 해소 시점에서 수행되고 위반 시 시작 실패(abort, U0 R-RELOAD-04)한다.

**(b) NFR/규칙/속성 실현**:
- **U2-NFR-MNT-02**(생성자 주입): 세 컴포넌트 생성자는 해소된 타입드 값을 인자로 받고 `ConfigProvider`/`ConfigSnapshot`을 파라미터로 갖지 않는다(AC-1 생성자 주입 검증). FD `domain-entities.md` §5의 config 뷰(`WatchConfig`/`ReconConfig`/`VaultConfig`)는 **주입되는 타입드 값의 개념 형상**으로 재해석되며, U2는 `WatcherConfig` 전체를 읽지 않는다. 검증 속성 PROP-DE-U2-03(경계·무효 조합).
- **U0 FROZEN 유지**: wave-level DEC-FEDERATED-KEYS Q6=A를 U2에 일관 적용 — U0는 코어 6필드만 타입드 노출하고 비코어는 조립 루트가 주입한다.

**(c) 구조 노트/불변식**: 불변식 — U2는 non-core 값에 대해 `ConfigReloadObserver`를 구독하지 않는다(AC-3). 세 값은 D4에 따라 고정 duration/플래그(적응형 튜닝 없음). 값 해소·주입은 U8 조립 루트 소유, U2는 타입드 값 수신자.

**(d) 트레이드오프 — MVP 트림 명시**: federated 파라미터의 **라이브 리로드는 MVP 범위 밖**이다 — `debounce_ms`/`reconciliation_interval_s`/`confirm_empty` 변경은 **재시작으로 반영**한다(**MVP 트림**). U0 코어 필드(토큰 등) 리로드만 U0 관찰자 팬아웃으로 전파되며, U2 비코어 값은 생성자 주입 시점에 고정된다. 근거: 재조정 백스톱이 정확성에 영향 없음(값 변경은 다음 기동부터 적용) — 스코프를 크게 줄이는 문서화된 트림. 대안(U2가 non-core 리로드 관찰자 구독)은 미채택 — 라이브 재주입 흐름·타이머 재설정 로직을 신설해 스코프가 커진다.

---

## 8. PBT 실현 = `proptest` 상속 + `change-detect` `proptest-support` feature 노출 (MNT-01) [T9, PBT-07/09]

**(a) 패턴/결정**: PBT 프레임워크는 워크스페이스 상속으로 `proptest`(U0 PBT-09 확정, 재선택 아님)를 dev-dependency로 사용한다. U2 속성(PROP-U2-01~08, PROP-DE-U2-01~03)이 요구하는 제너레이터 — (a) `RawFsEvent` 이벤트 도착 타임라인, (b) `(now 증가, busy/idle 전이, record_result)` 스케줄러 명령 시퀀스, (c) 가드 입력 조합(`Availability` x 매니페스트-공허성 x `last_committed`-유무 x `confirm_empty`), (d) `WatchBackend` 목 — 을 `change-detect`의 **비기본 cargo feature `proptest-support`** 뒤에 노출한다(U0 미러). U0 도메인 타입 제너레이터(`Manifest`/`RelativePath`/`Timestamp`)는 `foundation`의 `proptest-support` feature를 dev에서 켜 **재사용**한다(재작성 금지, 단일 출처).

**(b) NFR/규칙/속성 실현**:
- **U2-NFR-MNT-01**(제너레이터 전략): PBT-07(제너레이터 재사용) + PBT-09(프레임워크 상속)를 실현. `proptest`가 U2 프로덕션 빌드에 유출되지 않도록 `proptest-support` 하의 optional dev-dependency로 게이트(AC-1). U0 제너레이터를 dev에서 재사용하고 U2에서 재정의하지 않는다(AC-2, 드리프트 방지).
- 각 패턴의 속성 정렬: §1 -> PROP-U2-01/02/03, §2 -> PROP-U2-04/06, §4 -> PROP-U2-05/PROP-DE-U2-02, §5 -> PROP-U2-07/08, §7 -> PROP-DE-U2-03.

**(c) 구조 노트/불변식**: 제너레이터 모듈은 **런타임 의존 그래프 밖**의 test-support 논리 단위다(`logical-components.md` §5.2). 비기본 feature이므로 릴리스 런타임 바이너리에 포함되지 않는다.

**(d) 트레이드오프**: 케이스 수·shrink 튜닝·고정 시드 vs 시드 로깅·CI 통합(PBT-08)은 **Code Generation / Build-and-Test 이월**(U0 동일 정책). 이 단계는 프레임워크 상속 + 제너레이터 노출만 실현한다.

---

## 9. MANDATORY 카테고리 N/A 판정표

아래 카테고리는 U2에 신규 NFR 설계 패턴을 안착시키지 않는다(발명 금지). 각 판정은 확정 세트(nfr-requirements.md §7 · requirements RESILIENCY-02/§6/§7 · plans §4)에서 온 것이다.

| 카테고리 | 판정 | 근거 |
|---|---|---|
| **Scalability** | N/A | U2는 이벤트 상세를 큐잉/축적하지 않는 순수 트리거·판정 lib(FQ-2 페이로드 없는 트리거, 버스트당 1). 자체 처리량/샤딩 축 없음. 100k 파일 / 20 GiB 스캔 스케일(NFR-02)은 U1 소관. 신규 확장성 패턴 없음(nfr-requirements.md §7) |
| **Performance(수치 목표)** | N/A(정성 계약 + 지연 상한만) | U2의 유일 수치 축은 감지 지연이며 §1 `T_debounce` 상한으로 이미 정해진다. throughput/peak-memory 게이트 근거 없음(콘텐츠 미판독). 스캔·해시 메모리 바운드(NFR-02)는 U1. 신규 성능 패턴 없음(§1·§2 정성 리소스 계약이 전부) |
| **Availability** | N/A | lib 크레이트라 SLA 없음. 전체 Watcher는 단일 사용자·사용자 재시작 로컬 프로세스라 RTO/availability-SLA가 RESILIENCY-02(§6)에서 이미 N/A 확정 |
| **Resiliency DR / RTO / HA / 배포·롤백 / multi-AZ / 서킷브레이커 / auto-scaling** | N/A | RESILIENCY-02에서 RTO/availability = N/A(단일 사용자 로컬), DR/HA/autoscale/failover(RESILIENCY-05~14)는 클라우드 워크로드 대상 N/A. U2는 순수 감지 로직이라 인프라 관심사 없음 |
| **Resiliency RPO(무손실 지속)** | N/A for U2 실행(U2는 기여만) | RPO=0 zero-loss는 U4 durable state가 전달. U2는 재조정 백스톱(§2)·시작 재조정으로 검출 지연 상한을 기여하나 상태를 지속하지 않는다(무지속, business-logic-model.md §5) |
| **Usability** | N/A | U2는 UI/CLI/트레이 표면 없는 lib. `watcher status` 표면화는 U8이 U2 조회 표면(`watch_state`/`last_event_at`/`next_recon_due`/`last_result`)을 **읽어** 수행. config 검증 오류 메시지는 U0 소관 |
| **Security(강제 + 잔존 통제)** | N/A | Security Baseline OFF. U2는 시크릿·토큰·네트워크 미취급 — TLS(U0-NFR-SEC-01)·토큰 위생(U0-NFR-SEC-02)이 적용될 표면 자체가 없음. `notify`는 로컬 FS 감시만. RISK-01/02 수용. 디바운스·재조정의 업로드 빈도 통제는 RISK-02 부분 완화(기능적 부수효과, 신규 통제 아님) |
| **Logical Components** | 다뤄짐(N/A 아님) | Application Design은 공개 3 컴포넌트만 확정, FD는 타입·규칙·흐름만 정의 — 내부 논리 분해(명명·책임·의존·트레이트 seam)는 NFR Design에서 결정. 상세는 자매 산출물 `logical-components.md`가 소유(추적성 문서 맵, 물리 모듈 증식 강제 아님) |

---

## 10. Autopilot Decisions (질문 대체 — 권장안 + MVP 편향)

> NFR-Req 단계 T1~T10을 재오픈하지 않으며, 이 표는 **NFR Design 단계에서 패턴 형상을 확정한 결정**만 기록한다(설계 실현 수준).

| # | 주제(topic) | 채택안(chosen) | MVP 트림? | 근거(rationale) |
|---|---|---|---|---|
| ND1 | 디바운스 실현 형상 | 볼트-전체 단일 정적-구간 타이머(std `Instant`/`recv_timeout`), 오버플로 합성 change 투입(§1) | 예 | hand-rolled가 U2 고유 의미(버스트당 1·오버플로 정규화)에 최소·순수. 외부 디바운서 크레이트 회피(T2) |
| ND2 | 재조정 스케줄러 형상 | 발행-시각 앵커 순수 `tick(now) -> Option<TriggerSignal>`, `record_result`는 `last_result`만 갱신(§2) | 아니오 | 완료-앵커는 NFR-03 상한 위반. 순수 tick이 PBT 재생 + U8 스케줄 정합 + async 회피를 동시 충족(T3, D7) |
| ND3 | 동시성/전송 모델 | async 런타임 없음 + `std::sync::mpsc` 단방향 단일-소비자 스트림, 사이클 잠금 U8 소유(§3) | 예 | D14 계약을 std로 충족, 신규 크레이트 0. U2 순수성 보존(T4, R-PURE-02) |
| ND4 | 이식성 경계 실현 | `WatchBackend` 트레이트 + per-OS 어댑터, `notify`를 어댑터 뒤 캡슐화, `DegradedBackend` 폴백 허용(§4) | 예 | NFR-05를 트레이트 경계로 충족. degraded 폴백의 정확성 바닥 = recon 상한. 3-OS 네이티브 완비는 Code Gen 이월(T1/T7, D3) |
| ND5 | 파괴적-빈-커밋 가드 실현 | total-function 판정 표(G1~G4) 무상태 판정, unavailable 3-변이 동일 HOLD(§5) | 예 | 무상태가 자동 재개·멱등을 별도 절차 없이 만족. unavailable 세분 동작 미도입(D9/D10) |
| ND6 | 패닉프리 강제 | 순수 판정 모듈에 clippy lint-gate(`deny unwrap/expect/indexing_slicing/panic`), 런타임 방어 래퍼 없음(§6, U0 §1 미러) | 아니오 | 컴파일타임 강제가 핫경로 생존성(NFR-03)을 지탱. `catch_unwind` 방어는 계약 상충으로 미채택(U0 Q8=A 상속) |
| ND7 | federated config 해소 | U8 조립 루트가 타입드 값(`Duration`/`bool`) 생성자 주입, U2는 `ConfigProvider` 미읽기, non-core 라이브리로드 제외(§7) | 예 | wave-level DEC-FEDERATED-KEYS Q6=A 적용. U0 FROZEN 유지. 라이브리로드 제외로 스코프 대폭 축소(재시작 반영) |
| ND8 | PBT 제너레이터 노출 | `change-detect`의 비기본 `proptest-support` feature 노출 + U0 제너레이터 dev 재사용(§8) | 아니오 | U0 미러(단일 출처·프로덕션 유출 방지). PBT-07/09 실현(T9) |

---

## 11. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 단계 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **준수 — 이 단계 blocking 없음** | PBT-09(프레임워크 `proptest`)는 NFR-Req에서 상속 충족(재선택 아님). PBT-01(속성 식별)은 FD 완료. 이 단계의 각 패턴이 확정 속성에 정렬: §1 -> PROP-U2-01/02/03, §2 -> PROP-U2-04/06, §4 -> PROP-U2-05/PROP-DE-U2-02, §5 -> PROP-U2-07/08, §7 -> PROP-DE-U2-03. no-panic(§6)은 PROP-U2-07/01/04 실행 중 관찰. PBT-07(제너레이터 재사용)은 §8 + `logical-components.md` §5로 실현. PBT-08(케이스/시드/CI)은 Code Generation/Build-and-Test 이월 |
| **Resiliency Baseline** | ON | **준수(부분 적용 + 대체로 N/A) — blocking 없음** | RESILIENCY-01: 전체 Watcher = Critical(zero-loss 목표). U2 기여의 설계 실현: 재조정 백스톱(§2, `<= T_recon`)·오버플로 회복(§1, D6)·파괴적-빈-커밋 가드(§5)·시작 재조정 + 무지속(§2/§6, U4 협력). degraded 폴백(§4)의 정확성 바닥도 recon 상한. RTO/RPO 수치·DR·HA·배포/롤백·multi-AZ·서킷브레이커·auto-scaling·카오스는 순수 감지 lib에 **N/A**(§9). 신규 U2 resiliency 인프라 결정 없음 |
| **Security Baseline** | OFF | **N/A — 미로딩·미강제** | U2는 시크릿·토큰·네트워크 미취급 — 강제/잔존 통제가 적용될 표면 없음(§9). `notify`는 로컬 FS 감시만. RISK-01/02 수용. 디바운스·재조정의 업로드 빈도 통제는 RISK-02 부분 완화(기능적 부수효과, 신규 통제 아님) |

**블로킹 판정**: 이 단계에 blocking finding 없음. PBT-09는 NFR-Req에서 충족되어 U2가 상속하고 제너레이터 노출(§8)이 U2-NFR-MNT-01을 실현하며, Resiliency는 부분 적용/N/A, Security는 N/A다.

---

## 12. 추적표 (설계 패턴 -> NFR ID -> 규칙 ID -> 속성 -> 논리 컴포넌트)

| 설계 패턴 | NFR ID | 규칙 ID | 속성(PBT) | 안착 논리 컴포넌트 |
|---|---|---|---|---|
| §1 디바운스 단일-타이머 + 오버플로 합성 | U2-NFR-REL-01, U2-NFR-REL-03, U2-NFR-PERF-01 | R-DEB-01/02/04/05 | PROP-U2-01/02/03 | `FilesystemWatcher` -> EventNormalizer · DebounceTimer · TriggerEmitter |
| §2 발행-시각 앵커 순수 `tick` 스케줄러 | U2-NFR-REL-02 [KEY], U2-NFR-PERF-01 | R-RECON-01/02/03/04/06 | PROP-U2-04/06 | `ReconciliationScheduler` -> ReconTick · ReconStateStore |
| §3 std-only 동시성 + `mpsc` 스트림 | U2-NFR-PERF-01, U2-NFR-REL-02 | R-DEB-01(순서), R-RECON-06 | (통합/계약 테스트, D14) | TriggerModel(`TriggerStream`) · DebounceTimer |
| §4 `WatchBackend` 어댑터 + degraded 폴백 | U2-NFR-PORT-01 [KEY], U2-NFR-REL-05 | (WatchBackend 정규화 계약) | PROP-U2-05, PROP-DE-U2-02 | `FilesystemWatcher` -> WatchBackend(trait) · per-OS Adapters · DegradedBackend |
| §5 파괴적-빈-커밋 total-function 가드 | U2-NFR-REL-04, U2-NFR-REL-05 | R-GUARD-01/02/03/04/05/06 | PROP-U2-07/08 | `VaultAvailabilityGuard` -> ReachabilityChecker · DiffGuard |
| §6 순수성 + clippy lint-gate + `WatchError` 격리 | U2-NFR-REL-05, U2-NFR-REL-02 | R-PURE-01/02, R-GUARD-06 | PROP-U2-07(no-panic 흡수), U0 Q8=A 상속 | 세 컴포넌트 순수 표면 · ErrorSurface(`WatchError`) |
| §7 federated config 생성자 주입 | U2-NFR-MNT-02 | (config 뷰 read-only 투영) | PROP-DE-U2-03 | ConfigViews(`WatchConfig`/`ReconConfig`/`VaultConfig` 개념 형상) |
| §8 `proptest` 상속 + `proptest-support` 노출 | U2-NFR-MNT-01 | (제너레이터 계약) | PBT-07/PBT-09 | ProptestGenerators(test-support, 런타임 그래프 밖) |

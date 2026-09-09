# U4 sync-state — NFR Requirements (비기능 요구사항 / 품질 속성)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U4 Resilience & Retry** -> NFR Requirements -> 산출물 1/2 (`nfr-requirements.md`)
**작성일**: 2026-09-08
**크레이트**: `sync-state` (lib) · **소속 컴포넌트**: `SyncStateStore`, `RetryBackoffController`
**입력 아티팩트**: `functional-design/domain-entities.md`·`business-rules.md`·`business-logic-model.md`(U4 FD 산출물) · `plans/u4-sync-state-functional-design-plan.md`(§3 AUTOPILOT 결정 D-01..D-14) · `inception/requirements/requirements.md`(§5 NFR-03/04/11/13 · §6 Resiliency 매핑 · §7 RISK-01) · 활성 확장 `property-based-testing.md`(PBT-09 이 단계 강제 확인)·`resiliency-baseline.md` · U0 NFR Requirements 산출물(스타일·NFR-id/추적표 템플릿)
**전제(재오픈 금지)**: FQ-2=A(최신 상태 대체, 폴더가 진실의 원천) · Q2=B(트리거당 1회 직렬 사이클) · Q6=C(경로->해시 맵) · U4 FD 결정 D-01..D-14 확정 · U0 FROZEN(코덱·`CoreTypes`·`ConfigProvider`·오류 taxonomy) · Federated config = watcher-bin(U8)이 raw config 파싱 -> 타입드 값(`Duration`·`PathBuf`) 하향 주입(U4는 `ConfigProvider` 미독취) · Live-reload of federated params = OUT of MVP(재시작)

> **문서 성격**: 이 문서는 U4가 소유하는 타입(`domain-entities.md`)·규칙(`business-rules.md`)·흐름(`business-logic-model.md`) **위에 얹는 NFR(품질 속성) 계층**이다. FD의 비즈니스 규칙을 재기술하지 않고 **규칙 ID(R-*)·속성 ID(PROP-U4-*)로 참조**하며, 각 NFR에 근거(아티팩트 + 섹션 + 소스 NFR/규칙/PBT ID)와 수용 기준을 붙인다. 구체 크레이트·기법 선택의 정본은 자매 산출물 `tech-stack-decisions.md`이며, 이 문서는 각 품질 속성을 **실현하는 메커니즘**으로 그 결정을 참조한다.
>
> **표기 규약**: 기술중립을 지향하되(품질 속성 중심), tech-stack-decisions.md가 위임한 곳에서만 구체 크레이트를 인용한다. 다이어그램은 ASCII 박스 없이 **화살표 표기(A -> B)** 와 표/목록으로 기술한다. English 식별자(크레이트·타입·config 키·규칙/NFR ID)는 원문 그대로 유지하고 Rust 제네릭/타입(예: `Vec<u8>`, `BTreeMap<Sha256Digest, ByteCount>`)은 백틱으로 감싼다.

---

## 1. NFR 개요 및 U4 특성

U4 `sync-state`는 **"장애 복구(failure recovery)" 테마의 얇은 lib**로, 두 개의 상태를 공유하지 않는 컴포넌트로 구성된다: (a) `SyncStateStore` — 마지막 커밋 매니페스트 + dirty 신호 + 진행 중 재개 오프셋을 **crash-atomic**하게 디스크에 지속/복구하는 **zero-loss 저장 계층**(RESILIENCY-01 = High criticality), (b) `RetryBackoffController` — 전송 오류를 재시도 정책으로 분류하고 지수 백오프·오프라인 판정·에스컬레이션을 계산하는 **순수·결정적 인메모리 상태머신**.

U4는 자체 런타임·스레드 풀·네트워크 축이 없다(단일 writer는 U8/U2가 보장, R-STATE-03). 따라서 확장성·가용성·성능 수치 목표의 상당수가 **N/A**다(§7). 반대로 U4가 실현하는 **신뢰성 계약(crash-atomic 지속 · last-good 롤백 · 무손실 round-trip · latest-state-wins · 타임아웃+백오프 graceful degradation)** 은 NFR-03의 zero-loss(RPO=0) 목표를 **직접 실행하는 지점**이므로 이 문서의 핵심이다.

**본 문서가 확정하는 U4 NFR 카탈로그(카테고리별 ID)**:

| 카테고리 | NFR ID | 요약 |
|---|---|---|
| 신뢰성 | U4-NFR-REL-01 | crash-atomic 지속 쓰기(temp+rename+fsync, RPO=0) |
| 신뢰성 | U4-NFR-REL-02 | last-good 롤백 / 손상 복구 시 non-crash |
| 신뢰성 | U4-NFR-REL-03 | `PersistedState` 무손실 round-trip |
| 신뢰성 | U4-NFR-REL-04 | latest-state-wins 지속 불변식 |
| 신뢰성 | U4-NFR-REL-05 | 타임아웃+백오프 graceful offline degradation |
| 성능/리소스 | U4-NFR-PERF-01 | 지속 비용 = 상태 크기 선형·유계 / 백오프 계산 O(1)(정성 계약, 수치 N/A) |
| 유지보수성 | U4-NFR-MNT-01 | `RetryBackoffController` 순수·결정적(주입 시계 + RNG 시드) |
| 유지보수성 | U4-NFR-MNT-02 | PBT 제너레이터 재사용(U0 `proptest-support` 소비 + U4 자기 제너레이터) |
| 유지보수성 | U4-NFR-MNT-03 | 커버리지/문서 정책(no %-게이트 + PBT + 예제 앵커) |

**신설 통제 없음 카테고리**: 보안(SEC)·사용성(USE)은 U4에 신설 요구가 없다(§5·§6에서 N/A 근거 확정).

---

## 2. 신뢰성 (Reliability)

U4의 신뢰성은 "**크래시·재시작·오프라인을 넘어 미반영/커밋된 변경을 잃지 않는 것**"이다(NFR-03). 이는 네 개의 지속 계약(원자 쓰기·last-good 복구·무손실 왕복·latest-state-wins)과 한 개의 재시도 계약(타임아웃+백오프 열화)으로 실현된다.

### 2.1 U4-NFR-REL-01 — crash-atomic 지속 쓰기 (temp+rename+fsync, RPO=0)

- **요구**: `SyncStateStore`의 모든 상태 쓰기(`mark_dirty`/`commit_manifest`/`persist_resume_offset`/`clear_resume_offsets`)는 **원자적 all-or-nothing**이어야 한다 — 최종 상태 파일은 항상 **완결된 이전 상태(last-good)** 또는 **완결된 새 상태** 중 하나만 관측되며, 부분 기록된 중간 상태는 존재할 수 없다. 쓰기 파이프라인은 반드시 순서 `U0 encode -> 같은 디렉터리 신규 임시 파일 기록 -> 임시 파일 fsync -> atomic rename(임시 -> 최종) -> 부모 디렉터리 fsync`를 따른다. 어느 단계에서 크래시(프로세스 kill·전원 손실)해도 최종 파일은 손상되지 않는다.
- **근거**: `business-rules.md` §1 R-STATE-01(temp+rename 원자 쓰기)·R-STATE-02(단일 결합 문서)·R-STATE-03(단일 writer) · `business-logic-model.md` §2.1(persist 파이프라인) · requirements.md §5.2 NFR-03(enqueued/committed zero-loss) · US-E3-01("원자적 임시+rename") · US-E7-09(zero-loss) · FD 결정 D-01/D-02/D-03. 실현 메커니즘 = `std::fs` 임시 파일 + `std::fs::rename` + `File::sync_all`(정본 tech-stack-decisions.md §2·§3) — WAL 미채택(D-01, MVP 트림), 상태가 소형(FQ-2=A: 매니페스트 1개 + boolean + 소형 맵)이라 전체 재기록이 저렴하고 rename 원자성만으로 NFR-03을 충족한다.
- **수용 기준**:
  - (AC-1) 쓰기 단계별(직렬화/temp 기록/fsync/rename 전후) 크래시 주입점을 열거한 PBT(PROP-U4-02, 카테고리 Invariant + 크래시 주입)가 어떤 주입점에서도 이후 `open_and_recover`가 완결 상태만 반환함을 반례 없이 검증한다.
  - (AC-2) rename은 **동일 파일시스템 내**에서만 원자적임을 계약으로 명시한다 — 임시 파일은 최종 파일과 **같은 디렉터리**에 생성한다(cross-device rename 회피).
  - (AC-3) 부모 디렉터리 fsync는 rename 메타데이터 내구성을 위한 배리어다(D-03, 트림 대상 아님). 플랫폼별 디렉터리 fsync 비지원(Windows)은 best-effort로 처리하고 rename 자체의 replace-atomicity에 의존함을 문서화한다(tech-stack §2.1).
- **연계**: 이 원자성은 U0-NFR-REL-01(코덱 무손실)과 결합해 RPO=0 지속을 완성한다 — 원자 쓰기가 부분 상태를 배제하고, 무손실 코덱이 복원값의 정확성을 보장한다(§9 Resiliency).

### 2.2 U4-NFR-REL-02 — last-good 롤백 / 손상 복구 시 non-crash

- **요구**: `open_and_recover(cfg)`는 로드 시 **패닉하지 않고** 항상 유효한 초기 상태로 진입해야 한다. 판정: (i) 최종 파일 부재 -> 빈 초기 상태 `{None, dirty=false, {}}`; (ii) 정상 디코드 + 잔존 임시 파일 존재 -> 잔존 임시 파일 정리 후 최종 파일을 last-good으로 채택; (iii) 최종 파일 디코드 실패(방어적 손상 경로) -> **크래시 대신** 빈 초기 상태로 롤백 + `dirty=true` 강제 + `StateError::CorruptRecovered{recovered_to: Empty}`(비치명 신호) 반환. 손상 복구는 **재조정을 유발**하여 마지막 커밋을 잃더라도 관측/적재된 변경이 다음 사이클 재스냅샷으로 **zero-loss로 재발견**되게 한다.
- **근거**: `business-rules.md` §2 R-STATE-04(복구 판정 표) · `business-logic-model.md` §2.2(open_and_recover 시퀀스) · requirements.md §5.2 NFR-03(≤ T_recon 재검출)·FR-04(재조정) · FD 결정 D-04 · `domain-entities.md` §2.4(`StateError`/`RecoveredState`). 실현 전제 = U0-NFR-REL-02(순수 표면 panic-free total) — `decode`가 절단·손상·적대적 바이트열에도 `Err(CodecError)`로만 표면화하므로 U4 복구 핫 경로가 재-크래시하지 않는다.
- **수용 기준**:
  - (AC-1) 임의 손상 바이트/절단/잔존 임시 파일 조합 제너레이터로 `open_and_recover`를 실행해 어떤 입력도 패닉 없이 `Ok`(빈/last-good) 또는 `Err(CorruptRecovered)`로 종결한다(PROP-U4-02의 손상 경로 부분).
  - (AC-2) 디코드 실패 복구는 반드시 `dirty=true`를 남겨 다음 사이클 재조정을 강제한다(R-STATE-04). `recovered_to`는 복구된 상태(Empty/LastGood)를 정확히 보고한다.
  - (AC-3) NFR-03 수용 (ii)의 "미관측 변경 검출 지연 ≤ T_recon"에서 **T_recon 타이머 자체는 U2/U8 소관**이며, U4의 계약은 "손상/부재 시 `dirty=true`로 재조정을 트리거한다"까지다(경계 명시 — U4는 T_recon 수치를 소유하지 않는다).

### 2.3 U4-NFR-REL-03 — `PersistedState` 무손실 round-trip

- **요구**: 디스크에 지속되는 모든 `PersistedState s`에 대해 `decode(encode(s)) == s`가 성립한다. `last_committed`의 `None`/빈 매니페스트(0 엔트리)/다수 엔트리, `dirty` 양값, `resume_offsets`의 빈 맵/다수 엔트리·경계 오프셋(0·대값 `ByteCount`)이 모두 정보 손실 없이 복원된다. `ResumeOffsetMap`은 `BTreeMap`으로 **결정적 정렬**되어 round-trip이 안정적이다.
- **근거**: `domain-entities.md` §2.1(PersistedState round-trip 불변식)·§2.2(ResumeOffsetMap 결정 정렬) · `business-logic-model.md` §5.1 / PROP-U4-03 · requirements.md §5.5 NFR-13(직렬화 무손실)·US-E7-06 · §12.2 FQ-3 오버레이(queue entries -> sync-state 정정). 실현 코덱 = **U0 `foundation::core_types::codec::{encode, decode}`**(내부 백엔드 `ciborium`, U0 FROZEN) — U4는 코덱을 **호출만** 하고 파일 I/O만 소유한다(정본 tech-stack-decisions.md §2). U4는 `ciborium` 직접 의존을 두지 않는다.
- **수용 기준**:
  - (AC-1) `PersistedState` 도메인 제너레이터(U0 `Manifest` 제너레이터 재사용 + `resume_offsets` 빈/다수·경계 `ByteCount` + `dirty` 양값) 기반 PBT round-trip(PROP-U4-03, 카테고리 Round-trip, PBT-02)이 반례 없이 통과한다.
  - (AC-2) 인코딩/디코딩 실패는 `StateError::Serde`(= U0 `CodecError`, Fatal 계열, `is_retryable()==false`)로 표면화되며 패닉하지 않는다(U4-NFR-REL-02와 결합).
- **비고**: 이는 U0-NFR-REL-01(코덱 무손실, PROP-BL-01)의 U4 애그리게이트 재확인이다 — 실행은 U4 크레이트에서 U4 자기 제너레이터로 수행한다(§4.2 MNT-02).

### 2.4 U4-NFR-REL-04 — latest-state-wins 지속 불변식

- **요구**: `commit_manifest` 호출들의 임의 순서·인터리빙에 대해, 지속된 `last_committed_manifest()`는 항상 **가장 최근에 커밋된 매니페스트**와 같다(구조적 최신-상태-대체). 별도 coalesce/큐 병합 로직 없이 "커밋된 상태 = 마지막 폴더 스냅샷"이 성립한다(FQ-2=A). 커밋은 `{새 매니페스트, dirty=false, resume_offsets=비움}`을 **한 번의 원자 쓰기**로 반영한다(clear 흡수).
- **근거**: `business-rules.md` §3 R-STATE-05/06(dirty 라이프사이클)·§4 R-RESUME-03(commit이 offsets clear) · `business-logic-model.md` §2.4(커밋 흐름) / PROP-U4-04 · requirements.md §5.5 NFR-11(coalescing latest-state-wins)·US-E7-04 · FD 결정 D-06. FQ-2=A상 오프라인/디바운스 중 쌓인 편집의 coalesce는 다음 사이클 재스냅샷으로 **구조적으로** 성립하므로(별도 coalesce 코드 없음), U4의 속성은 큐 인터리빙이 아니라 **커밋 지속 전이의 불변식**이다.
- **수용 기준**:
  - (AC-1) 상관된 매니페스트 커밋 시퀀스(같은/다른 경로의 add/modify/delete 파생) 제너레이터로 PROP-U4-04(카테고리 Invariant)를 검증해 `last_committed_manifest()`가 항상 마지막 커밋과 일치한다.
  - (AC-2) 커밋 성공 시 `resume_offsets`가 같은 원자 쓰기로 비워져 stale 오프셋이 다음 사이클로 새지 않는다(R-RESUME-03).

### 2.5 U4-NFR-REL-05 — 타임아웃+백오프 graceful offline degradation

- **요구**: 모든 재시도 대상 전송 실패(`Transient`/`Offline`/`Backpressure`)는 **지수 백오프 + full jitter + 상한**으로 다음 재시도 시각을 산출해야 한다: `base_delay(attempt) = min(cap, initial_delay * multiplier ^ attempt)`, `retry_after = rand(0, base_delay)`. 불변식 `0 <= retry_after <= cap`(무한 증가 방지). 오프라인은 **능동 프로브 없이** `Timeout`/`Network` 분류만으로 판정한다(`is_offline=true`, 별도 서버 헬스체크 왕복 없음). `on_success()`는 `attempt`·`consecutive_failures`·`next_retry_at`을 모두 리셋한다.
- **근거**: `business-rules.md` §5 R-RETRY-01(classify 매핑)·R-RETRY-02(순수 분류 오프라인 판정)·§6 R-BACKOFF-01/02/03(백오프 공식·기본값·리셋)·§7 R-ESCAL-01/02·R-AUTH-01 · `business-logic-model.md` §3(classify->decision 흐름·백오프 상태머신) · requirements.md §5.2 NFR-04(타임아웃 + 지수 백오프 + graceful offline degradation)·RESILIENCY-10 · US-E3-03/04 · FR-11/FR-19 케이스 2 · FD 결정 D-08/D-09/D-11/D-12/D-13.
- **백오프 수치 확정(D-10 이월분 해소)**: `BackoffConfig` 권장 기본 = `initial_delay=1s`, `multiplier=2.0`, `cap=300s`(5분), `jitter=full`. 에스컬레이션 임계 `N = WatcherConfig.notify_consecutive_failures`(U0 소유, 기본 3). 이 값들은 **federated config 키 `backoff`**(U4 소유, D-14)로 선언되며 **U8 watcher-bin이 raw config를 파싱해 타입드 `BackoffConfig`(`Duration` 필드)로 U4에 생성자 주입**한다 — U4는 `ConfigProvider`를 읽지 않는다. federated 파라미터의 live-reload는 MVP 범위 밖(재시작으로 변경).
- **수용 기준**:
  - (AC-1) 백오프 상태머신 PBT(PROP-U4-05, 카테고리 Invariant/상태 기반, US-E7-11): 임의 `on_failure`/`on_success` 시퀀스에서 (a) `retry_after in [0, cap]`; (b) `base_delay`(지터 전)가 `attempt` 누적에 대해 `cap`까지 비감소; (c) `on_success` 후 리셋; (d) `AuthFailed`/`Permanent` -> `retry_after == None`.
  - (AC-2) `classify` 전역성(PROP-U4-06): 5개 `TransportErrorClass` 변형이 정확히 하나의 `RetryClass`로 total 매핑되고 전송 경로에서 `Permanent`를 산출하지 않는다(유한 도메인 전수 검증).
  - (AC-3) 에스컬레이션 게이트(PROP-U4-07): `escalate == true`는 `RetryClass == Transient && consecutive_failures >= N`에서만 성립하고 `Offline`/`Backpressure`/`AuthFailed`에서는 임의 카운트에서도 `false`다(US-E3-04 오프라인 반복 알림 방지, `is_offline => !escalate` 불변식).
  - (AC-4) 오프라인 판정은 능동 서버 왕복을 발생시키지 않는다(R-RETRY-02 정합, NFR-04 graceful degradation).
- **비고**: 재개 오프셋 지속(R-RESUME) 덕에 오프라인 중 진행 중 전송이 재연결 시 마지막 ack부터 이어진다(FR-08); 별도 drain 루프 없이 다음 사이클이 곧 drain이다(FQ-2=A). 크래시 주입/오프라인/재개 하니스의 구체 형태는 **NFR Design/Operations 이월**(RESILIENCY-14, US-E7-11).

---

## 3. 성능 / 리소스 (Performance)

### 3.1 U4-NFR-PERF-01 — 지속 비용 선형·유계 / 백오프 계산 O(1) (정성 계약; 수치 목표 N/A)

- **요구(정성 계약만)**: (a) `SyncStateStore`의 지속 대상은 소형 상태다 — 매니페스트 1개(SafetyLimits 상한 <= 100k 엔트리, U0 R-LIMIT-01) + `dirty` boolean + `resume_offsets`(진행 중 blob 수만큼, 통상 소수). encode/전체 재기록 비용은 **엔트리 수에 선형·유계**이며 전량 인메모리 `Vec<u8>` 버퍼드 변환이다(스트리밍 아님 — 원자 temp+rename가 완결 파일을 요구, U0 tech-stack §2.1과 정합). (b) `RetryBackoffController`의 `classify`/`on_failure`/`on_success`는 **O(1)** 산술·비교로 파일/네트워크 I/O가 없다.
- **명시적 결정 — 수치 목표 없음(근거)**: U4에 throughput/latency/peak-memory 수치 게이트를 두지 않는다. (a) 지속 비용은 매니페스트 엔트리 수에 종속(입력 크기 종속)이라 절대 수치 목표 근거가 없다. (b) U4는 자체 처리량 축이 없는 얇은 lib이며 쓰기 빈도는 상위 직렬 사이클(Q2=B)이 지배한다. (c) 대규모 스트리밍 해시 메모리 바운드(NFR-02)는 U1 소관이지 U4 값 지속의 관심사가 아니다.
- **근거**: requirements.md §5.1 NFR-02(스트리밍 = U1)·§5.5 NFR-14(SafetyLimits 경계 = U1 실행) · U0-NFR-PERF-01(코덱 선형·유계, 동일 논리 상속) · FD 결정 D-01/D-02(소형 상태 전체 재기록 허용) · plans §3.
- **수용 기준**:
  - (AC-1) `encode`/원자 쓰기가 100k 엔트리 근접 매니페스트에 대해 비선형 폭증·무한 재귀 없이 완결한다(PROP-U4-03 round-trip 제너레이터의 대량 엔트리 경계로 간접 검증).
  - (AC-2) 어떤 절대 처리량/지연/피크메모리 임계도 U4 게이트로 설정되지 않는다(수치 게이트 부재를 명시 문서화).

---

## 4. 유지보수성 (Maintainability)

### 4.1 U4-NFR-MNT-01 — `RetryBackoffController` 순수·결정적 (주입 시계 + RNG 시드)

- **요구**: `RetryBackoffController`는 **순수·결정적**이어야 한다 — `on_failure(err, now: Instant)`의 현재 시각과 full jitter의 난수를 **모두 외부 주입**(주입 시계 + 주입 가능한 RNG 시드)으로 받아, 같은 (상태, `now`, seed)이면 같은 `RetryDecision`을 산출한다. 파일·네트워크·전역 시계·`thread_rng` 등 비결정적 부수효과를 두지 않는다.
- **선정 근거(왜 명시적 NFR인가)**: 백오프·에스컬레이션은 상태머신 PBT(PROP-U4-05/07, US-E7-11)의 대상이다. 결정성이 없으면 실패 케이스의 정확 재현·shrinking이 불가능해 PBT 자체가 무의미해진다. 따라서 "구현 관례"가 아니라 U4의 **명시적 테스트가능성 계약**으로 승격한다. full jitter의 난수원은 주입 시드 기반 **소형 자체 결정적 PRNG**로 실현하며 외부 RNG 크레이트를 도입하지 않는다(정본 tech-stack-decisions.md §4; `rand` 미채택 근거 포함).
- **근거**: `business-rules.md` §6 R-BACKOFF-01(결정성)·R-BACKOFF-03 · `business-logic-model.md` §3.2(결정성 비고) · FD 결정 D-13 · property-based-testing.md PBT-06(상태 기반 속성은 결정 실행 필요).
- **수용 기준**:
  - (AC-1) 동일 (시작 상태, 실패 클래스 시퀀스, `now` 시퀀스, seed)으로 두 번 실행하면 `RetryDecision` 시퀀스가 관측적으로 동일하다(재현성).
  - (AC-2) 컨트롤러 표면에 파일/네트워크/전역 시계 접근이 없음을 코드 계약으로 유지한다(주입 경계 D-13).

### 4.2 U4-NFR-MNT-02 — PBT 제너레이터 재사용 (U0 소비 + U4 자기 제너레이터)

- **요구**: U4의 PBT는 (a) U0 도메인 제너레이터(`Manifest`·`Sha256Digest`·`ByteCount`)를 **재사용**하고, (b) U4 고유 제너레이터(`PersistedState`·명령 시퀀스·크래시 주입점·`TransportErrorClass` 시퀀스 + 주입 시계/시드)를 **정의**한다. 재사용은 U0의 `proptest-support` 비기본 feature를 dev-dependency에서 켜서 확보하며, U4 자기 제너레이터도 동일한 비기본 `proptest-support` feature 뒤에 게이트해 `proptest`가 프로덕션 빌드 그래프에 유입되지 않게 한다(U0 MNT-02 패턴 미러링).
- **선정 근거**: 단일 출처 제너레이터 -> 단위 간 정의 드리프트 방지(U0-NFR-MNT-02 상속). feature 게이트 -> non-default라 릴리스 빌드에서 `proptest` 배제. U4가 U0 매니페스트 제너레이터를 재작성하지 않는다.
- **근거**: `domain-entities.md` §5(제너레이터 총괄)·`business-rules.md` §10·`business-logic-model.md` §5(PBT-07 제너레이터 요구) · property-based-testing.md PBT-07 · U0-NFR-MNT-02(`proptest-support` 노출 계약) · FD 결정. feature 이름·게이팅 정본 = tech-stack-decisions.md §5.
- **수용 기준**:
  - (AC-1) U4 크레이트의 제너레이터/PBT는 non-default `proptest-support` feature 뒤에 위치하며 기본 빌드에 `proptest` 의존이 나타나지 않는다.
  - (AC-2) U4는 `foundation`을 dev-dependency로 `features = ["proptest-support"]`와 함께 두어 U0 제너레이터를 재사용한다(재정의 없음).

### 4.3 U4-NFR-MNT-03 — 커버리지 / 문서 정책

- **요구**: (a) **전역 커버리지 %-게이트를 두지 않는다** — 실질 검증은 PBT(§2 속성들) + 예제 앵커(PBT-10: business-critical 경로에 예제 기반 테스트 병행)로 확보한다. (b) 공개 API 문서화는 워크스페이스 기본 관례를 따른다(U0의 공유 계약 크레이트 수준 `deny(missing_docs)` 강제는 U0 한정이며, U4는 워크스페이스 기본값을 상속하되 공개 표면 문서화를 권장한다).
- **선정 근거**: U0-NFR-MNT-03과 동일 논리 — 지속·백오프 로직은 속성/모델 기반 검증(상태머신 PBT·round-trip·crash-injection)이 line/branch % 게이트보다 실질적이다. business-critical 경로(crash-atomic 지속·복구·백오프·분류)는 PBT 단독이 아니라 예제 테스트를 병행한다(PBT-10).
- **근거**: property-based-testing.md PBT-10(PBT는 예제 테스트를 대체하지 않고 보완) · U0-NFR-MNT-03 · requirements.md §5.7 NFR-17. 상세 CI 커버리지·MSRV 검증 통합은 Build-and-Test 이월.
- **수용 기준**:
  - (AC-1) crash-atomic 지속·복구·백오프·분류 경로는 PBT와 예제 기반 테스트를 **모두** 가진다(PBT-10). 어떤 핵심 경로도 PBT 단독 커버리지가 아니다.
  - (AC-2) 커버리지 %-게이트가 U4에 설정되지 않음을 명시 문서화한다(정량 게이트 부재).

---

## 5. 보안-잔존 (Security residual) — N/A (신설 통제 없음)

**전제**: Security Baseline 확장 = **OFF**. U4에는 실현할 잔존 보안 통제가 **없다** — 유일 잔존 통제인 TLS/`https` 강제는 U5 `AuthTransport`/U0 config 검증 소관이고, 토큰 redaction 위생은 U0 소관이다. U4는 네트워크·자격증명 표면이 없다.

- **RISK-01(수용)**: `SyncStateStore`가 지속하는 상태 파일(마지막 커밋 매니페스트 = `relative_path` + `raw_sha256`, dirty, 재개 오프셋)은 **OS 보안 저장소 밖 평문**으로 저장된다. 이는 requirements §7/§12.2에서 **문서화된 수용 위험 RISK-01**이며(파일 경로는 민감할 수 있고, 저엔트로피 시크릿의 SHA-256은 사전 역산 가능), Security OFF 하에서 **암호화 저장·키관리를 신설하지 않는다**. `domain-entities.md` §2.3 RISK-01 주석과 정합.
- **판정**: U4는 신규 보안 통제를 신설하지 않는다. §7 N/A 표에 근거를 확정한다.

---

## 6. 사용성 (Usability) — N/A (사람-대면 표면 없음)

U4 `sync-state`는 **UI/CLI/트레이/사람-대면 표면이 없는 순수 백엔드 lib**다. config 값은 사람이 읽는 파일에서 오지만 그 파싱·검증·오류 메시지 표면은 U0 `ConfigProvider`(§U0-NFR-USE-01)와 U8 watcher-bin 소관이며, U4는 타입드 값(`BackoffConfig`·`StateConfig`)을 **주입받을 뿐**이다. `StateError` taxonomy는 사람 메시지가 아니라 U8 코디네이터가 소비하는 프로그램적 신호다. 따라서 사용성 요구는 U4에 신설하지 않는다(§7 N/A). 사용자 경험 표면은 U6/U7 소관.

---

## 7. N/A 카테고리 근거표

아래 카테고리는 U4에 요구를 신설하지 않는다(발명 금지). 각 판정은 확정 세트(requirements RESILIENCY-02/§6, FD plans §4)에서 온 것이다.

| 카테고리 | 판정 | 근거 |
|---|---|---|
| **확장성(Scalability)** | N/A | U4는 얇은 lib으로 자체 런타임·스레드·처리량 축이 없음. 단일 writer는 U8/U2 보장(R-STATE-03). 100k/20 GiB 스트리밍 스케일(NFR-02)은 U1 소관. |
| **가용성(Availability)** | N/A | lib 크레이트라 SLA 없음. 전체 Watcher는 단일 사용자·사용자 재시작 로컬 프로세스라 RTO/availability-SLA는 requirements RESILIENCY-02(§6)에서 N/A 확정. |
| **성능 수치 목표(Performance numeric)** | N/A(정성 계약만) | 지속 비용 = 매니페스트 엔트리 수 선형·유계, 백오프 = O(1). throughput/latency/peak-memory 수치 게이트 근거 없음. U4-NFR-PERF-01의 정성 계약만 문서화. (§3.1) |
| **Resiliency DR / RTO** | N/A(신규 결정 없음) | DR/RTO/availability = N/A는 requirements RESILIENCY-02(single-region·사용자 재시작)에서 확정. **RPO=zero-loss는 U4가 실행하는 목표**(U4-NFR-REL-01/02/03/04)이므로 N/A가 아니라 §2의 핵심 계약이다. RESILIENCY-11/13(DR 전략/failover 런북)은 미래 서버 소관. |
| **보안 강제 통제(Security enforced)** | N/A | Security Baseline OFF, RISK-01 수용. 암호화 저장·키관리·시크릿 스캐닝 신설 N/A. U4에는 실현할 잔존 통제도 없음(TLS=U5, 토큰=U0). (§5) |
| **사용성(config 파일 외)** | N/A | U4는 UI/CLI/트레이 표면 없는 lib. config 값은 U8이 타입드로 주입(U4는 `ConfigProvider` 미독취). 사용자 경험 표면은 U6/U7 소관. (§6) |

---

## 8. NFR to 요구사항 추적표

각 U4 NFR을 소스 NFR ID(requirements §5) + 규칙 ID(business-rules) + PBT 속성 ID로 매핑한다.

| U4 NFR ID | 요약 | 소스 NFR/FR ID | 규칙 ID | PBT ID |
|---|---|---|---|---|
| U4-NFR-REL-01 | crash-atomic 지속 쓰기 | NFR-03, US-E3-01, US-E7-09 | R-STATE-01/02/03 | PROP-U4-02 (Invariant + 크래시 주입) |
| U4-NFR-REL-02 | last-good 롤백 / non-crash 복구 | NFR-03, FR-04, US-E7-08 | R-STATE-04 | PROP-U4-01/02 |
| U4-NFR-REL-03 | `PersistedState` 무손실 round-trip | NFR-13, US-E7-06 | (PersistedState 불변식) | PROP-U4-03 (Round-trip, PBT-02) |
| U4-NFR-REL-04 | latest-state-wins 지속 불변식 | NFR-11, US-E7-04 | R-STATE-05/06, R-RESUME-03 | PROP-U4-04 (Invariant) |
| U4-NFR-REL-05 | 타임아웃+백오프 offline degradation | NFR-04, FR-11, FR-19(케이스2), US-E3-03/04, US-E7-11 | R-RETRY-01/02, R-BACKOFF-01/02/03, R-ESCAL-01/02, R-AUTH-01 | PROP-U4-05/06/07 |
| U4-NFR-PERF-01 | 지속 선형·유계 / 백오프 O(1)(수치 N/A) | NFR-02(U1 위임), NFR-14(U1 실행) | R-STATE-02(전체 재기록), R-LIMIT-01(U0 상수) | PROP-U4-03 대량 엔트리(간접) |
| U4-NFR-MNT-01 | 컨트롤러 순수·결정적 | NFR-17, US-E7-11 | R-BACKOFF-01/03 | PROP-U4-05 (결정 실행 전제) |
| U4-NFR-MNT-02 | PBT 제너레이터 재사용 | NFR-17, NFR-08..14(기반) | (제너레이터 계약) | PBT-07, PROP-U4-01/03/05 |
| U4-NFR-MNT-03 | 커버리지/문서 정책 | NFR-17 | (공개 계약) | PBT-10 |
| (SEC) | 잔존 통제 없음 / RISK-01 수용 | NFR-06(U5/U0), RISK-01 | (§2.3 RISK-01 주석) | N/A |
| (USE) | 사람-대면 표면 없음 | (해당 없음) | (config 주입 = U8) | N/A |

---

## 9. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 단계 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제 충족 (PBT-09 확인)** | 프레임워크(PBT-09) = `proptest`로 확정(워크스페이스 전역, U0에서 확정된 결정을 U4가 상속; tech-stack-decisions.md §5). PBT-07(제너레이터 재사용)은 U4-NFR-MNT-02(`proptest-support` 비기본 feature + U0 제너레이터 소비)로 실현. §2의 신뢰성 NFR이 FD에서 식별된 속성(PROP-U4-01 상태머신·02 crash-atomic·03 round-trip·04 latest-state-wins·05 백오프·06 classify·07 에스컬레이션)을 수용 기준으로 계약화. no-panic·결정성은 U4-NFR-REL-02/MNT-01로 요구화. shrinking/시드/CI(PBT-08)·크래시 주입 하니스는 NFR Design / Code Generation / Build-and-Test 이월. **blocking 없음.** |
| **Resiliency Baseline** | ON | **핵심 적용** | RESILIENCY-01: U4 = **High criticality**(zero-loss 저장 계층). U4-NFR-REL-01~04(crash-atomic·last-good·무손실·latest-state-wins)가 RPO=0 지속(NFR-03)의 **실행 지점**. U4-NFR-REL-05(타임아웃+백오프+graceful offline)가 RESILIENCY-10(NFR-04). RESILIENCY-14(회복탄력성 테스트)의 씨앗 = PROP-U4-02/05; 크래시 주입·오프라인·재개 하니스 상세는 **NFR Design/Operations 이월**(US-E7-11). RTO/HA/DR 수치·배포는 인프라 소관 **N/A**(RESILIENCY-02, §7). |
| **Security Baseline** | OFF | **N/A** | 미로딩·미강제. U4에 실현할 잔존 통제 없음(TLS=U5, 토큰=U0). 로컬 평문 상태 파일 = 문서화된 수용 위험 RISK-01(requirements §7/§12.2, §5). 암호화 저장·키관리 신설 없음. |

**블로킹 판정**: PBT-09가 워크스페이스 `proptest`로 확정되어 U4가 상속하고(§4.2·tech-stack §5), 활성 확장의 U4-적용 규칙이 모두 §2·§4의 NFR + 수용 기준으로 계약화되므로 blocking finding이 없다. Resiliency는 핵심 적용(수치는 N/A), Security는 N/A로 blocking 없음.

---

## 10. 후속 단계 이월 항목 (참고)

- 크래시 주입 하니스(프로세스 kill·전원 손실 모사)·오프라인 시뮬레이션·재개 중단 테스트의 구체 형태 -> **NFR Design / Operations**(RESILIENCY-14, US-E7-11).
- PBT 케이스 수·shrink 튜닝·고정 시드 vs 시드 로깅·`proptest-regressions` 정책·CI 통합(PBT-08) -> Code Generation / Build-and-Test.
- patch 버전 핀 + MSRV(1.85) 대비 CI 검증 -> Build-and-Test(U0와 동일 정책).
- 구체 크레이트/기법 결정(std `fs` 원자 I/O·자체 결정적 PRNG·U0 코덱 재사용·`thiserror`·`proptest-support` feature)의 정본 -> 자매 산출물 `tech-stack-decisions.md`.
</content>
</invoke>

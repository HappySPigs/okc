# U4 sync-state — Logical Components (논리 컴포넌트 분해 + 추적성 맵)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U4 Resilience & Retry** -> NFR Design -> 산출물 2/2 (`logical-components.md`)
**작성일**: 2026-09-08
**크레이트**: `sync-state` (lib) · **소속 컴포넌트(공개 애그리게이트)**: `SyncStateStore`, `RetryBackoffController`
**입력 아티팩트**: 자매 산출물 `nfr-design/nfr-design-patterns.md`(§10 패턴 -> 논리 컴포넌트 안착 맵이 이 문서의 권위 근거) · `functional-design/`(domain-entities §2~§3 · business-rules · business-logic-model §2~§4) · `nfr-requirements/`(NFR ID·tech-stack 결정) · U0 `nfr-design/logical-components.md`(스타일·추적표 템플릿)
**규칙**: `construction/nfr-design.md` Step 6 · `common/content-validation.md`(ASCII 화살표 `A -> B`, 박스 문자 미사용, Rust 제네릭 백틱) · `common/ascii-diagram-standards.md` · 활성 확장 `property-based-testing.md`(ON, Full) · `resiliency-baseline.md`(ON)

> **문서 성격(입도 가드)**: 이 문서는 Application Design이 확정한 **공개 2 애그리게이트**(`SyncStateStore`·`RetryBackoffController`)를 상위 애그리게이트로 유지한 채, 그 내부의 **이미 확정된 기능(FD 타입·규칙·흐름)을 명명된 논리 컴포넌트로 전개**한다. 이는 발명이 아니라 확정 기능의 명명·정렬이며 FD 흐름에서 near-free로 파생된다. 각 논리 컴포넌트는 (a) 책임, (b) 공개 인터페이스 표면, (c) 안착하는 확정 NFR 패턴(자매 산출물 `nfr-design-patterns.md` §10 안착 맵과 1:1)으로 기술한다.
>
> **이것은 추적성 문서 맵이며 물리 모듈/크레이트 증식 강제가 아니다.** 논리 컴포넌트 -> 물리 파일/모듈 레이아웃 매핑은 **Code Generation에서 최소로** 결정된다(다수 논리 컴포넌트가 한 모듈에 상주할 수 있음). 목적은 NFR·규칙·PBT 속성 -> 명명 단위 추적성 극대화와 RESILIENCY-01(U4 = High) 의존 매핑 강화다.
>
> **표기 규약**: 화살표는 ASCII `A -> B`만 사용(유니코드 화살표 금지). 박스/선-그리기 문자 미사용. Rust 제네릭/타입/식별자(예: `Vec<u8>`, `BTreeMap<Sha256Digest, ByteCount>`, `Result<T, StateError>`, `Option<Manifest>`, `Instant`)는 백틱으로 감싼다.

---

## 1. 논리 컴포넌트 전개 개요

| 공개 애그리게이트 | 성격 | 논리 컴포넌트(확정 명명) |
|---|---|---|
| `SyncStateStore` | crash-atomic 지속/복구(유일 I/O 소유자, 상태 보유) | StatePathResolver · AtomicWriter · StateCodecAdapter · StateRecoverer · ResumeOffsetTracker |
| `RetryBackoffController` | 순수·결정적 인메모리 재시도 상태머신(I/O 없음) | ErrorClassifier · BackoffScheduler · JitterSource · EscalationGate |
| (런타임 그래프 밖) | 테스트 전용 — 비-default `proptest-support` feature | ProptestGenerators (test-support 논리 단위, §5.2) |

> **두 애그리게이트는 상태를 공유하지 않는다** — `SyncStateStore`(디스크 지속)와 `RetryBackoffController`(인메모리 재시도)는 서로 다른 PBT 프로파일(크래시 주입 vs 백오프 상태머신)을 가진 독립 응집체다(business-logic-model §4). 공통 어휘는 U0 `CoreTypes` 타입뿐이다.

---

## 2. `SyncStateStore` 애그리게이트 — 논리 컴포넌트

> `SyncStateStore`는 U4의 유일한 I/O·상태 보유 컴포넌트다. 공개 메서드: `open_and_recover(cfg: &StateConfig) -> Result<Self, StateError>` · `last_committed_manifest() -> Option<&Manifest>` · `is_dirty() -> bool` · `mark_dirty()` · `commit_manifest(m: Manifest)` · `resume_offset(blob: &Sha256Digest) -> Option<ByteCount>` · `persist_resume_offset(blob, off)` · `clear_resume_offsets()`.

### 2.1 StatePathResolver
- **책임**: `StateConfig.state_path`(U8 주입 `PathBuf`)를 해소한다 — 지정 시 그대로, 부재 시 플랫폼 기본 경로. 최종 상태 파일 경로와 같은 디렉터리의 임시 파일 경로(`<state_path>.tmp`)를 결정해 AtomicWriter에 제공한다.
- **공개 인터페이스 표면**: `open_and_recover` 진입 시 내부 경로 해소(외부 표면 아님). 임시 파일이 **최종 파일과 같은 디렉터리**에 위치함을 보증(cross-device rename 회피).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §2 last-good 롤백(**U4-NFR-REL-02**, 경로 해소) + §1 파이프라인의 same-dir temp 전제(**U4-NFR-REL-01** AC-2).

### 2.2 AtomicWriter
- **책임**: §1 원자 쓰기 파이프라인의 실행 지점 — `bytes -> 같은 디렉터리 temp 기록 -> File::sync_all() -> std::fs::rename(atomic replace) -> 부모 디렉터리 fsync(best-effort)`. 모든 지속 mutator(`mark_dirty`/`commit_manifest`/`persist_resume_offset`/`clear_resume_offsets`)가 이 단일 파이프라인을 경유한다. non-streaming buffered 전체 재기록.
- **공개 인터페이스 표면**: 내부 `persist(&PersistedState) -> Result<(), StateError>`(mutator가 호출). `std::fs`만 사용, 외부 크레이트 없음. 디렉터리 fsync는 `cfg(unix)`/Windows best-effort 조건부(§8 이식성).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §1 crash-atomic 파이프라인(**U4-NFR-REL-01**, R-STATE-01/02/03) + §4 non-streaming buffered(**U4-NFR-PERF-01**) + §8 이식성(디렉터리 fsync 조건부).

### 2.3 StateCodecAdapter
- **책임**: `PersistedState`/`ResumeOffsetMap`를 **U0 코덱** `foundation::core_types::codec::{encode, decode}`(FROZEN, ciborium 백엔드)에 위임하는 얇은 어댑터. U4는 `ciborium`을 직접 의존하지 않고 U0 표면만 호출한다. 바이트 변환은 U0 소유, 파일 I/O는 AtomicWriter/StateRecoverer 소유.
- **공개 인터페이스 표면**: `encode(&PersistedState) -> Result<Vec<u8>, StateError>`(내부) · `decode(&[u8]) -> Result<PersistedState, StateError>`(내부). `PersistedState`/`ResumeOffsetMap = BTreeMap<Sha256Digest, ByteCount>`는 `serde::Serialize`/`Deserialize` 파생만 두어 U0 `encode<T: Serialize>`/`decode<T: DeserializeOwned>` 시그니처에 맞춘다. `BTreeMap` 결정적 정렬이 round-trip 안정성 보장.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §3 무손실 round-trip(**U4-NFR-REL-03**, PROP-U4-03) + §7 오류 처리(인코딩/디코딩 실패 -> `StateError::Serde` = U0 `CodecError` Fatal).

### 2.4 StateRecoverer
- **책임**: `open_and_recover`의 복구 판정 total 함수(§2 패턴). 최종 파일 부재 -> 빈 상태; 정상 decode -> 잔존 temp 정리 후 last-good 채택; decode 실패(손상) -> 빈 상태 + `dirty=true` 강제 + `StateError::CorruptRecovered{recovered_to}` 반환. panic-free.
- **공개 인터페이스 표면**: `open_and_recover(cfg: &StateConfig) -> Result<SyncStateStore, StateError>`(공개 진입점). `RecoveredState`(`Empty`/`LastGood`)로 복구 목적지 보고. 어떤 손상/절단/잔존-temp 입력도 `Ok`(빈/last-good) 또는 `Err(CorruptRecovered)`로 종결(패닉 없음).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §2 last-good 롤백/non-crash(**U4-NFR-REL-02**, R-STATE-04; U0-NFR-REL-02 panic-free `decode` 전제 의존) + §7 오류 처리(`CorruptRecovered` 구조화 신호).

### 2.5 ResumeOffsetTracker
- **책임**: 진행 중 업로드의 blob별 마지막 ack 재개 오프셋(`ResumeOffsetMap`) 관리 — read/persist/clear. blob은 `raw_sha256`(`Sha256Digest`)로 keying(Q6=C). `commit_manifest` 성공 시 같은 원자 쓰기로 비워진다(latest-state-wins 커밋 전이의 일부).
- **공개 인터페이스 표면**: `resume_offset(blob: &Sha256Digest) -> Option<ByteCount>` · `persist_resume_offset(blob, off)` · `clear_resume_offsets()`. 모든 mutator는 AtomicWriter 파이프라인을 경유(별도 부분 저장 없음).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §3 latest-state-wins + 재개 오프셋 지속(**U4-NFR-REL-04**, R-RESUME-01/02/03) — commit 원자 clear로 stale 오프셋 누출 방지.

---

## 3. `RetryBackoffController` 애그리게이트 — 논리 컴포넌트

> `RetryBackoffController`는 순수·결정적 인메모리 상태머신이다(I/O·네트워크·전역 시계 없음). 공개 메서드: `new(cfg: &BackoffConfig, n: u32, seed: u64) -> Self` · `classify(err: &TransportError) -> RetryClass` · `on_failure(err: &TransportError, now: Instant) -> RetryDecision` · `on_success()` · `next_retry_at() -> Option<Instant>` · `consecutive_failures() -> u32`.

### 3.1 ErrorClassifier
- **책임**: U0 `TransportErrorClass` -> U4 `RetryClass` **total 매핑**(5 변형, 누락·중복 없음). 전송 경로에서 `Permanent`를 산출하지 않는다(예약 변형). 오프라인은 `Timeout`/`Network`를 `Offline`로 순수 분류만으로 판정(능동 프로브 없음).
- **공개 인터페이스 표면**: `classify(&TransportError) -> RetryClass`(순수 함수, total). `RetryClass = { Transient, Offline, Backpressure, AuthFailed, Permanent }`.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §6 classify total 매핑 + 프로브-없는 오프라인(**U4-NFR-REL-05**, R-RETRY-01/02; PROP-U4-06).

### 3.2 BackoffScheduler
- **책임**: 지수 백오프 스케줄 계산 — `base_delay(attempt) = min(cap, initial_delay * multiplier ^ attempt)`, `retry_after = jitter(0, base_delay)`, `next_retry_at = now + retry_after`. `attempt` 카운트 보유·전이. `on_success()` 시 `attempt`·`next_retry_at` 리셋. `std::time::{Duration, Instant}` 산술만, O(1).
- **공개 인터페이스 표면**: `on_failure`에서 재시도 지연 산출(내부) · `next_retry_at() -> Option<Instant>`(status 관측용). 불변식: `0 <= retry_after <= cap`, `base_delay`는 `attempt` 누적에 대해 `cap`까지 비감소. `now: Instant` 주입(전역 시계 미사용).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §5 순수 결정적 백오프 상태머신(**U4-NFR-REL-05, U4-NFR-MNT-01**, R-BACKOFF-01/02/03; PROP-U4-05) + §4 백오프 O(1)(**U4-NFR-PERF-01**).

### 3.3 JitterSource
- **책임**: full jitter의 `[0, base_delay]` 균등 난수 생성. **자체 구현 소형 결정적 PRNG(splitmix64 계열, ~5~10줄)**, 시드는 컨트롤러 생성 시 주입. 외부 RNG 크레이트 없음.
- **공개 인터페이스 표면**: 내부 `uniform(0, base_delay) -> Duration`(BackoffScheduler가 호출). 같은 시드·호출 시퀀스 -> 같은 난수열(결정성). 암호학적 품질 불필요(thundering-herd 완화 목적).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §5 주입 시드 결정성(**U4-NFR-MNT-01**, D-13; PROP-U4-05 재현/shrinking). `rand` 미채택 근거의 실현 지점.

### 3.4 EscalationGate
- **책임**: 연속 실패 카운트(`consecutive_failures`) 보유·전이 + `escalate` 게이트 판정. 재시도 대상 실패(`Transient`/`Offline`/`Backpressure`)마다 증가, `on_success()`에서 0 리셋. `AuthFailed`는 카운터 불변(증가·리셋 안 함, R-AUTH-01). `escalate = (RetryClass == Transient) AND (consecutive_failures >= N)`.
- **공개 인터페이스 표면**: `on_failure` 반환 `RetryDecision { retry_after, is_offline, escalate }` 조립 · `consecutive_failures() -> u32`(status 관측용). 불변식: `is_offline => !escalate`; `AuthFailed`/`Permanent` -> `retry_after == None`. `N`은 U0 core field `notify_consecutive_failures`의 U8 주입값(단일 출처).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §6 에스컬레이션 게이트(**U4-NFR-REL-05**, R-ESCAL-01/02, R-AUTH-01; PROP-U4-07) + §9 federated N 주입.

---

## 4. U4 내부/외부 의존 엣지 (비순환 확인)

> 규약: `A -> B` = "A가 B에 의존한다(B의 타입/함수/계약을 사용)". U4는 DAG 상 **U0 `foundation`에만** 의존하는 소비자 말단이다 — U4를 소비하는 하위 단위는 없고(U1~U8이 U4를 소비하나 U4는 그들을 import하지 않음), U4는 어떤 U0 sink/observer 계약도 구현하지 않는다.

### 4.1 크레이트-레벨 의존 엣지

| 방향 | 관계 |
|---|---|
| `sync-state`(U4) -> `foundation`(U0) | 유일한 크레이트 의존. `CoreTypes`(Manifest·Sha256Digest·ByteCount·TransportError·TransportErrorClass·TransferResult) + 코덱(`encode`/`decode`) 소비 |
| `sync-state` -> `std::fs` / `std::time` | 원자 I/O(§2.2) / 백오프 타이밍(§3.2). 외부 크레이트 아님 |
| `sync-state` -> (U1~U8) | **없음** — U4는 어떤 상위/형제 단위도 import하지 않음(순수 소비자 말단) |
| `sync-state` -> U0 sink/observer 트레이트 **구현** | **없음** — U4는 network·log·config-reload 표면이 없어 `Logger`/`StatusSink`/`ConfigReloadObserver` 등을 구현하지 않음 |

### 4.2 논리 컴포넌트 내부 의존 엣지 표

| 논리 컴포넌트 | 소속 애그리게이트 | 의존 대상 |
|---|---|---|
| StatePathResolver | `SyncStateStore` | (없음 — `StateConfig`/`PathBuf` 소비) |
| AtomicWriter | `SyncStateStore` | StateCodecAdapter(encode) · StatePathResolver(temp/최종 경로) · `std::fs` |
| StateCodecAdapter | `SyncStateStore` | U0 코덱(`foundation::...::codec`) · U0 `CoreTypes`(Manifest 등) |
| StateRecoverer | `SyncStateStore` | StateCodecAdapter(decode) · StatePathResolver · AtomicWriter(temp 정리 시) |
| ResumeOffsetTracker | `SyncStateStore` | AtomicWriter(persist 경유) · U0 `Sha256Digest`/`ByteCount` |
| ErrorClassifier | `RetryBackoffController` | U0 `TransportError`/`TransportErrorClass` |
| BackoffScheduler | `RetryBackoffController` | JitterSource · `std::time` |
| JitterSource | `RetryBackoffController` | (없음 — 자체 PRNG, 주입 시드) |
| EscalationGate | `RetryBackoffController` | ErrorClassifier(RetryClass) |

### 4.3 ASCII 의존 스케치 (박스 미사용)

```
SyncStateStore 애그리게이트 (I/O + 상태 보유):

  StateRecoverer  ->  StateCodecAdapter  ->  U0 codec(encode/decode)
       |                    ^
       v                    |
  AtomicWriter  ------------+           ->  std::fs (rename/File/sync_all)
       ^                    |
       |                    |
  ResumeOffsetTracker       StatePathResolver  ->  StateConfig(PathBuf, U8 주입)

RetryBackoffController 애그리게이트 (순수·결정적, I/O 없음):

  EscalationGate  ->  ErrorClassifier  ->  U0 TransportError/TransportErrorClass
       |
       v
  BackoffScheduler  ->  JitterSource(자체 splitmix64, 주입 시드)
       |
       v
  std::time (Duration/Instant, 주입 now)

  방향 요약:  sync-state.*  ->  foundation(U0).* / std   (한 방향)
             U4  ->  (U1~U8)   = 없음                     => 사이클 없음
```

### 4.4 비순환 및 무-import 확인

- **비순환(acyclic)**: 모든 엣지는 U4 논리 컴포넌트 -> U0 `CoreTypes`/코덱 또는 `std` 방향으로만 흐른다. 두 애그리게이트는 상태를 공유하지 않아 상호 엣지가 없다(business-logic-model §4). U4 -> 상위/형제 단위 엣지가 없어 DAG 소비자 말단으로 비순환을 유지한다(component-dependency 빌드 순서 Foundation -> U1 -> **U4** -> ... 와 정합).
- **U0 계약 구현 없음**: U4는 U0 sink 트레이트(`Logger`/`StatusSink`/`HistorySink`/`CriticalEventSink`)나 `ConfigReloadObserver`를 **구현하지 않는다** — U4는 `RetryDecision`(is_offline/escalate)을 **반환만** 하고, status/notify push와 config-reload fan-out은 상위(U8 코디네이터/U0 observer)가 소유한다. federated 파라미터 live-reload는 MVP 밖이므로 U4는 관찰자 구독도 하지 않는다(재시작 재구성).

---

## 5. PBT 타깃 정렬 (PBT-01 / 확장 Full)

> 이 단계는 신규 PBT 결정을 내리지 않는다(PBT-09 프레임워크 = NFR-Req에서 `proptest` 워크스페이스 상속으로 SATISFIED). 확정 속성(FD의 PROP-U4-01~07)을 명명 논리 컴포넌트에 정렬해 추적성을 강화한다. NFR ID -> 속성 매핑은 `nfr-design-patterns.md` §10/§13과 일관된다.

### 5.1 컴포넌트 -> Testable Property 정렬표

| 논리 컴포넌트 | 정렬 속성(FD 소유) | 카테고리 | 실현 NFR / 규칙 |
|---|---|---|---|
| AtomicWriter · StateRecoverer | PROP-U4-02 (crash-atomic zero-loss, 쓰기 단계별 크래시 주입) | Invariant + 크래시 주입 | U4-NFR-REL-01/02(R-STATE-01/04) |
| StateRecoverer | PROP-U4-01 (상태머신 model-based 명령 시퀀스 + 크래시-복구 삽입) | Induction / 상태 기반 | U4-NFR-REL-02(R-STATE-04, PBT-06) |
| StateCodecAdapter | PROP-U4-03 (`decode(encode(s)) == s`) | Round-trip | U4-NFR-REL-03(PBT-02) |
| ResumeOffsetTracker · AtomicWriter | PROP-U4-04 (latest-state-wins, commit 순서 불변식) | Invariant | U4-NFR-REL-04(R-STATE-05/06, R-RESUME-03) |
| BackoffScheduler · JitterSource | PROP-U4-05 (백오프 상태머신 불변식, 주입 시계+시드 결정 실행) | Invariant / 상태 기반 | U4-NFR-REL-05, U4-NFR-MNT-01(R-BACKOFF-01/03) |
| ErrorClassifier | PROP-U4-06 (`classify` 전역성, 전송 경로 `Permanent` 미산출) | Invariant + Easy verification | U4-NFR-REL-05(R-RETRY-01) |
| EscalationGate | PROP-U4-07 (에스컬레이션 임계, 오프라인/백프레셔/Auth 제외) | Invariant / Oracle | U4-NFR-REL-05(R-ESCAL-02, R-AUTH-01) |

> **속성 없음(No PBT properties identified)**: StatePathResolver(경로 해소 = 결정적 구성, PROP-U4-01/02에 흡수), `StateError`/`RecoveredState`(운영 오류 값 — 복구 동작은 PROP-U4-02), R-BACKOFF-02 기본값(튜닝 상수 — 예제 테스트 적합)은 독립 PBT 속성이 없다(domain-entities §5.1 · business-rules §10과 일관). **예제 앵커 병행(PBT-10)**: crash-atomic 지속·복구·백오프·분류 등 business-critical 경로는 PBT 단독이 아니라 예제 기반 테스트를 병행한다(U4-NFR-MNT-03).

### 5.2 ProptestGenerators — 런타임 그래프 밖 test-support 논리 단위 (PBT-07)

- **배치**: 도메인 제너레이터(`proptest`)는 **런타임 의존 그래프(§4) 밖**의 별도 test-support 논리 단위다. 비-default cargo feature(`proptest-support`)로 게이트되어 릴리스 런타임 바이너리에 포함되지 않는다(U4-NFR-MNT-02, U0 패턴 미러링).
- **U0 제너레이터 재사용**: `foundation`을 **dev-dependency로 `features = ["proptest-support"]`** 와 함께 두어 U0 도메인 제너레이터(`Manifest`·`Sha256Digest`·`ByteCount`)를 재사용한다 — 재정의 없음(드리프트 방지).
- **U4 자기 제너레이터 책임**: `PersistedState` 제너레이터(U0 `Manifest` 제너레이터 조합 + `resume_offsets` 빈/다수·경계 `ByteCount` + `dirty` 양값), 명령 시퀀스 제너레이터(`mark_dirty`/`commit_manifest`/`persist_resume_offset`/`clear_resume_offsets` 임의 교차), 크래시 주입점 제너레이터(직렬화/temp/fsync/rename 전후 + 손상 바이트), `TransportErrorClass` 시퀀스 + 주입 시계(단조 `Instant`)/시드 제너레이터.
- **의존 방향**: `ProptestGenerators -> {sync-state 런타임 타입, foundation(proptest-support)}`. 런타임 컴포넌트는 이 단위에 의존하지 않으므로 §4 런타임 DAG를 오염시키지 않는다.
- **이월**: 구체 구현·shrinking·고정 시드·CI 통합(PBT-08)과 **크래시 주입 하니스(프로세스 kill·전원 손실 모사)의 구체 형태**는 Code Generation / Build-and-Test 이월(RESILIENCY-14, US-E7-11).

---

## 6. RESILIENCY-01 매핑 — U4 = High criticality (zero-loss 저장 계층)

- **분류**: U4 `sync-state`는 **High**이다 — zero-loss 지속(RPO=0) 저장 계층으로, 크래시·재시작·오프라인을 넘어 미반영/커밋된 변경을 잃지 않는 계약(NFR-03)의 **실행 지점**이다. U0(Critical, DAG 루트) 다음 가는 회복력 핵심이다.
- **회복력 기여(컴포넌트 입도)**:
  - **crash-atomic 지속(U4-NFR-REL-01, AtomicWriter)** — temp+rename+fsync가 부분 상태 미커밋을 보증(RPO=0).
  - **last-good 롤백/non-crash 복구(U4-NFR-REL-02, StateRecoverer)** — 손상 시 패닉 대신 빈 상태+dirty로 재조정 유발 -> 데몬 재-크래시 방지. U0 panic-free `decode`(Codec)에 의존.
  - **무손실 round-trip(U4-NFR-REL-03, StateCodecAdapter)** — U0 코덱 재사용으로 복원값 정확성 보증.
  - **latest-state-wins(U4-NFR-REL-04, ResumeOffsetTracker)** — 커밋 원자 전이가 마지막 스냅샷 = 지속 상태 불변식 성립.
  - **타임아웃+백오프 graceful offline(U4-NFR-REL-05, BackoffScheduler/ErrorClassifier/EscalationGate)** — 프로브-없는 오프라인 판정 + 지수 백오프 + 상한(RESILIENCY-10, NFR-04).
- **N/A 범위**: RTO/RPO 수치·DR/HA·서킷브레이커·auto-scaling·배포/롤백·카오스(RESILIENCY-02/05~14)는 얇은 lib에 부적용. RESILIENCY-14 회복력 테스트의 씨앗(크래시 주입/오프라인/재개 하니스)은 §5.2 이월. 신규 인프라 통제 없음.
- **fine-grained 맵의 추적 이점**: 공개 2 애그리게이트를 9개 명명 논리 컴포넌트로 전개함으로써 (NFR ID -> 규칙 ID -> **명명 컴포넌트** -> Testable Property)의 4단 추적 사슬이 성립한다. U4가 U0의 어느 컴포넌트에 의존하는지(StateCodecAdapter -> Codec, ErrorClassifier -> ErrorTaxonomy, StateRecoverer -> Codec panic-free)를 컴포넌트 입도로 지목할 수 있어 회복력 계약 변경의 영향 반경이 애그리게이트가 아니라 컴포넌트 입도로 추적된다.

---

## 7. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 산출물 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **준수 — blocking 없음** | §5가 각 논리 컴포넌트를 확정 속성(PROP-U4-01~07)에 정렬(PBT-01), ProptestGenerators를 test-support 논리 단위로 문서화하고 U0 제너레이터 재사용·feature 게이트 명시(PBT-07). 신규 PBT 결정 없음(PBT-09는 NFR-Req 충족). PBT-08·크래시 주입 하니스는 §5.2 이월. 예제 앵커 병행(PBT-10) 명시 |
| **Resiliency Baseline** | ON | **핵심 적용 + 일부 N/A — blocking 없음** | §6이 U4 = High 분류와 컴포넌트 입도 회복력 기여를 확정(REL-01~05 컴포넌트 안착). RTO/RPO/DR/HA/서킷브레이커/카오스는 얇은 lib에 N/A. 신규 resiliency 결정 없음(하니스는 이월) |
| **Security Baseline** | OFF | **N/A — 미로딩·미강제** | 확장 OFF, RISK-01 수용. U4는 네트워크·자격증명 표면 없어 실현할 잔존 통제 없음(TLS=U5, 토큰=U0). 로컬 평문 상태 파일 = 문서화된 수용 위험. 암호화 저장·키관리 신설 없음 |

**블로킹 판정**: 이 산출물에 blocking finding 없음. §2~§3 컴포넌트 안착은 `nfr-design-patterns.md` §10 맵과 1:1이며, §4 의존 그래프는 비순환(U4 = DAG 소비자 말단, 상위/형제 단위 무-import, U0 계약 무-구현)이다.

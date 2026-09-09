# U4 sync-state — Domain Entities (도메인 엔티티 / 값 타입)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U4 Resilience & Retry** -> Functional Design -> 산출물 1/3 (`domain-entities.md`)
**작성일**: 2026-09-08
**크레이트**: `sync-state` (lib) · **소속 컴포넌트**: `SyncStateStore`, `RetryBackoffController`
**전제(확정 답변)**: FQ-1=A(표준 SHA-256, 권위 `vault_content_id`는 서버) · FQ-2=A(최신 상태 대체, 폴더가 진실의 원천) · Q2(사이클)=B(트리거당 1회 직렬 사이클) · Q6=C(권위 있는 경로->해시 맵 + `manifest_digest`) · AUTOPILOT 결정 D-01..D-14(계획서 §3)

> **문서 성격**: 이 문서는 U4가 **도입/특화하는 값 타입**을 정의한다. `Manifest`/`ManifestEntry`/`SyncState`/`Sha256Digest`/`ByteCount`/`TransportError`/`TransportErrorClass`/`TransferResult`/`ClassifiedError` 등 **U0 `CoreTypes` 타입은 이름으로 참조만** 하며 재정의하지 않는다(단일 출처 = `crates/foundation/src/core_types/**`). `business-rules.md`(검증·결정 규칙)와 `business-logic-model.md`(지속·복구·백오프 흐름)가 이 문서의 타입을 참조한다.
>
> **표기 규약**: 비즈니스 의미 중심의 **기술중립 설계**다. Rust스러운 시그니처는 **참고용(reference-only)** 이며 인프라·스레딩·I/O 메커니즘이 아니라 개념적 형상을 표현한다. 다이어그램은 ASCII 박스 없이 **화살표 표기(A -> B)** 로 기술한다. English 식별자명은 원문 그대로 유지한다.

---

## 1. U0에서 소비하는 타입 (참조만 — 재정의 없음)

U4는 아래 U0 타입을 **소비**한다. 형상·불변식의 단일 출처는 U0이며, 여기서는 U4가 **어떻게 사용/지속하는지**만 기술한다.

| U0 타입 | U4에서의 용도 |
|---|---|
| `Manifest` (+ `ManifestEntry`, `RelativePath`, `Sha256Digest`, `ManifestDigest`, `ByteCount`) | 마지막 커밋 매니페스트로 지속 -> 사이클 시작 시 diff 기준선으로 제공(diff 계산은 U1) |
| `SyncState` (`Idle`/`Dirty`/`Uploading`/`Committed`) | 상태머신 참조 모델(전이 표 T1~T8은 U0 `business-logic-model.md` §2.3). U4는 dirty 플래그와 이 상태를 **지속/복구**하고 stateful PBT를 실행 |
| `TransportError` / `TransportErrorClass` | `RetryBackoffController.classify()` 입력 — 재시도 판정의 원천(U5 `AuthTransport` 산출) |
| `TransferResult` (`Success`/`Partial`/`Failed`) | `Partial.resume_offset`이 재개 오프셋의 원천값(개념 불변식 `resume_offset <= bytes`, U0 정의) |
| `ClassifiedError` / `ErrorClass` | `Failed.error` 소비; `ErrorClass::Fatal`은 재시도 무의미(`is_retryable()==false`) |
| `encode`/`decode` (CBOR 코덱) | `PersistedState`의 무손실 직렬화(NFR-13). U4는 코덱을 호출만, 저장 파일 I/O는 U4가 수행 |
| `WatcherConfig.notify_consecutive_failures` (기본 3) | FR-19 케이스 2 에스컬레이션 임계 `N`(U0 소유 키, U4는 소비) |

> **원칙**: U4는 U0 타입을 감싸거나 재정의하지 않는다. 아래 U4 신규 타입은 U0 타입을 **필드로 조합**할 뿐이다.

---

## 2. `SyncStateStore` 도입 타입

### 2.1 PersistedState

- **목적**: 최신 상태 대체 모델(FQ-2=A)의 **지속 애그리게이트** — 디스크에 원자적으로 저장/복구되는 단일 결합 문서(결정 D-02). 이벤트별 큐가 아니라 "마지막 커밋 매니페스트 1개 + dirty 신호 + 진행 중 재개 오프셋"의 스냅샷이다.
- **필드 스키마**(참고 형상):

  ```
  struct PersistedState {
      last_committed: Option<Manifest>,     // 최초 실행 전/커밋 이력 없음이면 None
      dirty: bool,                          // 미커밋 변경 존재 신호(단일 boolean, 큐 아님)
      resume_offsets: ResumeOffsetMap,      // 진행 중 업로드 blob별 마지막 ack 오프셋
  }
  ```

- **불변식**:
  - `last_committed`가 `Some(m)`이면 `m`은 U0 `Manifest` 불변식(canonical 정렬·경로 유일·`manifest_digest` 정합)을 만족한다(계산·정렬은 U1, U4는 그대로 지속).
  - `PersistedState`는 **무손실 CBOR round-trip 대상**(NFR-13, PROP-U4-03): `decode(encode(s)) == s`. `None`·빈 맵·빈 매니페스트(0 엔트리)·경계 오프셋(0·대값) 모두 보존.
  - **하나의 원자 쓰기 단위**: 세 필드는 개별 저장되지 않고 함께 인코딩·교체된다(중간 상태 없음, 결정 D-02/D-06).
- **정규화/검증**: U4는 매니페스트 내용을 재정규화하지 않는다(U0/U1 계약 신뢰). 디코드 실패(손상)는 값 검증이 아니라 **복구 규칙**으로 처리한다(`business-rules.md` R-STATE-04).
- **동등성**: 파생 구조적 동등(round-trip 검증에 사용).

### 2.2 ResumeOffsetMap

- **목적**: 진행 중 업로드의 **blob별 마지막 ack 재개 오프셋** 맵(FR-08 재개 전송 지원, U3 소비). 재개 대상 blob은 have/want 협상 키인 `raw_sha256`으로 식별된다(Q6=C, 결정 D-05).
- **참고 형상**: `type ResumeOffsetMap = BTreeMap<Sha256Digest, ByteCount>`.
- **불변식**:
  - 키 = blob의 `raw_sha256`(U0 `Sha256Digest`). `BTreeMap`으로 **결정적 정렬** -> round-trip·PBT 안정(PROP-U4-03).
  - 값 = 해당 blob에서 마지막으로 ack된 바이트 오프셋(`ByteCount`). 개념 불변식: 오프셋 `<=` 그 blob의 전체 크기(원천은 U0 `TransferResult::Partial`).
  - 커밋 성공 시 **비워진다**(결정 D-06): stale 오프셋이 다음 사이클로 새지 않는다.
- **비고**: `blob` 인자(component-methods `resume_offset(blob)`·`persist_resume_offset(blob, off)`)의 개념 키는 이 `Sha256Digest`다. 별도 `BlobId` newtype이 필요하면 그것도 `raw_sha256`을 감싼다(무손실 대상).

### 2.3 StateConfig (config 투영)

- **목적**: `SyncStateStore.open_and_recover(cfg: &StateConfig)`의 입력 — **상태 파일 위치**를 담는 config 투영(US-E3-01 "큐 저장 위치가 config에서 설정 가능"). U4가 소유하는 federated config 키(결정 D-14).
- **필드 스키마**:

  | 필드 | 필수 | 형식 | 기본값 |
  |---|---|---|---|
  | `state_path` | 아니오 | 상태 파일 절대 경로(부재 시 플랫폼 기본 경로) | 플랫폼 기본 |

- **불변식/규칙**: 이 키가 config JSON에 존재하면 U0 federated known-key union에 포함되어야 하며(R-CFG-STRICT-01, U0), 아니면 unknown 키로 거부된다. 경로 자체의 존재/권한 검사는 `open_and_recover`의 로드타임 관심사(`business-logic-model.md` §2).
- **RISK-01**: 상태 파일은 OS 보안 저장소 밖 평문으로 저장됨(문서화된 수용 위험, Security OFF).

### 2.4 StateError (오류 taxonomy)

- **목적**: `SyncStateStore` 연산의 실패 taxonomy(component-methods §U4).
- **참고 형상**:

  ```
  enum StateError {
      CorruptRecovered { recovered_to: RecoveredState },  // 손상 감지 -> last-good/빈 상태로 복구됨(비치명 신호)
      Io(/* source */),                                    // 파일 I/O 실패
      Serde(/* source, = CodecError */),                   // 인코딩/디코딩 실패(Fatal 계열)
  }
  ```

- **불변식/의미**:
  - `CorruptRecovered`는 **오류라기보다 복구 신호**다 — 부분/손상 기록을 마지막 정상 상태(또는 빈 초기 상태)로 롤백했음을 알리며, `recovered_to`로 복구된 상태(빈/last-good)를 보고한다. 호출부(U8)는 이를 받아 재조정(FR-04)을 강제한다.
  - `Serde`는 U0 `CodecError`(Fatal, `is_retryable()==false`)에 대응한다.
  - `Io`는 디스크 쓰기/읽기 실패(재시도 가능성은 상위 판단).
- **`RecoveredState`**(보조): `{ Empty, LastGood }` — 복구가 빈 초기 상태로 갔는지(main 파일 부재/디코드 불가) last-good으로 갔는지(정상 main + 잔존 temp 정리) 구분.

---

## 3. `RetryBackoffController` 도입 타입

### 3.1 RetryClass

- **목적**: U0 `TransportErrorClass`를 U4 재시도 정책 관점으로 재분류한 5원 enum(component-methods §U4).
- **참고 형상**: `enum RetryClass { Transient, Offline, Backpressure, AuthFailed, Permanent }`.
- **의미**(매핑 규칙은 `business-rules.md` R-RETRY-01):

  | 변형 | 의미 | 재시도 | 오프라인 | 에스컬레이션 |
  |---|---|---|---|---|
  | `Transient` | 서버측 일시 오류(5xx) | 백오프 후 예 | 아니오 | 카운트 대상(N 도달 시 escalate) |
  | `Offline` | 타임아웃/연결오류 | 백오프 후 예 | **예** | 제외(US-E3-04 반복 알림 방지) |
  | `Backpressure` | PROJECT_BUSY/queue-full/429 | 백오프 후 예(정상 지연) | 아니오 | 제외 |
  | `AuthFailed` | HTTP 401 | **아니오**(U5 위임) | 아니오 | 제외(FR-19 케이스 1은 별도, U6) |
  | `Permanent` | Fatal 계열(CodecError/HashMismatch/프로토콜 위반) | 아니오 | 아니오 | 제외 |

- **비고**: `Permanent`는 전송 경로(`TransportErrorClass`)에서 **산출되지 않는다** — 코디네이터가 `UploadError::HashMismatch`/`Aborted`나 `CodecError`를 Fatal로 넘길 때만 도달한다(결정 D-08). `classify()`는 5개 `TransportErrorClass` 변형에 대해 total이며 `Permanent`를 내지 않는다.

### 3.2 RetryDecision

- **목적**: `on_failure(err, now)`의 반환 — 한 번의 실패에 대한 재시도 판단 결과(component-methods §U4).
- **참고 형상**:

  ```
  struct RetryDecision {
      retry_after: Option<Duration>,   // None = 재시도 안 함(AuthFailed/Permanent)
      is_offline: bool,                // true = OperationalState::Offline push 근거(US-E3-04)
      escalate: bool,                  // true = FR-19 케이스 2 신호(실제 알림은 U6)
  }
  ```

- **불변식**:
  - `AuthFailed`/`Permanent` -> `retry_after == None`.
  - `is_offline == true`이면 `escalate == false`(오프라인은 에스컬레이션하지 않음, 결정 D-11).
  - `retry_after`가 `Some(d)`이면 `0 <= d <= cap`(상한, 결정 D-09).
- **비고**: 실제 push(status/notify)는 U4가 수행하지 않는다 — 코디네이터(U8)가 이 결정을 읽어 U6로 push한다.

### 3.3 BackoffConfig (config 투영)

- **목적**: `RetryBackoffController.new(cfg: &BackoffConfig)` 입력 — 지수 백오프 스케줄 파라미터(U4 소유 federated config 키 `backoff`, 결정 D-14).
- **필드 스키마**(권장 기본값 — 정확 수치는 NFR Requirements 이월, 결정 D-10):

  | 필드 | 형식 | 권장 기본 | 규칙 |
  |---|---|---|---|
  | `initial_delay` | 지속시간 | 1s | `> 0` |
  | `multiplier` | 실수 | 2.0 | `>= 1.0` |
  | `cap` | 지속시간 | 300s | `>= initial_delay` |
  | `jitter` | 열거(full/none) | full | 결정 D-09 full jitter |

- **불변식**: `initial_delay <= cap`, `multiplier >= 1.0`. 위반은 config 검증 실패(strict). N(연속 실패 임계)은 **BackoffConfig가 아니라 U0 `notify_consecutive_failures`** 에서 온다(단일 출처).

---

## 4. 엔티티 관계 (화살표 표기)

```
StateConfig  -> [SyncStateStore.open_and_recover]  -> PersistedState (복구됨)
PersistedState = { last_committed: Option<Manifest>[U0], dirty: bool, resume_offsets: ResumeOffsetMap }
ResumeOffsetMap : Sha256Digest[U0] -> ByteCount[U0]

TransportError[U0] -> [RetryBackoffController.classify] -> RetryClass
(RetryClass, now, BackoffConfig, consecutive_failures) -> [on_failure] -> RetryDecision
```

**텍스트 설명**: `SyncStateStore`는 `StateConfig`로 파일 경로를 해소해 `PersistedState`를 복구/지속하며, 그 안의 `last_committed`(U0 `Manifest`)와 `resume_offsets`를 상위 단위에 인자로 제공한다. `RetryBackoffController`는 U0가 산출한 `TransportError`를 `RetryClass`로 분류하고, 백오프 상태(연속 실패 카운트)와 `BackoffConfig`·주입 시계로 `RetryDecision`을 계산한다. 두 컴포넌트는 상태를 공유하지 않으며(서로 다른 PBT 프로파일), U0 타입만을 공통 어휘로 쓴다.

---

## 5. Testable Properties (PBT-01) — 엔티티/타입 계층

> **확장 강제(PBT-01, Full)**: 이 문서는 **타입/값 계층**의 속성을 식별한다. 로직/흐름 속성은 `business-logic-model.md`, 규칙 계층 속성은 `business-rules.md`가 소유한다(중복 회피). 프레임워크(PBT-09)는 Rust=proptest 유력, NFR Requirements 이월.

- **PROP-U4-03 — `PersistedState` 무손실 round-trip** (카테고리: **Round-trip**; PBT-02; NFR-13/US-E7-06 연계)
  - **속성**: 임의 `PersistedState s`에 대해 `decode(encode(s)) == s`. `last_committed`의 `None`/빈 매니페스트/다수 엔트리, `dirty` 양값, `resume_offsets`의 빈 맵/다수 엔트리·경계 오프셋(0·대값) 모두 정보 손실 없이 복원.
  - **제너레이터(PBT-07)**: `PersistedState` 도메인 제너레이터 — U0 `Manifest` 제너레이터(정규 `RelativePath`, 0/단일/다수 엔트리) 조합 + `resume_offsets`(빈/다수, 경계 `ByteCount`) + `dirty` 양값.
  - **비고**: U0 코덱 PROP-BL-01의 U4 애그리게이트 재확인(실행은 U4 크레이트, 자기 제너레이터).

- **PROP-U4-06(형상 부분) — `RetryClass`/`RetryDecision` 불변식** (카테고리: **Invariant**; Easy verification)
  - **속성**: 임의 `RetryDecision`에서 `is_offline == true => escalate == false`; `retry_after == Some(d) => 0 <= d <= cap`; `AuthFailed`/`Permanent` 분류 -> `retry_after == None`. (판정 매핑의 전역성은 `business-rules.md` PROP-U4-06 본체.)
  - **제너레이터(PBT-07)**: `TransportErrorClass` 전 변이(유한 도메인, 전수) + 임의 `consecutive_failures`·`now`·시드.

### 5.1 속성 없음(No PBT properties identified) 판정

| 타입/요소 | 판정 | 근거 |
|---|---|---|
| `StateConfig`/`BackoffConfig` | round-trip은 U0 config PROP-BR-02에 흡수; **독립 값 속성 없음** | config 투영 — 검증 규칙(범위)은 `business-rules.md`, 파싱 round-trip은 U0 소유 |
| `StateError`/`RecoveredState` | **No PBT properties identified**(값 계층) | 운영 오류(thiserror 계열) — 값 변환 아님. 복구 동작 속성은 PROP-U4-02(`business-rules.md`) |

---

## 6. 확장 컴플라이언스 요약 (이 산출물 범위)

| 확장 | 활성 | 이 산출물 적용 | 판정 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **준수** | §5 Testable Properties(PROP-U4-03 round-trip, PROP-U4-06 형상 불변식) + 제너레이터(PBT-07) 요구 기재. 속성 없는 값 타입은 §5.1에 판정. |
| **Resiliency Baseline** | ON | **부분 적용** | `PersistedState`의 무손실 round-trip(PROP-U4-03)이 U4 zero-loss 지속(RPO=0, NFR-03)의 값 계층 기반임을 문서화. RTO/HA/DR 수치는 이 순수 타입 문서에 N/A. |
| **Security Baseline** | OFF | **N/A** | 미로딩·미강제. 로컬 평문 상태 파일(§2.3)은 문서화된 수용 위험 RISK-01. |

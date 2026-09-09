# U4 sync-state — Business Rules (결정 규칙 / 검증 로직 / 제약)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U4 Resilience & Retry** -> Functional Design -> 산출물 2/3 (`business-rules.md`)
**작성일**: 2026-09-08
**크레이트**: `sync-state` (lib) · **소속 컴포넌트**: `SyncStateStore`, `RetryBackoffController`
**전제(확정 답변)**: FQ-1=A · FQ-2=A(최신 상태 대체) · Q2(사이클)=B · Q6=C(경로->해시 맵 + `manifest_digest`) · AUTOPILOT 결정 D-01..D-14(계획서 §3)

> **문서 성격**: 이 문서는 U4가 소유하는 **결정 규칙·검증 로직·제약·엣지케이스 판정**을 정의한다. 타입 정의는 `domain-entities.md`가 소유하며 이 문서는 그 타입명(`PersistedState`/`ResumeOffsetMap`/`RetryClass`/`RetryDecision`/`StateError`)과 U0 타입명을 **그대로 재사용**한다(재정의·모순 없음). 지속·복구·백오프 흐름/시퀀스는 `business-logic-model.md`가 소유한다.
>
> **표기 규약**: 비즈니스 의미 중심의 **기술중립 설계**다. Rust스러운 시그니처는 **참고용**이며 인프라·스레딩·I/O 메커니즘이 아니라 규칙의 개념 형상을 표현한다. 다이어그램은 ASCII 박스 없이 **화살표 표기(A -> B)** 로 기술한다. English 식별자명은 원문 그대로 유지한다.

---

## A. SyncStateStore 규칙 (지속·복구)

### 1. crash-atomic 쓰기 (NFR-03, US-E3-01)

**규칙 R-STATE-01 (temp+rename 원자 쓰기, 결정 D-01/D-03)**: `PersistedState`의 모든 지속 쓰기(`mark_dirty`/`commit_manifest`/`persist_resume_offset`/`clear_resume_offsets`)는 다음 순서를 **반드시** 따른다:

```
1) PersistedState 를 U0 encode 로 CBOR 바이트열로 직렬화
2) 같은 디렉터리의 신규 임시 파일에 바이트열을 기록
3) 임시 파일 fsync (내용이 디스크에 도달)
4) 임시 파일 -> 최종 상태 파일로 atomic rename (동일 파일시스템)
5) 부모 디렉터리 fsync (rename 메타데이터가 디스크에 도달)
```

- **불변식**: 최종 상태 파일은 항상 **완결된 last-good** 이거나 **완결된 새 상태**다 — 부분 기록된 중간 상태는 존재할 수 없다(rename 원자성). 어느 단계에서 크래시해도 최종 파일은 손상되지 않는다.
- **WAL 미채택(MVP 트림 D-01)**: 별도 write-ahead log를 두지 않는다 — 상태가 소형(FQ-2=A: 매니페스트 1개 + boolean + 소형 맵)이라 전체 재기록이 저렴하고, rename 원자성만으로 NFR-03을 충족한다.

**규칙 R-STATE-02 (단일 결합 문서, 결정 D-02)**: 세 필드(`last_committed`/`dirty`/`resume_offsets`)는 개별 파일이 아니라 **하나의 `PersistedState` 문서**로 함께 인코딩·교체된다. 어떤 쓰기든 문서 전체를 새로 기록하므로 필드 간 불일치(예: 매니페스트는 갱신됐으나 dirty는 이전 값)가 발생할 수 없다.

**규칙 R-STATE-03 (단일 writer 가정, 결정 D-07)**: `SyncStateStore`는 프로세스 내 **직렬 사이클(Q2=B)** + 프로세스 간 **SingleInstanceLock(FR-23)** 이 보장하는 단일 writer를 전제한다. 따라서 U4는 내부 파일 잠금이나 동시 쓰기 조정 로직을 두지 않는다. (다중 writer는 상위 계약 위반이며 U4 범위 밖.)

### 2. 부분쓰기 롤백 / last-good 복구 (NFR-03, 결정 D-04)

**규칙 R-STATE-04 (open_and_recover 복구 판정)**: `open_and_recover(cfg)`는 로드 시 다음을 판정한다:

| 로드 시 상태 | 복구 결과 | 반환 |
|---|---|---|
| 최종 상태 파일 **부재**(최초 실행) | 빈 초기 상태 `{ last_committed: None, dirty: false, resume_offsets: {} }` | `Ok`(빈 상태) |
| 최종 상태 파일 **정상 디코드** + 잔존 임시 파일 존재 | 잔존 임시 파일 **삭제/무시**(부분 쓰기 흔적), 최종 파일을 last-good으로 채택 | `Ok(last-good)` (선택적 `CorruptRecovered{recovered_to: LastGood}` 신호) |
| 최종 상태 파일 **디코드 실패**(손상) | 빈 초기 상태로 롤백 + **`dirty = true` 강제**(재조정 유발) | `Err(StateError::CorruptRecovered{recovered_to: Empty})` (비치명 — 호출부가 계속 진행) |

- **근거**: rename 원자성상 최종 파일은 손상되지 않아야 하나(R-STATE-01), 디스크 수준 손상 등 방어적 경로에서 디코드 실패 시 **크래시하지 않고** 빈 상태 + dirty로 복구한다. FQ-2=A상 다음 사이클이 폴더를 재스냅샷하므로 마지막 커밋을 잃어도 **관측/적재된 변경은 zero-loss로 재발견**된다(NFR-03, FR-04).
- **부분쓰기 롤백의 실체**: "롤백"은 손상된 부분 파일을 되돌리는 것이 아니라, atomic rename 덕에 **애초에 부분 상태가 커밋되지 않음**을 의미한다. 잔존 임시 파일은 실패한 쓰기의 흔적이며 무해하게 정리된다.

### 3. dirty 플래그 라이프사이클 (FQ-2=A, Q8=A)

**규칙 R-STATE-05 (dirty = 단일 신호, 큐 아님)**: `dirty`는 "다음 사이클이 재스냅샷해야 함"을 나타내는 **단일 boolean**이다. 누적 이벤트 목록이 아니다(FQ-2=A: 폴더가 진실의 원천).

**규칙 R-STATE-06 (dirty 전이, U0 전이 모델 T1~T8 정합)**:
- `mark_dirty()` -> `dirty = true` 지속(T1/T5/T6/T8에서 호출).
- `commit_manifest(m)` -> 새 `last_committed = m` + `dirty = false` + `resume_offsets = {}` 를 **한 번의 원자 쓰기**로 반영(결정 D-06). 이는 U0 전이 T2(사이클 시작 시 dirty clear) 및 T4의 지속 실행 지점이다.
- 사이클 진행 중(`Uploading`/`Committed`) 새 변경 감지는 상태 전이가 아니라 `mark_dirty()`만 호출(T5/T6). 커밋 완료 시점에 `dirty`가 참이면 다음 사이클이 재스냅샷한다(T7).

### 4. 재개 오프셋 규칙 (FR-08 재개 전송 지원)

**규칙 R-RESUME-01 (keying, 결정 D-05)**: 재개 오프셋은 blob의 `raw_sha256`(`Sha256Digest`)로 keying하며 `BTreeMap`으로 결정적 정렬한다. `resume_offset(blob) -> Option<ByteCount>`는 진행 중 오프셋이 있으면 반환, 없으면 `None`(처음부터 전송).

**규칙 R-RESUME-02 (persist 시점)**: `persist_resume_offset(blob, off)`는 U3가 청크 ack마다 호출한다. 원천값은 U0 `TransferResult::Partial.resume_offset`이며 개념 불변식 `off <= blob 전체 크기`를 만족한다(강제는 U3, U4는 지속).

**규칙 R-RESUME-03 (clear 시점, 결정 D-06)**:
- `commit_manifest`가 성공하면 재개 오프셋은 stale이므로 **같은 원자 쓰기로 비워진다**(R-STATE-06).
- `clear_resume_offsets()`는 별도 경로 — 커밋 없이 진행 중 전송을 폐기해야 할 때(예: `UploadError::HashMismatch`로 커밋 중단, 다음 사이클 재스냅샷)에 호출한다.

---

## B. RetryBackoffController 규칙 (분류·백오프·에스컬레이션)

### 5. 전송 오류 분류 (US-E3-03, 결정 D-08)

**규칙 R-RETRY-01 (`classify`: `TransportErrorClass` -> `RetryClass`, 전체 매핑·total)**:

| U0 `TransportErrorClass` | 대표 신호 | -> `RetryClass` | 재시도 | `is_offline` | 근거 |
|---|---|---|---|---|---|
| `AuthFailed` | HTTP 401 | `AuthFailed` | **아니오** | 아니오 | 재시도해도 동일 실패 — U5 인증 흐름 위임(FR-19 케이스 1은 U6 별도) |
| `ServerError` | HTTP 5xx | `Transient` | 예 | 아니오 | 서버측 일시 오류 — 백오프 후 성공 가능 |
| `Backpressure` | PROJECT_BUSY/queue-full/429 | `Backpressure` | 예 | 아니오 | 오류 아닌 정상 지연 신호 — 백오프(로그는 정상 지연으로 기록) |
| `Timeout` | 요청 타임아웃 | `Offline` | 예 | **예** | 별도 서버 왕복 없이 오프라인 판정(US-E3-04, NFR-04) |
| `Network` | 연결 실패 | `Offline` | 예 | **예** | 동일 — 오프라인으로 판정 |

- **`Permanent`의 출처**: `classify()`는 위 5개 `TransportErrorClass` 변형에 total하게 매핑하며 `Permanent`를 **산출하지 않는다**. `Permanent`는 코디네이터가 Fatal 계열(`CodecError`·`UploadError::HashMismatch`/`Aborted`)을 넘길 때만 도달하는 예약 변형이다(결정 D-08). `ErrorClass::Fatal.is_retryable() == false`(U0)와 정합.

**규칙 R-RETRY-02 (오프라인 판정 = 순수 분류 기반, US-E3-04·MVP D-08)**: 오프라인은 **능동 프로브 없이** `Timeout`/`Network` 분류만으로 판정한다(`is_offline = true`). 별도 서버 헬스체크 왕복을 발생시키지 않는다(NFR-04 정합).

### 6. 백오프 스케줄 (FR-11, NFR-04, 결정 D-09/D-10)

**규칙 R-BACKOFF-01 (지수 + full jitter + 상한)**: 재시도 대상(`Transient`/`Offline`/`Backpressure`) 실패 시 다음 지연을 계산한다:

```
base_delay(attempt) = min(cap, initial_delay * multiplier ^ attempt)
retry_after         = rand(0, base_delay(attempt))          // full jitter (D-09)
```

- `attempt`는 성공 이후 누적된 재시도 대상 실패 횟수(0부터).
- **불변식**: `0 <= retry_after <= cap`(상한이 무한 증가를 방지, FR-11). `base_delay`는 `attempt` 증가에 대해 `cap`까지 **비감소(non-decreasing)**.
- **정책 엔진 없음(MVP 트림 D-09)**: 단일 공식만 지원한다 — 오류 클래스별 상이한 스케줄·서킷브레이커·재시도 예산 등은 두지 않는다.
- **결정성(결정 D-13)**: `on_failure(err, now: Instant)`는 주입 시계와 주입 가능한 RNG 시드로 계산해 순수·재현 가능하다(PBT용). 파일/네트워크 I/O 없음.

**규칙 R-BACKOFF-02 (기본값·이월)**: `BackoffConfig` 권장 기본 = `initial_delay=1s`, `multiplier=2.0`, `cap=300s`, `jitter=full`. **정확 수치·튜닝은 NFR Requirements 이월**(결정 D-10, component-methods "구체 스케줄 -> NFR Design 이월").

**규칙 R-BACKOFF-03 (next_retry_at·reset)**: `on_failure`는 `next_retry_at = now + retry_after`를 저장하고 `next_retry_at()`로 노출한다(status 관측용). `on_success()`는 `attempt` 카운트·`consecutive_failures`·`next_retry_at`을 **모두 리셋**한다(US-E3-03 "성공 시 백오프 초기화").

### 7. 에스컬레이션 (FR-19 케이스 2, 결정 D-11)

**규칙 R-ESCAL-01 (연속 실패 카운트)**: `consecutive_failures`는 재시도 대상 실패(`Transient`/`Offline`/`Backpressure`)마다 증가하고 `on_success()`에서 0으로 리셋된다. `consecutive_failures()`로 노출된다(status 관측용).

**규칙 R-ESCAL-02 (escalate 게이트)**: `RetryDecision.escalate`는 다음 **모두** 성립할 때만 `true`다:

```
escalate = (RetryClass == Transient) AND (consecutive_failures >= N)
           where N = WatcherConfig.notify_consecutive_failures (U0, 기본 3)
```

- **오프라인 제외(US-E3-04)**: `is_offline == true`(= `Offline` 분류)이면 `escalate = false` — 오프라인 기간은 status의 `OperationalState::Offline`로만 표시하고 **반복 오류 알림을 내지 않는다**. (D-02의 `RetryDecision` 불변식 `is_offline => !escalate`와 정합.)
- **백프레셔 제외**: `Backpressure`는 정상 지연이므로 escalate하지 않는다.
- **AuthFailed 제외**: FR-19 케이스 1(인증 실패 알림)은 별도 조건이며 U6 `CriticalErrorNotifier`가 직접 소유한다 — U4의 케이스 2 카운터와 무관.

**규칙 R-AUTH-01 (AuthFailed 처리, 결정 D-12)**: `classify == AuthFailed`이면 `RetryDecision { retry_after: None, is_offline: false, escalate: false }`를 반환하고 `consecutive_failures`를 **증가시키지도 리셋하지도 않는다**(불변). 사이클은 재-인증 대기로 U5에 위임된다.

---

## C. Testable Properties (PBT-01) — 규칙 계층

> **확장 강제(PBT-01, Full)**: 각 속성에 카테고리 라벨 {Round-trip, Invariant, Idempotence, Oracle, Induction, Easy verification}과 도메인 제너레이터(PBT-07) 요구를 기재한다. 타입 계층 속성은 `domain-entities.md` §5, 흐름 계층 속성은 `business-logic-model.md` §6이 소유(중복 회피). 프레임워크(PBT-09)는 Rust=proptest 유력, NFR Requirements 이월.

### 8. SyncStateStore 속성

- **PROP-U4-01 — 상태머신 model-based 명령 시퀀스** (카테고리: **Induction/상태 기반**; PBT-06; US-E7-08)
  - **속성**: 임의 명령 시퀀스 `{ mark_dirty, commit_manifest(m), persist_resume_offset(b,o), clear_resume_offsets, (persist), (recover) }`에 대해, 실제 `SyncStateStore`의 관측 가능한 상태(`last_committed_manifest`/`is_dirty`/`resume_offset`)는 참조 모델(U0 전이 표 T1~T8 + PersistedState 의미)과 **항상 일치**한다.
  - **크래시-복구 삽입**: 시퀀스 임의 지점에 persist->recover(크래시 모사)를 삽입해도 복구 후 관측 상태는 크래시가 없었을 때와 동일하다(NFR-03 정합, PROP-U4-02와 연결).
  - **제너레이터(PBT-07)**: 명령 시퀀스 제너레이터(임의 교차) + U0 `Manifest` 제너레이터 + `Sha256Digest`/`ByteCount` 제너레이터 + 임의 지점 크래시 주입.
  - **참조 오라클**: U0 `business-logic-model.md` §2.3 전이 표 + `PersistedState` 필드 의미(단일 dirty 신호, commit이 offsets clear).

- **PROP-U4-02 — crash-atomic zero-loss / 부분쓰기 무손상** (카테고리: **Invariant** + 크래시 주입; NFR-03; US-E7-09)
  - **속성**: 어떤 쓰기의 어느 단계(직렬화/temp 기록/fsync/rename 전후)에서 크래시하더라도, 이후 `open_and_recover`는 **완결된 last-good** 또는 **완결된 새 상태**만 반환하며 절대 부분/손상 상태를 반환하지 않는다.
  - **zero-loss 속성**: 커밋된 마지막 매니페스트는 크래시·재시작을 넘어 보존된다. main 파일 디코드 실패(방어 경로)에서도 `dirty=true` 복구로 재조정(FR-04, T_recon)이 유발되어 관측/적재 변경이 유실되지 않는다(R-STATE-04).
  - **제너레이터(PBT-07)**: 쓰기 단계별 크래시 주입점 열거 + 임의 이전 상태(빈/last-good/진행 중 offsets) + 손상 바이트 주입.
  - **비고(이월)**: 실제 파일시스템 크래시 주입 하니스(프로세스 kill·전원 손실 모사)의 상세 형태는 **NFR Design/Operations 이월**(US-E7-11, RESILIENCY-14). 여기서는 속성·오라클을 확정.

- **PROP-U4-04 — latest-state-wins 불변식** (카테고리: **Invariant**; NFR-11; US-E7-04)
  - **속성**: `commit_manifest` 호출들의 임의 순서에 대해, 지속된 `last_committed_manifest()`는 **가장 최근 커밋된 매니페스트**와 항상 같다(구조적 최신-상태-대체). 별도 coalesce 로직 없이 "커밋된 상태 = 마지막 폴더 스냅샷" 불변식이 성립.
  - **제너레이터(PBT-07)**: 상관된 매니페스트 커밋 시퀀스(같은/다른 경로의 add/modify/delete 파생).
  - **비고**: FQ-2=A상 coalesce는 재스냅샷으로 구조적으로 성립하므로, U4 속성은 큐 인터리빙이 아니라 **커밋 지속 전이의 불변식**이다(requirements §12.2, NFR-11 오버레이).

### 9. RetryBackoffController 속성

- **PROP-U4-05 — 백오프 상태머신** (카테고리: **Invariant/상태 기반**; US-E7-11)
  - **속성**: 임의 `on_failure`/`on_success` 시퀀스에 대해 — (a) `retry_after`는 항상 `[0, cap]`; (b) `base_delay`(지터 전)는 `on_failure` 누적에 대해 `cap`까지 비감소; (c) `on_success()` 후 `attempt`·`consecutive_failures`가 0으로 리셋되고 다음 `retry_after`가 `initial_delay` 기반으로 복귀; (d) `AuthFailed`/`Permanent`는 `retry_after == None`.
  - **제너레이터(PBT-07)**: `TransportErrorClass` 시퀀스(성공 섞임) + 임의 `now: Instant` 단조 증가 + 고정 RNG 시드(결정 실행) + `BackoffConfig` 경계(작은 cap, mult=1.0 등).

- **PROP-U4-06 — `classify` 전역성 + 에스컬레이션 게이트** (카테고리: **Invariant/Oracle**; Easy verification)
  - **속성**: 모든 `TransportErrorClass` 변형은 정확히 하나의 `RetryClass`로 매핑된다(R-RETRY-01, 누락·중복 없음, 유한 도메인 전수 검증). `Permanent`는 전송 경로에서 산출되지 않는다.
  - **오라클**: R-RETRY-01 매핑 표.
  - **제너레이터(PBT-07)**: `TransportErrorClass` 전 변이 열거(Easy verification).

- **PROP-U4-07 — 에스컬레이션 임계** (카테고리: **Invariant/Oracle**; FR-19 케이스 2)
  - **속성**: `escalate`는 `RetryClass == Transient && consecutive_failures >= N`에서만 `true`이며, `Offline`/`Backpressure`/`AuthFailed` 실패에서는 임의 카운트에서도 `false`다(R-ESCAL-02, US-E3-04). `on_success()`는 카운터를 0으로 리셋한다.
  - **제너레이터(PBT-07)**: 실패 클래스 시퀀스(오프라인 장기 지속 포함) + `N` 경계(1·기본3·대값) + 성공 삽입.

### 10. 속성 없음(No PBT properties identified) 판정

| 규칙/요소 | 판정 | 근거 |
|---|---|---|
| §1 R-STATE-01 fsync 순서 | PROP-U4-02(crash-atomic)에 흡수 | 순서 자체는 구현 계약 — 독립 값 속성 아님, 크래시 주입 속성으로 검증 |
| §5 R-RESUME 재개 오프셋 지속 | PROP-U4-01(상태머신)에 흡수 | persist/clear/read는 상태머신 명령 — 별도 속성 불필요 |
| §B R-BACKOFF-02 기본값 | **No PBT properties identified** | 튜닝 상수(NFR 이월) — 예제 기반 단위테스트 적합 |

> **제너레이터(PBT-07) 총괄**: 위 속성은 `Manifest`/`Sha256Digest`/`ByteCount` 도메인 제너레이터, 명령 시퀀스 제너레이터, 크래시 주입점 제너레이터, `TransportErrorClass` 시퀀스 + 주입 시계/시드를 요구한다. shrinking·고정 시드·CI 통합(PBT-08)과 크래시 주입 하니스는 Code Generation/Build-and-Test 및 NFR Design 이월이며, 여기서는 **요구 사실과 대상**을 명시한다.

---

## 11. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 문서 적용 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **준수** | §C에 규칙 계층 속성 식별(PROP-U4-01 상태머신 PBT-06, PROP-U4-02 crash-atomic, PROP-U4-04 latest-state-wins, PROP-U4-05 백오프 상태머신, PROP-U4-06/07 분류·에스컬레이션). 카테고리 라벨 + 제너레이터(PBT-07) 요구 기재. 속성 없는 요소는 §10에 판정. |
| **Resiliency Baseline** | ON | **핵심 적용** | RESILIENCY-01: U4 = High(zero-loss 저장 계층). R-STATE-01~04 crash-atomic + last-good 롤백 = RPO=0 지속의 실행 규칙(NFR-03). R-RETRY/R-BACKOFF = 타임아웃+백오프(RESILIENCY-10, NFR-04). 크래시 주입/오프라인/재개 테스트 상세는 NFR Design/Operations 이월(RESILIENCY-14, US-E7-11). RTO/HA/DR 수치는 인프라 소관 N/A. |
| **Security Baseline** | OFF | **N/A** | 미로딩·미강제. 로컬 평문 상태 파일(마지막커밋+dirty+재개오프셋)은 문서화된 수용 위험 RISK-01(requirements §12.2). TLS는 U5 소관. |

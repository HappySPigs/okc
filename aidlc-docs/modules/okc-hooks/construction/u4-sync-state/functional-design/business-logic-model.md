# U4 sync-state — Business Logic Model (핵심 로직 · 알고리즘 · 데이터 흐름)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U4 Resilience & Retry** -> Functional Design -> 산출물 3/3 (`business-logic-model.md`)
**작성일**: 2026-09-08
**크레이트**: `sync-state` (lib) · **소속 컴포넌트**: `SyncStateStore`, `RetryBackoffController`
**전제(확정 답변)**: FQ-1=A · FQ-2=A(최신 상태 대체, 폴더가 진실의 원천) · Q2(사이클)=B(트리거당 1회 직렬 사이클) · Q6=C(경로->해시 맵 + `manifest_digest`) · AUTOPILOT 결정 D-01..D-14(계획서 §3)

> **문서 성격**: 이 문서는 U4가 소유하는 타입(`domain-entities.md`)과 규칙(`business-rules.md`) 위에서 동작하는 **핵심 로직·알고리즘·데이터 흐름**을 기술하고, **교차 단위 경계(무엇이 인자로 들어오고 무엇이 다른 단위 소유인지)** 를 명시한다. 타입/규칙은 재정의하지 않고 참조한다.
>
> **표기 규약**: 비즈니스 의미 중심의 **기술중립 설계**다. Rust스러운 시그니처는 **참고용(reference-only)** 이며 인프라·스레딩·I/O 메커니즘이 아니라 개념적 흐름을 표현한다. 다이어그램은 ASCII 박스 없이 **화살표 표기(A -> B)** 와 순서 목록으로 기술한다. English 식별자명은 원문 그대로 유지한다.

---

## 1. 교차 단위 경계 (무엇을 소유하고 무엇을 인자로 받는가)

U4는 의존이 **U0 하나뿐**인 얇은 단위다. 아래 표가 U4의 소유/비소유 경계를 확정한다.

| 관심사 | 소유 | U4와의 관계 |
|---|---|---|
| `Manifest`/`SyncState`/오류 taxonomy/코덱 | **U0** | U4가 소비·지속(재정의 없음) |
| 마지막 커밋 매니페스트·dirty·재개 오프셋의 **지속/복구** | **U4 `SyncStateStore`** | 이 문서 §2 |
| 볼트 스캔·해시·`Manifest` 빌드 | **U1** | U4 무관(U4는 완성된 `Manifest`를 인자로 받아 지속) |
| diff 계산(`ChangeSet`) | **U1 `ManifestDiffer`** | U4가 last-committed를 **인자로 전달**, U4는 U1 미의존(순수 유지) |
| blob 전송·재검증·커밋·no-op 조기종료 | **U3 `UploadProtocolDriver`** | U4는 재개 오프셋 읽기/쓰기만 제공 |
| HTTP·TLS·토큰·`TransportError` 산출 | **U5 `AuthTransport`** | U4는 산출된 `TransportError`를 **분류만**(§3) |
| status/알림/히스토리 push | **U6** | U4는 `RetryDecision`(is_offline/escalate)을 **반환만**, push는 코디네이터 |
| 사이클 오케스트레이션·직렬 잠금·SingleInstanceLock | **U8/U2** | 단일 writer 보장(R-STATE-03) |

> **핵심**: U4는 "무엇이 바뀌었는지"를 계산하지 않는다(U1 diff). U4는 "**마지막으로 커밋된 것이 무엇인지**"와 "**지금 재시도해야 하는지/언제**"만 안다.

---

## 2. SyncStateStore — 지속·복구 흐름

### 2.1 저장(persist) 파이프라인 (crash-atomic, R-STATE-01)

모든 상태 쓰기는 동일한 원자 파이프라인을 탄다:

```
PersistedState 값(메모리)
  -> [U0 encode: CBOR 바이트열]
  -> 신규 임시 파일에 기록
  -> 임시 파일 fsync
  -> atomic rename(임시 -> 최종 상태 파일)
  -> 부모 디렉터리 fsync
  -> 디스크(완결된 새 상태)
```

- 어느 단계에서 크래시해도 최종 상태 파일은 **직전 완결 상태(last-good)** 로 남는다 — rename이 원자적이므로 부분 상태가 커밋되지 않는다.
- `encode`(순수 바이트 변환)는 U0 소유, **파일 열기·fsync·rename은 U4 소유**(U0 코덱은 I/O 없음).

### 2.2 적재/복구(open_and_recover) 시퀀스 (R-STATE-04)

순서 목록:

1. `StateConfig`로 상태 파일 경로 해소(부재 시 플랫폼 기본 경로).
2. 최종 상태 파일이 **부재** -> 빈 초기 상태 `{ None, dirty=false, {} }` 반환(최초 실행).
3. 최종 상태 파일이 **존재** -> U0 `decode`로 `PersistedState` 복원 시도.
   - **성공** -> 잔존 임시 파일이 있으면 삭제(부분 쓰기 흔적) -> 복원 상태를 last-good으로 채택.
   - **실패(손상)** -> 빈 상태로 롤백 + `dirty=true` 강제 -> `StateError::CorruptRecovered{ recovered_to: Empty }`(비치명 신호). 다음 사이클 재조정이 zero-loss로 흡수(FR-04, NFR-03).

```
open_and_recover(cfg)
  -> 경로 해소
  -> (파일 부재 ? 빈 상태 : decode 시도)
  -> (decode 성공 ? 잔존 temp 정리 + last-good : 빈 상태 + dirty=true + CorruptRecovered)
```

### 2.3 사이클 시작 시 diff 기준선 제공 (FQ-2=A)

```
사이클 시작(U8 코디네이터)
  -> SyncStateStore.last_committed_manifest() -> Option<&Manifest>[U0]
  -> (U1 VaultScanner 재스냅샷 -> 현재 Manifest)
  -> U1 ManifestDiffer.diff(last_committed, current) -> ChangeSet   // U4는 인자만 전달, U1 미의존
  -> (ChangeSet.is_empty() ? no-op 조기종료 : U3 업로드 진행)
```

- U4는 `last_committed`를 **읽어 인자로 넘길 뿐** diff를 계산하지 않는다(순수 경계 유지). `ManifestDigest` 동치 no-op 판정은 U0/U1 규칙.

### 2.4 커밋·재개 오프셋 흐름 (R-STATE-06, R-RESUME)

```
업로드 진행 중(U3)
  -> 청크 ack 마다 SyncStateStore.persist_resume_offset(raw_sha256, off)   // ResumeOffsetMap 갱신, 원자 쓰기
  -> (실패/HashMismatch ? clear_resume_offsets() + mark_dirty() : 계속)

모든 want blob 전송 완료 + 서버 커밋 성공(U3)
  -> SyncStateStore.commit_manifest(new_manifest)
     = 한 번의 원자 쓰기로 { last_committed=new_manifest, dirty=false, resume_offsets={} }   // clear 흡수(D-06)
```

- **dirty 재진입(T7)**: 커밋 완료 시점에 사이클 진행 중 새 변경으로 `mark_dirty()`가 호출됐으면 `dirty=true`가 남아 다음 사이클이 재스냅샷한다.
- **실패(T8)**: 업로드 실패 시 별도 `Failed` 상태 없이 `mark_dirty()`(또는 dirty 유지)로 두고 `RetryBackoffController`가 재시도 시각을 정한다.

---

## 3. RetryBackoffController — 분류·백오프 흐름

### 3.1 classify -> decision 데이터 흐름

```
U5 AuthTransport 산출 TransportError[U0]
  -> RetryBackoffController.classify(err) -> RetryClass                 // R-RETRY-01 매핑
  -> on_failure(err, now: Instant)
       -> RetryClass 별 분기:
            AuthFailed  -> { retry_after: None, is_offline: false, escalate: false }   // 카운터 불변, U5 위임(R-AUTH-01)
            Permanent   -> { retry_after: None, is_offline: false, escalate: false }   // Fatal 계열(코디네이터가 전달)
            Transient   -> consecutive_failures += 1; retry_after = backoff();
                           escalate = (consecutive_failures >= N)                       // R-ESCAL-02
            Backpressure-> consecutive_failures += 1(백오프용); retry_after = backoff(); escalate = false
            Offline     -> consecutive_failures += 1(백오프용); retry_after = backoff();
                           is_offline = true; escalate = false                          // US-E3-04
  -> RetryDecision 반환(코디네이터가 next_retry_at 대기 + is_offline/escalate를 U6로 push)
```

### 3.2 백오프 상태머신 (R-BACKOFF-01, 결정 D-09/D-13)

- **상태**: `attempt`(재시도 대상 실패 누적), `consecutive_failures`, `next_retry_at`.
- **전이**:

  | 이벤트 | attempt | consecutive_failures | next_retry_at |
  |---|---|---|---|
  | `on_failure`(재시도 대상) | +1 | +1 | `now + rand(0, min(cap, initial*mult^attempt))` |
  | `on_failure`(AuthFailed/Permanent) | 불변 | 불변 | 불변(None) |
  | `on_success` | 0 | 0 | clear(None) |

- **불변식**: `0 <= retry_after <= cap`; `base_delay`는 `attempt`에 대해 `cap`까지 비감소; full jitter로 `retry_after`가 `[0, base_delay]` 내 균등.
- **결정성**: 주입 `now`와 RNG 시드로 순수 계산 -> PBT 재현 가능(PROP-U4-05). 파일/네트워크 I/O 없음.

### 3.3 오프라인 -> 재연결 배출(drain) 흐름 (US-E3-04, FQ-2=A)

- 오프라인 판정(`is_offline`)은 코디네이터가 `OperationalState::Offline`로 push한다(U6). **별도 drain 루프 없음** — 다음 사이클이 곧 drain이다(FQ-2=A). 재연결 시 최신 폴더를 재스냅샷하므로 오프라인 중 쌓인 편집은 자동 흡수된다(US-E3-02: 별도 coalesce 로직 없이 최신 스냅샷으로 합쳐짐).
- 배출 중 재-오프라인 시: 진행 중 재개 오프셋(`ResumeOffsetMap`)은 지속돼 있으므로(§2.4), 재연결 시 U3가 마지막 ack부터 이어서 전송한다(FR-08).

---

## 4. 두 컴포넌트의 합성 (scan -> build -> diff -> limits -> transfer -> commit 에서 U4의 위치)

전체 사이클(U8 오케스트레이션)에서 U4의 접점만 표시(비-U4 단계는 소유 단위 표기):

```
[U8] 트리거 -> 사이클 시작(직렬 잠금)
[U4] SyncStateStore.last_committed_manifest()  ---------------------.
[U1] VaultScanner 재스냅샷 -> ManifestBuilder -> current Manifest    |
[U1] ManifestDiffer.diff(last_committed, current) -> ChangeSet  <----'  (U4가 인자 전달)
[U1] SafetyLimitsValidator 런타임 한도 재검사(NFR-14)
[U3] no-op 판정 -> negotiate(have/want) -> transfer_wanted
        [U4] resume_offset(blob) 읽기 / persist_resume_offset 쓰기   (재개 지원)
[U5] AuthTransport 전송 -> 실패 시 TransportError
        [U4] classify + on_failure -> RetryDecision(retry_after/is_offline/escalate)
[U3] 커밋 성공
        [U4] commit_manifest(new) (dirty clear + offsets clear, 원자)
        [U4] on_success() (백오프·카운터 리셋)
[U8] RetryDecision 소비 -> U6로 status/notify push, next_retry_at 대기
```

- **U4의 두 컴포넌트는 상태를 공유하지 않는다**: `SyncStateStore`(지속)와 `RetryBackoffController`(인메모리 재시도 상태)는 서로 다른 PBT 프로파일(크래시 주입 vs 백오프 상태머신)을 가진 독립 응집체다.

---

## 5. Testable Properties (PBT-01) — 로직/흐름 계층

> **확장 강제(PBT-01, Full)**: 이 산출물은 **로직·흐름 계층**의 속성을 식별한다. 타입 계층은 `domain-entities.md` §5, 규칙 계층은 `business-rules.md` §C가 소유(중복 회피). 각 속성에 카테고리 라벨 + 제너레이터(PBT-07) 요구를 기재. 프레임워크(PBT-09)는 Rust=proptest 유력, NFR Requirements 이월.

### 5.1 SyncStateStore (컴포넌트 노트)

- **PROP-U4-01(흐름 관점) — 지속/복구 라운드 흐름**: §2 파이프라인 임의 명령 시퀀스에 persist->recover(크래시 모사) 삽입 후에도 관측 상태가 참조 모델과 동일(Induction, PBT-06/US-E7-08). 참조 오라클 = U0 전이 표 T1~T8 + `PersistedState` 의미. 본체·제너레이터는 `business-rules.md` §8.
- **PROP-U4-02(흐름 관점) — crash-atomic zero-loss**: §2.1 쓰기의 어느 단계 크래시에도 §2.2 복구가 완결 상태만 반환(Invariant + 크래시 주입, NFR-03/US-E7-09). 크래시 주입 하니스 상세는 NFR Design/Operations 이월(RESILIENCY-14).
- **PROP-U4-04(흐름 관점) — latest-state-wins**: §2.4 커밋 시퀀스에서 `last_committed_manifest()` == 마지막 커밋(구조적 최신-상태-대체, NFR-11/US-E7-04).

### 5.2 RetryBackoffController (컴포넌트 노트)

- **PROP-U4-05 — 백오프 상태머신**: §3.2 전이표의 불변식(retry_after in [0,cap], base_delay 비감소, on_success 리셋, AuthFailed/Permanent -> None)을 임의 `on_failure`/`on_success` 시퀀스로 검증(Invariant/상태 기반, US-E7-11). 주입 시계 + 고정 시드로 결정 실행. 본체는 `business-rules.md` §9.
- **PROP-U4-06/07 — classify 전역성 + 에스컬레이션 게이트**: §3.1 매핑·게이트가 `TransportErrorClass` 전 변이와 오프라인/백프레셔 제외를 만족(Oracle/Invariant). 본체는 `business-rules.md` §9.

### 5.3 속성 없음(No PBT properties identified) 판정

| 로직/흐름 요소 | 판정 | 근거 |
|---|---|---|
| §2.3 diff 기준선 제공 | **No PBT properties identified**(U4 계층) | diff 정확성 속성은 **U1 소유**(US-E7-05/NFR-12) — U4는 인자 전달만, 값 변환 없음 |
| §3.3 오프라인 drain 흐름 | PROP-U4-05/07에 흡수 | 별도 drain 로직 없음(FQ-2=A) — 재시도 상태머신에 귀속 |
| §4 사이클 합성 | 각 컴포넌트 속성에 분해 | U4 접점은 개별 컴포넌트 속성으로 커버, 오케스트레이션은 U8 소관 |

> **제너레이터(PBT-07) 총괄**: `Manifest`·`Sha256Digest`·`ByteCount` 도메인 제너레이터, 명령/크래시 주입 시퀀스, `TransportErrorClass` 시퀀스 + 주입 시계/시드를 요구한다. shrinking·고정 시드·CI 통합(PBT-08)과 크래시 주입 하니스는 Code Generation/Build-and-Test 및 NFR Design 이월.

---

## 6. 확장 컴플라이언스 요약 (이 산출물 범위)

| 확장 | 활성 | 이 산출물 적용 | 판정 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제·준수** | §5에 로직/흐름 속성 식별(PROP-U4-01/02/04 지속·복구, PROP-U4-05/06/07 백오프). 카테고리 라벨·제너레이터 요구 기재. 속성 없는 요소는 §5.3에 판정. 상태머신 PBT(PBT-06, US-E7-08)의 참조 모델 = U0 전이표. |
| **Resiliency Baseline** | ON | **핵심 적용** | RESILIENCY-01: U4 = High(zero-loss 저장 계층). §2 crash-atomic 파이프라인 + 복구 = RPO=0 지속의 흐름 실행 지점(NFR-03). §3 백오프/오프라인 = 타임아웃+graceful degradation(RESILIENCY-10, NFR-04). 크래시 주입/오프라인/재개 하니스는 NFR Design/Operations 이월(RESILIENCY-14, US-E7-11). RTO/HA/DR 수치·배포는 인프라 소관 N/A. |
| **Security Baseline** | OFF | **N/A** | 미로딩·미강제. 로컬 평문 상태 파일은 문서화된 수용 위험 RISK-01(requirements §12.2). TLS는 U5 소관. |

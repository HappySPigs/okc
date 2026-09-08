# U6 Observability — Business Logic Model (핵심 로직 · 알고리즘 · 데이터 흐름)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U6 Observability** -> Functional Design -> 산출물 3/3 (`business-logic-model.md`)
**작성일**: 2026-09-08
**크레이트**: `observability` (lib) · **소속 컴포넌트**: `StructuredLogger`, `StatusService`, `UploadHistoryStore`, `CriticalErrorNotifier`, `TrayIndicator`(선택)
**전제(AUTOPILOT 확정, plan §3)**: D1..D14

> **문서 성격**: `domain-entities.md`(타입)와 `business-rules.md`(규칙) 위에서 동작하는 **컴포넌트별 알고리즘·워크플로·데이터 흐름 + 합성**을 기술한다. 크로스-유닛 경계(주입 vs 소유)를 명시한다. 타입/규칙은 자매 산출물을 참조한다(중복 정의 회피).
>
> **표기 규약**: 기술중립 설계. Rust스러운 시그니처는 참고용. 다이어그램은 ASCII 박스 없이 **화살표 표기(A -> B)** 와 순서 목록으로 기술한다. English 식별자명 유지.

---

## 0. 합성·경계 개요 (주입 vs 소유)

U6의 4개 실체 싱크는 U0 트레이트를 구현하고, **조립 루트 U8이 생성·주입**한다. U6는 U1~U5를 역참조하지 않는다(R-PUSH-01).

**주입 방향(화살표 — "A -> B" 는 A가 B를 하위에 주입):**

```
U8(조립 루트)
  -> StructuredLogger  를 Arc<dyn Logger>          로 U1~U5/U8 에 하향 주입
  -> StatusService     를 Arc<dyn StatusSink>       로 U1~U5/U8 에 하향 주입 (+ Arc<dyn ReadJudgment> 를 U7a/U7b 에)
  -> UploadHistoryStore 를 Arc<dyn HistorySink>     로 U8 코디네이터에 주입
  -> CriticalErrorNotifier 를 Arc<dyn CriticalEventSink> 로 U4/U5/U7a/U8 에 주입
  -> TrayIndicator(no-op) 를 CriticalErrorNotifier + (렌더용) StatusService 소비자에 주입
```

**U6-내부 배선(모두 U6, U8이 조립):**

```
CriticalErrorNotifier -> Arc<dyn Logger>(StructuredLogger)      : 상향 심각도 로그
CriticalErrorNotifier -> Arc<dyn StatusSink>(StatusService)     : raise_condition
CriticalErrorNotifier -> StatusService escalation 입력(고유 메서드, D6) : 연속실패 health 반영
CriticalErrorNotifier -> TrayIndicator(no-op)                   : notify(선택)
```

**owned-elsewhere(U6가 받기만 하는 것):**

| 개념 | 소유/push 주체 | U6 역할 |
|---|---|---|
| 로그 레코드(`LogRecord`) 내용 | 각 하위 단위(U1~U5/U8) | 방출(직렬화·기록)만 |
| 상태 신호(`set_operational`/`raise_condition`/...) | U8 코디네이터·U4·U5 | 집계·판정만 |
| 히스토리 레코드 조립·push 시점 | U8 `SyncCycleCoordinator`(사이클 종료) | append 지속만(D11) |
| 중대오류 발생 원천(401/프리플라이트/롤백) | U5/U1·U3/U7a | 표면화(fan-out)만 |
| `cycle_id` 발급 | U8 코디네이터 | 로그 상관 필드로 통과 |
| config 값(`log_level`/rotation/N) | U0 `ConfigProvider` | 구독·소비 |
| 안전 한도 상수·`LimitReport` | U1 `SafetyLimitsValidator` | `report_preflight_exceeded` 표면화만 |
| 자동 롤백 판정 | U7a `AutoUpdater`(`update_probe` 소비) | `update_probe` 판정 제공 + `report_update_rollback` 표면화 |

---

## 1. StructuredLogger 로직 (US-E5-01, FR-17)

### 1.1 방출 파이프라인 (log/event -> JSON-line)

```
호출자(U1~U5/U8) -> Logger.event(level, event, cycle_id, fields)  또는  Logger.log(record)
  -> [event 는 LogRecord 조립: {level, event, cycle_id, message, fields}]
  -> [레벨 필터: level >= 활성 log_level 아니면 drop (R-LOG-02)]
  -> [timestamp stamping: 로거가 방출 시각 부가 (D4)]
  -> [redaction 확인: 토큰 원문 배제 (R-LOG-04/SEC-02)]
  -> [JSON-line 직렬화: {timestamp, level, event, cycle_id, message, fields} + 개행]
  -> [동기 best-effort 쓰기 to log_file (R-LOG-03; IO 오류 삼킴)]
  -> [크기 초과 시 rotation (R-LOG-05): log_file -> 회전, log_max_retained 초과분 삭제]
```

- **infallible 표면**: 어느 단계 실패도 `log`/`event` 를 실패시키지 않는다(R-PUSH-02). rotation 실패는 내부 `LogError` 로 분류되나 삼킨다.
- **cycle_id 상관**: 코디네이터가 사이클마다 `CycleId` 를 발급해 그 사이클의 모든 로그에 통과시킨다 -> 사후 분석 시 한 사이클의 이벤트를 상관 조회 가능.

### 1.2 리로드 재적용 (`ConfigReloadObserver`)

```
U0 ConfigProvider 성공 swap -> on_config_reload() 통지(push, 페이로드 없음)
  -> StructuredLogger 가 ConfigProvider.current() 재조회
  -> log_level / rotation 파라미터 재적용 (R-LOG-06)
  -> 실패 시 reload() -> Err(LogError) (데몬 불정지, keep-last-good)
```

---

## 2. StatusService 로직 — 2축 집계 (US-E5-02, Q9=B)

### 2.1 push 집계 흐름

```
하위 단위 -> StatusSink 뮤테이터 (in-memory, infallible):
  set_operational(state)          -> 축1 갱신
  raise_condition / clear_condition -> 축2 집합 add/remove (멱등, R-STATUS-02)
  record_sync_success(at)         -> last_success 갱신
  set_dirty / set_resume_progress -> dirty / resume 갱신
  set_liveness(signal)            -> liveness 신호 단조 누적 (R-STATUS-06)
CriticalErrorNotifier -> escalation 입력(고유 메서드, D6) -> 연속실패 플래그 갱신
```

- **단일 집계 지점**: 모든 상태 신호가 `StatusService` 한 곳에 모인다. 동시 push(다중 스레드)와 동시 읽기(CLI/ControlPlane)가 공존하므로 각 연산은 원자적으로 관측된다(스레딩 메커니즘은 NFR/Code-Gen 이월; 개념상 완결 스냅샷만 노출).

### 2.2 판정 흐름 (snapshot / health_check / update_probe)

```
snapshot() -> StatusSnapshot 파생 (R-STATUS-03):
  { operational, conditions(집합), last_success, dirty, resume,
    offline = (operational==Offline), consent = 파생(ConsentBlocked/last_success) }

health_check() -> Health (R-STATUS-04):
  if conditions ∩ {AuthFailed,OverLimit,UpdateRolledBack} ≠ ∅  OR  escalation:
       Unhealthy{ reasons: 활성 사유별 HealthReason }
  else Healthy
  (ConsentBlocked/Offline/dirty/VaultUnavailable 는 사유 아님 — US-E5-04 닫힌 4조건 밖; VaultUnavailable 은 status/로그에만 노출)

update_probe() -> Liveness (R-STATUS-05, 격리):
  if {IdleReached, CredentialReadable} ⊆ 관측신호: Alive
  else NotReady{ missing }
  (운영 조건/상태와 완전 무관 -> 롤백 루프 방지)
```

- **경계**: `health_check()` 종료코드 매핑(0 healthy / non-zero)은 U7b `OperatorCli` 소관(US-E5-02); U6는 `Health` 값만 제공. `update_probe()` 는 U7a `AutoUpdater` 전용 소비.

---

## 3. UploadHistoryStore 로직 — append-only (US-E5-03, FR-16)

### 3.1 append 흐름 (사이클 종료 -> 지속)

```
U8 SyncCycleCoordinator (사이클 종료)
  -> UploadHistoryRecord 조립 { timestamp, bytes_transferred, error_detail }  (U0 FROZEN 3필드; 성공 = error_detail None / 실패 = Some)
  -> HistorySink.append(record)  [infallible, R-HIST-03]
     -> [CoreTypes.encode(record) -> CBOR 바이트 (R-HIST-02, NFR-13)]
     -> [길이 프레이밍: len 헤더 + CBOR 프레임]
     -> [append-only 파일에 원자적 append (수정/삭제 없음, R-HIST-01)]
     -> [IO/코덱 오류 시 삼키고 로거로 남김 (R-PUSH-02)]
```

- **경계**: 레코드 조립(3필드 채움)은 코디네이터(U8, 사이클 결과 소비); U6는 append 지속만(D11). U6는 U3를 호출하지 않는다.

### 3.2 query 흐름 (CLI 조회)

```
U7b OperatorCli/ControlPlane -> UploadHistoryStore.query(HistoryQuery)
  -> [파일을 프레임 순차 디코드 (decode, R-HIST-02)]
  -> [트레일링 부분 프레임 = truncated tail 로 간주해 정상 prefix 반환 (R-HIST-04)]
  -> [중간 프레임 손상 = HistoryError::Corrupt]
  -> [AND 필터 적용: since/only_failures (R-HIST-05)]
  -> Result<Vec<UploadHistoryRecord>, HistoryError>
```

- **인덱싱 없음(D8 트림)**: 임베디드 DB/보조 인덱스 없이 전체 스캔 + 선형 필터. MVP 규모(단일 사용자 데스크톱 데몬)에 충분.

---

## 4. CriticalErrorNotifier 로직 — 닫힌 4조건 (US-E5-04, FR-19)

### 4.1 fan-out 흐름 (케이스별)

```
report_auth_failure(TransportError)                     [케이스1]
  -> Logger(Error) + StatusSink.raise_condition(AuthFailed) + Tray.notify(no-op)

report_cycle_result(Success)                            [케이스2 리셋]
  -> consecutive_failure_count = 0; escalation 해제 (R-CRIT-02)
report_cycle_result(Failure(ClassifiedError))           [케이스2 카운트]
  -> consecutive_failure_count += 1
  -> if count == N (notify_consecutive_failures, 기본 3):   [임계 도달 1회 발화]
       Logger(Error) + StatusService escalation 세팅(D6) + Tray.notify(no-op)
     (후속 실패 재발화 없음, R-CRIT-02)

report_preflight_exceeded(LimitReport)                  [케이스3]
  -> Logger(Warn/Error) + StatusSink.raise_condition(OverLimit) + Tray.notify(no-op)

report_update_rollback(from, to, RollbackReason)        [케이스4]
  -> Logger(Error) + StatusSink.raise_condition(UpdateRolledBack) + Tray.notify(no-op)
```

- **닫힘(R-CRIT-01)**: 위 4개 진입점 외 어떤 것도 능동 표면화하지 않는다. 비목록 오류는 하위 단위가 그냥 `Logger` 로만 남긴다(U6는 관여 안 함).
- **조건 해제(R-CRIT-04)**: `OverLimit`/`AuthFailed` 등 활성 조건의 clear 는 원천 회복 시 상위(U8/U3)가 `clear_condition` push; 케이스2 escalation 은 사이클 성공 시 자체 리셋.

### 4.2 3중 표면 = 데몬 모델 활성 표면

US-E5-04 활성 표면 = 상향 심각도 로그 + health unhealthy 전환 + CLI status 반영. Tray는 **추가** 표면일 뿐이며 no-op(D1)이어도 앞 3개는 항상 성립(트레이 비의존).

---

## 5. TrayIndicator 로직 (선택 — no-op, D1)

```
start(TrayConfig) -> Ok(None)      // 헤드리스/MVP: 실제 트레이 미생성
render(snapshot)  -> 무연산
notify(message)   -> 무연산
stop()            -> 무연산
```

- 실제 데스크톱 트레이 백엔드(3-OS)는 MVP DROP. 향후 confirm 시 이 no-op을 실체 구현으로 교체(다른 컴포넌트 불변).

---

## 6. Testable Properties (PBT-01) — 로직/흐름 계층

> **확장 강제(PBT-01, Full)**: 로직·흐름 계층 속성을 식별한다. 타입 round-trip은 `domain-entities.md` §8, 규칙 오라클/불변은 `business-rules.md` §8이 소유(중복 회피). 여기서는 **컴포넌트 워크플로/합성** 관점 속성을 재확인·집약한다.

### 6.1 컴포넌트별 Testable-Properties 노트

| 컴포넌트 | 핵심 흐름 속성 | 소유 PROP |
|---|---|---|
| `StructuredLogger` | 방출 라인 파싱 무결(round-trip) + 레벨 필터 정확 | PROP-U6-DE-02, PROP-U6-BR-01 |
| `StatusService` | 2축 독립·파생 결정성 + health 오라클 + **update_probe 격리** | PROP-U6-BR-02/03/04/05 |
| `UploadHistoryStore` | 레코드 무손실 round-trip + append-only 불변 + query 오라클 + truncated-tail 관용 | PROP-U6-DE-01, PROP-U6-BR-08/09 |
| `CriticalErrorNotifier` | 닫힌 4조건 불변 + 연속실패 임계 상태머신 | PROP-U6-BR-06/07 |
| `TrayIndicator`(no-op) | 없음(동작 부재, D1) | No PBT properties identified |

### 6.2 흐름 계층 통합 속성(합성 관점)

- **PROP-U6-BL-01 — fan-out 정합성** (카테고리: **Oracle**): 임의 `report_*` 호출에 대해, 3중 fan-out(로그 방출 1건 + 정확한 조건/ escalation 상태 변화 + tray notify 시도)이 케이스별 참조 매핑(R-CRIT-03)과 일치한다. tray no-op 여부와 무관하게 로그+status 반영은 항상 발생.
  - **제너레이터(PBT-07)**: 4개 report 입력(임의 `TransportError`/`CycleOutcome`/`LimitReport`/`Version` 쌍) 시퀀스.
- **PROP-U6-BL-02 — 관측 실패 비차단(infallible 합성)** (카테고리: **Invariant**): 히스토리 append IO 오류·로그 쓰기 오류를 주입해도 push 표면은 오류를 전파하지 않고 반환한다(R-PUSH-02) — 호출자(코어) 관점에서 관측 실패가 사이클을 막지 않음.
  - **제너레이터(PBT-07)**: IO 오류 주입 어댑터 + push 호출 시퀀스(단위테스트/모킹 성격, PBT는 오류-주입 조합 탐색).

### 6.3 속성 없음(No PBT properties identified) 판정

| 흐름 요소 | 판정 | 근거 |
|---|---|---|
| rotation 트리거 흐름(§1.1) | 예제 기반 단위테스트 | 크기 임계 전후 결정적 소수 케이스 — 예제테스트 적합 |
| reload 재적용(§1.2) | U0 config 흐름(PROP-BL-04)에 흡수 | 리로드 멱등성은 U0 소유; U6는 재적용 수신만 |
| 주입 방향(§0) | **No PBT properties identified** | 구조(합성) — 컴파일타임 크레이트 그래프 강제 |
| TrayIndicator no-op(§5) | **No PBT properties identified** | 동작 부재(D1) |

> **제너레이터(PBT-07) 총괄**: 위 속성은 `LogRecord`·`UploadHistoryRecord`·상태 명령 시퀀스·`report_*` 시퀀스·IO-오류 주입 어댑터 제너레이터를 요구한다. 구현·shrinking·고정 시드·CI(PBT-08)는 Code Generation/Build-and-Test 이월, 프레임워크(PBT-09, proptest 유력)는 NFR Requirements 이월이다.

---

## 7. 확장 컴플라이언스 요약 (이 산출물 범위)

| 확장 | 활성 | 이 산출물 적용 | 판정 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제·준수** | §6에 흐름/합성 속성(PROP-U6-BL-01/02) + 컴포넌트별 소유 PROP 집약. fan-out 오라클·관측 실패 비차단 명시. 속성 없는 흐름은 §6.3 판정. |
| **Resiliency Baseline** | ON | **부분 적용** | 관측 push best-effort·infallible(PROP-U6-BL-02)로 코어 비차단. 히스토리 append-only + truncated-tail 관용(§3.2)으로 크래시 후 재기동 질의 성공. `update_probe` 격리(§2.2)가 RESILIENCY-04 자동 롤백 게이트 근거. RTO/RPO 수치·배포·HA/DR은 인프라/상위 단계 -> N/A. |
| **Security Baseline** | OFF | **N/A** | 미로딩. 로그·히스토리 로컬 평문(RISK-01)은 수용 위험. 토큰 redaction(SEC-02)은 §1.1 방출 파이프라인에 잔존 통제로 반영. |

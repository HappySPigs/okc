# U6 Observability — Domain Entities (도메인 엔티티 / 값 타입)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U6 Observability** -> Functional Design -> 산출물 1/3 (`domain-entities.md`)
**작성일**: 2026-09-08
**크레이트**: `observability` (lib) · **소속 컴포넌트**: `StructuredLogger`, `StatusService`, `UploadHistoryStore`, `CriticalErrorNotifier`, `TrayIndicator`(선택)
**전제(AUTOPILOT 확정, plan §3)**: D1(트레이 DROP=no-op) · D2(JSON-line + 크기 rotation) · D4(로거 stamping) · D8(append-only CBOR 프레임) · D10(히스토리 스키마) · D13(N=config) · D14(U6 config 키)

> **문서 성격**: 이 문서는 U6가 **도입하거나 특화하는 값 타입**을 정의한다. **U0 `foundation` 소유 타입은 재정의하지 않으며**(정확한 이름으로 참조하고 소비/확장 방식만 기술), U0가 "하류(U6) 확정"으로 위임한 자리표시(placeholder) 스키마(`LogRecord`/`UploadHistoryRecord`/`HealthReason`)의 **최종 필드 스키마**를 여기서 확정한다. 규칙·불변식은 `business-rules.md`, 알고리즘·흐름은 `business-logic-model.md`가 소유한다.
>
> **표기 규약**: 비즈니스 의미 중심의 **기술중립 설계**. Rust스러운 시그니처는 **참고용(reference-only)** 이며 인프라·스레딩·I/O 메커니즘이 아니라 개념 형상을 표현한다. 다이어그램은 ASCII 박스 없이 **화살표 표기(A -> B)** 와 표/목록으로 기술한다. English 식별자명은 원문 그대로 유지한다.

---

## 0. U0에서 소비하는 타입(재정의 금지 — 참조만)

U6는 아래 U0 타입/트레이트를 **구현/소비**한다. 재정의하지 않는다(drop-list, plan §5).

| U0 타입/트레이트 | U6에서의 역할 |
|---|---|
| `Logger`(트레이트) | `StructuredLogger`가 구현 |
| `StatusSink` + `ReadJudgment`(트레이트) | `StatusService`가 둘 다 구현 |
| `HistorySink`(트레이트) | `UploadHistoryStore`가 구현 |
| `CriticalEventSink`(트레이트) | `CriticalErrorNotifier`가 구현 |
| `ConfigReloadObserver`(트레이트) | `StructuredLogger`가 구현(로그레벨 재적용) |
| `LogRecord`/`LogLevel`/`LogFields`/`CycleId` | 로그 방출 입력(§1.1에서 스키마 확정) |
| `UploadHistoryRecord` | 히스토리 append 레코드(§2.1에서 스키마 확정) |
| `OperationalState`/`ActiveCondition`/`LivenessSignal`/`StatusSnapshot`/`ConsentState` | 2축 상태 어휘(§3에서 소비 규칙) |
| `Health`/`HealthReason`/`Liveness` | 판정 반환 형상(§3.2/§3.3) |
| `TransportError`/`ClassifiedError`/`CycleOutcome`/`LimitReport`/`Version`/`RollbackReason` | 중대오류 보고 입력(§4) |
| `Timestamp`/`ByteCount`/`ManifestDigest`/`TokenSecret` | 프리미티브 |
| `encode`/`decode`(CBOR 코덱, NFR-13) | 히스토리 지속 round-trip |

---

## 1. 로그 도메인 값 타입 (StructuredLogger)

### 1.1 방출 로그 라인 스키마 (`LogRecord` 확장 + stamped `timestamp`)

U0 `LogRecord` 자리표시는 `{ level: LogLevel, event: String, cycle_id: Option<CycleId>, message: String, fields: LogFields }` 이며 **`timestamp` 필드가 없다**(sink.rs). U6는 U0를 변경하지 않으므로(쓰기 범위 밖), **방출 시각을 `StructuredLogger`가 stamping** 한다(D4). 즉 디스크에 기록되는 **JSON-line 방출 레코드**의 개념 스키마는:

| 필드 | 개념 타입 | 소재 | 의미 |
|---|---|---|---|
| `timestamp` | `Timestamp`(UTC, 결정적 표현) | **방출 시점 로거가 stamping** | 레코드 방출 벽시계 시각(US-E5-01 필수) |
| `level` | `LogLevel`(U0) | `LogRecord` | 심각도(`Trace`<`Debug`<`Info`<`Warn`<`Error`) |
| `event` | 문자열 | `LogRecord` | 이벤트 식별자(예: `cycle.start`, `preflight.exceeded`) |
| `cycle_id` | `Option<CycleId>`(U0) | `LogRecord` | 사이클 상관관계 키(있으면) |
| `message` | 문자열 | `LogRecord` | 사람이 읽는 메시지 |
| `fields` | `LogFields`(U0, `Vec<(String,String)>`) | `LogRecord` | 구조화 필드맵 |

참고 형상(방출 라인 = U0 `LogRecord` + stamped `timestamp`):
```
struct EmittedLogLine { timestamp: Timestamp, /* + LogRecord 전 필드 */ }
```

- **최소 필드 계약(US-E5-01)**: 각 방출 라인은 `{ timestamp, level, event, cycle_id, message }` 를 항상 포함한다(`cycle_id` 는 `null` 허용).
- **불변식**: 방출 라인은 **기계 판독 가능한 JSON-line 1건**(개행으로 구분)이다. `message`/`fields`/`event` 는 유니코드·개행을 포함할 수 있으며 JSON 문자열 이스케이프로 무손실 표현된다.
- **redaction(SEC-02 잔존 통제)**: `fields`/`message` 는 config 유래 비밀(토큰)을 원문으로 담지 않는다. 토큰은 U0 `TokenSecret`(redacting `Debug`/`Display`)로만 표현되며, 로거는 per-field 프로젝션만 방출한다(규칙 상세 `business-rules.md` R-LOG-04).

### 1.2 `LogConfig` (U6 소유 config 섹션 — D2/D14)

`StructuredLogger`가 소비하는 로그 설정. U6 federated known-key(§5) 중 로그 관련 필드의 타입드 뷰.

| 필드 | 개념 타입 | 필수? | 기본값 | 의미 |
|---|---|---|---|---|
| `log_level` | `LogLevel`(U0 소유 필드) | 아니오 | `Info` | 최소 방출 레벨(U0 `WatcherConfig.log_level` 재사용) |
| `log_file` | 문자열(파일 경로) | 아니오 | 플랫폼 기본 로그 경로 | JSON-line 방출 대상 파일 |
| `log_max_size_bytes` | `ByteCount` | 아니오 | 문서화된 기본값 | rotation 트리거 크기 임계 |
| `log_max_retained` | 비음 정수 | 아니오 | 문서화된 기본값 | 보존할 회전 파일 수(나이/기간 대신 개수 근사 — D2 MVP 트림) |

- **불변식**: `log_max_size_bytes >= 1`, `log_max_retained >= 0`. `log_level`은 U0 열거값. rotation 은 **크기 기반**만(age/기간 기반 미구현, D2 트림).

### 1.3 `LogError` (내부 오류 — 표면 밖)

component-methods의 `LogError{ Io, InvalidPath, RotationFailed }` 를 내부 오류로 유지하되, **`Logger::log`/`event` push 표면은 infallible**(U0 계약)이므로 이 오류는 표면에 전파되지 않고 best-effort로 삼켜진다(D3, 규칙 `business-rules.md` R-LOG-03). `reload(cfg)` 만 `Result<(), LogError>` 로 실패를 반환한다(config 재적용 경로).

---

## 2. 히스토리 도메인 값 타입 (UploadHistoryStore)

### 2.1 `UploadHistoryRecord` 최종 필드 스키마 (D10 — U0 자리표시 확정 = 3필드 MVP)

U0 `UploadHistoryRecord` 자리표시는 `{ timestamp, bytes_transferred, error_detail }`(FROZEN, sink.rs) 뿐이며 U0 note가 "최종 스키마는 하류(U6)가 확정"으로 위임했다. U6는 이 위임을 **U0의 기존 3필드를 그대로 최종 MVP 스키마로 확정**하는 것으로 이행하며 U0 구조체는 **편집하지 않는다**(FROZEN 유지). `HistorySink::append(&self, record: UploadHistoryRecord)`(U0 FROZEN) 페이로드가 정확히 이 3필드이므로 다른 필드는 물리적으로 U6까지 전달될 수 없다:

| 필드 | 개념 타입 | 의미 | 비고 |
|---|---|---|---|
| `timestamp` | `Timestamp`(U0) | 사이클 종료 시각 | `--since` 질의 키 |
| `bytes_transferred` | `ByteCount`(U0) | 전송 바이트 수 | |
| `error_detail` | `Option<String>` | 사이클 결과 상세 | `None` = 성공, `Some(_)` = 실패 상세(자유형·유니코드, RISK-01: 민감정보 혼입 가능·수용) |

참고 형상(U0 기존 정의 그대로 — 재정의 아님):
```
struct UploadHistoryRecord {
    timestamp: Timestamp,
    bytes_transferred: ByteCount,
    error_detail: Option<String>,
}
```

- **성공/실패 파생**: 별도 status 필드 없이 `error_detail` 로 판정한다 — `None` 이면 성공, `Some(detail)` 이면 실패(상세는 문자열). CLI history 표시/필터는 이 파생을 사용한다(`business-rules.md` R-HIST-05).
- **불변식**: 3필드 전부 무손실 round-trip 대상(NFR-13, `decode(encode(r)) == r`). append-only 계약상 레코드는 **한 번 append되면 불변**(수정/삭제 없음, US-E5-03).
- **U0 편집 없음**: 본 FD는 U0의 FROZEN 3필드를 최종 스키마로 **확정만** 하며 U0 파일을 이 단계 또는 Code Generation에서 편집하지 않는다.
- **POST-MVP 이월(MVP 트림)**: (1) 명시적 상태 enum(성공/실패/부분 3분류)과 (2) 업로드 콘텐츠 아이덴티티 링크(로컬 스냅샷 다이제스트 + 서버 콘텐츠 id)는 MVP에서 제외한다 — 둘 다 U0 FROZEN 3필드 `append` 페이로드로는 U6에 도달할 수 없어 U0 트레이트 변경 없이는 실을 수 없기 때문이며, U0를 FROZEN으로 유지하기 위해 이월한다.

### 2.2 콘텐츠 아이덴티티 링크 (POST-MVP 이월)

MVP 히스토리 레코드(§2.1, U0 FROZEN 3필드)는 업로드 콘텐츠 아이덴티티 링크를 담지 않는다. 로컬 스냅샷 다이제스트(`ManifestDigest`)와 서버 권위 콘텐츠 id(성공 커밋 응답에서만 획득, FQ-1=A)를 레코드에 연결하는 것은 **POST-MVP 이월**이다 — U0 FROZEN `append` 페이로드(3필드)로는 이 값들이 U6에 도달할 수 없어 U0 트레이트 변경 없이는 실을 수 없기 때문이다. MVP는 `timestamp`/`bytes_transferred`/`error_detail`(성공/실패 파생)만으로 "언제·얼마나·성공했는가"를 추적한다(US-E5-03 감사 목적의 최소 충족).

### 2.3 `HistoryQuery` / `HistoryError`

```
struct HistoryQuery {                                         // CLI watcher history 필터
    since: Option<Timestamp>,
    only_failures: Option<bool>,  // None = 무제약, Some(true) = 실패만, Some(false) = 성공만 (error_detail 파생)
}
enum HistoryError { Io, Serde, Corrupt }                      // query() 경로 오류
```

- **`HistoryQuery` 의미**: 두 필터는 **AND 결합**(모두 만족하는 레코드 반환). 필드가 `None`이면 해당 축 제약 없음. `only_failures` 는 `error_detail` 유무로 파생(성공 = `None` / 실패 = `Some`)해 필터한다(규칙 `business-rules.md` R-HIST-05). 콘텐츠 id 기준 필터는 콘텐츠 링크가 MVP 레코드에 없으므로 POST-MVP 이월(§2.2).
- **`HistoryError`**: `append` 는 infallible(오류 삼킴, D9); `query` 만 이 오류를 반환한다. `Corrupt` 는 파일 **중간** 손상(트레일링 truncated-tail은 정상 prefix 반환 — R-HIST-04).

---

## 3. 상태 도메인 값 소비 규칙 (StatusService)

U6는 2축 상태 어휘를 **재정의하지 않고**(U0 소유) 집계·판정 규칙만 정의한다. `StatusService`는 `StatusSink`(push 뮤테이터) + `ReadJudgment`(`health_check`/`update_probe`) + 고유 `snapshot()` 를 구현하는 **단일 집계 지점**이다.

### 3.1 `StatusSnapshot` 파생 필드 규칙

`StatusSnapshot`(U0)의 각 필드가 push 입력으로부터 어떻게 채워지는지(파생 규칙 상세 `business-rules.md`):

| `StatusSnapshot` 필드 | 채움 소스 | 규칙 |
|---|---|---|
| `operational` | `set_operational` 최신값 | 축1 권위값(push된 마지막 값) |
| `conditions` | `raise_condition`/`clear_condition` | 축2 = raise된 조건 - clear된 조건(중복 없는 집합) |
| `last_success` | `record_sync_success(at)` | 최신 성공 `Timestamp` |
| `dirty` | `set_dirty(bool)` | 미커밋 변경 대기 표시 |
| `resume` | `set_resume_progress(t, total)` | 진행 중 전송 `(전송량, 전체)` |
| `consent` | `raise/clear_condition(ConsentBlocked)` + `record_sync_success` | **파생**: `ConsentBlocked` 활성이면 `Blocked`; 아니고 성공 이력 있으면 `Granted`; 그 외 `Unknown`(D5 파생 — `StatusSink`에 `set_consent` 부재) |
| `offline` | `operational == Offline` | **파생**: 운영 상태가 `Offline`이면 true |

- **`consent`/`offline` 파생 근거**: `StatusSink` 트레이트(U0)에 전용 setter가 없으므로 기존 push 신호에서 결정론적으로 도출한다(신규 U0 트레이트 메서드 미도입, D5/D6).

### 3.2 `Health` 판정 형상 (health_check, D5)

U0 `Health{ Healthy | Unhealthy{ reasons: Vec<HealthReason> } }` 를 사용. U6는 `HealthReason` 자리표시(자유형 문자열)의 **내용 규칙**을 확정:

- `Unhealthy` 는 **health-critical 조건**(`AuthFailed`/`OverLimit`/`UpdateRolledBack`) 중 하나 이상 활성이거나 **연속실패 escalation**(§4, D6)이 활성일 때. 각 활성 사유마다 `HealthReason` 1건. 이 집합은 US-E5-04 닫힌 4조건(케이스1/2/3/4)과 정확히 일치한다(`business-rules.md` R-STATUS-04).
- `ConsentBlocked`/`Offline`/`dirty` 는 **의도된 정상 상태**로 `Unhealthy` 사유가 아니다(D5, 알림 피로 회피). `VaultUnavailable` 도 US-E5-04 닫힌 집합 밖이라 `health_check()` 를 flip 하지 않으나, 축2 운영 조건으로 `snapshot()`/status·로그에는 계속 노출된다.

### 3.3 `Liveness` 판정 형상 (update_probe, D7)

U0 `Liveness{ Alive | NotReady{ missing: Vec<LivenessSignal> } }` 를 사용:

- `Alive` iff `{IdleReached, CredentialReadable}` 두 신호 모두 관측됨.
- 아니면 `NotReady{ missing }`(미관측 신호 목록).
- **운영 조건(축2)과 완전 분리** — `AuthFailed`/`OverLimit` 등 어떤 활성 조건도 `update_probe` 결과에 영향을 주지 않는다(§0 노트3 롤백 루프 방지, U7a `AutoUpdater` 전용 소비).

### 3.4 연속실패 escalation 내부 상태 (D6)

`StatusService`는 `CriticalErrorNotifier`가 U6-내부로 push하는 **연속실패 escalation 플래그**(boolean)를 보유한다(비트레이트 고유 입력). 이 플래그가 활성이면 `health_check()`가 `Unhealthy` 사유에 포함한다. `snapshot()` 에도 반영(CLI status). 이는 `ActiveCondition`(U0 닫힌 enum) 밖의 U6-내부 상태이며 두 U6 컴포넌트가 조립 루트(U8)에서 배선된다(U1~U5 역참조 없음).

---

## 4. 중대오류 도메인 값 소비 (CriticalErrorNotifier)

`CriticalEventSink`(U0) 4개 메서드의 입력 타입은 모두 U0 소유이며 재정의하지 않는다:

| 메서드 | 입력 타입(U0) | FR-19 케이스 |
|---|---|---|
| `report_auth_failure` | `TransportError` | 케이스1: 401/토큰 거부 |
| `report_cycle_result` | `CycleOutcome`(`Success`\|`Failure(ClassifiedError)`) | 케이스2: 연속 실패 카운트 |
| `report_preflight_exceeded` | `LimitReport` | 케이스3: 프리플라이트 초과 |
| `report_update_rollback` | `Version`, `Version`, `RollbackReason` | 케이스4: 자동 업데이트 롤백 |

- **U6 도입 내부 상태**: `consecutive_failure_count`(비음 정수) — `report_cycle_result(Failure)` 마다 +1, `Success` 시 0으로 리셋. 임계 `N = notify_consecutive_failures`(U0 config, 기본 3, D13). 이 값은 값 타입이 아니라 컴포넌트 내부 상태이므로 규칙은 `business-rules.md` R-CRIT-02.
- **닫힌 집합 불변식**: 능동 표면화는 위 4개 메서드로만 진입한다(닫힘, US-E5-04). 비목록 오류는 로그 전용(로거로만).

---

## 5. TrayIndicator 값 타입 (선택 — no-op, D1)

MVP는 실제 데스크톱 트레이를 **DROP**하고 nullable no-op 싱크만 둔다(§0 수정2). 관련 값 타입은 최소:

| 타입 | 형상 | MVP 동작 |
|---|---|---|
| `TrayConfig` | `{ tray_enabled: bool }` | config 파싱만(호환 유지); no-op은 값 무시 |
| `TrayHandle` | 불투명 핸들 | no-op은 항상 `None` 반환(`start() -> Ok(None)`) |
| `NotificationMessage` | `{ title: String, body: String }` | no-op은 무시 |
| `TrayError` | `{ Unsupported, InitFailed }` | 비치명(모두 best-effort) |

- **불변식**: `TrayIndicator` 부재/no-op는 다른 어떤 표면(로그·status·health·history)도 막지 않는다(US-E5-04/05, FR-18). `notify`/`render` 는 no-op에서 무연산.

---

## 6. U6 config 키 (federated known-key — D14)

U0 `FOUNDATION_CONFIG_KEYS`(model.rs) 패턴을 따라 U6가 자기 키를 선언한다. `watcher-bin`(U8)이 전 단위 키를 federated union으로 집계해 `ConfigProvider`에 주입하며, 이 선언이 누락되면 R-CFG-STRICT-01(strict unknown-key reject, U0)이 U6 키를 오탐 거부한다.

참고 형상:
```
const OBSERVABILITY_CONFIG_KEYS: &[&str] =
    &["log_file", "log_max_size_bytes", "log_max_retained", "history_file", "tray_enabled"];
```

- `log_level`/`notify_consecutive_failures` 는 **U0 소유 키**이므로 U6 목록에 포함하지 않는다(U6는 소비만).
- `tray_enabled` 는 D1(트레이 DROP)로 인해 파싱만 되고 동작은 no-op(호환 유지).

---

## 7. 엔티티 관계 개요 (화살표 표기 — ASCII 박스 미사용)

**구현/소비 방향("A -> B" 는 A가 B에 의존/구현):**

- `StructuredLogger` -- implements --> `Logger`(U0) + `ConfigReloadObserver`(U0)
- `StatusService` -- implements --> `StatusSink`(U0) + `ReadJudgment`(U0), -- 고유 --> `snapshot() -> StatusSnapshot`(U0)
- `UploadHistoryStore` -- implements --> `HistorySink`(U0), -- 고유 --> `query(HistoryQuery) -> Result<Vec<UploadHistoryRecord>, HistoryError>`
- `CriticalErrorNotifier` -- implements --> `CriticalEventSink`(U0), -- 주입 --> `Arc<dyn Logger>` + `Arc<dyn StatusSink>`(+ escalation 입력) + `TrayIndicator`(no-op)
- `TrayIndicator`(no-op) -- 주입받음 --> `CriticalErrorNotifier`(§0 수정2)

**값 합성("A -> B" 는 A가 B를 필드로 포함):**

- `UploadHistoryRecord` -> `Timestamp`(1) + `ByteCount`(1) + `Option<String>`(error_detail) — U0 FROZEN 3필드(재정의 아님)
- `EmittedLogLine` -> `Timestamp`(stamped) + `LogRecord`(U0 전 필드)
- `StatusSnapshot`(U0) <- 파생 <- push 신호(`set_operational`/`raise_condition`/... + 연속실패 escalation)

**지속 포맷 경로:**

- `UploadHistoryRecord` -> `CoreTypes.encode`(CBOR) -> 길이-프레이밍 프레임 -> append-only 파일(U6 I/O) -> `decode` -> 원 레코드(round-trip 무손실, NFR-13)
- `EmittedLogLine` -> JSON-line 직렬화 -> `log_file`(크기 rotation) — **JSON**(사람 가독), CBOR 아님(로그는 히스토리와 다른 경로)

**텍스트 설명(다이어그램 대안)**: U6의 4개 실체 싱크는 모두 U0가 정의한 트레이트를 구현하고 U8이 하위 단위에 하향 주입한다. 값 타입 관점에서 U6가 새로 확정하는 것은 (1) 히스토리 레코드 최종 스키마(U0 FROZEN `UploadHistoryRecord` 3필드를 최종 MVP 스키마로 확정 — 재정의 아님), (2) 방출 로그 라인(U0 `LogRecord` + stamped `timestamp`), (3) U6 config 섹션(`LogConfig`/`TrayConfig` + `OBSERVABILITY_CONFIG_KEYS`)뿐이며, 상태 어휘(`StatusSnapshot`/`Health`/`Liveness`)는 U0 형상을 소비하고 파생/판정 규칙만 `business-rules.md`에 둔다. 히스토리는 무손실 CBOR로, 로그는 사람이 읽는 JSON-line으로 서로 다른 경로에 지속된다.

---

## 8. Testable Properties (PBT-01) — 엔티티/타입 계층

> **확장 강제(PBT-01, Full)**: 이 산출물은 U6 엔티티/타입 계층 속성을 식별한다. 규칙 계층 속성(레벨 필터·health 오라클·닫힌 집합)은 `business-rules.md`, 흐름 계층 속성(append/query·fan-out)은 `business-logic-model.md`가 소유한다(중복 회피). 카테고리 라벨 {Round-trip, Invariant, Idempotence, Oracle, Induction, Easy verification} + 제너레이터(PBT-07) 요구를 기재한다.

### 8.1 히스토리 레코드 무손실 round-trip (핵심 — NFR-13 소비)

- **PROP-U6-DE-01 — `decode(encode(r)) == r`** (카테고리: **Round-trip**; PBT-02; US-E5-03/NFR-13)
  - **대상**: 모든 `UploadHistoryRecord`(3필드: 임의 `timestamp`, `bytes_transferred` 경계 0·대값, `error_detail` `None`(성공)/유니코드·개행/빈 문자열(실패)).
  - **속성**: 임의 레코드 `r`에 대해 U0 CBOR 코덱 round-trip 무손실. U0 `PROP-DE-01`(코덱)을 U6 레코드 타입으로 재확인(U0 note "각 단위에서 자기 제너레이터로 재확인").
  - **제너레이터(PBT-07)**: `UploadHistoryRecord` 도메인 제너레이터 — `error_detail` `None`(성공)/유니코드·개행/빈 문자열(실패), 0/경계/대값 `bytes_transferred`, 임의 `timestamp`.

### 8.2 로그 라인 라운드트립(방출 파싱 무결)

- **PROP-U6-DE-02 — 방출 로그 라인 파싱 무결** (카테고리: **Round-trip**)
  - **속성**: 임의 `LogRecord` + stamped `timestamp` 로 방출한 JSON-line을 재파싱하면 `{timestamp, level, event, cycle_id, message, fields}` 가 손실 없이 복원된다(유니코드·개행 `message`/`fields` 포함, `cycle_id` `None` 포함).
  - **제너레이터(PBT-07)**: `LogRecord` 제너레이터 — 전 `LogLevel`, `cycle_id` 존재/부재, 유니코드/개행/빈 `message`, 다수/빈 `fields`.

### 8.3 속성 없음(No PBT properties identified) 판정

| 타입 | 판정 | 근거 |
|---|---|---|
| `LogConfig`/`ObservabilityConfig` 키 | **독립 속성 없음**(config round-trip은 U0 PROP-BR-02에 흡수) | 값 뷰 — 검증 규칙은 `business-rules.md` |
| `Health`/`HealthReason`/`Liveness`(U0) | round-trip 외 **독립 속성 없음**(판정 오라클은 `business-rules.md`) | 반환 형상 — 판정 규칙은 규칙 계층 |
| `StatusSnapshot`(U0) 파생 | 파생 규칙 속성은 `business-rules.md`(PROP-U6-BR-*) | 집계 스냅샷 |
| `TrayConfig`/`TrayHandle`/`NotificationMessage` | **No PBT properties identified** | no-op(D1) — 동작 부재 |

> **제너레이터(PBT-07) 총괄**: 위 속성은 `UploadHistoryRecord`·`LogRecord` 도메인 제너레이터를 요구한다. 구체 구현·shrinking·고정 시드·CI 통합(PBT-08)은 Code Generation/Build-and-Test 이월이며, 프레임워크(PBT-09, Rust=proptest 유력)는 NFR Requirements 이월이다.

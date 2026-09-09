# U6 Observability — Business Rules (결정 규칙 / 검증 로직 / 제약)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U6 Observability** -> Functional Design -> 산출물 2/3 (`business-rules.md`)
**작성일**: 2026-09-08
**크레이트**: `observability` (lib) · **소속 컴포넌트**: `StructuredLogger`, `StatusService`, `UploadHistoryStore`, `CriticalErrorNotifier`, `TrayIndicator`(선택)
**전제(AUTOPILOT 확정, plan §3)**: D1..D14

> **문서 성격**: U6가 소유하는 **결정 규칙·검증 로직·제약·불변식**을 정의한다. 타입 정의는 `domain-entities.md`, 알고리즘·흐름은 `business-logic-model.md`가 소유하며 이 문서는 그 타입명·필드명을 **그대로 재사용**한다. U0 소유 타입/트레이트는 재정의하지 않는다(drop-list, plan §5).
>
> **표기 규약**: 기술중립 설계. Rust스러운 시그니처는 참고용. 다이어그램은 ASCII 박스 없이 **화살표 표기(A -> B)**. English 식별자명 유지.

---

## 1. push-only 구조 불변 (아키텍처 제약)

**규칙 R-PUSH-01 (U1~U5 역참조 금지)**: U6 컴포넌트는 U1~U5의 어떤 타입/함수도 참조하지 않는다. 관측을 위해 하위 단위가 **U0 트레이트**(`Logger`/`StatusSink`/`HistorySink`/`CriticalEventSink`)를 주입받아 값을 **push** 할 뿐이며, U6는 값을 **받기만** 한다. `observability` 크레이트의 `[dependencies]` 는 `foundation`(U0) 만이다(unit-of-work §3.2).

- **강제 방식**: 컴파일타임(Cargo 크레이트 그래프) — U6가 U1~U5 크레이트를 의존에 넣지 않으므로 역참조가 코드 리뷰/컴파일에 드러난다. **속성 테스트 대상 아님**(구조 불변, PROP는 §8.6 참조).

**규칙 R-PUSH-02 (관측 push 는 best-effort·infallible)**: `Logger::log`/`event`, `StatusSink` 전 뮤테이터, `HistorySink::append`, `CriticalEventSink` 전 report 는 **실패를 호출자에게 전파하지 않는다**(U0 계약). 관측 실패(로그 파일 쓰기 실패, 히스토리 append IO 오류 등)는 내부에서 삼키고 가능하면 로거로 남기며, **동기화 코어(U1~U5/U8)를 절대 막지 않는다**.

**규칙 R-PUSH-03 (읽기 판정만 Result/값 반환)**: 실패를 표면화하는 U6 표면은 **읽기/재적용 경로**뿐이다 — `UploadHistoryStore::query() -> Result<_, HistoryError>`, `StructuredLogger::reload() -> Result<_, LogError>`. `StatusService::snapshot()/health_check()/update_probe()` 는 in-memory·infallible.

---

## 2. StructuredLogger 규칙 (US-E5-01, FR-17)

**규칙 R-LOG-01 (JSON-line 방출)**: 각 로그 이벤트는 개행으로 구분되는 **JSON-line 1건**으로 방출된다. 필드 최소 집합 `{ timestamp, level, event, cycle_id, message }`(+ `fields`)를 포함한다(`domain-entities.md` §1.1). `cycle_id` 부재는 `null` 로 표현한다.

**규칙 R-LOG-02 (레벨 필터, D2)**: 방출은 `level >= 활성 log_level`(U0 `LogLevel` `Ord`: `Trace<Debug<Info<Warn<Error`) 인 레코드만 기록한다. 활성 미만 레벨은 방출하지 않는다(부수효과 없음).

**규칙 R-LOG-03 (동기 best-effort 쓰기, D3)**: 방출은 동기적으로(직렬화 접근 하) 수행되며, IO/rotation 오류는 `LogError` 로 내부 분류되나 **`log`/`event` 표면은 삼켜 infallible 유지**(R-PUSH-02). 비동기 로깅 스레드·bounded 채널·배칭은 MVP 미구현(D3 트림).

**규칙 R-LOG-04 (SEC-02 잔존 통제 — 토큰 원문 금지)**: Security Baseline OFF임에도, 로그는 config 유래 비밀(토큰)을 **원문으로 방출하지 않는다**. 토큰은 U0 `TokenSecret`(redacting `Debug`/`Display`)로만 표현되며 `message`/`fields` 에 평문 토큰을 넣지 않는다(U0 sink.rs SEC-02 계약). `error_detail`/`fields` 에 우발적으로 비밀이 섞일 수 있음은 RISK-01 수용 위험으로 문서화된다.

**규칙 R-LOG-05 (크기 기반 rotation, D2)**: `log_file` 크기가 `log_max_size_bytes` 를 초과하면 회전한다. 보존은 **회전 파일 수 `log_max_retained`** 로 상한한다(가장 오래된 초과분 삭제). **나이/기간 기반 보존은 MVP 미구현**(개수 근사, D2 트림).

**규칙 R-LOG-06 (리로드 재적용, `ConfigReloadObserver`)**: config 성공 리로드 시 `on_config_reload()` 통지를 받아 `log_level`(및 rotation 파라미터)을 **재적용**한다. 재적용 실패는 `reload() -> Err(LogError)` 로 표면화하되 데몬을 막지 않는다(U0 keep-last-good 정합).

---

## 3. StatusService 규칙 — 2축 상태 모델 (US-E5-02, Q9=B)

**규칙 R-STATUS-01 (2축 독립성)**: 상태는 **축1 = `operational`(단일 값)** + **축2 = `conditions`(동시 성립 집합)** 로 독립 표현된다. `set_operational` 은 축1만, `raise_condition`/`clear_condition` 은 축2만 변경한다. **축2 조건은 축1을 자동 변경하지 않고, 그 역도 없다**(D17 — 결합 로직 없음, 2축 직교). 예: `Syncing` 중 `OverLimit`+`AuthFailed` 동시 성립 가능.

**규칙 R-STATUS-02 (조건 집합 멱등)**: `raise_condition(c)` 는 `c` 를 집합에 추가(이미 있으면 무변화), `clear_condition(c)` 는 제거(없으면 무변화). `conditions` 는 **중복 없는 집합**이며 순서는 결정적(`ActiveCondition` `Ord` 정렬 권장).

**규칙 R-STATUS-03 (snapshot 파생 규칙)**: `snapshot()` 은 push된 최신 신호로부터 `StatusSnapshot` 을 도출한다(`domain-entities.md` §3.1):
- `operational` = 마지막 `set_operational` 값(초기 `Idle`).
- `conditions` = raise - clear 집합.
- `last_success` = 마지막 `record_sync_success(at)` 의 `at`.
- `dirty`/`resume` = 마지막 `set_dirty`/`set_resume_progress`.
- `offline` = `(operational == Offline)`(파생).
- `consent` = `ConsentBlocked` 활성이면 `Blocked`; 아니고 성공 이력(`last_success` 존재) 있으면 `Granted`; 그 외 `Unknown`(D5 파생 — `StatusSink`에 `set_consent` 부재).

**규칙 R-STATUS-04 (health_check 도출, D5)**: `health_check()` 는:
- `Unhealthy{reasons}` iff `conditions ∩ {AuthFailed, OverLimit, UpdateRolledBack} ≠ ∅` **또는** 연속실패 escalation 플래그 활성(§4, D6). 각 활성 사유마다 `HealthReason` 1건.
- 그 외 `Healthy`.
- **제외**: `ConsentBlocked`/`Offline`/`dirty` 는 의도된 정상 상태로 `Unhealthy` 사유가 아니다(알림 피로 회피).
- **권위(US-E5-04 닫힌 집합 AC)**: health-critical 집합은 US-E5-04 acceptance 의 닫힌 4조건 {`AuthFailed`(케이스1), 연속실패 escalation(케이스2), `OverLimit`(케이스3), `UpdateRolledBack`(케이스4)}과 **정확히 일치**하며, 비목록 조건은 health 를 flip 하지 않는다(`healthy` 유지). `VaultUnavailable` 은 축2 운영 조건으로 `snapshot()`/status·로그에는 계속 노출되나 US-E5-04 닫힌 집합 밖이라 `health_check()` 는 flip 하지 않는다.

**규칙 R-STATUS-05 (update_probe 격리, D7 — 롤백 루프 방지 핵심)**: `update_probe()` 는:
- `Alive` iff `{IdleReached, CredentialReadable}` 두 신호 모두 관측됨.
- 아니면 `NotReady{missing}`.
- **운영 조건(축2)·운영상태(축1)와 완전 분리** — 어떤 `ActiveCondition`(`AuthFailed`/`OverLimit`/...)도 `update_probe` 결과를 바꾸지 않는다(§0 노트3, U7a `AutoUpdater` 전용). 이 격리가 일시 조건에 의한 좋은 새 버전의 오판 롤백을 막는다.

**규칙 R-STATUS-06 (liveness 신호 단조)**: `set_liveness(signal)` 로 관측된 신호는 startup 이후 **단조 누적**된다(한 번 관측되면 유지). `record_sync_success` 는 `last_success` 갱신 시 `IdleReached` 도달을 함의하지 않는다(신호는 명시 push).

---

## 4. CriticalErrorNotifier 규칙 — 닫힌 4조건 (US-E5-04, FR-19)

**규칙 R-CRIT-01 (닫힌 집합)**: 능동 표면화는 **정확히 4개 진입점**으로만 발생한다:
1. `report_auth_failure`(케이스1: 401/토큰 거부)
2. `report_cycle_result`(케이스2: N회 연속 실패)
3. `report_preflight_exceeded`(케이스3: 프리플라이트 초과, FR-05)
4. `report_update_rollback`(케이스4: 자동 업데이트 롤백, FR-20)

그 밖의 어떤 이벤트·비목록 일시 오류도 능동 표면화하지 않는다(로그 전용, US-E5-01/04). 이 집합은 **닫혀 있다**.

**규칙 R-CRIT-02 (케이스2 연속 실패 임계, D12/D13)**:
- `report_cycle_result(Failure(_))` -> `consecutive_failure_count += 1`.
- `report_cycle_result(Success)` -> `consecutive_failure_count = 0` **그리고** 연속실패 escalation 플래그 해제(리셋).
- `consecutive_failure_count` 가 임계 `N`(= `notify_consecutive_failures`, U0 config, 기본 3)에 **도달하는 순간 1회** 능동 표면화하고 escalation 플래그를 세팅한다.
- **후속 실패마다 재발화하지 않는다**(escalation 유지, 중복 억제 MVP — D12). 성공 시에만 리셋.

**규칙 R-CRIT-03 (표면화 3중 fan-out)**: 각 능동 표면화는 3개 표면으로 fan-out 한다(component-methods §CriticalErrorNotifier):
1. **상향 심각도 구조화 로그**(`Logger` 로 `Warn`/`Error` 레벨 방출).
2. **`StatusService` 반영** — 케이스1 `raise_condition(AuthFailed)`, 케이스3 `raise_condition(OverLimit)`, 케이스4 `raise_condition(UpdateRolledBack)`, 케이스2 escalation 플래그 push(§3.4, D6). 이로써 `health_check()` unhealthy 전환(R-STATUS-04) + CLI status 반영.
3. **`TrayIndicator.notify`(선택)** — no-op이면 무연산(D1).

**규칙 R-CRIT-04 (조건 해제)**: 능동 조건은 원천 회복 시 해제된다 — 케이스3 `OverLimit` 는 한도 이내 복귀 시 다음 사이클이 `clear_condition(OverLimit)`(U8/U3 push, US-E5-02 체크리스트); 케이스2 escalation 은 사이클 성공 시 리셋(R-CRIT-02). 해제 push 소유는 상위(U8/U3)이며 U6는 반영만 한다.

---

## 5. UploadHistoryStore 규칙 — append-only (US-E5-03, FR-16)

**규칙 R-HIST-01 (append-only 불변성)**: `append(record)` 는 **신규 레코드 추가만** 허용한다. 기존 레코드는 **절대 수정/삭제되지 않는다**. append 순서가 보존되며 `query` 는 append 순서(또는 결정적 정렬)로 반환한다.

**규칙 R-HIST-02 (무손실 지속, NFR-13, D8)**: 레코드는 U0 CBOR 코덱(`encode`/`decode`)으로 **길이-프레이밍 프레임**으로 append-only 파일에 지속된다. 모든 `UploadHistoryRecord r` 에 대해 `decode(encode(r)) == r`(무손실 round-trip, US-E5-03 소비).

**규칙 R-HIST-03 (append infallible, D9)**: `HistorySink::append` 는 infallible(U0 계약). IO/코덱 오류는 내부에서 삼키고 로거로 남긴다(R-PUSH-02). append 실패가 사이클을 막지 않는다.

**규칙 R-HIST-04 (truncated-tail 관용, D9 — 크래시 관용)**: `query` 시 프레임을 순차 디코드하다가 **트레일링(파일 끝) 프레임이 부분/디코드 불가**이면, 이를 append 도중 크래시로 생긴 truncated tail로 간주해 **정상 디코드된 prefix를 성공 반환**한다(append-only 불변 유지 — 이전 레코드 보존). 파일 **중간** 프레임 손상만 `HistoryError::Corrupt` 로 표면화한다.

**규칙 R-HIST-05 (query 필터 = AND 결합, 오라클)**: `query(HistoryQuery)` 는 두 필터의 **AND** 를 만족하는 레코드만 반환한다:
- `since`: `record.timestamp >= since`.
- `only_failures`: `Some(true)` 이면 `record.error_detail.is_some()`(실패만), `Some(false)` 이면 `record.error_detail.is_none()`(성공만). 성공/실패는 `error_detail` 유무로 파생한다.
- 각 필터가 `None`이면 해당 축 제약 없음. 필터 전부 `None`이면 전 레코드 반환. 콘텐츠 id 기준 필터는 콘텐츠 링크가 MVP 레코드에 없어 POST-MVP 이월(`domain-entities.md` §2.2).

**규칙 R-HIST-06 (히스토리도 평문 — RISK-01)**: 히스토리 파일은 OS 보안 저장소 밖 **로컬 평문**이며 `error_detail` 에 민감정보가 섞일 수 있음은 수용 위험이다(로컬 산출물 정리는 E6/U7a `Uninstaller` 소관).

---

## 6. TrayIndicator 규칙 (선택 — no-op, D1, FR-18)

**규칙 R-TRAY-01 (MVP no-op)**: MVP는 실제 데스크톱 트레이를 구현하지 않는다. `TrayIndicator` 는 nullable **no-op 싱크**로, `start() -> Ok(None)`, `render`/`notify` 무연산, `stop` 무연산이다.

**규칙 R-TRAY-02 (비의존·비치명)**: 트레이 부재/no-op는 로그·status·health·history 표면화를 **막지 않는다**(US-E5-04/05). 모든 트레이 오류(`TrayError`)는 비치명이다. `tray_enabled` config 는 파싱만 되고(호환) 동작에 영향 없다.

---

## 7. config 검증 규칙 (U6 섹션 — D14)

**규칙 R-CFG-U6-01 (federated key 선언)**: U6는 `OBSERVABILITY_CONFIG_KEYS = {log_file, log_max_size_bytes, log_max_retained, history_file, tray_enabled}` 를 선언해 U8이 federated union으로 집계하게 한다(U0 R-CFG-STRICT-01 정합 — 미선언 시 U6 키가 unknown으로 오탐 거부됨). `log_level`/`notify_consecutive_failures` 는 U0 소유 키라 미포함.

**규칙 R-CFG-U6-02 (per-field 검증)**:
| 필드 | 검증 규칙 | 위반 시 |
|---|---|---|
| `log_file` | 부재 시 플랫폼 기본 경로. 존재 시 비어있지 않은 문자열 | 빈 문자열 실패 |
| `log_max_size_bytes` | 부재 시 기본값. `>= 1` | 0/음수/비정수 실패 |
| `log_max_retained` | 부재 시 기본값. `>= 0` | 음수/비정수 실패 |
| `history_file` | 부재 시 플랫폼 기본 경로. 존재 시 비어있지 않은 문자열 | 빈 문자열 실패 |
| `tray_enabled` | 부재 시 `false`. 불리언 | 불리언 아니면 실패 |

- 검증 실패 시 동작(최초 로드 abort / 리로드 keep-last-good)은 U0 `ConfigProvider`(R-RELOAD-03/04) 소관이며 U6는 스키마만 제공한다.

---

## 8. Testable Properties (PBT-01) — 규칙(RULES) 계층

> **확장 강제(PBT-01, Full)**: 검증·결정 규칙 계층 속성을 식별한다. 히스토리/로그 round-trip은 `domain-entities.md` §8, 흐름 속성은 `business-logic-model.md` §6이 소유(중복 회피). 카테고리 라벨 + 제너레이터(PBT-07) 요구 기재.

### 8.1 로그 레벨 필터

- **PROP-U6-BR-01 — 레벨 필터 정확성** (카테고리: **Oracle** + **Easy verification**)
  - **속성**: 임의 `(활성 log_level L, 레코드 레벨 r)` 에 대해 방출됨 iff `r >= L`(R-LOG-02). 유한 도메인(5x5 레벨 조합) 전수 검증 가능.
  - **제너레이터(PBT-07)**: `LogLevel` 전 변이 x 전 변이 열거 + 임의 `LogRecord`.

### 8.2 2축 상태 독립성 + snapshot 파생

- **PROP-U6-BR-02 — 2축 독립성** (카테고리: **Invariant** + **Induction**; 상태 기반)
  - **속성**: 임의 명령 시퀀스(`set_operational`/`raise_condition`/`clear_condition` 인터리빙)에 대해 `snapshot().operational == 마지막 set_operational 값` **그리고** `snapshot().conditions == (raise 집합 - clear 집합)`. 두 축이 서로를 변경하지 않는다(R-STATUS-01/02).
  - **제너레이터(PBT-07)**: `OperationalState`/`ActiveCondition` 명령 시퀀스 제너레이터(중복 raise/clear 포함).

- **PROP-U6-BR-03 — consent/offline 파생 결정성** (카테고리: **Oracle**)
  - **속성**: `snapshot().offline == (operational == Offline)`; `snapshot().consent` 는 R-STATUS-03 파생 규칙(참조 오라클)과 일치.
  - **제너레이터(PBT-07)**: 명령 시퀀스 + `ConsentBlocked` raise/clear + `record_sync_success` 유무.

### 8.3 health / update_probe 판정 오라클

- **PROP-U6-BR-04 — health_check 오라클** (카테고리: **Oracle**)
  - **속성**: `health_check() == Unhealthy` iff `conditions ∩ {AuthFailed,OverLimit,UpdateRolledBack} ≠ ∅` 또는 escalation 활성(R-STATUS-04). `ConsentBlocked`/`Offline`/`dirty`/`VaultUnavailable` 단독으로는 `Healthy`(비목록 조건은 health flip 없음).
  - **제너레이터(PBT-07)**: 임의 조건 부분집합 + escalation 플래그 on/off.

- **PROP-U6-BR-05 — update_probe 격리(롤백 루프 방지)** (카테고리: **Invariant**; 핵심)
  - **속성**: 임의의 활성 조건 집합/운영상태에 대해 `update_probe()` 는 **오직** `{IdleReached, CredentialReadable}` 관측 여부에만 의존한다(R-STATUS-05). 즉 두 신호를 고정한 채 `AuthFailed`/`OverLimit` 등을 임의로 raise/clear 해도 `update_probe()` 결과는 불변.
  - **제너레이터(PBT-07)**: (liveness 신호 부분집합) x (임의 조건 부분집합 + 운영상태) 곱집합.

### 8.4 닫힌 4조건 + 연속 실패 상태머신

- **PROP-U6-BR-06 — 닫힌 집합 불변** (카테고리: **Invariant**)
  - **속성**: 임의 로그 이벤트/비목록 오류 스트림은 어떤 `raise_condition`/`tray.notify` 도 유발하지 않는다. 능동 표면화는 4개 `report_*` 진입점으로만 발생(R-CRIT-01).
  - **제너레이터(PBT-07)**: 임의 `LogRecord` 스트림 + 4개 report 호출 시퀀스 혼합.

- **PROP-U6-BR-07 — 연속 실패 임계 상태머신** (카테고리: **Induction** / 상태 기반 + **Oracle**)
  - **속성**: 임의 `report_cycle_result` 시퀀스(Success/Failure 혼합)에 대해, escalation 은 연속 Failure 카운트가 `N` 에 **도달하는 순간 정확히 1회** 발화하고, Success 마다 카운트·escalation 리셋(R-CRIT-02). 참조 모델 = 단순 카운터.
  - **제너레이터(PBT-07)**: Success/Failure 시퀀스 + 임의 `N`(>=1, 경계 1·기본 3·대값).

### 8.5 히스토리 append-only + query 오라클

- **PROP-U6-BR-08 — append-only 불변 + query 오라클** (카테고리: **Invariant** + **Oracle**; Induction)
  - **속성(불변)**: 임의 append 시퀀스 후 `query(all)` 은 append된 전 레코드를 **append 순서로, 누락·수정·삭제 없이** 반환(R-HIST-01). 후속 append가 이전 레코드를 바꾸지 않는다.
  - **속성(오라클)**: `query(filter)` 는 in-memory 참조 목록에 동일 필터 술어를 적용한 결과와 정확히 일치(R-HIST-05, AND 결합).
  - **제너레이터(PBT-07)**: `UploadHistoryRecord` 시퀀스 + 임의 `HistoryQuery`(since/only_failures 조합, `None` 포함).

- **PROP-U6-BR-09 — truncated-tail 관용** (카테고리: **Invariant**; 크래시 관용)
  - **속성**: append 시퀀스 지속 후 마지막 프레임을 임의 길이로 절단하면, `query` 는 **절단 이전 정상 prefix 전부**를 성공 반환한다(중간 손상만 `Corrupt`)(R-HIST-04).
  - **제너레이터(PBT-07)**: 레코드 시퀀스 + 트레일링 바이트 절단 오프셋(프레임 경계 내/경계 걸침).

### 8.6 속성 없음(No PBT properties identified) 판정

| 규칙/요소 | 판정 | 근거 |
|---|---|---|
| R-PUSH-01 (U1~U5 역참조 금지) | **No PBT properties identified** | 구조 불변 — 컴파일타임(크레이트 그래프) 강제, 속성 테스트 대상 아님 |
| R-PUSH-02/03 (infallible push / 읽기만 Result) | **No PBT properties identified** | 계약(반환 형상) — 타입 시스템 강제 |
| R-LOG-03/05 (동기 쓰기·rotation) | 예제 기반 단위테스트 | IO 부수효과 — 결정적 소수 케이스(rotation 임계 전후) 예제테스트가 적합 |
| R-LOG-04 (토큰 redaction) | 예제 기반(+U0 `TokenSecret` 속성 재사용) | 잔존 통제 — redaction 은 U0 타입이 보장, U6는 미사용 확인 |
| R-TRAY-01/02 (no-op) | **No PBT properties identified** | 동작 부재(D1) |
| R-CFG-U6-02 (config 검증) | U0 config round-trip(PROP-BR-02/05)에 흡수 | strict-reject/round-trip 은 U0 소유 |

> **제너레이터(PBT-07) 총괄**: 위 속성은 `LogRecord`·`UploadHistoryRecord`·상태 명령 시퀀스·`report_*` 시퀀스 제너레이터를 요구한다. 구현·shrinking·고정 시드·CI(PBT-08)는 Code Generation/Build-and-Test 이월, 프레임워크(PBT-09, proptest 유력)는 NFR Requirements 이월이다.

---

## 9. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 문서 적용 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **준수** | §8 Testable Properties 제공 — 레벨 필터(Oracle), 2축 독립·파생(Invariant/Oracle), health 오라클·**update_probe 격리(핵심)**, 닫힌 4조건·연속실패 상태머신, 히스토리 append-only·query 오라클·truncated-tail. 제너레이터(PBT-07) 요구 기재. 속성 없는 규칙은 §8.6에 판정. |
| **Resiliency Baseline** | ON | **부분 적용** | RESILIENCY-05(구조화 로깅, R-LOG-*)·06(헬스/status, R-STATUS-*)·15(사고 표면화, R-CRIT-*) 직접 구현. best-effort infallible push(R-PUSH-02)로 관측 실패가 코어를 막지 않음. append-only + truncated-tail 관용(R-HIST-01/04) = 크래시 관용. `update_probe`(R-STATUS-05)가 RESILIENCY-04 자동 롤백 게이트 근거. RTO/RPO 수치·배포·HA/DR은 인프라/상위 단계 -> N/A. |
| **Security Baseline** | OFF | **N/A** | 미로딩·미강제. 로그·히스토리 로컬 평문(RISK-01, R-LOG-04/R-HIST-06)은 수용 위험. 잔존 통제 = 토큰 redaction(SEC-02, R-LOG-04). |

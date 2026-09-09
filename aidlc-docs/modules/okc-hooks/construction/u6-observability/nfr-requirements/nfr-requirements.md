# U6 Observability — NFR Requirements (비기능 요구사항 / 품질 속성)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U6 Observability** -> NFR Requirements -> 산출물 1/2 (`nfr-requirements.md`)
**작성일**: 2026-09-08
**크레이트**: `observability` (lib) · **소속 컴포넌트**: `StructuredLogger`, `StatusService`, `UploadHistoryStore`, `CriticalErrorNotifier`, `TrayIndicator`(선택 no-op)
**입력 아티팩트**: `functional-design/domain-entities.md`·`business-rules.md`·`business-logic-model.md`(U6 FD 산출물) · `inception/requirements/requirements.md`(§3.5 FR-16~19 / §5.2·§5.6 NFR / §6 Resiliency 매핑 / §7 RISK-01 / §12·§13 오버레이) · `u0-foundation/nfr-requirements/{nfr-requirements.md,tech-stack-decisions.md}`(스타일 템플릿 + 상속 스택) · 활성 확장 `property-based-testing.md`(PBT-09)·`resiliency-baseline.md`
**전제(AUTOPILOT 확정, 재오픈 금지)**: FD D1..D14 · Q9=B(2축 상태) · FQ-1/2/3=A(오버레이) · U0 FROZEN(값 타입·CBOR 코덱·싱크 계약 트레이트·오류 taxonomy·`ConfigProvider`/`WatcherConfig` 6 코어 필드) · federated 비코어 config(`log_file`/`log_max_size_bytes`/`log_max_retained`/`history_file`/`tray_enabled`)는 U8 조립 루트가 파싱·해소해 **생성자 주입**(downward injection, `ConfigProvider` 미독) · 확장 Resiliency ON / PBT ON(Full) / Security OFF

> **문서 성격**: 이 문서는 U6가 소유하는 타입(`domain-entities.md`)·규칙(`business-rules.md`)·흐름(`business-logic-model.md`) **위에 얹는 NFR(품질 속성) 계층**이다. FD의 비즈니스 규칙을 재기술하지 않고 **규칙 ID로 참조**하며, 각 NFR에 근거(아티팩트 + 섹션 + NFR/규칙/PBT ID)와 수용 기준을 붙인다. 구체 크레이트·툴체인 선택의 정본은 자매 산출물 `tech-stack-decisions.md`이며, 이 문서는 각 품질 속성을 **실현하는 메커니즘**으로 그 결정을 참조한다.
>
> **표기 규약**: 기술중립을 지향하되, tech-stack-decisions.md가 명시적으로 위임한 곳에서만 구체 크레이트를 인용한다. 다이어그램은 ASCII 박스 없이 **화살표 표기(A -> B)** 와 표/목록으로 기술한다. English 식별자명(크레이트·타입·config 키·규칙/NFR ID)은 원문 그대로 유지하고, Rust 제네릭/타입(예: `Arc<dyn Logger>`, `Mutex<StatusInner>`)은 백틱으로 감싼다.

---

## 1. NFR 개요 및 U6 특성

U6 `observability`는 **순수 lib**(U0 싱크 계약 트레이트 4종의 실체 구현 + 2축 상태 집계 + append-only 히스토리 + 닫힌 4조건 표면화)이며, U1~U5를 **역참조하지 않는다**(R-PUSH-01). U6의 4개 실체 싱크는 U0 트레이트를 구현하고 **U8 조립 루트가 생성·하향 주입**하며, 하위 단위는 값을 **push** 할 뿐이다. 따라서 U6의 핵심 품질 속성은 **관측 계약의 신뢰성** — 관측 실패가 동기화 코어를 절대 막지 않는 best-effort·infallible 비차단(R-PUSH-02), append-only 무손실 지속과 크래시 관용, 다중 스레드 push/read 하 상태 집계의 원자적 관측, 그리고 트레이 비의존 3중 사고 표면화다. 자체 처리량 축·SLA·배포 표면이 없어 확장성/가용성/성능 수치 목표 등은 **N/A**다(§7).

**본 문서가 확정하는 U6 NFR 카탈로그(카테고리별 ID)**:

| 카테고리 | NFR ID | 요약 |
|---|---|---|
| 신뢰성 | U6-NFR-REL-01 | 관측 push best-effort·infallible 비차단(전 싱크) |
| 신뢰성 | U6-NFR-REL-02 | 히스토리 append-only 무손실 round-trip + truncated-tail 크래시 관용 |
| 신뢰성 | U6-NFR-REL-03 | `StatusService` 단일 집계 스레드 안전(동시 push/read 원자 관측) |
| 신뢰성 | U6-NFR-REL-04 | 중대오류 3중 fan-out 표면화(트레이 비의존) |
| 신뢰성 | U6-NFR-REL-05 | 로그레벨/rotation 리로드 재적용 keep-last-good 비치명 |
| 성능/리소스 | U6-NFR-PERF-01 | 로그 쓰기 동기 직렬화 best-effort·sync-path 유계(수치 목표 N/A) |
| 성능/리소스 | U6-NFR-PERF-02 | 히스토리 query 전체 스캔 선형·유계(인덱스 없음, 수치 목표 N/A) |
| 유지보수성 | U6-NFR-MNT-01 | 오류 타입 파생 전략(thiserror 운영 오류 + 순수 serde 값 타입) |
| 유지보수성 | U6-NFR-MNT-02 | PBT 제너레이터(U0 `proptest-support` 재사용 + U6 로컬 제너레이터 노출) |
| 유지보수성 | U6-NFR-MNT-03 | 커버리지/문서 정책(no %-게이트 + PBT + 예제 병행) |
| 보안-잔존 | U6-NFR-SEC-01 | 로그/히스토리 토큰 redaction 잔존 통제(원문 미방출) |
| 사용성 | U6-NFR-USE-01 | JSON-line 최소 필드 계약(기계 판독 관측 표면) |

---

## 2. 신뢰성 (Reliability)

U6의 신뢰성은 "**관측이 코어를 오염·차단하지 않는다**"이다 — 어떤 관측 실패(로그 쓰기 오류, 히스토리 append IO 오류)도 동기화 사이클을 막지 않고 삼켜지며(R-PUSH-02), 히스토리는 크래시 후에도 정상 prefix를 잃지 않고, 상태 집계는 다중 스레드에서 완결 스냅샷만 노출하며, 사고 표면화는 트레이 유무와 무관하게 항상 성립한다.

### 2.1 U6-NFR-REL-01 — 관측 push best-effort·infallible 비차단 (전 싱크)

- **요구**: `Logger::log`/`event`, `StatusSink` 전 뮤테이터, `HistorySink::append`, `CriticalEventSink` 전 `report_*` 는 **실패를 호출자에게 전파하지 않으며**(U0 계약), 실패 시 내부에서 삼키고 가능하면 로거로 남긴다. 이 push 표면은 **동기화 코어(U1~U5/U8)를 절대 막지 않는다** — 재시도 루프·무한 블로킹·패닉으로 사이클을 정지시키지 않는다. 실패를 표면화하는 U6 표면은 **읽기/재적용 경로**뿐이다: `UploadHistoryStore::query() -> Result<_, HistoryError>`, `StructuredLogger::reload() -> Result<_, LogError>`(R-PUSH-03).
- **근거**: `business-rules.md` §1 R-PUSH-02/03 · `business-logic-model.md` §6.2 PROP-U6-BL-02(관측 실패 비차단 Invariant) · requirements.md §5.6 NFR-15 · §6 RESILIENCY-05/15. 실현 = U0 트레이트의 infallible 반환 형상 + 내부 오류 삼킴(정본 tech-stack-decisions.md §5 오류 파생).
- **선정 근거(왜 명시적 NFR인가)**: U6 싱크는 동기화 핫 경로에서 호출된다(사이클 시작/종료마다 로그·히스토리 append·상태 갱신). 여기서 관측 오류가 전파되거나 블로킹되면 관측이 곧 코어의 단일 장애점이 되어 "관측은 부수적·비치명"이라는 계약을 정면으로 위반한다. 따라서 라이브러리 기본에 맡기지 않고 U6의 명시적 신뢰성 불변식으로 승격한다.
- **수용 기준**:
  - (AC-1) IO 오류(로그 파일 쓰기 실패, 히스토리 append 실패)를 주입해도 push 표면은 오류를 전파하지 않고 반환한다(PROP-U6-BL-02, 카테고리 Invariant; 오류-주입 어댑터 + push 시퀀스).
  - (AC-2) 읽기/재적용 경로(`query`/`reload`)만 `Result<_, HistoryError>`/`Result<_, LogError>` 로 실패를 표면화하며, 나머지 표면은 반환 형상이 infallible이다(타입 시스템 강제).

### 2.2 U6-NFR-REL-02 — 히스토리 append-only 무손실 round-trip + truncated-tail 크래시 관용

- **요구**: 모든 `UploadHistoryRecord r` 에 대해 U0 CBOR 코덱 round-trip이 무손실이다(`decode(encode(r)) == r`, NFR-13). 히스토리 파일은 **append-only** 로, 기존 레코드가 절대 수정/삭제되지 않으며 `query` 는 append 순서(또는 결정적 정렬)로 반환한다. **크래시 관용**: `query` 시 트레일링(파일 끝) 프레임이 부분/디코드 불가이면 append 도중 크래시로 생긴 truncated tail로 간주해 **정상 디코드된 prefix를 성공 반환**하고, 파일 **중간** 프레임 손상만 `HistoryError::Corrupt` 로 표면화한다.
- **근거**: `business-rules.md` §5 R-HIST-01(append-only)·R-HIST-02(무손실 프레임)·R-HIST-04(truncated-tail 관용) · `domain-entities.md` §2.1(스키마 무손실 round-trip 대상) · `business-logic-model.md` §3 · requirements.md §5.5 NFR-13 · §3.5 FR-16 · §6 RESILIENCY-02(로컬 상태 재구성). 실현 = U0 `ciborium` 코덱(U0-NFR-REL-01 재확인) + 길이 프레이밍 + `std::fs` append(정본 tech-stack-decisions.md §2·§3).
- **선정 근거**: 히스토리는 감사 목적의 append-only 로그다. 자동 업데이트 후 스키마 진화(새 필드)에도 강한 U0 CBOR 코덱을 재사용해 무손실을 보장하고, append 도중 크래시가 이전 레코드를 오염시키지 않도록 truncated-tail을 관용해 재기동 query 성공을 보장한다(RESILIENCY 로컬 재구성 정합). 이는 U0-NFR-REL-01(코덱 무손실) 위에 U6 지속 계층이 얹는 append-only + 크래시 관용 불변식이다.
- **수용 기준**:
  - (AC-1) `UploadHistoryRecord` 도메인 제너레이터(3필드: `error_detail` `None`(성공)/유니코드·개행/빈 문자열(실패), 0/경계/대값 `bytes_transferred`, 임의 `timestamp`) 기반 round-trip PBT가 반례 없이 통과한다(PROP-U6-DE-01, 카테고리 Round-trip, PBT-02).
  - (AC-2) 임의 append 시퀀스 후 `query(all)` 은 전 레코드를 누락·수정·삭제 없이 append 순서로 반환하고(PROP-U6-BR-08, Invariant), 후속 append가 이전 레코드를 바꾸지 않는다.
  - (AC-3) 마지막 프레임을 임의 오프셋으로 절단해도 `query` 는 절단 이전 정상 prefix 전부를 성공 반환하며, 중간 손상만 `Corrupt` 로 표면화한다(PROP-U6-BR-09, Invariant; 크래시 관용).
- **연계**: U6 히스토리는 **비권위 감사 로그**로 그 append 실패는 삼켜진다(REL-01) — zero-loss 지속(RPO=0, NFR-03)은 U4 `SyncStateStore` 소관이지 U6가 아니다(§7 Resiliency 참조).

### 2.3 U6-NFR-REL-03 — `StatusService` 단일 집계 스레드 안전 (동시 push/read 원자 관측)

- **요구**: `StatusService` 는 모든 상태 신호(`set_operational`/`raise_condition`/`clear_condition`/`record_sync_success`/`set_dirty`/`set_resume_progress`/`set_liveness` + 연속실패 escalation 입력)가 모이는 **단일 집계 지점**이다. 다중 스레드의 동시 push(동기화 코어·U4·U5·U8)와 동시 read(`snapshot`/`health_check`/`update_probe` from CLI/`ControlPlane`)가 공존하므로, 각 연산은 **원자적으로 관측**되어야 한다 — 읽기 스레드는 항상 **완결된 스냅샷 하나만** 관측하고, 부분 갱신(축1과 축2가 반쯤 반영된 중간 상태)은 노출되지 않는다. 2축(operational 단일값 + conditions 집합)은 직교하며 서로를 자동 변경하지 않는다(R-STATUS-01).
- **근거**: `business-rules.md` §3 R-STATUS-01(2축 독립)·R-STATUS-02(집합 멱등) · `business-logic-model.md` §2.1(단일 집계 지점, 동시 push/read 공존; 스레딩 메커니즘 NFR 이월) · `domain-entities.md` §3(단일 집계 지점) · requirements.md §3.5 FR-18 · §6 RESILIENCY-06. 실현 프리미티브 = `std::sync::Mutex<StatusInner>`(정본 tech-stack-decisions.md §4) — `Send`+`Sync` 계약(U0 트레이트)을 만족하며 `snapshot()` 는 락 하에서 완결 값을 복제해 노출한다.
- **선정 근거**: FD가 스레딩을 NFR 단계로 이월했고(§2.1), `StatusSink`/`ReadJudgment` 트레이트는 `Send`+`Sync` 를 요구한다. 단일 `Mutex` 는 2축 상태의 원자적 관측을 최소 코드로 실현한다 — 읽기가 핫 경로가 아닌(CLI status는 저빈도) 데스크톱 데몬 규모에서 `RwLock`/락-프리 구조의 복잡도는 불필요하다(MVP 트림, 대안 비교 tech-stack-decisions.md §4).
- **수용 기준**:
  - (AC-1) 임의 명령 시퀀스(2축 인터리빙) 후 `snapshot().operational == 마지막 set_operational 값` 이고 `snapshot().conditions == (raise 집합 - clear 집합)` 이며 두 축이 서로를 변경하지 않는다(PROP-U6-BR-02, Invariant).
  - (AC-2) 동시 push/read 하에서 어떤 읽기도 부분 갱신 상태를 관측하지 않는다(완결 스냅샷만; `Mutex` 원자 경계로 실현).
  - (AC-3) `update_probe()` 는 `{IdleReached, CredentialReadable}` 관측 여부에만 의존하고 어떤 `ActiveCondition`/운영상태에도 영향받지 않는다(PROP-U6-BR-05, Invariant) — 이 격리가 일시 조건에 의한 오판 롤백을 막아 RESILIENCY-04 자동 롤백 게이트의 안정성 근거가 된다.

### 2.4 U6-NFR-REL-04 — 중대오류 3중 fan-out 표면화 (트레이 비의존)

- **요구**: 능동 표면화는 **정확히 4개 진입점**(`report_auth_failure`/`report_cycle_result`/`report_preflight_exceeded`/`report_update_rollback`)으로만 발생하며(닫힌 집합), 각 표면화는 **3중 fan-out**한다 — (1) 상향 심각도 구조화 로그, (2) `StatusService` 반영(케이스별 `raise_condition` 또는 escalation 플래그) -> `health_check()` unhealthy 전환 + CLI status 반영, (3) `TrayIndicator.notify`(선택). **핵심**: 앞 2개 표면(로그 + status/health)은 **트레이 유무·no-op 여부와 무관하게 항상 성립**한다. 연속실패(케이스2)는 임계 `N`(주입된 `notify_consecutive_failures`, 기본 3) **도달 순간 정확히 1회** 발화하고 성공 시 리셋한다(중복 억제).
- **근거**: `business-rules.md` §4 R-CRIT-01(닫힌 집합)·R-CRIT-02(연속실패 임계)·R-CRIT-03(3중 fan-out) · `business-logic-model.md` §4.1/§4.2 · `domain-entities.md` §3.4(escalation 플래그) · requirements.md §3.5 FR-19 · §5.6 NFR-15 · §6 RESILIENCY-15. `TrayIndicator` no-op 결정(D1)의 정본 tech-stack-decisions.md §6(트레이 크레이트 미채택).
- **선정 근거**: FR-19가 요구하는 사고 표면화의 신뢰성은 트레이 백엔드(3-OS, MVP DROP)에 의존해서는 안 된다. 데몬 모델(GUI-less)에서 활성 표면은 로그 + health/status이며 트레이는 부가 표면일 뿐이므로, no-op 트레이여도 사고가 항상 로그·health·CLI status로 표면화됨을 신뢰성 불변식으로 명문화한다.
- **수용 기준**:
  - (AC-1) 임의 로그 이벤트/비목록 오류 스트림은 어떤 `raise_condition`/`tray.notify` 도 유발하지 않고, 능동 표면화는 4개 `report_*` 로만 발생한다(PROP-U6-BR-06, Invariant, 닫힌 집합).
  - (AC-2) 임의 `report_cycle_result` 시퀀스에 대해 escalation은 연속 Failure 카운트가 `N` 도달 순간 정확히 1회 발화하고 Success마다 리셋한다(PROP-U6-BR-07, Induction/Oracle; 참조 모델 = 단순 카운터).
  - (AC-3) 임의 `report_*` 호출의 3중 fan-out이 케이스별 참조 매핑과 일치하며(PROP-U6-BL-01, Oracle), 트레이 no-op 여부와 무관하게 로그 + status 반영은 항상 발생한다.

### 2.5 U6-NFR-REL-05 — 로그레벨/rotation 리로드 재적용 keep-last-good 비치명

- **요구**: `StructuredLogger` 는 `ConfigReloadObserver` 를 구현해 U0 `ConfigProvider` 성공 스왑 시 `on_config_reload()` 통지(push, 페이로드 없음)를 받아 `log_level`(및 rotation 파라미터)을 **재적용**한다. 재적용 실패는 `reload() -> Err(LogError)` 로 표면화하되 **데몬을 막지 않는다**(U0 keep-last-good 정합) — 실패 시 직전 정상 설정을 유지하고 계속 실행한다.
- **근거**: `business-rules.md` §2 R-LOG-06(리로드 재적용) · `business-logic-model.md` §1.2(리로드 재적용 흐름) · `domain-entities.md` §0(`ConfigReloadObserver` 구현) · U0 `nfr-requirements.md` §2.5 U0-NFR-REL-05(keep-last-good)·§2.4 U0-NFR-REL-04(관찰자 팬아웃 격리). `log_level` 은 U0 코어 필드로 `ConfigProvider.current()` 재조회로 얻고, `log_file`/rotation 파라미터는 U8이 주입한 값이므로 **MVP에서 rotation 파라미터 live-reload는 out-of-scope**(재시작으로 변경; federated 비코어 값은 live-reload 안 됨). 재적용은 `log_level` 코어 필드 fan-out에 한정한다.
- **선정 근거**: U0 관찰자 팬아웃은 per-observer `catch_unwind` 로 격리되므로(U0-NFR-REL-04), `StructuredLogger` 의 재적용 실패가 팬아웃을 오염시키지 않는다. 재적용을 keep-last-good·비치명으로 두어 잘못된 리로드가 로깅을 정지시키지 않게 한다.
- **수용 기준**:
  - (AC-1) `on_config_reload()` 통지 후 `StructuredLogger` 는 `ConfigProvider.current()` 를 재조회해 `log_level` 을 재적용한다.
  - (AC-2) 재적용 실패는 `reload() -> Err(LogError)` 로만 표면화되고 데몬은 계속 실행하며, 이전 성공 설정을 유지한다(keep-last-good).
  - (AC-3) federated rotation 파라미터(`log_max_size_bytes`/`log_max_retained`)의 live-reload는 MVP에서 미구현이며(재시작 반영), 이 이월을 명시한다.

---

## 3. 성능 / 리소스 (Performance)

### 3.1 U6-NFR-PERF-01 — 로그 쓰기 동기 직렬화 best-effort·sync-path 유계 (수치 목표 N/A)

- **요구(정성 계약만)**: 로그 방출은 **동기적으로 직렬화 접근 하**(단일 `Mutex` 가드 하 순차 쓰기) 수행되며, 각 방출은 **JSON-line 1건 append + (크기 초과 시) rotation** 이라는 **유계 작업**이다 — 무한 큐 성장·무한 재시도·비선형 폭증이 없다. IO/rotation 오류는 삼켜져(REL-01) sync-path를 블로킹하지 않는다. **비동기 로깅 스레드·bounded 채널·drop-on-full 백프레셔 큐는 MVP 미구현**(D3 트림) — 백프레셔의 "비차단" 속성은 별도 큐가 아니라 **best-effort·infallible 삼킴**(REL-01)으로 실현하고, 직렬화 락은 단일 소형 라인 쓰기 시간만 보유한다.
- **명시적 결정 — 수치 목표 없음(근거)**: U6 로그 쓰기에 **throughput·latency·peak-memory 수치 게이트를 두지 않는다**. 근거: (a) 쓰기 비용은 라인 길이·rotation 임계에 선형·유계라 절대 수치 목표를 정의할 도메인 근거가 없다. (b) U6는 자체 처리량 축이 없는 순수 lib이며 단일 사용자 데스크톱 데몬 규모다. (c) 로그는 감사·관측용으로 사이클 지연 예산에 지배적이지 않다(사이클 병목은 U3 전송이지 로그가 아님).
- **근거**: `business-rules.md` §2 R-LOG-03(동기 best-effort 쓰기, 비동기 로깅 MVP 미구현)·R-LOG-05(크기 rotation, 개수 보존) · `business-logic-model.md` §1.1(방출 파이프라인) · requirements.md §5.6 NFR-15 · §6 RESILIENCY-05. 동시성 프리미티브(`std::sync::Mutex`) + rotation 메커니즘(`std::fs`) 정본 = tech-stack-decisions.md §3·§4.
- **수용 기준**:
  - (AC-1) 다중 스레드 동시 방출 하에서 로그 라인이 인터리빙·손상 없이 각 1건으로 직렬 기록된다(단일 `Mutex` 직렬화 경계).
  - (AC-2) rotation은 `log_max_size_bytes` 초과 시 회전하고 `log_max_retained` 개수로 보존을 상한한다(나이/기간 기반 미구현 — 예제 기반 단위테스트로 임계 전후 검증, `business-rules.md` §8.6).
  - (AC-3) 어떤 절대 throughput/latency/peak-memory 임계도 U6 게이트로 설정되지 않으며, 비동기 로깅/bounded 채널은 MVP 이월임을 명시한다.

### 3.2 U6-NFR-PERF-02 — 히스토리 query 전체 스캔 선형·유계 (인덱스 없음; 수치 목표 N/A)

- **요구(정성 계약만)**: `query(HistoryQuery)` 는 append-only 파일을 **프레임 순차 디코드 후 선형 필터**(since/only_failures AND 결합)를 적용한다 — 임베디드 DB·보조 인덱스 없음(D8 트림). 비용은 히스토리 레코드 수에 선형·유계이며, 단일 사용자 데스크톱 데몬 규모(사이클당 최대 1레코드 append)에서 충분하다. 수치 목표 게이트 없음.
- **근거**: `business-rules.md` §5 R-HIST-05(AND 결합 필터) · `business-logic-model.md` §3.2(전체 스캔 + 선형 필터, 인덱싱 없음 D8 트림) · `domain-entities.md` §2.3(`HistoryQuery`) · requirements.md §3.5 FR-16 · §5.5 NFR-13. 정본 tech-stack-decisions.md §3(인덱스/DB 미채택).
- **수용 기준**:
  - (AC-1) `query(filter)` 는 in-memory 참조 목록에 동일 필터 술어(AND)를 적용한 결과와 정확히 일치한다(PROP-U6-BR-08 오라클, Oracle).
  - (AC-2) 어떤 절대 query latency/throughput 임계도 설정되지 않으며, 인덱싱/임베디드 DB는 MVP 이월(비도입)임을 명시한다.

---

## 4. 유지보수성 (Maintainability)

### 4.1 U6-NFR-MNT-01 — 오류 타입 파생 전략 (U0 정합)

- **요구**: U6 오류 타입을 U0 워크스페이스 관례(Q4=A)에 맞춰 두 부류로 파생한다:
  - **운영 오류(`Result` 반환, 읽기/재적용 경로)**: `LogError`(`Io`/`InvalidPath`/`RotationFailed`) · `HistoryError`(`Io`/`Serde`/`Corrupt`)에 **`thiserror`** 파생을 사용해 `Display`/`std::error::Error` 를 얻는다. 단, 이들은 push 표면에는 전파되지 않고(REL-01) `query`/`reload` 경로에서만 반환된다.
  - **지속 값 타입(CBOR round-trip 대상, throw 아님)**: `UploadHistoryRecord`(U0 FROZEN 3필드)는 **순수 serde struct로 유지**한다 — U0 `ciborium` 코덱으로 round-trip되는 값이다(U0-NFR-REL-01 대상에 이미 열거).
- **선정 근거**: U0가 정의한 오류 계약(운영 오류=`thiserror`, 분류/지속 값=순수 serde)을 U6가 재사용해 워크스페이스 전역 일관을 유지한다. `anyhow`식 타입소거는 U0가 이미 거부(구조화 변이 소실). U6 값 타입에 `thiserror` 를 붙이면 CBOR round-trip 대상이 아닌 오류 표면으로 오분류되므로 순수 serde를 유지한다.
- **근거**: `domain-entities.md` §1.3(`LogError` 내부 오류)·§2.3(`HistoryError`)·§2.1(`UploadHistoryRecord` round-trip 대상) · U0 `tech-stack-decisions.md` §5(오류 파생 전략, U1~U8 전역 관례) · requirements.md §5.7 NFR-17. `thiserror` 의존은 워크스페이스 상속(신규 아님).
- **수용 기준**:
  - (AC-1) `LogError`/`HistoryError` 는 `thiserror` 파생으로 `Display`/`Error` 를 제공하고 `query`/`reload` 경로에서만 반환된다.
  - (AC-2) `UploadHistoryRecord`(U0 FROZEN 3필드)는 `serde::Serialize`/`Deserialize` 만 파생하고 U0 CBOR round-trip이 무손실이다(PROP-U6-DE-01 대상).

### 4.2 U6-NFR-MNT-02 — PBT 도메인 제너레이터 (U0 재사용 + U6 로컬 노출, PBT-07)

- **요구**: U6 PBT는 (a) U0가 `proptest-support` 비기본 feature로 노출한 도메인 타입 제너레이터(`Timestamp`·`ManifestDigest`·`LogLevel`·`ActiveCondition`/`OperationalState`/`LivenessSignal`·자유형 `detail` 등)를 **재사용**하고, (b) U6가 확정한 타입(`UploadHistoryRecord`·`LogRecord` 방출 라인·상태 명령 시퀀스·`report_*` 시퀀스·IO 오류 주입 어댑터)의 제너레이터를 U6 자신의 **비기본 cargo feature `proptest-support`** 로 노출한다(U0 패턴 mirror). 하위/상위 소비 단위(U7b query 소비, Code Generation)는 dev-dependency에서 이 feature를 켠다.
- **선정 근거**: U0 단일 출처 제너레이터 재사용으로 단위 간 정의 드리프트를 막고, U6 고유 타입 제너레이터는 U6가 소유해 노출한다. 비기본 feature 게이트로 `proptest` 가 프로덕션 빌드에 유출되지 않는다(U0 §7.2 정합). 별도 testkit 크레이트(버전 관리 부담)나 각 소비 단위의 제너레이터 재작성(일관성 저하)은 배제한다.
- **근거**: `domain-entities.md` §8(제너레이터 총괄)·`business-rules.md` §8(제너레이터 요구)·`business-logic-model.md` §6(흐름 제너레이터) · property-based-testing.md PBT-07(제너레이터 재사용성)·PBT-09(프레임워크) · U0 `nfr-requirements.md` §4.2 / `tech-stack-decisions.md` §7. feature 이름·게이팅 정본 = tech-stack-decisions.md §5.
- **수용 기준**:
  - (AC-1) `observability` 의 제너레이터 모듈은 non-default feature(`proptest-support`) 뒤에 위치하고 기본 빌드에 `proptest` 의존이 나타나지 않는다.
  - (AC-2) 제너레이터는 문서화된 도메인 제약을 존중한다(`UploadHistoryRecord` 3필드: `error_detail` `None`(성공)/유니코드·개행/빈 문자열(실패)·0/경계/대값 `bytes_transferred`·임의 `timestamp`; `LogRecord` 전 `LogLevel`·`cycle_id` 유무·유니코드/개행/빈 `message`·다수/빈 `fields`; 상태 명령·`report_*` 시퀀스).
  - (AC-3) U0 제너레이터를 재사용하는 대상 타입(U0 소유 프리미티브/enum)은 재작성하지 않고 U0 feature를 활성화해 소비한다.

### 4.3 U6-NFR-MNT-03 — 커버리지 / rustdoc 문서 정책 (U0 정합, PBT-10)

- **요구**: (a) **전역 커버리지 %-게이트를 두지 않는다** — 실질 검증은 PBT(§2 속성들) + 예제 앵커(PBT-10: rotation 임계·config 검증 등 business-critical 경로에 예제 기반 테스트 병행)로 확보한다. (b) `observability` 공개 API 문서화 정책은 U0 `#![deny(missing_docs)]` 계약을 소비·정합하되, U6 자체 공개 표면에도 rustdoc 주석을 유지한다.
- **선정 근거**: U6는 순수 값 집계·지속 lib이라 line/branch % 게이트가 과할 수 있고, IO 부수효과(rotation·append) 흐름은 결정적 소수 케이스라 예제테스트가 적합하다(`business-rules.md` §8.6). PBT + 예제 병행이 실질을 검증한다.
- **근거**: property-based-testing.md PBT-10(보완 테스트 전략) · U0 `nfr-requirements.md` §4.3 / `tech-stack-decisions.md` §9 · requirements.md §5.7 NFR-17. 상세 CI 커버리지 통합은 Build-and-Test 이월.
- **수용 기준**:
  - (AC-1) business-critical 경로(히스토리 round-trip·health/닫힌집합 판정·rotation·config 검증)는 PBT와 예제 기반 테스트를 병행한다(어떤 핵심 경로도 PBT 단독 아님).
  - (AC-2) 커버리지 %-게이트는 설정하지 않으며 CI 통합은 Build-and-Test 이월임을 명시한다.

> **포터빌리티(NFR-05) 연계**: U6는 `std::fs`·`std::sync`·`std::time` 및 워크스페이스 공용 크레이트만 사용하는 순수 lib이라 플랫폼별 코드가 없다(3-OS 트레이 백엔드는 D1로 DROP=no-op). 크로스플랫폼 재현 빌드는 U0가 전파하는 Edition 2024 + 고정 MSRV(워크스페이스 상속)로 보장되며, U6는 별도 포터빌리티 NFR을 신설하지 않는다(tech-stack-decisions.md §1).

---

## 5. 보안-잔존 (Security residual)

**전제**: Security Baseline 확장 = **OFF**(requirements.md §2.3 Q1=B). 암호화 저장·키관리·시크릿 스캐닝 등 강제 통제는 신설하지 않으며, 로그·히스토리의 로컬 평문(RISK-01)은 **문서화된 수용 위험**이다(requirements §7·§13, `business-rules.md` R-LOG-04/R-HIST-06). 아래 항목은 신설 통제가 아니라 유일 잔존 위생(토큰 로그-유출 완화)의 실현이다.

### 5.1 U6-NFR-SEC-01 — 로그/히스토리 토큰 redaction 잔존 통제 (원문 미방출)

- **요구(신규 통제 아님 — U0 잔존 통제의 소비)**: `StructuredLogger` 방출 라인(`message`/`fields`/`event`)과 `UploadHistoryStore` 레코드(`error_detail`)는 config 유래 비밀(토큰)을 **원문으로 방출하지 않는다**. 토큰은 U0 `TokenSecret`(redacting `Debug`/`Display`, 실제 값은 `.expose()` 로만)으로만 표현되며, U6 로거는 per-field 프로젝션만 방출하고 평문 토큰을 `message`/`fields`/`error_detail` 에 넣지 않는다. `error_detail`/`fields` 에 우발적으로 비밀이 섞일 수 있음은 RISK-01 수용 위험으로 문서화된다.
- **선정 근거**: U0가 `WatcherConfig.token` 을 자체 redacting newtype으로 감싸는 결정(U0-NFR-SEC-02, Q11=A)의 **소비 지점**이 바로 U6 로거/히스토리 직렬화다. U6는 그 계약을 준수(평문 토큰 미방출)해 로컬 평문 로그로의 토큰 유출을 방어한다 — 외부 의존 0의 저비용 위생이며 Security Baseline을 켜는 것이 아니다.
- **근거**: `business-rules.md` §2 R-LOG-04(토큰 원문 금지 잔존 통제)·§5 R-HIST-06(히스토리 평문 RISK-01) · `domain-entities.md` §1.1(redaction 방출 규칙) · requirements.md §7 RISK-01(로컬 평문 산출물에 로그/히스토리 포함)·§13 · U0 `nfr-requirements.md` §5.2 U0-NFR-SEC-02 / `tech-stack-decisions.md` §8.
- **수용 기준**:
  - (AC-1) 로그 방출 라인과 히스토리 레코드에 실제 토큰 값이 평문으로 나타나지 않는다(토큰은 U0 `TokenSecret` redaction 경유 — 예제 기반 + U0 `TokenSecret` 속성 재사용, `business-rules.md` §8.6).
  - (AC-2) 로그/히스토리 파일이 OS 보안 저장소 밖 로컬 평문임(RISK-01)과 `error_detail`/`fields` 의 우발적 비밀 혼입 가능성이 수용 위험임을 명문화한다(로컬 산출물 정리는 U7a `Uninstaller`/E6 소관).
  - (AC-3) 이는 잔존 위생일 뿐 Security Baseline 통제를 켜는 것이 아니다(확장 여전히 OFF).

---

## 6. 사용성 (Usability)

### 6.1 U6-NFR-USE-01 — JSON-line 최소 필드 계약 (기계 판독 관측 표면)

- **요구**: `StructuredLogger` 방출 라인은 **개행으로 구분되는 기계 판독 가능한 JSON-line 1건**으로, **최소 필드 집합 `{ timestamp, level, event, cycle_id, message }`(+ `fields`)를 항상 포함**한다(`cycle_id` 부재는 `null`). `cycle_id` 상관으로 한 사이클의 이벤트를 사후 상관 조회할 수 있다. 이는 데몬(GUI-less)의 주 운영자-대면 관측 표면이므로 스키마 안정성과 무손실 파싱(유니코드·개행 이스케이프)을 보장한다.
- **선정 근거**: 트레이가 no-op(D1)이고 CLI(`OperatorCli`)는 U7b 소관이므로, U6의 사람/도구 대면 사용성 표면은 **구조화 로그 라인**이다. 최소 필드 계약을 고정해 로그 집계·상관 도구가 안정적으로 파싱하게 한다(RESILIENCY-05 구조화 로깅 정합).
- **근거**: `business-rules.md` §2 R-LOG-01(JSON-line 방출, 최소 필드) · `domain-entities.md` §1.1(방출 라인 스키마·불변식) · `business-logic-model.md` §1.1(cycle_id 상관) · requirements.md §3.5 FR-17 · §5.6 NFR-15 · US-E5-01. JSON 직렬화 크레이트(`serde_json`) 정본 = tech-stack-decisions.md §2.
- **수용 기준**:
  - (AC-1) 각 방출 라인은 최소 필드 집합을 포함하고 `cycle_id` 부재는 `null` 로 표현된다.
  - (AC-2) 임의 `LogRecord` + stamped `timestamp` 로 방출한 JSON-line을 재파싱하면 전 필드가 무손실 복원된다(유니코드/개행 `message`/`fields` 포함, PROP-U6-DE-02, Round-trip).
- **그 외 사용성 = N/A**: 트레이 GUI 표면(D1 no-op)·CLI 종료코드 매핑(U7b `OperatorCli` 소관, `health_check()` 는 `Health` 값만 제공)은 U6 범위 밖이다(§7).

---

## 7. N/A 카테고리 근거표

아래 카테고리는 U6에 요구를 신설하지 않는다(발명 금지). 각 판정은 확정 세트(FD 전제 D1..D14, requirements RESILIENCY-02/§6)에서 온 것이다.

| 카테고리 | 판정 | 근거 |
|---|---|---|
| **확장성(Scalability)** | N/A | U6는 순수 lib(in-process 싱크 + 값 집계 + append-only 지속)로 자체 런타임·처리량 축이 없음. 단일 사용자 데스크톱 데몬 규모(사이클당 최대 1 히스토리 레코드, 저빈도 CLI 읽기)라 스케일 축 부재. 히스토리 전체 스캔은 레코드 수에 선형·유계(U6-NFR-PERF-02). |
| **가용성(Availability)** | N/A | lib 크레이트라 SLA 없음. 전체 Watcher는 단일 사용자·사용자 재시작 로컬 프로세스라 RTO/availability-SLA가 requirements RESILIENCY-02(§6)에서 이미 N/A 확정. |
| **성능 수치 목표(Performance numeric)** | N/A(정성 계약만) | 로그 쓰기(라인 길이·rotation 임계 선형)·히스토리 query(레코드 수 선형)에 throughput/latency/peak-memory 수치 게이트 근거 없음. 사이클 병목은 U3 전송이지 관측이 아님. U6-NFR-PERF-01/02의 정성 계약만 문서화. |
| **Resiliency DR / RTO / RPO** | N/A(신규 결정 없음) | DR/RTO/availability는 RESILIENCY-02에서 N/A 확정(단일 사용자 로컬). **RPO=0(zero-loss)은 U4 `SyncStateStore` durable state 소관**이며, U6 히스토리는 **비권위 감사 로그**로 append 실패가 삼켜지고(REL-01) truncated-tail 관용(REL-02)으로 재구성된다 — U6에 별도 RPO 수치 신설 없음. 배포/롤백은 U7a `AutoUpdater`(U6는 `update_probe` 판정만 제공). |
| **사용성(로그 라인 외)** | N/A | 트레이 GUI 표면 = D1 no-op(DROP). CLI 종료코드/명령 표면 = U7b `OperatorCli` 소관(U6는 `Health`/`query` 값만 제공). U6의 in-scope 사용성은 JSON-line 계약(U6-NFR-USE-01) 하나. |
| **보안 강제 통제(Security enforced)** | N/A | Security Baseline OFF, RISK-01 수용(로그/히스토리 로컬 평문, R-LOG-04/R-HIST-06). 암호화 저장·키관리·시크릿 스캐닝 신설 N/A. 유일 in-scope 잔존 결정 = 토큰 redaction 위생(U6-NFR-SEC-01), U0 `TokenSecret` 소비. |

---

## 8. NFR to 요구사항 추적표

각 U6 NFR을 소스 NFR/FR ID(requirements) + 규칙 ID(business-rules) + PBT 속성/규칙 ID로 매핑한다.

| U6 NFR ID | 요약 | 소스 NFR/FR ID | 규칙 ID | PBT ID |
|---|---|---|---|---|
| U6-NFR-REL-01 | 관측 push best-effort·infallible 비차단 | NFR-15, FR-17/18/19, RESILIENCY-05/15 | R-PUSH-02/03 | PROP-U6-BL-02 (Invariant) |
| U6-NFR-REL-02 | 히스토리 append-only 무손실 round-trip + truncated-tail | NFR-13, FR-16, RESILIENCY-02 | R-HIST-01/02/04 | PROP-U6-DE-01(Round-trip)·PROP-U6-BR-08/09 |
| U6-NFR-REL-03 | `StatusService` 단일 집계 스레드 안전 | FR-18, RESILIENCY-06 | R-STATUS-01/02/05 | PROP-U6-BR-02·05 (Invariant) |
| U6-NFR-REL-04 | 중대오류 3중 fan-out(트레이 비의존) | FR-19, NFR-15, RESILIENCY-15 | R-CRIT-01/02/03 | PROP-U6-BR-06/07·PROP-U6-BL-01 |
| U6-NFR-REL-05 | 로그레벨 리로드 재적용 keep-last-good | FR-17 | R-LOG-06 | (U0 config 흐름 PROP-BL-04 흡수) |
| U6-NFR-PERF-01 | 로그 쓰기 동기 직렬화·유계(수치 N/A) | NFR-15, RESILIENCY-05 | R-LOG-03/05 | (rotation 예제테스트, §8.6) |
| U6-NFR-PERF-02 | 히스토리 query 선형·유계(수치 N/A) | FR-16, NFR-13 | R-HIST-05 | PROP-U6-BR-08 (Oracle) |
| U6-NFR-MNT-01 | 오류 타입 파생 전략 | NFR-17 | (U0 오류 계약 소비) | PROP-U6-DE-01 (값 타입 round-trip) |
| U6-NFR-MNT-02 | PBT 제너레이터 재사용 + 로컬 노출 | NFR-17, NFR-13(기반) | (제너레이터 계약) | PBT-07 |
| U6-NFR-MNT-03 | 커버리지/문서 정책 | NFR-17 | (공개 계약) | PBT-10 |
| U6-NFR-SEC-01 | 로그/히스토리 토큰 redaction 잔존 | NFR-06(잔존), RISK-01 | R-LOG-04, R-HIST-06 | (U0 `TokenSecret` 속성 재사용) |
| U6-NFR-USE-01 | JSON-line 최소 필드 계약 | FR-17, NFR-15, US-E5-01 | R-LOG-01 | PROP-U6-DE-02 (Round-trip) |
| (포터빌리티 연계) | Edition 2024 + 고정 MSRV로 실현(std-only) | NFR-05 | (tech-stack 결정) | N/A(빌드 재현성) |

---

## 9. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 단계 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제·준수 (PBT-09 상속 충족)** | 프레임워크(PBT-09) = U0에서 확정된 `proptest` 를 워크스페이스 상속으로 재사용(재결정 아님). U6는 PBT-07(제너레이터 재사용 + 로컬 노출)을 U6-NFR-MNT-02(`proptest-support` 비기본 feature, U0 mirror)로 실현하고, FD가 식별한 속성(PROP-U6-DE-01/02·BR-01~09·BL-01/02)을 §2 수용 기준으로 요구화한다. 속성 없는 요소(구조 불변·no-op·rotation IO)는 FD §8.6 판정 계승. PBT-08(케이스 수·shrinking·시드·CI) 상세는 Code Generation / Build-and-Test 이월. **PBT-09가 상위(U0)에서 확정되었으므로 blocking 없음.** |
| **Resiliency Baseline** | ON | **부분 적용 + 일부 N/A** | RESILIENCY-05(구조화 로깅) = U6-NFR-USE-01/PERF-01·R-LOG-*, RESILIENCY-06(헬스/status) = U6-NFR-REL-03·R-STATUS-*, RESILIENCY-15(사고 표면화) = U6-NFR-REL-04·R-CRIT-* 직접 구현. best-effort infallible 비차단(U6-NFR-REL-01)으로 관측 실패가 코어를 막지 않음. append-only + truncated-tail 관용(U6-NFR-REL-02)으로 크래시 후 재기동 query 성공. `update_probe` 격리(U6-NFR-REL-03 AC-3)가 RESILIENCY-04 자동 롤백 게이트 안정성 근거. **RTO/RPO 수치·DR·HA·배포/롤백은 U6(순수 lib·비권위 감사 로그)에 N/A**(RESILIENCY-02, requirements §6) — RPO=0은 U4 소관. 신규 U6 Resiliency 수치 결정 없음. |
| **Security Baseline** | OFF | **N/A (잔존만 표면화)** | 미로딩·미강제. RISK-01(로그/히스토리 로컬 평문, R-LOG-04/R-HIST-06) 수용. 암호화 저장·키관리·시크릿 스캐닝 신설 없음. 유일 in-scope 잔존 결정 = 토큰 redaction 위생(U6-NFR-SEC-01), U0 `TokenSecret`(U0-NFR-SEC-02) 소비. |

**블로킹 판정**: PBT-09가 상위 U0에서 `proptest` 로 확정되어 워크스페이스 상속되고, U6는 PBT-07 제너레이터 노출(`proptest-support`)을 tech-stack-decisions.md에 문서화하므로 Property-Based Testing 확장의 blocking finding은 없다. Resiliency·Security는 N/A(또는 부분 적용)로 blocking 없음.

---

## 10. 후속 단계 이월 항목 (참고)

- PBT 케이스 수·shrinking·고정 시드·CI 통합(PBT-08) -> Code Generation / Build-and-Test.
- 비동기 로깅 스레드 + bounded 채널 + drop-on-full 백프레셔(D3 트림) -> 향후 확장(현재 동기 best-effort로 충분).
- federated rotation 파라미터(`log_max_size_bytes`/`log_max_retained`) live-reload(현재 재시작 반영) -> 향후 확장.
- 히스토리 인덱싱/임베디드 DB(D8 트림) -> 향후 확장(현재 전체 스캔으로 충분).
- 실제 데스크톱 트레이 백엔드(D1 no-op DROP, 3-OS) -> 향후 confirm 시 no-op 교체.
- 상세 CI 커버리지 통합(U6-NFR-MNT-03) -> Build-and-Test.
- 구체 크레이트/툴체인 결정(로그 JSON=`serde_json`·히스토리 CBOR=`ciborium`·동시성=`std::sync::Mutex`·파일 I/O=`std::fs`·오류=`thiserror`·PBT=`proptest`+`proptest-support` feature·트레이 크레이트 미채택)의 정본 -> 자매 산출물 `tech-stack-decisions.md`.

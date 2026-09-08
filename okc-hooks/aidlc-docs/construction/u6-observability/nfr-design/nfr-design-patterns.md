# U6 Observability — NFR Design Patterns (NFR 실현 설계 패턴)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U6 Observability** -> NFR Design -> 산출물 1/2 (`nfr-design-patterns.md`)
**작성일**: 2026-09-08
**크레이트**: `observability` (lib) · **소속 컴포넌트**: `StructuredLogger`, `StatusService`, `UploadHistoryStore`, `CriticalErrorNotifier`, `TrayIndicator`(선택 no-op)
**입력 아티팩트**: `nfr-requirements/nfr-requirements.md`·`nfr-requirements/tech-stack-decisions.md`(U6 NFR Requirements 산출물) · `functional-design/{domain-entities,business-rules,business-logic-model}.md`(규칙/타입/흐름 ID) · `u0-foundation/nfr-design/{nfr-design-patterns.md,logical-components.md}`(스타일 템플릿) · `inception/application-design/component-methods.md`(공개 5 컴포넌트 메서드 표면) · 활성 확장 `property-based-testing.md`(ON, Full)·`resiliency-baseline.md`(ON)
**규칙**: `construction/nfr-design.md` Step 6 · `common/content-validation.md`(ASCII 화살표 `A -> B`, 박스/유니코드-다이어그램 문자 미사용, Rust 제네릭 백틱)

> **문서 성격**: 이 문서는 U6가 확정한 **품질 속성(NFR Requirements)** 을 **어떤 설계 패턴으로 실현하는가** 를 기록한다. 여기서 새로 결정하는 것은 없다 — NFR Requirements(카테고리별 NFR)와 tech-stack-decisions(구체 크레이트), Functional Design(규칙 ID), AUTOPILOT 확정 답변(FD D1..D14)을 **설계 패턴으로 실현**한다. 각 패턴은 (a) 패턴/결정 진술, (b) 실현하는 NFR·규칙 근거, (c) 구조 노트/불변식, (d) 명시적 트레이드오프로 기술한다. 논리 컴포넌트 분해의 상세 맵은 자매 산출물 `logical-components.md`가 소유하며, 이 문서는 각 패턴이 어느 논리 컴포넌트에 안착하는지만 §12 추적표에서 참조한다.
>
> **표기 규약**: 화살표는 ASCII `A -> B`만 사용한다(유니코드 화살표 금지). 박스/선-그리기/유니코드 수학 기호를 쓰지 않고 집합 관계는 한국어 산문으로 기술한다. Rust 제네릭/타입/식별자(예: `Arc<dyn Logger>`, `Mutex<StatusInner>`, `Vec<u8>`, `PathBuf`)는 백틱으로 감싼다. English 식별자(크레이트·타입·config 키·규칙/NFR ID)는 원문 그대로 유지한다.

---

## 0. Federated-config 해소 — U6는 RESOLVED TYPED 값을 생성자로 주입받는다 (전파 원칙)

**핵심 결정(wave-level 정합)**: U6 `observability`는 비코어 config를 **`ConfigProvider` 로 읽지 않는다**. 비코어 키(`log_file`/`log_max_size_bytes`/`log_max_retained`/`history_file`/`tray_enabled`)는 U8 조립 루트(`watcher-bin`)가 원시 config 파일을 파싱·해소해 **RESOLVED TYPED 값**으로 각 U6 컴포넌트 **생성자에 하향 주입**한다(downward injection; DEC-FEDERATED-KEYS Q6=A). 주입되는 타입 매핑:

| 비코어 config 키 | 주입 타입(RESOLVED) | 주입 대상 컴포넌트 |
|---|---|---|
| `log_file` | `PathBuf`(부재 시 U8이 플랫폼 기본 경로 해소) | `StructuredLogger` |
| `log_max_size_bytes` | `ByteCount`(U0) | `StructuredLogger`(rotation 파라미터) |
| `log_max_retained` | `usize` | `StructuredLogger`(rotation 파라미터) |
| `history_file` | `PathBuf`(부재 시 U8이 플랫폼 기본 경로 해소) | `UploadHistoryStore` |
| `tray_enabled` | `bool` | `TrayIndicator`(no-op은 무시) |

- **U0 FROZEN 유지**: U6는 U0 `ConfigProvider.current()` 로 **오직 코어 필드**(`log_level`, `notify_consecutive_failures`)만 읽는다. `log_level` 은 `StructuredLogger` 의 레벨 필터에, `notify_consecutive_failures`(임계 `N`, 기본 3)는 `CriticalErrorNotifier` 의 연속실패 상태머신에 소비된다. 비코어 값은 절대 `ConfigProvider` 경유가 아니다.
- **live-reload 범위**: 코어 필드(`log_level` 등)만 U0 관찰자 팬아웃(`ConfigReloadObserver`)으로 fan-out되어 live-reload된다. **federated 비코어 파라미터(rotation 크기/개수·파일 경로)의 live-reload는 MVP out-of-scope**(재시작으로 변경) — §7 REL-05 패턴 참조. `OBSERVABILITY_CONFIG_KEYS` const 선언(U8 federated union 집계용, R-CFG-U6-01)은 파싱-수용 목적일 뿐 U6가 타입드로 읽는 경로가 아니다.
- **결론**: `federated_config_handled = true` — U6는 생성자 주입 계약을 명문화하고 `ConfigProvider` 비코어 독을 배제한다.

---

## Autopilot Decisions (미결점 자체 확정 — MVP 바이어스)

> AUTOPILOT(게이트 waived): 각 미결점을 RECOMMENDED·MVP 바이어스로 자체 확정한다. NFR Requirements/tech-stack에서 이미 확정된 항목은 재오픈하지 않고 **설계 패턴 수준의 실현 결정**만 아래에 기록한다.

| topic | chosen | MVP-trim? | rationale |
|---|---|---|---|
| 관측 실패 비차단 실현 | infallible 트레이트 반환 형상 + 내부 오류 삼킴(별도 큐 없음) | 예 | REL-01. best-effort 삼킴이 비차단을 실현. 재시도/블로킹/패닉 경로 부재. |
| 히스토리 프레이밍 | 고정폭 length prefix(`u64` LE) + U0 `ciborium` CBOR payload + `std::fs` append | 아니오 | R-HIST-02/NFR-13. truncated-tail 판정을 결정적으로. 별도 프레이밍 크레이트 불요. |
| 크래시 관용 실현 | 순차 프레임 디코드 중 **트레일링** 부분 프레임을 truncated-tail로 관용(정상 prefix 반환), 중간 손상만 `Corrupt` | 아니오 | R-HIST-04. append 도중 크래시가 이전 레코드를 오염 못 함. fsync 강제·journaling은 미도입(MVP). |
| `StatusService` 동시성 | `std::sync::Mutex<StatusInner>` 단일 락, `snapshot` 은 락 하 완결 값 복제 | 예(RwLock 이월) | REL-03. `Send`+`Sync`(U0 트레이트) 충족 최소 수단. CLI 읽기 저빈도. |
| 로그 writer 동시성 | `std::sync::Mutex` 로 파일 핸들+크기+rotation 파라미터 직렬화(동기 best-effort) | 예 | PERF-01. 락은 소형 라인 1건 쓰기 시간만 보유. 비동기 스레드·bounded 채널 이월. |
| 연속실패 카운터 | `Mutex` 내부 상태 또는 `AtomicUsize` + escalation 플래그 최소 | 아니오 | REL-04. 임계 도달 1회 발화·Success 리셋. std 프리미티브로 충분. |
| rotation 실현 | append 후 `metadata().len()` 확인 + `rename` 회전 + 초과 개수 삭제 | 예 | R-LOG-05. 나이/기간 기반 미도입(개수 근사). |
| now-source(타임스탬프) | U0 `Timestamp`(방출 시 stamping, now = `std::time::SystemTime`); 테스트에선 clock seam 주입 | 아니오 | D4. `chrono`/`time` 미도입. 결정적 테스트를 위한 clock seam만 노출. |
| 트레이 백엔드 | no-op 어댑터(`start() -> Ok(None)`, 나머지 무연산), 트레이 크레이트 미도입 | 예 | D1. GUI-less 데몬. 향후 confirm 시 실체 어댑터로 교체. |
| federated 비코어 live-reload | 미실현(재시작 반영); 코어 필드(`log_level`)만 fan-out reload | 예 | §0/§7. U0 FROZEN 유지 + 주입 값 불변(MVP). |
| PBT 제너레이터 노출 | `proptest-support` 비기본 cargo feature(U0 mirror) + U0 제너레이터 재사용 | 아니오 | MNT-02. 프로덕션 빌드에 `proptest` 미유출. |
| 신규 워크스페이스 외부 의존 | 0건(전부 U0 핀 상속, 나머지 std) | 예 | tech-stack §10. U6 신규 크레이트 없음. |

---

## 1. 관측 push best-effort·infallible 비차단 (REL-01) — infallible 반환 형상 + 내부 오류 삼킴, 재시도·블로킹 부재

**(a) 패턴/결정**: U6의 4개 실체 싱크가 구현하는 U0 트레이트 push 표면(`Logger::log`/`event`, `StatusSink` 전 뮤테이터, `HistorySink::append`, `CriticalEventSink` 전 `report_*`)은 **오류를 반환하지 않는 형상**(반환값 없음/infallible)을 유지한다. 내부에서 발생하는 IO/코덱/rotation 오류(`LogError`/`HistoryError`)는 **삼키고**(가능하면 로거로 남기고) 즉시 반환한다. **재시도 루프·무한 블로킹·패닉으로 사이클을 정지시키지 않는다** — 락은 단일 소형 작업 시간만 보유하고, 오류는 재시도 없이 폐기(best-effort)한다. 실패를 표면화하는 U6 표면은 **읽기/재적용 경로**(`UploadHistoryStore::query() -> Result<_, HistoryError>`, `StructuredLogger::reload() -> Result<_, LogError>`)뿐이다.

**(b) NFR/규칙 실현**:
- **U6-NFR-REL-01**(관측 push 비차단)의 "실패를 호출자에게 전파하지 않으며 코어를 절대 막지 않는다"를, 라이브러리 기본이 아니라 **트레이트 반환 형상(타입 시스템)** + **내부 삼킴 규율**로 실현한다.
- **규칙 R-PUSH-02**(best-effort infallible) · **R-PUSH-03**(읽기/재적용만 `Result`) · **R-LOG-03**(동기 best-effort 쓰기) · **R-HIST-03**(append infallible).
- 이 패턴은 §2(히스토리 append 삼킴)·§5(로그 쓰기 삼킴)·§4(fan-out 내 로그/status 반영)의 **공통 기반 불변식**이다.

**(c) 구조 노트/불변식**: 불변식 — 어떤 push 호출도 오류를 상향 전파하지 않고 유한 시간에 반환한다(무한 재시도·블로킹 부재). 관측 실패는 관측 표면에 국한되며 동기화 코어(U1~U5/U8)의 제어 흐름을 바꾸지 않는다. `query`/`reload` 만 `Result` 로 실패를 표면화한다(읽기 경로는 코어 핫 경로가 아님).

**(d) 트레이드오프**: (1) push 표면에 `Result` 를 반환해 호출자가 처리하게 하는 안(대안)은 **미채택** — 관측이 코어의 단일 장애점이 되어 "관측은 부수적·비치명" 계약을 위반한다. (2) 실패 시 재시도·백오프를 두는 안도 미채택 — 핫 경로 블로킹 위험. 채택안의 대가는 일시적 관측 손실 가능성(로그 라인 유실 등)이나, 이는 감사·관측 표면에 수용 가능하며 히스토리는 append-only 무손실(§2)로 별도 보증된다.

---

## 2. 히스토리 append-only 무손실 지속 + 길이-프레이밍 + truncated-tail 크래시 관용 (REL-02, PERF-02) — U0 코덱 재사용 + 스트리밍 프레임 I/O

**(a) 패턴/결정**: `UploadHistoryStore` 지속은 **U0 CBOR 코덱(`encode`/`decode`, 백엔드 `ciborium`)** 으로 레코드를 인코딩하고 **고정폭 length prefix(`u64` LE) + CBOR payload** 프레임을 append-only 파일에 `std::fs` 로 **스트리밍 append**(전체 파일을 메모리에 적재하지 않음)한다. `query` 는 파일을 **프레임 단위 순차 디코드**하며(전체 파일을 한 번에 역직렬화하지 않음), 트레일링(파일 끝) 프레임이 부분/디코드 불가이면 append 도중 크래시로 생긴 **truncated tail로 간주해 정상 prefix를 성공 반환**한다. 파일 **중간** 프레임 손상만 `HistoryError::Corrupt` 로 표면화한다. 필터(`since`/`only_failures`)는 디코드 스트림에 **선형 AND 결합**으로 적용한다(보조 인덱스·임베디드 DB 없음; 성공/실패는 `error_detail` 유무로 파생).

**(b) NFR/규칙 실현**:
- **U6-NFR-REL-02**(append-only 무손실 + truncated-tail 크래시 관용) 및 **U6-NFR-PERF-02**(query 전체 스캔 선형·유계)를 함께 실현한다.
- **규칙 R-HIST-01**(append-only 불변) · **R-HIST-02**(무손실 프레임, `decode(encode(r)) == r`) · **R-HIST-04**(truncated-tail 관용) · **R-HIST-05**(AND 필터 오라클).
- **U0-NFR-REL-01**(코덱 무손실 round-trip) 위에 U6 지속 계층이 얹는 append-only + 크래시 관용 불변식이다. RESILIENCY-02(로컬 상태 재구성) 정합.

**(c) 구조 노트/불변식 — 원자성/크래시 안전**: 불변식 — (1) 기존 레코드는 절대 수정/삭제되지 않고 `query` 는 append 순서(또는 결정적 정렬)로 반환한다. (2) 프레임 경계는 length prefix로 자기기술적이라, append 중 크래시로 마지막 프레임이 잘려도 이전 완결 프레임들의 디코드가 영향받지 않는다(prefix 보존). (3) 무손실 대상은 U0 FROZEN 3필드(`Timestamp` `timestamp`·`ByteCount` `bytes_transferred`·`Option<String>` `error_detail`)이며 자기기술적 CBOR라 향후(POST-MVP) 스키마 진화(새 필드)에도 강하다. append 단위 원자성은 단일 프레임 write(락 하)로 근사하며 **fsync 강제·저널링은 MVP 미도입**(truncated-tail 관용이 대체 안전망).

**(d) 트레이드오프**: (1) 임베디드 DB(`sqlite`/`sled`)·보조 인덱스는 **미채택**(D8) — 사이클당 최대 1레코드 감사 로그 규모에 과대하고 신규 의존·마이그레이션 부담. 대가는 query가 레코드 수 선형(인덱스 없음)이나 데스크톱 규모에 충분. (2) 매 append `fsync` + 저널링은 미채택 — 히스토리는 비권위 감사 로그(RPO=0은 U4 소관)라 truncated-tail 관용으로 충분하고, 강제 sync는 sync-path 비용을 키운다. (3) JSON 프레이밍 대신 CBOR 선택 — 무손실·자기기술 스키마 진화 강건성(사람 가독은 로그 경로가 담당).

---

## 3. `StatusService` 단일 집계 락 원자 관측 + update_probe 격리 (REL-03) — `std::sync::Mutex<StatusInner>` 단일 락

**(a) 패턴/결정**: 2축 상태(축1 `operational` 단일값 + 축2 `conditions` 집합 + `last_success`/`dirty`/`resume`/liveness 신호 + 연속실패 escalation 플래그)를 **`std::sync::Mutex<StatusInner>`** 단일 락으로 보호한다. 모든 push 뮤테이터는 락 하에서 in-memory 상태를 갱신하고, `snapshot()` 은 락 하에서 **완결 값을 복제**해 반환한다. `health_check()`/`update_probe()` 도 락 하 파생이라 부분 갱신을 노출하지 않는다. `update_probe()` 는 **오직 `{IdleReached, CredentialReadable}` 두 liveness 신호 관측 여부에만 의존**하고 어떤 `ActiveCondition`/운영상태에도 영향받지 않는다(격리).

**(b) NFR/규칙 실현**:
- **U6-NFR-REL-03**(단일 집계 스레드 안전, 동시 push/read 원자 관측)의 "읽기 스레드는 항상 완결 스냅샷 하나만 관측"을 단일 `Mutex` 원자 경계로 실현한다.
- **규칙 R-STATUS-01**(2축 독립성) · **R-STATUS-02**(집합 멱등) · **R-STATUS-03**(snapshot 파생) · **R-STATUS-04**(health 오라클) · **R-STATUS-05**(update_probe 격리, 롤백 루프 방지) · **R-STATUS-06**(liveness 단조).
- `update_probe` 격리(R-STATUS-05)가 **RESILIENCY-04 자동 롤백 게이트의 안정성 근거**다(일시 조건에 의한 오판 롤백 방지, U7a `AutoUpdater` 전용 소비).

**(c) 구조 노트/불변식**: 불변식 — (1) 동시 push/read 하에서 어떤 읽기도 축1과 축2가 반쯤 반영된 중간 상태를 관측하지 않는다(완결 스냅샷만). (2) 두 축은 직교하며 서로를 자동 변경하지 않는다(`Syncing` 중 `OverLimit`+`AuthFailed` 동시 성립 가능). (3) health-critical 집합은 US-E5-04 닫힌 4조건(`AuthFailed`/`OverLimit`/`UpdateRolledBack` + escalation)과 정확히 일치하며 비목록 조건(`ConsentBlocked`/`Offline`/`dirty`/`VaultUnavailable`)은 health를 flip하지 않는다. (4) `update_probe` 결과는 두 liveness 신호를 고정한 채 임의 조건을 raise/clear해도 불변.

**(d) 트레이드오프**: (1) `RwLock<StatusInner>` 는 다중 동시 읽기에 유리하나 CLI status 읽기가 **저빈도**(사이클 지배적 아님)라 이점이 없고 writer starvation 여지를 추가 -> **이월**(MVP 트림). (2) 락-프리(per-field atomics + 스냅샷 조립)는 가변 길이 `conditions` 집합에 부적합·과복잡 -> 미채택. 단일 `Mutex` 가 최소 코드로 원자 관측을 실현한다. 대가는 push/read가 짧은 임계구역을 공유하나 상태 갱신이 in-memory·경량이라 무시할 만하다.

---

## 4. 중대오류 닫힌 4조건 3중 fan-out (트레이 비의존) + 연속실패 상태머신 (REL-04)

**(a) 패턴/결정**: `CriticalErrorNotifier` 는 능동 표면화를 **정확히 4개 진입점**(`report_auth_failure`/`report_cycle_result`/`report_preflight_exceeded`/`report_update_rollback`)으로만 발생시키는 **닫힌 집합**을 유지한다. 각 능동 표면화는 **3중 fan-out** 한다: (1) `Arc<dyn Logger>`(주입) 로 상향 심각도(`Warn`/`Error`) 구조화 로그, (2) `Arc<dyn StatusSink>`(주입) 로 케이스별 `raise_condition` 또는 escalation 플래그 push -> `health_check()` unhealthy 전환 + CLI status 반영, (3) `TrayIndicator.notify`(no-op). 연속실패(케이스2)는 `consecutive_failure_count`(`Mutex` 내부 상태 또는 `AtomicUsize`)를 유지해 임계 `N`(주입된 `notify_consecutive_failures`, 기본 3) **도달 순간 정확히 1회** 발화하고 escalation 플래그를 세팅하며, Success 시 카운트·플래그를 리셋한다(중복 억제).

**(b) NFR/규칙 실현**:
- **U6-NFR-REL-04**(중대오류 3중 fan-out, 트레이 비의존)의 "앞 2개 표면(로그 + status/health)은 트레이 유무와 무관하게 항상 성립"을 실현한다.
- **규칙 R-CRIT-01**(닫힌 집합) · **R-CRIT-02**(연속실패 임계 상태머신) · **R-CRIT-03**(3중 fan-out) · **R-CRIT-04**(조건 해제는 상위 소유). RESILIENCY-15(사고 표면화) 직접 구현.
- fan-out 대상(`Logger`/`StatusSink`)은 U8 조립 루트가 U6-내부에서 배선한다(U1~U5 역참조 없음, R-PUSH-01).

**(c) 구조 노트/불변식**: 불변식 — (1) 4개 진입점 외 어떤 이벤트·비목록 일시 오류도 능동 표면화하지 않는다(로그 전용). (2) 트레이가 no-op이어도 로그 + status/health 표면은 항상 성립(트레이 비의존). (3) 연속실패 상태머신은 참조 모델(단순 카운터)과 동형이며 임계 도달 시 1회만 발화(후속 실패 재발화 없음, 성공 시 리셋). escalation 플래그는 `ActiveCondition`(U0 닫힌 enum) 밖의 U6-내부 상태로 §3 `StatusService` 에 push된다.

**(d) 트레이드오프**: (1) 실제 데스크톱 트레이 백엔드(3-OS)는 **DROP=no-op**(D1) — GUI-less 데몬 모델에서 사고 표면화의 신뢰성이 트레이에 종속되면 안 된다. 대가는 데스크톱 팝업 부재이나 로그+health+CLI status가 항상 표면화하므로 US-E5-04 활성 표면은 완결된다. (2) 후속 실패마다 재발화하는 안은 미채택(D12 중복 억제) — 알림 피로 회피. 대가는 임계 도달 후 추가 실패가 즉시 재알림되지 않으나 escalation 상태가 유지되어 health는 계속 unhealthy.

---

## 5. 로그 쓰기 동기 직렬화 + 스트리밍 append + 크기/개수 rotation + JSON-line 최소 필드 계약 (PERF-01, USE-01)

**(a) 패턴/결정**: `StructuredLogger` 는 방출 라인을 **`serde_json`** 으로 JSON-line(개행 구분 1건)으로 직렬화하고, 파일 writer 상태(핸들 + 현재 크기 + rotation 파라미터)를 **`std::sync::Mutex`** 로 감싸 동시 방출을 **동기 직렬화**한다. 각 방출은 (i) 레벨 필터(`level >= 활성 log_level`), (ii) 방출 시각 stamping(U0 `Timestamp`, now = `std::time::SystemTime`), (iii) redaction 확인, (iv) `serde_json::to_string` + 개행 **스트리밍 append**(전체 파일 미적재), (v) 크기 초과 시 rotation 이라는 **유계 작업**이다. rotation은 append 후 `metadata().len()` 이 주입된 `log_max_size_bytes` 초과 시 `rename` 으로 회전하고 회전 파일 수가 `log_max_retained` 초과 시 가장 오래된 것을 삭제(개수 보존)한다. 방출 라인은 **최소 필드 집합 `{ timestamp, level, event, cycle_id, message }`(+ `fields`)를 항상 포함**하며 `cycle_id` 부재는 `null` 로 표현한다.

**(b) NFR/규칙 실현**:
- **U6-NFR-PERF-01**(로그 쓰기 동기 직렬화·유계, 수치 목표 N/A) + **U6-NFR-USE-01**(JSON-line 최소 필드 계약) + **U6-NFR-REL-05** 일부(rotation 파라미터 재적용은 §7).
- **규칙 R-LOG-01**(JSON-line 방출, 최소 필드) · **R-LOG-02**(레벨 필터) · **R-LOG-03**(동기 best-effort 쓰기) · **R-LOG-05**(크기 rotation, 개수 보존). RESILIENCY-05(구조화 로깅) 직접 구현.
- IO/rotation 오류는 §1(REL-01) 불변식으로 삼켜져 sync-path를 블로킹하지 않는다.

**(c) 구조 노트/불변식 — 스트리밍/비-전체파일 + 백프레셔**: 불변식 — (1) 다중 스레드 동시 방출 하에서 각 라인이 인터리빙·손상 없이 1건으로 직렬 기록된다(단일 `Mutex` 직렬화 경계). (2) 락은 단일 소형 라인 append(+ 간헐 rotation) 시간만 보유한다. (3) 무한 큐 성장·무한 재시도·비선형 폭증이 없다 — **백프레셔의 "비차단" 속성은 별도 bounded 채널·drop-on-full 큐가 아니라 best-effort·infallible 삼킴(§1)으로 실현**한다. (4) 방출 라인은 유니코드·개행을 JSON 문자열 이스케이프로 무손실 표현한다(파싱 무결, PROP-U6-DE-02).

**(d) 트레이드오프**: (1) `tracing`/`tracing-subscriber`/`tracing-appender` 생태계는 **미채택** — span·subscriber·layer 추상이 append-only 라인 1건 방출에 과대·신규 의존. `serde_json` 수제 writer로 충분(MVP 트림). (2) **비동기 로깅 스레드 + bounded 채널 + drop-on-full 백프레셔는 MVP 미구현**(D3 트림) — 동기 best-effort로 충분하며 async offload는 향후 확장 이월. 대가는 방출이 호출 스레드에서 동기 수행되나 라인 1건 쓰기라 사이클 지연 예산에 지배적이지 않다(병목은 U3 전송). (3) 나이/기간 기반 rotation은 미도입(D2) — 크기+개수 근사로 충분. (4) **성능 수치 게이트 없음**(throughput/latency/peak-memory) — 쓰기 비용이 라인 길이·rotation 임계에 선형·유계라 절대 수치 목표의 도메인 근거가 없다(U0 패턴).

---

## 6. 히스토리 query 스트리밍 순차 스캔 선형 필터 (PERF-02) — §2에 안착

> 별도 패턴이 아니라 §2 지속 패턴의 **읽기 경로 절**이다(동일 프레임 I/O seam). `query(HistoryQuery)` 는 파일을 프레임 순차 디코드하며 `since`/`only_failures` 를 **선형 AND 결합**으로 적용한다 — 임베디드 DB·보조 인덱스 없음(D8 트림). 비용은 레코드 수에 선형·유계이며 단일 사용자 데스크톱 데몬 규모(사이클당 최대 1레코드 append)에 충분하다. **성능 수치 게이트 없음**(query latency/throughput). 오라클 검증 = in-memory 참조 목록에 동일 AND 술어를 적용한 결과와 정확히 일치(PROP-U6-BR-08). 실현 NFR = **U6-NFR-PERF-02**, 규칙 = **R-HIST-05**.

---

## 7. config 리로드 재적용 keep-last-good + federated 주입 어댑터 (REL-05) — 코어 필드만 fan-out, 비코어는 생성자 주입

**(a) 패턴/결정**: `StructuredLogger` 는 U0 `ConfigReloadObserver` 를 구현해 U0 `ConfigProvider` 성공 스왑 시 `on_config_reload()` 통지(push, 페이로드 없음)를 받아 `ConfigProvider.current()` 를 재조회하고 **`log_level`(U0 코어 필드)을 재적용**한다. 재적용 실패는 `reload() -> Err(LogError)` 로 표면화하되 **데몬을 막지 않고 직전 정상 설정을 유지**한다(keep-last-good). **비코어 파라미터(`log_file`/`log_max_size_bytes`/`log_max_retained`)는 §0 federated 주입으로 생성자에서 받은 값을 유지하며 live-reload되지 않는다**(재시작 반영, MVP out-of-scope).

**(b) NFR/규칙 실현**:
- **U6-NFR-REL-05**(로그레벨/rotation 리로드 재적용 keep-last-good 비치명)의 "재적용 실패가 로깅을 정지시키지 않음"을 실현한다.
- **규칙 R-LOG-06**(리로드 재적용). U0 관찰자 팬아웃은 per-observer `catch_unwind` 로 격리되므로(U0-NFR-REL-04), `StructuredLogger` 의 재적용 실패가 팬아웃을 오염시키지 않는다.
- §0 federated-config 해소 원칙의 **live-reload 범위 한정**(코어 필드만)을 설계로 못박는다.

**(c) 구조 노트/불변식 — 포터빌리티/어댑터**: 불변식 — (1) `on_config_reload()` 통지 후 `log_level` 만 재적용되고, 재적용 실패 시 이전 성공 설정을 유지한다(keep-last-good). (2) rotation 파라미터·파일 경로는 주입 시점에 고정되어 재기동 전까지 불변(federated live-reload 미구현). (3) `ConfigProvider.current()` 재조회는 U0 코어 필드에 한정되며 U6는 U0를 FROZEN으로 소비만 한다(비코어 독 없음). config 값이 **U8 주입(어댑터)** 으로 U6에 들어오는 구조라 U6 내부에 config 파싱·플랫폼 분기 코드가 없다.

**(d) 트레이드오프**: (1) federated 비코어 파라미터의 live-reload를 지원하는 안은 **미채택**(MVP out-of-scope) — U0 `ConfigProvider` 는 6 코어 필드만 타입드로 노출하고 비코어 live-reload를 지원하려면 U0 FROZEN을 깨거나 U6에 별도 재주입 채널이 필요해 구현 범위가 커진다. 대가는 rotation 크기/개수 변경에 재시작이 필요하나 드문 운영 변경이라 수용. (2) 재적용 실패 시 데몬을 정지시키는 안은 미채택 — 잘못된 리로드가 로깅을 죽이면 안 됨(keep-last-good).

---

## 8. 오류 taxonomy 파생 (MNT-01) — `thiserror` 운영 오류 + 순수 serde 지속 값 타입

**(a) 패턴/결정**: U6 오류 타입을 U0 워크스페이스 관례(Q4=A)에 맞춰 두 부류로 파생한다:
- **운영 오류(반환용, 읽기/재적용 경로)**: `LogError`(`Io`/`InvalidPath`/`RotationFailed`) · `HistoryError`(`Io`/`Serde`/`Corrupt`)에 **`thiserror`** 파생을 사용해 `Display`/`std::error::Error` 를 얻는다. 이들은 push 표면에 전파되지 않고(§1 REL-01) `query`/`reload` 경로에서만 반환된다.
- **지속 값 타입(CBOR round-trip 대상, throw 아님)**: `UploadHistoryRecord`(U0 FROZEN 3필드)는 **순수 serde struct로 유지**(U0 `ciborium` 코덱 round-trip 값, §2).

**(b) NFR/규칙 실현**: **U6-NFR-MNT-01**(오류 타입 파생 전략). U0 `tech-stack-decisions.md` §5 오류 관례 상속(`anyhow` 식 타입소거 거부 — 구조화 변이 소실). `thiserror` 는 워크스페이스 핀 상속(신규 아님).

**(c) 구조 노트/불변식**: 불변식 — (1) `LogError`/`HistoryError` 는 `thiserror` 파생으로 `Display`/`Error` 를 제공하고 `query`/`reload` 경로에서만 반환된다. (2) `UploadHistoryRecord`(U0 FROZEN 3필드)는 `serde::Serialize`/`Deserialize` 만 파생하고 U0 CBOR round-trip이 무손실이다(PROP-U6-DE-01). 값 타입에 `thiserror` 를 붙이면 CBOR round-trip 대상이 아닌 오류 표면으로 오분류되므로 순수 serde를 유지한다.

**(d) 트레이드오프**: `anyhow` 타입소거는 U0가 이미 거부(구조화 변이 소실). 신규 오류 프레임워크 도입 없이 U0 관례를 재사용해 워크스페이스 전역 일관을 유지한다. 대가는 없음(순수 상속).

---

## 9. 토큰 redaction 잔존 통제 소비 (SEC-01) — U0 `TokenSecret` 계약 준수, 평문 미방출

**(a) 패턴/결정**: `StructuredLogger` 방출 라인(`message`/`fields`/`event`)과 `UploadHistoryStore` 레코드(`error_detail`)는 config 유래 비밀(토큰)을 **원문으로 방출하지 않는다**. 토큰은 U0 `TokenSecret`(redacting `Debug`/`Display`, 실제 값은 `.expose()` 로만)으로만 표현되며, U6 로거는 per-field 프로젝션만 방출하고 평문 토큰을 넣지 않는다. 이는 U0 no-Serialize-to-log 계약(U0-NFR-SEC-02, Q7=A)의 **소비-측 준수**다.

**(b) NFR/규칙 실현**: **U6-NFR-SEC-01**(로그/히스토리 토큰 redaction 잔존 통제). **규칙 R-LOG-04**(토큰 원문 금지) · **R-HIST-06**(히스토리 평문 RISK-01). U0-NFR-SEC-02 계약의 U6 소비 지점.

**(c) 구조 노트/불변식**: 불변식 — 로그 방출 라인과 히스토리 레코드에 실제 토큰 값이 평문으로 나타나지 않는다(U0 `TokenSecret` redaction 경유). `error_detail`/`fields` 에 우발적으로 비밀이 섞일 수 있음은 **RISK-01 수용 위험**으로 문서화되며(로컬 평문 산출물), 로컬 산출물 정리는 U7a `Uninstaller`/E6 소관이다.

**(d) 트레이드오프**: Security Baseline은 **OFF** — 암호화 저장·키관리·시크릿 스캐닝은 신설하지 않는다. 이는 신규 통제가 아니라 U0가 이미 채택한 redacting newtype의 **소비 계약 준수**(외부 의존 0의 저비용 위생)일 뿐이다. 대가는 `error_detail`/`fields` 우발 혼입이 코드로 완전 차단되지 않고 규율에 의존하나(RISK-01 수용), 명시적 토큰 필드는 `TokenSecret` 로 방어된다.

---

## 10. PBT 제너레이터·속성 실현 + 포터빌리티(std-only) + 어댑터 seam (MNT-02, MNT-03, NFR-05 연계)

**(a) 패턴/결정**:
- **PBT 실현**: `observability` 에 **비기본(non-default) cargo feature `proptest-support`** 를 두고 U6 고유 타입 제너레이터(`UploadHistoryRecord`(3필드), `LogRecord` 방출 라인, 상태 명령 시퀀스, `report_*` 시퀀스, IO 오류 주입 어댑터)를 노출한다(U0 패턴 mirror). U0 소유 타입(`Timestamp`·`ManifestDigest`·`LogLevel`·`ActiveCondition`/`OperationalState`/`LivenessSignal`·자유형 `detail`) 제너레이터는 U0 `proptest-support` 를 dev에서 켜 **재사용**(재작성 금지). FD 식별 속성(PROP-U6-DE-01/02·BR-01~09·BL-01/02)을 이 제너레이터로 실현한다.
- **포터빌리티(std-only)**: U6는 `std::fs`·`std::sync`·`std::time`·`std::io` 및 워크스페이스 공용 크레이트만 사용하는 순수 lib이라 플랫폼별 코드가 없다(3-OS 트레이는 D1 no-op DROP). 크로스플랫폼 재현 빌드는 U0가 전파하는 Edition 2024 + 고정 MSRV(1.85) 상속으로 보장.
- **어댑터 seam**: (i) **TrayBackend seam** — no-op 어댑터 vs 향후 실체 데스크톱 트레이(교체 지점, D1). (ii) **Clock/now-source seam** — `std::time::SystemTime` 기본, 테스트에서 결정적 clock 주입(타임스탬프 stamping의 테스트 가능성). (iii) **FileIo seam** — IO 오류 주입 어댑터로 REL-01 비차단(PROP-U6-BL-02) 속성 검증.

**(b) NFR/규칙 실현**: **U6-NFR-MNT-02**(PBT 제너레이터 재사용 + 로컬 노출, PBT-07) + **U6-NFR-MNT-03**(커버리지/문서 정책, PBT-10 + `#![deny(missing_docs)]` 소비). 포터빌리티는 requirements NFR-05 연계(신규 NFR 신설 없음). 비기본 feature 게이트로 `proptest` 가 프로덕션 빌드에 유출되지 않는다(U0 정합).

**(c) 구조 노트/불변식**: 불변식 — (1) 제너레이터 모듈은 `proptest-support` feature 뒤에 위치하고 기본 빌드에 `proptest` 의존이 나타나지 않는다. (2) 제너레이터는 문서화된 도메인 제약을 존중한다(`error_detail` `None`(성공)/유니코드·개행/빈 문자열(실패)·경계/대값 `bytes_transferred`). (3) business-critical 경로(히스토리 round-trip·health/닫힌집합 판정·rotation·config 검증)는 **PBT + 예제 기반 테스트를 병행**한다(PBT 단독 아님). (4) 커버리지 %-게이트는 설정하지 않으며 CI 통합은 Build-and-Test 이월. 어댑터 seam은 향후 트레이 실체화·결정적 테스트를 컴포넌트 교체만으로 가능케 한다(타 컴포넌트 불변).

**(d) 트레이드오프**: (1) 별도 testkit 크레이트(버전 관리 부담)나 각 소비 단위의 제너레이터 재작성(일관성 저하)은 배제 — 단일 출처(U0 + U6 각자 소유 타입) + 비기본 feature. (2) rotation IO 흐름은 PBT 대신 예제 기반 단위테스트(결정적 소수 케이스, `business-rules.md` §8.6). (3) 실제 트레이 백엔드는 미도입(seam만 노출) — GUI-less 데몬 MVP, 향후 confirm 시 교체.

---

## 11. MANDATORY 카테고리 N/A 판정표

| 카테고리 | 판정 | 근거 |
|---|---|---|
| **Scalability** | N/A | U6는 순수 lib(in-process 싱크 + 값 집계 + append-only 지속)로 자체 런타임·처리량 축이 없음. 단일 사용자 데스크톱 데몬 규모(사이클당 최대 1 히스토리 레코드, 저빈도 CLI 읽기). 히스토리 전체 스캔은 레코드 수에 선형·유계(§2/§6). 신규 확장성 패턴 없음. |
| **Availability** | N/A | lib 크레이트라 SLA 없음. 전체 Watcher는 단일 사용자·사용자 재시작 로컬 프로세스라 RTO/availability-SLA가 requirements RESILIENCY-02에서 이미 N/A 확정. |
| **Performance(수치)** | N/A(정성 계약만) | 로그 쓰기(라인 길이·rotation 임계 선형)·히스토리 query(레코드 수 선형)에 throughput/latency/peak-memory 수치 게이트 근거 없음(§5/§6). 사이클 병목은 U3 전송이지 관측이 아님. |
| **Resiliency DR/RTO/RPO** | N/A(신규 결정 없음) | DR/RTO/availability는 RESILIENCY-02에서 N/A 확정(단일 사용자 로컬). **RPO=0(zero-loss)은 U4 `SyncStateStore` 소관**이며, U6 히스토리는 **비권위 감사 로그**로 append 실패가 삼켜지고(§1) truncated-tail 관용(§2)으로 재구성. 배포/롤백은 U7a `AutoUpdater`(U6는 `update_probe` 판정만 제공, §3). |
| **Security(강제 통제)** | N/A(잔존만) | Security Baseline OFF, RISK-01 수용(로그/히스토리 로컬 평문). 암호화 저장·키관리·시크릿 스캐닝 신설 없음. 유일 in-scope 잔존 = 토큰 redaction 위생(§9), U0 `TokenSecret` 소비. |
| **Logical Components** | 다뤄짐(N/A 아님) | 공개 5 컴포넌트의 내부 논리 분해(명명·책임·의존·trait seam)는 자매 산출물 `logical-components.md`가 소유(추적성 문서 맵, 물리 모듈 증식 강제 아님). |

---

## 12. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 단계 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **준수 — blocking 없음** | PBT-09(프레임워크 = `proptest`)는 U0 상속으로 이미 충족. 이 단계 각 패턴이 확정 속성에 정렬: §1 -> PROP-U6-BL-02(비차단), §2 -> PROP-U6-DE-01·BR-08/09(round-trip·append-only·truncated-tail), §3 -> PROP-U6-BR-02/04/05(2축·health·update_probe 격리), §4 -> PROP-U6-BR-06/07·BL-01(닫힌집합·상태머신·fan-out), §5 -> PROP-U6-DE-02(로그 라인 round-trip)·PROP-U6-BR-01(레벨 필터), §10 -> 제너레이터(PBT-07). PBT-08(케이스/시드/CI)은 Code Generation/Build-and-Test 이월. |
| **Resiliency Baseline** | ON | **준수(부분 적용 + 일부 N/A) — blocking 없음** | RESILIENCY-05(구조화 로깅) = §5, RESILIENCY-06(status/health) = §3, RESILIENCY-15(사고 표면화) = §4 직접 구현. best-effort infallible 비차단(§1)으로 관측 실패가 코어를 막지 않음. append-only + truncated-tail 관용(§2)으로 크래시 후 재기동 query 성공. `update_probe` 격리(§3)가 RESILIENCY-04 자동 롤백 게이트 안정성 근거. RTO/RPO 수치·DR·HA·배포/롤백은 순수 lib·비권위 감사 로그에 N/A(RESILIENCY-02) — RPO=0은 U4 소관. 신규 U6 resiliency 결정 없음. |
| **Security Baseline** | OFF | **N/A — 미로딩·미강제** | 확장 OFF, RISK-01 수용(로그/히스토리 로컬 평문). 강제 통제 신설 없음. 유일 잔존 위생 = 토큰 redaction 소비 계약(§9, U0 `TokenSecret`/U0-NFR-SEC-02 소비) — 확장을 켜는 것이 아니라 이미 채택된 계약 경계 준수. |

**블로킹 판정**: 이 단계에 blocking finding 없음. PBT-09는 U0에서 충족, Resiliency는 부분 적용 + 일부 N/A, Security Baseline은 OFF로 N/A다.

---

## 13. 추적표 (패턴 -> NFR ID -> 규칙 ID -> 논리 컴포넌트)

| 설계 패턴 | NFR ID | 규칙 ID | 안착 논리 컴포넌트(`logical-components.md`) |
|---|---|---|---|
| §1 관측 push best-effort·infallible 비차단 | U6-NFR-REL-01 | R-PUSH-02/03, R-LOG-03, R-HIST-03 | 전 실체 싱크 공통(LogWriter · StatusAggregator · HistoryAppender · CriticalRouter) |
| §2 히스토리 append-only 무손실 + 프레이밍 + truncated-tail | U6-NFR-REL-02, U6-NFR-PERF-02 | R-HIST-01/02/04/05 | `UploadHistoryStore` -> HistoryFramer · HistoryAppender · HistoryScanner · HistoryQueryFilter |
| §3 `StatusService` 단일 집계 락 + update_probe 격리 | U6-NFR-REL-03 | R-STATUS-01/02/03/04/05/06 | `StatusService` -> StatusAggregator · StatusSnapshotDeriver · HealthOracle · LivenessProbe |
| §4 중대오류 닫힌 4조건 3중 fan-out + 상태머신 | U6-NFR-REL-04 | R-CRIT-01/02/03/04 | `CriticalErrorNotifier` -> CriticalRouter · FailureEscalator |
| §5 로그 동기 직렬화 + rotation + JSON-line 최소 필드 | U6-NFR-PERF-01, U6-NFR-USE-01 | R-LOG-01/02/03/05 | `StructuredLogger` -> LogEmitter · LogWriter · LogRotator |
| §6 히스토리 query 순차 스캔 선형 필터 | U6-NFR-PERF-02 | R-HIST-05 | `UploadHistoryStore` -> HistoryScanner · HistoryQueryFilter |
| §7 리로드 재적용 keep-last-good + federated 주입 | U6-NFR-REL-05 | R-LOG-06, R-CFG-U6-01/02 | `StructuredLogger` -> LogReloadAdapter; ObservabilityConfig 주입 seam |
| §8 오류 taxonomy 파생 | U6-NFR-MNT-01 | (U0 오류 계약 소비) | ErrorTypes(`LogError`/`HistoryError`) · 지속 값 타입 |
| §9 토큰 redaction 잔존 통제 소비 | U6-NFR-SEC-01 | R-LOG-04, R-HIST-06 | LogEmitter(redaction 준수) · HistoryFramer |
| §10 PBT 제너레이터·포터빌리티·어댑터 seam | U6-NFR-MNT-02/03 | (제너레이터/공개 계약) | ProptestGenerators(test-support) · TrayBackend/Clock/FileIo seam |

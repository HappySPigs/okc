# U6 Observability — Tech Stack Decisions (기술 스택 결정)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U6 Observability** -> NFR Requirements -> 산출물 2/2 (`tech-stack-decisions.md`)
**작성일**: 2026-09-08
**크레이트**: `observability` (lib) · **소속 컴포넌트**: `StructuredLogger`, `StatusService`, `UploadHistoryStore`, `CriticalErrorNotifier`, `TrayIndicator`(선택 no-op)
**입력 아티팩트**: `functional-design/{domain-entities,business-rules,business-logic-model}.md`(U6 Functional Design) · `requirements.md`(§3.5 / §5 NFR / §12·§13 오버레이) · 활성 확장 `property-based-testing.md`(PBT-09)·`resiliency-baseline.md` · `u0-foundation/nfr-requirements/{nfr-requirements.md,tech-stack-decisions.md}`(상속 스택 정본)
**규칙**: `construction/nfr-requirements.md` Step 6 · `common/content-validation.md`(Unicode 박스 문자 미사용, 화살표 표기 `A -> B`, 표/코드블록 검증)

> **문서 성격**: 이 문서는 U6의 **구체 크레이트/툴체인 선택을 기록하는 유일한 산출물**이다. 자매 산출물 `nfr-requirements.md`가 카테고리별 NFR(기술중립)을 정의하고, 이 문서는 그 NFR을 실현할 **구체 기술 결정**과 각 결정의 **근거·전파 범위**를 확정한다. English 식별자(크레이트명·타입명·config 키·rule/NFR ID)는 원문 그대로 유지하고, Rust 제네릭/타입은 백틱으로 감싼다(예: `Mutex<StatusInner>`).
>
> **AUTOPILOT**: 사용자 위임(게이트 waived)에 따라 각 미결점을 RECOMMENDED·MVP 바이어스로 자체 확정하고, MVP 트림은 §11·아래 Autopilot Decisions 표에 명시한다. 이미 확정된 항목(U0 스택, FD D1..D14)은 재오픈하지 않는다.

---

## 0. 전파 원칙 (U6 = Wave DAG 소비 단위, U0 상속)

U6 `observability`는 U0 `foundation`에만 의존하는 순수 lib이다(R-PUSH-01, U1~U5 역참조 없음). 따라서 **언어·edition·MSRV·공용 크레이트·PBT 프레임워크는 U0가 확정한 워크스페이스 전역 기본값을 상속(inherit)** 하고 U6는 신규 워크스페이스 결정을 만들지 않는다. U6가 이 문서에서 확정하는 것은 **(a) 상속 크레이트의 U6 내 사용 방식**, (b) U6 고유의 동시성/파일 I/O 프리미티브 선택(모두 `std`), (c) U6의 `proptest-support` feature 노출(U0 mirror)뿐이다.

- **전파 표기 규약**: 각 결정 절 말미에 "전파 범위"를 명시한다(`U6 내부 한정` / `U0 상속` / `특정 소비 단위`).
- **버전 정책**: U6는 내부 크레이트(`publish=false` + path 의존 + `[workspace.package]` 상속)이며 외부 크레이트는 전부 U0가 `[workspace.dependencies]`에 핀한 것을 `crate.workspace = true` 로 물려받는다(드리프트 없음). **U6는 신규 외부 크레이트를 추가하지 않는다**(§10).
- **N/A 카테고리(참고, 결정 없음)**: 확장성/가용성/성능 수치 목표/DR·RTO·RPO/로그 라인 외 사용성/보안 강제 통제는 U6에서 N/A로 확정(근거는 `nfr-requirements.md` §7). 이 문서는 이들에 대한 기술 선택을 만들지 않는다.
- **federated config 주입**: 비코어 config(`log_file`/`log_max_size_bytes`/`log_max_retained`/`history_file`/`tray_enabled`)는 U8 조립 루트가 원시 config를 파싱·해소해 **RESOLVED TYPED 값**(`PathBuf`/`ByteCount`/`usize`/`bool`)으로 U6 컴포넌트 **생성자에 하향 주입**한다. U6는 이 값을 `ConfigProvider` 로 읽지 않는다(U0 FROZEN). U0 코어 필드(`log_level`/`notify_consecutive_failures`)만 `ConfigProvider.current()` 로 읽고 live-reload는 코어 필드 fan-out으로 한정한다(`nfr-requirements.md` §2.5).

---

## Autopilot Decisions (미결점 자체 확정 — MVP 바이어스)

| topic | chosen | MVP-trim? | rationale |
|---|---|---|---|
| 로그 프레임워크 | `serde_json` 수제 JSON-line writer(`tracing`/`tracing-subscriber` 미채택) | 예 | U6는 append-only JSON-line 1건 방출만 필요. `tracing` 생태계(subscriber·appender·span)는 데몬 규모에 과대·신규 의존. `serde_json`(U0 핀 상속)으로 최소 실현. |
| 로그 쓰기 동시성 | `std::sync::Mutex` 하 동기 직렬화 best-effort 쓰기(비동기 스레드·bounded 채널 없음) | 예 | D3 정합. 비차단 속성은 best-effort·infallible 삼킴(REL-01)으로 실현. 비동기 offload + drop-on-full 큐는 이월. |
| `StatusService` 동시성 프리미티브 | `std::sync::Mutex<StatusInner>` 단일 락(`snapshot` 은 락 하 복제) | 예(RwLock 이월) | `Send`+`Sync`(U0 트레이트) 충족 최소 수단. CLI 읽기 저빈도라 `RwLock`/락-프리 불필요. |
| `CriticalErrorNotifier` 카운터 | `Mutex` 내부 상태(`consecutive_failure_count`) 또는 `AtomicUsize` 최소 | 아니오 | 단일 카운터 + escalation 플래그. std 프리미티브로 충분, 신규 의존 0. |
| 히스토리 지속 포맷 | U0 `ciborium` 코덱 `encode`/`decode` + 길이 프레이밍(len 헤더 + CBOR 프레임) + `std::fs` append | 아니오 | R-HIST-02/NFR-13 무손실. 코덱은 U0 소유, U6는 프레임 I/O만. 임베디드 DB 미도입. |
| 히스토리 길이 프레이밍 | 고정폭 length prefix(`u32`/`u64` LE) + CBOR payload | 아니오 | truncated-tail 판정(트레일링 부분 프레임)을 단순·결정적으로. 별도 크레이트 불요. |
| 히스토리 query 실행 | 전체 스캔 + 선형 AND 필터(인덱스/DB 없음) | 예 | D8 트림. 단일 사용자 규모 충분. |
| 로그 rotation | `std::fs` metadata len 확인 + rename + 오래된 초과분 삭제(개수 보존) | 예 | 나이/기간 기반·`logrotate`/`tracing-appender` 미도입(D2). 크기+개수 근사. |
| 타임스탬프 소스 | U0 `Timestamp`(방출 시 로거 stamping; now-source = `std::time::SystemTime`) | 아니오 | D4. `chrono`/`time` 크레이트 미도입 — U0 `Timestamp` 값 타입 재사용. |
| 트레이 백엔드 | no-op(`start() -> Ok(None)`, 나머지 무연산); 트레이 크레이트 미도입 | 예 | D1. 3-OS 트레이 백엔드 DROP. 향후 confirm 시 교체(타 컴포넌트 불변). |
| 오류 타입 파생 | `LogError`/`HistoryError` = `thiserror`; `UploadHistoryRecord`(U0 FROZEN 3필드) = 순수 serde | 아니오 | U0 Q4=A 관례 상속. push 표면 infallible이라 오류는 `query`/`reload`만 반환. |
| PBT 프레임워크/제너레이터 | `proptest`(U0 상속) + U6 `proptest-support` 비기본 feature(U0 mirror) | 아니오 | PBT-09 U0 확정 상속. PBT-07 제너레이터는 U0 재사용 + U6 로컬 노출. |
| 성능 수치 게이트 | 없음(정성 계약만) | 아니오 | U0 패턴. 로그/query 선형·유계, 데스크톱 규모 수치 근거 없음. |
| 신규 워크스페이스 의존성 | 없음(전부 U0 핀 상속) | 예 | serde_json/ciborium/serde/thiserror/proptest 모두 U0가 핀. U6 신규 크레이트 0. |

---

## 1. 언어 / 툴체인 (U0 상속, NFR-17/NFR-05)

U6는 신규 언어/edition/MSRV 결정을 만들지 않고 U0가 확정한 워크스페이스 전역값을 상속한다:

| 항목 | 결정 | 근거 |
|---|---|---|
| 언어 | **Rust** (U0 상속) | NFR-17 확정. |
| Edition | **2024** (`edition.workspace = true`) | U0 §1. Rust 1.85+ 안정. |
| MSRV | **고정(pinned)** (`rust-version.workspace = true`) | U0 §1. NFR-05 크로스플랫폼 재현 빌드. Edition 2024가 MSRV 1.85 하한 강제. |

U6는 `std`(`std::fs`·`std::sync`·`std::time`·`std::io`)만으로 플랫폼 독립 lib을 구성하므로 플랫폼별 코드가 없다(3-OS 트레이는 D1 no-op DROP). MSRV 대비 CI 검증·patch 핀은 Build-and-Test 이월(U0 §1 정합).

**전파 범위**: **U0 상속**(U6는 재결정 없음).

---

## 2. 직렬화 — 로그(JSON) / 히스토리(CBOR) 이원 경로

U6는 U0의 이원 직렬화 원칙(사람 편집 = JSON, 내부 지속 = CBOR)을 두 개의 서로 다른 관측 경로에 적용한다. **두 크레이트 모두 U0가 이미 핀했으며 U6는 신규 추가 없음.**

### 2.1 로그 방출 = JSON-line via `serde_json` (U0 핀 상속)

- **결정**: `StructuredLogger` 방출 라인을 **`serde_json`** 으로 JSON-line(개행 구분 1건)으로 직렬화한다. 스키마 = `{ timestamp, level, event, cycle_id, message, fields }`(최소 필드 계약, `nfr-requirements.md` §6.1). `timestamp` 는 방출 시점 로거가 U0 `Timestamp` 로 stamping(D4).
- **근거**: `business-rules.md` §2 R-LOG-01(JSON-line) · `domain-entities.md` §1.1(방출 라인 스키마) · requirements.md §3.5 FR-17. `serde_json` 은 U0 §2.2에서 워크스페이스 핀(`1`).
- **`tracing` 미채택 근거**: `tracing`/`tracing-subscriber`/`tracing-appender` 생태계는 span·subscriber·layer 추상을 도입해 데몬 규모(append-only 라인 1건 방출)에 과대하고 신규 의존을 추가한다. 최소 요구는 `serde_json::to_string(&EmittedLogLine)` + 개행 append로 충족되므로 수제 writer를 채택한다(MVP 트림).
- **전파 범위**: **U6 내부 한정**(로그 방출). `serde_json` 의존은 U0 상속.

### 2.2 히스토리 지속 = U0 `ciborium` 코덱 + 길이 프레이밍 (U0 핀 상속)

- **결정**: `UploadHistoryRecord` 를 U0 CBOR 코덱(`encode`/`decode`, 백엔드 `ciborium`)으로 인코딩하고 **고정폭 length prefix(예: `u32`/`u64` LE) + CBOR payload** 프레임으로 append-only 파일에 `std::fs` append한다. `query` 는 프레임을 순차 디코드하며 트레일링 부분 프레임을 truncated tail로 관용한다(R-HIST-04).
- **근거**: `business-rules.md` §5 R-HIST-02(무손실 프레임)·R-HIST-04(truncated-tail) · `business-logic-model.md` §3.1(길이 프레이밍 append) · `domain-entities.md` §2.1(round-trip 대상) · requirements.md §5.5 NFR-13. `ciborium` 은 U0 §2.1에서 워크스페이스 핀(`0.2`)이며 전파 대상에 **U6** 명시.
- **채택 근거**: (a) U0 코덱 재사용으로 무손실 round-trip(U0-NFR-REL-01) 계약을 상속, (b) 자기기술적 CBOR라 자동 업데이트 후 히스토리 스키마 진화(새 필드)에 강함, (c) 고정폭 length prefix가 truncated-tail 판정을 단순·결정적으로 만들어 별도 프레이밍 크레이트가 불필요.
- **임베디드 DB/인덱스 미채택 근거(D8)**: `sqlite`/`sled` 등은 append-only 감사 로그(사이클당 최대 1레코드) 규모에 과대하고 신규 의존·마이그레이션 부담을 추가한다. 전체 스캔 + 선형 필터로 충분(`nfr-requirements.md` §3.2).
- **전파 범위**: **U6 내부 한정**(히스토리 파일 I/O). 코덱 정의는 U0 소유. `ciborium` 의존은 U0 상속.

---

## 3. 파일 I/O 및 rotation = `std::fs` (신규 의존 없음)

- **결정**: 로그 파일 append/rotation과 히스토리 파일 append/scan은 모두 **`std::fs`/`std::io`** 로 수행한다.
  - **로그 append**: `OpenOptions::new().append(true)` 로 방출 라인 append.
  - **rotation(R-LOG-05)**: append 후 `metadata().len()` 이 주입된 `log_max_size_bytes` 초과 시 `rename` 으로 회전하고, 회전 파일 수가 `log_max_retained` 초과 시 가장 오래된 것을 삭제(개수 보존). **나이/기간 기반 보존은 미구현**(D2 트림, 개수 근사).
  - **히스토리 append/scan**: append-only 파일에 프레임 append, `query` 시 순차 read + 프레임 디코드.
- **근거**: `business-rules.md` §2 R-LOG-05(크기 rotation·개수 보존)·§5 R-HIST-01(append-only) · `business-logic-model.md` §1.1/§3. `logrotate`/`tracing-appender`/외부 rotation 크레이트는 미채택(std로 충분, MVP).
- **전파 범위**: **U6 내부 한정**. 파일 경로(`log_file`/`history_file`)는 U8이 `PathBuf` 로 생성자 주입(부재 시 U8이 플랫폼 기본 경로 해소; §0 federated 주입).

---

## 4. 동시성 프리미티브 = `std::sync::Mutex` (신규 의존 없음)

U6 싱크는 U0 트레이트 계약상 `Send`+`Sync` 이며(`Arc<dyn Logger>` 등으로 다중 스레드에서 공유·주입), 동시 push와 동시 read가 공존한다(`business-logic-model.md` §2.1). FD가 스레딩 메커니즘을 NFR 단계로 이월했다.

### 4.1 `StatusService` = `Mutex<StatusInner>` (Q6-style 최소 락)

- **결정**: 2축 상태(operational 단일값 + conditions 집합 + last_success/dirty/resume/liveness + 연속실패 escalation 플래그)를 **`std::sync::Mutex<StatusInner>`** 단일 락으로 보호한다. `snapshot()` 은 락 하에서 완결 값을 복제해 반환하고, `health_check()`/`update_probe()` 도 락 하 파생이라 부분 갱신을 노출하지 않는다(원자 관측, `nfr-requirements.md` §2.3).
- **후보 대비**: `RwLock<StatusInner>` 는 다중 동시 읽기에 유리하나 CLI status 읽기가 **저빈도**(사이클 지배적 아님)라 이점이 없고 writer starvation 여지를 추가한다. 락-프리(per-field atomics + 스냅샷 조립)는 2축 집합 상태(가변 길이 `conditions`)에 부적합·과복잡. **단일 `Mutex` 가 최소 코드로 원자 관측을 실현**(MVP 트림, `RwLock` 이월).
- **근거**: `business-rules.md` §3 R-STATUS-01/02 · `business-logic-model.md` §2.1(단일 집계 지점, 동시 push/read) · `domain-entities.md` §3.4(escalation 플래그 U6-내부 상태).
- **전파 범위**: **U6 내부 한정**.

### 4.2 `StructuredLogger` / `UploadHistoryStore` writer = `Mutex` 직렬화

- **결정**: 로그 파일 writer 상태(파일 핸들 + 현재 크기 + rotation 파라미터)와 히스토리 파일 상태를 각각 **`std::sync::Mutex`** 로 감싸 동시 방출/append를 직렬화한다. 로그 방출은 동기 best-effort(D3) — 락은 단일 소형 라인 쓰기 시간만 보유하고, IO 오류는 삼켜 sync-path를 블로킹하지 않는다(REL-01). **비동기 로깅 스레드 + bounded 채널 + drop-on-full 백프레셔는 MVP 미구현**(D3 트림).
- **근거**: `business-rules.md` §2 R-LOG-03(동기 best-effort, 비동기 MVP 미구현)·§5 R-HIST-03(append infallible) · `nfr-requirements.md` §3.1.
- **전파 범위**: **U6 내부 한정**.

### 4.3 `CriticalErrorNotifier` 카운터

- **결정**: `consecutive_failure_count`(비음 정수)와 escalation 발화 상태를 `Mutex` 내부 상태 또는 `AtomicUsize` + 플래그로 최소 관리한다. 임계 `N`(주입된 `notify_consecutive_failures`, U0 코어 config, 기본 3) 도달 순간 1회 발화·Success 시 리셋(R-CRIT-02). 신규 의존 0.
- **전파 범위**: **U6 내부 한정**.

---

## 5. 오류 처리 (U0 관례 상속, Q4=A)

U6 오류 타입을 U0 워크스페이스 관례에 맞춰 두 부류로 파생한다:

| 부류 | 타입 | 파생 전략 | 근거 |
|---|---|---|---|
| **운영 오류(반환용, 읽기/재적용 경로)** | `LogError`(`Io`/`InvalidPath`/`RotationFailed`) · `HistoryError`(`Io`/`Serde`/`Corrupt`) | **`thiserror`** 파생 | `query`/`reload` 만 반환(push 표면은 infallible로 삼킴, REL-01). U0 §5 관례 상속. |
| **지속 값 타입(직렬화용)** | `UploadHistoryRecord`(U0 FROZEN 3필드) | **순수 serde struct 유지** | U0 CBOR round-trip 대상(R-HIST-02/NFR-13). throw 아님. |

- **`anyhow` 미사용**: U0가 이미 거부(구조화 변이 소실). U6도 동일.
- **push infallible 계약**: `Logger::log`/`event`·`StatusSink` 뮤테이터·`HistorySink::append`·`CriticalEventSink` report는 오류를 반환하지 않고 내부 삼킴(U0 트레이트 계약, R-PUSH-02) — `LogError`/`HistoryError` 는 이 표면에 전파되지 않는다.
- **근거**: `domain-entities.md` §1.3/§2.3 · U0 `tech-stack-decisions.md` §5. `thiserror` 는 U0 핀(`2`) 상속.
- **전파 범위**: **U6 내부 한정**(타입 정의). 관례는 U0 상속.

---

## 6. 트레이 = no-op (트레이 크레이트 미채택, D1)

- **결정**: `TrayIndicator` 는 nullable **no-op 싱크**로 `start() -> Ok(None)`, `render`/`notify`/`stop` 무연산이다. **실제 데스크톱 트레이 백엔드(3-OS)와 관련 크레이트(`tray-icon`/`ksni`/`systray` 등)를 도입하지 않는다.**
- **근거**: `business-rules.md` §6 R-TRAY-01/02 · `domain-entities.md` §5 · requirements.md §3.5 FR-18. GUI-less 데몬 모델(사용자 메모리: nginx식 daemon, tray optional)과 정합.
- **비의존 보증**: 트레이 no-op는 로그·status·health·history 표면화를 막지 않는다(REL-04, 3중 fan-out 중 로그 + status는 트레이 비의존 항상 성립). `tray_enabled` config는 U8이 파싱만 하고(호환) 동작 무영향.
- **향후 교체**: confirm 시 no-op을 실체 구현으로 교체(다른 컴포넌트 불변).
- **전파 범위**: **U6 내부 한정**.

---

## 7. 속성 기반 테스트 = `proptest` 상속 + U6 `proptest-support` feature (PBT-09/PBT-07)

> **PBT-09(프레임워크)는 U0에서 `proptest` 로 이미 확정**되어 워크스페이스 상속되므로 U6는 재결정하지 않는다. U6가 확정하는 것은 PBT-07(제너레이터 재사용 + U6 로컬 노출)의 실현 방식이다.

### 7.1 프레임워크 = `proptest` (U0 상속, Q9=A)

- **결정**: 워크스페이스 PBT 프레임워크 `proptest`(U0 핀 `1`, dev-dependency)를 U6 dev에서 그대로 사용한다. `quickcheck` 재검토 없음(U0에서 배제 확정).
- **근거**: U0 `tech-stack-decisions.md` §7.1 · property-based-testing.md PBT-09 · requirements.md §5.5.

### 7.2 제너레이터 노출 = U6 `proptest-support` 비기본 cargo feature (U0 mirror, PBT-07)

- **결정**: `observability` 에 **비기본(non-default) cargo feature `proptest-support`** 를 두고 U6 고유 타입 제너레이터를 노출한다(U0 패턴 mirror):
  - `UploadHistoryRecord` 제너레이터(3필드: `error_detail` `None`(성공)/유니코드·개행/빈 문자열(실패)·0/경계/대값 `bytes_transferred`·임의 `timestamp`).
  - `LogRecord` 방출 라인 제너레이터(전 `LogLevel`·`cycle_id` 유무·유니코드/개행/빈 `message`·다수/빈 `fields`).
  - 상태 명령 시퀀스 제너레이터(`set_operational`/`raise_condition`/`clear_condition`/... 인터리빙, 중복 raise/clear 포함).
  - `report_*` 시퀀스 제너레이터(4 case 혼합, Success/Failure 시퀀스 + 임의 `N`) + IO 오류 주입 어댑터.
  - **U0 재사용**: `Timestamp`·`ManifestDigest`·`LogLevel`·`ActiveCondition`/`OperationalState`/`LivenessSignal`·자유형 `detail` 등 U0 소유 타입 제너레이터는 U0 `proptest-support` 를 dev에서 켜 재사용(재작성 금지).
- **드리프트 방지 / feature 게이트**: 제너레이터 단일 출처(U0 + U6 각자 소유 타입) + 비기본 feature로 `proptest` 가 프로덕션 빌드에 유출되지 않음(U0 §7.2 정합).
- **소비 단위**: U6 자체 테스트(코덱 round-trip·health 오라클·닫힌집합·append-only) + Code Generation 단계. U7b가 query 소비 테스트에서 활용 가능.
- **근거**: `domain-entities.md` §8·`business-rules.md` §8·`business-logic-model.md` §6(제너레이터 총괄) · property-based-testing.md PBT-07 · U0 `tech-stack-decisions.md` §7.2.

### 7.3 이월 (PBT-08)

케이스 수·shrink 튜닝·고정 시드 vs 시드 로깅 정책·CI 파이프라인 통합은 **Code Generation / Build-and-Test 이월**(property-based-testing.md Enforcement Integration). 이 문서는 프레임워크 상속(PBT-09)과 제너레이터 노출(PBT-07)만 확정한다.

**전파 범위**: `proptest` 는 워크스페이스 공용 **dev-dependency**(U0 상속). `proptest-support` feature는 **U6 소유**(U6 고유 타입 제너레이터), U0 `proptest-support` 는 U6 dev에서 활성화해 U0 타입 재사용.

---

## 8. 타임스탬프 = U0 `Timestamp` (chrono/time 미채택, D4)

- **결정**: `StructuredLogger` 는 방출 시점에 **U0 `Timestamp`** 로 방출 시각을 stamping한다(D4). now-source는 `std::time::SystemTime`(UTC 결정적 표현으로 U0 `Timestamp` 구성). **`chrono`/`time` 등 신규 시간 크레이트를 도입하지 않는다** — U0 값 타입을 재사용한다.
- **근거**: `domain-entities.md` §1.1(방출 시 stamping)·§0(U0 `Timestamp` 소비) · `business-logic-model.md` §1.1. U0 `Timestamp` 는 CBOR round-trip 대상(U0-NFR-REL-01)이며 히스토리 레코드 `timestamp` 에도 재사용.
- **전파 범위**: **U6 내부 한정**(now-source stamping). `Timestamp` 정의는 U0 소유.

---

## 9. 문서 / 품질 린트 (U0 정합, Q13=A)

- **결정**: (a) 전역 커버리지 %-게이트 없음 — PBT(§7) + 예제 앵커(PBT-10: rotation 임계·query 필터·닫힌집합 판정 예제테스트)로 실질 검증. (b) `observability` 공개 표면에 rustdoc 주석 유지(U0 `#![deny(missing_docs)]` 계약 소비·정합).
- **근거**: property-based-testing.md PBT-10 · U0 `tech-stack-decisions.md` §9 · requirements.md §5.7 NFR-17. CI 커버리지 통합은 Build-and-Test 이월.
- **전파 범위**: **U6 내부 한정**(커버리지 정책은 U0 워크스페이스 기본 참고).

---

## 10. 의존성 요약표

**범례**: version-policy = 외부 크레이트는 U0 `[workspace.dependencies]` 핀을 `crate.workspace = true` 로 상속, 내부는 `[workspace.package]` 상속 + `publish=false` + path 의존. kind = normal(런타임) / dev(테스트). feature = 특기 사항. propagation = 전파 범위.

| crate | version-policy | kind | feature | U6 사용처 | grounding |
|---|---|---|---|---|---|
| `foundation`(U0) | 내부, path 의존 | normal | (U6 dev에서 `proptest-support`) | 값 타입·CBOR 코덱·싱크 트레이트·`Timestamp`·`TokenSecret`·오류 taxonomy·`ConfigProvider`(코어 필드) | R-PUSH-01(유일 의존) · `domain-entities.md` §0 |
| `serde` | workspace-inherited (핀: `1`) | normal | `derive` | `UploadHistoryRecord`(3필드)/`EmittedLogLine` 파생 | U0 §2 · R-HIST-02 |
| `serde_json` | workspace-inherited (핀: `1`) | normal | — | 로그 JSON-line 직렬화(§2.1) | Q1=B(로그=사람가독) · R-LOG-01 |
| `ciborium` | workspace-inherited (핀: `0.2`) | normal | — | 히스토리 무손실 CBOR 프레임(§2.2, U0 코덱 경유) | Q1=A · R-HIST-02 / NFR-13 |
| `thiserror` | workspace-inherited (핀: `2`) | normal | — | `LogError`/`HistoryError` 파생(§5) | Q4=A · U0 §5 |
| `proptest` | workspace-inherited (핀: `1`) | **dev** | `proptest-support`(비기본, U6 제너레이터 노출) | 코덱 round-trip·health 오라클·닫힌집합·append-only 속성(§7) | **PBT-09**(U0 상속)/PBT-07 |
| `std`(fs/sync/time/io) | 표준 라이브러리 | normal | — | 파일 append/rotation·`Mutex` 동시성·`SystemTime` stamping | §3·§4·§8 |

> **신규 크레이트 = 없음**: U6가 사용하는 모든 외부 크레이트(`serde`/`serde_json`/`ciborium`/`thiserror`/`proptest`)는 **U0가 이미 `[workspace.dependencies]`에 핀한 것**이며 U6는 이를 상속만 한다. U6는 **워크스페이스에 신규 외부 의존을 추가하지 않는다**. 나머지는 전부 `std`. 정확한 patch 핀과 MSRV(Edition 2024, Rust 1.85+) 정합 검증은 Build-and-Test 이월(U0 정합). `tracing`·`chrono`/`time`·임베디드 DB(`sqlite`/`sled`)·트레이 크레이트·rotation 크레이트는 **의도적 미채택**(§11).

---

## 11. 범위 절제 기록 (Scope Trims / Deferrals)

U6가 **의도적으로 도입하지 않은** 항목과 이월처:

| 절제 항목 | 결정 | 근거 / 이월처 |
|---|---|---|
| **`tracing`/`tracing-subscriber`/`tracing-appender`** | **미채택** | append-only JSON-line 1건 방출에 과대. `serde_json` 수제 writer로 충분(§2.1). |
| **비동기 로깅 스레드 + bounded 채널 + drop-on-full 백프레셔** | **이월(도입 안 함)** | D3. 동기 best-effort + infallible 삼킴(REL-01)이 비차단을 실현. 향후 확장. |
| **`StatusService` `RwLock`/락-프리** | **이월** | CLI 읽기 저빈도라 단일 `Mutex` 로 충분(§4.1). |
| **히스토리 임베디드 DB / 보조 인덱스** (`sqlite`/`sled`) | **미채택** | D8. 사이클당 1레코드 감사 로그에 과대. 전체 스캔 선형 필터로 충분(§2.2). |
| **로그 rotation 크레이트 / 나이·기간 기반 보존** | **미채택** | D2. `std::fs` 크기+개수 근사로 충분(§3). |
| **`chrono`/`time` 시간 크레이트** | **미채택** | U0 `Timestamp` + `std::time::SystemTime` 재사용(§8). |
| **트레이 크레이트** (`tray-icon`/`ksni` 등) | **미채택** | D1. 트레이 no-op DROP(§6). GUI-less 데몬 모델. |
| **신규 워크스페이스 외부 의존** | **0건** | 전부 U0 핀 상속(§10). |
| **PBT-08 상세**(케이스 수/shrink/시드/CI) | **이월** | Code Generation / Build-and-Test(§7.3). |
| **patch 핀 + MSRV CI 검증** | **이월** | U0 정합. Build-and-Test(§10). |
| **federated rotation 파라미터 live-reload** | **이월** | 재시작 반영(코어 필드만 fan-out reload, `nfr-requirements.md` §2.5). |

---

## 12. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 문서 적용 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제·준수 — PBT-09 상속 + PBT-07 노출** | §7이 `proptest`(U0 상속) + U6 `proptest-support` 비기본 feature(U0 mirror)로 제너레이터 노출 확정. FD 식별 속성을 `nfr-requirements.md` §2 수용 기준으로 요구화. PBT-08 상세는 Build-and-Test 이월. blocking 없음. |
| **Resiliency Baseline** | ON | **부분 적용 + 일부 N/A** | RESILIENCY-05(구조화 로깅) = `serde_json` JSON-line(§2.1), RESILIENCY-06(status/health) = `Mutex<StatusInner>`(§4.1), RESILIENCY-15(사고 표면화) = 3중 fan-out(트레이 비의존). 무손실 `ciborium` 프레임 + truncated-tail 관용(§2.2)이 크래시 후 재구성 실현. RTO/RPO 수치·DR·HA·배포/롤백은 U6(순수 lib·비권위 감사 로그)에 **N/A**(RESILIENCY-02) — RPO=0은 U4 소관. |
| **Security Baseline** | OFF | **N/A(잔존 표면화)** | 미로딩·미강제. RISK-01(로그/히스토리 로컬 평문) 수용. 유일 잔존 통제 = 토큰 redaction 위생 — U0 `TokenSecret`(U0 §8) 소비로 로그/히스토리에 평문 토큰 미방출(`nfr-requirements.md` §5.1). 암호화 저장·키관리·시크릿 스캐닝 신설 없음. |

**N/A NFR 카테고리(기술 선택 없음)**: 확장성 · 가용성 · 성능 수치 목표(로그/query 선형·유계 정성 계약만) · Resiliency DR/RTO/RPO · 로그 라인 외 사용성(트레이 no-op·CLI는 U7b) · 보안 강제 통제. 근거 상세는 자매 산출물 `nfr-requirements.md` §7.

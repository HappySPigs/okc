# U6 Observability — Logical Components (논리 컴포넌트 분해 + 추적성 맵)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U6 Observability** -> NFR Design -> 산출물 2/2 (`logical-components.md`)
**작성일**: 2026-09-08
**크레이트**: `observability` (lib) · **공개 컴포넌트(애그리게이트)**: `StructuredLogger`, `StatusService`, `UploadHistoryStore`, `CriticalErrorNotifier`, `TrayIndicator`(선택 no-op)
**입력 아티팩트**: 자매 산출물 `nfr-design/nfr-design-patterns.md`(§13 패턴 -> 논리 컴포넌트 안착 맵이 이 문서의 권위 근거) · `functional-design/{domain-entities,business-rules,business-logic-model}.md`(타입 §1~§5 · 규칙 · 흐름) · `inception/application-design/{components.md,component-methods.md,component-dependency.md}`(공개 5 컴포넌트 메서드 표면) · 활성 확장 `property-based-testing.md`(ON, Full)·`resiliency-baseline.md`(ON)
**규칙**: `construction/nfr-design.md` Step 6 · `common/content-validation.md`(ASCII 화살표 `A -> B`, 박스/유니코드-다이어그램 문자 미사용, Rust 제네릭 백틱)

> **문서 성격**: 이 문서는 Application Design이 확정한 **공개 5 컴포넌트**(4개 실체 싱크 + `TrayIndicator` no-op)를 상위 애그리게이트로 유지한 채, 그 내부의 **이미 확정된 기능(FD 타입·규칙·흐름)을 명명된 논리 컴포넌트로 전개**한다. 이는 발명이 아니라 확정 기능의 명명·정렬이며 FD 흐름에서 near-free로 파생된다. 각 논리 컴포넌트는 (a) 책임, (b) 공개/내부 인터페이스 표면, (c) 안착하는 확정 NFR 패턴(`nfr-design-patterns.md` §13 맵과 1:1)으로 기술한다.
>
> **이것은 추적성 문서 맵이며 물리 모듈/크레이트 증식 강제가 아니다.** 논리 컴포넌트 -> 물리 파일/모듈 레이아웃 매핑은 Code Generation에서 최소로 결정된다(다수 논리 컴포넌트가 한 모듈에 상주 가능). 목적은 NFR·규칙·PBT 속성 -> 명명 단위 추적성 극대화다.
>
> **의존 경계**: `observability` 크레이트의 `[dependencies]` 는 **`foundation`(U0) 만**이다(R-PUSH-01, U1~U5 역참조 없음). trait seam 예시로 흔히 거론되는 `HttpTransport`(U3)·watch-backend(U2)는 **U6 관심사가 아니다** — U6의 trait seam은 (i) U0 SinkContracts 구현/소비, (ii) TrayBackend, (iii) Clock/now-source, (iv) FileIo다(§4).
>
> **표기 규약**: 화살표는 ASCII `A -> B`만 사용(유니코드 화살표 금지). 박스/선-그리기/유니코드 수학 기호 미사용. Rust 제네릭/타입/식별자(예: `Arc<dyn Logger>`, `Mutex<StatusInner>`, `PathBuf`, `Vec<u8>`)는 백틱으로 감싼다.

---

## 1. 논리 컴포넌트 전개 개요

| 공개 컴포넌트(애그리게이트) | 구현/소비 U0 트레이트 | 논리 컴포넌트(확정 명명) |
|---|---|---|
| `StructuredLogger` | impl `Logger` + `ConfigReloadObserver` | LogEmitter · LogWriter · LogRotator · LogReloadAdapter |
| `StatusService` | impl `StatusSink` + `ReadJudgment` | StatusAggregator · StatusSnapshotDeriver · HealthOracle · LivenessProbe |
| `UploadHistoryStore` | impl `HistorySink` | HistoryFramer · HistoryAppender · HistoryScanner · HistoryQueryFilter |
| `CriticalErrorNotifier` | impl `CriticalEventSink` | CriticalRouter · FailureEscalator |
| `TrayIndicator`(no-op) | (U6 고유 계약) | NoOpTrayBackend |
| (횡단 값/타입) | — | ErrorTypes(`LogError`/`HistoryError`) · ObservabilityConfig(federated 주입 뷰) |
| (런타임 그래프 밖) | test-support(비-default `proptest-support` feature) | ProptestGenerators (§5.2) |

---

## 2. 공개 컴포넌트별 논리 컴포넌트

> 4개 실체 싱크는 U0 트레이트를 구현하고 **U8 조립 루트가 생성·하향 주입**한다. `StructuredLogger`/`StatusSink` 는 U1~U5/U8에, `ReadJudgment` 는 U7a/U7b에, `HistorySink` 는 U8 코디네이터에, `CriticalEventSink` 는 U4/U5/U7a/U8에 주입된다. federated 비코어 config 값은 각 컴포넌트 생성자에 RESOLVED TYPED로 주입된다(`nfr-design-patterns.md` §0).

### 2.1 `StructuredLogger` (impl `Logger` + `ConfigReloadObserver`)

**LogEmitter**
- **책임**: `log(record)`/`event(...)` 진입 -> `LogRecord` 조립 -> 레벨 필터(`level >= 활성 log_level`, R-LOG-02) -> 방출 시각 stamping(U0 `Timestamp`, Clock seam) -> redaction 확인(토큰 원문 배제, R-LOG-04) -> `serde_json` JSON-line 직렬화. push 표면은 infallible(오류 삼킴).
- **인터페이스 표면**: `fn log(&self, record: LogRecord)` · `fn event(&self, level, event, cycle_id, fields)`. 방출 스키마 `{ timestamp, level, event, cycle_id, message, fields }`(최소 필드 계약).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §5(**U6-NFR-PERF-01/USE-01**, R-LOG-01/02) + §9(**U6-NFR-SEC-01**, R-LOG-04 redaction 준수) + §1(**REL-01** infallible 삼킴).

**LogWriter**
- **책임**: 파일 핸들 + 현재 크기 상태를 `std::sync::Mutex` 로 감싸 방출 라인을 **동기 직렬화 스트리밍 append**(`OpenOptions::append(true)`). IO 오류는 `LogError` 로 내부 분류되나 삼킨다(비차단).
- **인터페이스 표면**: 내부 — `Mutex<LogWriterState>` 하 라인 append. 공개 표면 없음(LogEmitter가 소비).
- **안착 NFR 패턴**: §5(**U6-NFR-PERF-01**, R-LOG-03 동기 best-effort, `Mutex` 직렬화) + §1(**REL-01**).

**LogRotator**
- **책임**: append 후 `metadata().len()` 이 주입된 `log_max_size_bytes` 초과 시 `rename` 회전 + 회전 파일 수가 `log_max_retained` 초과 시 가장 오래된 것 삭제(개수 보존, R-LOG-05). 나이/기간 기반 미구현(D2 트림).
- **인터페이스 표면**: 내부 — LogWriter 경로에서 호출. rotation 실패도 `LogError` 삼킴(비차단).
- **안착 NFR 패턴**: §5(**U6-NFR-PERF-01**, R-LOG-05 크기+개수 rotation).

**LogReloadAdapter**
- **책임**: U0 `ConfigReloadObserver` 구현 — `on_config_reload()` 통지 시 `ConfigProvider.current()` 재조회로 `log_level`(U0 코어 필드) 재적용. 재적용 실패는 `reload() -> Err(LogError)` 로 표면화하되 keep-last-good. federated rotation 파라미터는 생성자 주입 값 유지(live-reload 없음).
- **인터페이스 표면**: `impl ConfigReloadObserver { fn on_config_reload(&self) }` · `fn reload(&self, cfg) -> Result<(), LogError>`.
- **안착 NFR 패턴**: §7(**U6-NFR-REL-05**, R-LOG-06 keep-last-good; §0 federated live-reload 범위 한정).

### 2.2 `StatusService` (impl `StatusSink` + `ReadJudgment`)

**StatusAggregator**
- **책임**: 2축 상태(`operational` + `conditions` 집합 + `last_success`/`dirty`/`resume`/liveness 신호 + 연속실패 escalation 플래그)를 `std::sync::Mutex<StatusInner>` 단일 락으로 보호하며 전 push 뮤테이터를 락 하 in-memory 갱신(멱등, R-STATUS-02). 단일 집계 지점.
- **인터페이스 표면**: `fn set_operational` · `raise_condition` · `clear_condition` · `record_sync_success` · `set_dirty` · `set_resume_progress` · `set_liveness`(전부 infallible) + escalation 입력(고유 메서드, `CriticalErrorNotifier` 가 push).
- **안착 NFR 패턴**: §3(**U6-NFR-REL-03**, `Mutex<StatusInner>`; R-STATUS-01/02/06) + §1(**REL-01**).

**StatusSnapshotDeriver**
- **책임**: `snapshot()` 이 락 하에서 완결 값을 복제해 `StatusSnapshot` 도출(파생: `offline = operational==Offline`; `consent` = R-STATUS-03 규칙). 부분 갱신 미노출(원자 관측).
- **인터페이스 표면**: `fn snapshot(&self) -> StatusSnapshot`(in-memory, infallible).
- **안착 NFR 패턴**: §3(**U6-NFR-REL-03**, R-STATUS-03 파생 결정성).

**HealthOracle**
- **책임**: `health_check()` -> `Health`. `Unhealthy` iff health-critical 조건(`AuthFailed`/`OverLimit`/`UpdateRolledBack`) 활성 또는 연속실패 escalation 활성(닫힌 4조건). 비목록 조건(`ConsentBlocked`/`Offline`/`dirty`/`VaultUnavailable`)은 flip 없음.
- **인터페이스 표면**: `fn health_check(&self) -> Health`(종료코드 매핑은 U7b 소관).
- **안착 NFR 패턴**: §3(**U6-NFR-REL-03**, R-STATUS-04 health 오라클).

**LivenessProbe**
- **책임**: `update_probe()` -> `Liveness`. `Alive` iff `{IdleReached, CredentialReadable}` 두 신호 모두 관측. **운영 조건/상태와 완전 분리**(격리) — 롤백 루프 방지, U7a 전용.
- **인터페이스 표면**: `fn update_probe(&self) -> Liveness`.
- **안착 NFR 패턴**: §3(**U6-NFR-REL-03** AC-3, R-STATUS-05 격리; RESILIENCY-04 자동 롤백 게이트 근거).

### 2.3 `UploadHistoryStore` (impl `HistorySink`)

**HistoryFramer**
- **책임**: U0 CBOR 코덱(`encode`/`decode`) + **고정폭 length prefix(`u64` LE) + CBOR payload** 프레이밍. 무손실 round-trip 대상(R-HIST-02/NFR-13). 코덱은 U0 소유, U6는 프레임 I/O만.
- **인터페이스 표면**: 내부 — 레코드 <-> 프레임 인코딩/디코딩.
- **안착 NFR 패턴**: §2(**U6-NFR-REL-02**, R-HIST-02) + §9(**U6-NFR-SEC-01**, `error_detail` 평문 RISK-01 문서화).

**HistoryAppender**
- **책임**: append-only 파일에 프레임 스트리밍 append(`std::fs`, 수정/삭제 없음, R-HIST-01). `HistorySink::append` 는 infallible — IO/코덱 오류 삼키고 로거로 남김(R-HIST-03).
- **인터페이스 표면**: `fn append(&self, record: UploadHistoryRecord)`(infallible push).
- **안착 NFR 패턴**: §2(**U6-NFR-REL-02**, R-HIST-01) + §1(**REL-01**).

**HistoryScanner**
- **책임**: `query` 시 파일 프레임 순차 디코드(전체 미적재 스트리밍). 트레일링 부분 프레임 = truncated tail로 관용해 정상 prefix 반환, 중간 손상만 `HistoryError::Corrupt`(R-HIST-04).
- **인터페이스 표면**: 내부 — 프레임 스트림 -> `Vec<UploadHistoryRecord>` 또는 `Corrupt`.
- **안착 NFR 패턴**: §2(**U6-NFR-REL-02**, R-HIST-04 truncated-tail) + §6(**U6-NFR-PERF-02**, 선형 스캔).

**HistoryQueryFilter**
- **책임**: `since`/`only_failures` 선형 **AND 결합** 필터 적용(`None`이면 제약 없음, R-HIST-05; 성공/실패는 `error_detail` 유무로 파생). 보조 인덱스 없음(D8 트림).
- **인터페이스 표면**: `fn query(&self, filter: HistoryQuery) -> Result<Vec<UploadHistoryRecord>, HistoryError>`(읽기 경로, `Result` 반환).
- **안착 NFR 패턴**: §6(**U6-NFR-PERF-02**, R-HIST-05 AND 오라클).

### 2.4 `CriticalErrorNotifier` (impl `CriticalEventSink`)

**CriticalRouter**
- **책임**: 닫힌 4개 진입점(`report_auth_failure`/`report_cycle_result`/`report_preflight_exceeded`/`report_update_rollback`)에서만 능동 표면화(R-CRIT-01). 각 표면화를 3중 fan-out: `Arc<dyn Logger>`(상향 심각도) + `Arc<dyn StatusSink>`(`raise_condition`/escalation) + `TrayIndicator.notify`(no-op). 앞 2개는 트레이 비의존 항상 성립.
- **인터페이스 표면**: `fn report_auth_failure` · `report_cycle_result` · `report_preflight_exceeded` · `report_update_rollback`(전부 infallible push).
- **안착 NFR 패턴**: §4(**U6-NFR-REL-04**, R-CRIT-01/03 3중 fan-out) + §1(**REL-01**).

**FailureEscalator**
- **책임**: `consecutive_failure_count`(`Mutex` 내부 상태 또는 `AtomicUsize`) + escalation 플래그. `report_cycle_result(Failure)` 마다 +1, `Success` 시 0 리셋. 임계 `N`(주입된 `notify_consecutive_failures`, 기본 3) 도달 순간 1회 발화 + escalation 세팅 -> StatusAggregator에 push(중복 억제, R-CRIT-02).
- **인터페이스 표면**: 내부 상태머신(CriticalRouter 케이스2 경로에서 구동).
- **안착 NFR 패턴**: §4(**U6-NFR-REL-04**, R-CRIT-02 연속실패 임계 상태머신).

### 2.5 `TrayIndicator` — NoOpTrayBackend (선택, D1)

- **책임**: nullable **no-op 어댑터** — `start() -> Ok(None)`, `render`/`notify`/`stop` 무연산. 실제 데스크톱 트레이 백엔드(3-OS)와 트레이 크레이트 미도입. 트레이 부재/no-op가 로그·status·health·history 표면화를 막지 않음(R-TRAY-02).
- **인터페이스 표면**: `fn start(&self, cfg: &TrayConfig) -> Result<Option<TrayHandle>, TrayError>` · `render` · `notify` · `stop`(모두 비치명).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §4(트레이 비의존 fan-out) + §10(TrayBackend 어댑터 seam — 향후 실체 교체 지점). 독립 PBT 속성 없음(동작 부재, D1).

### 2.6 횡단 논리 단위

**ErrorTypes**
- **책임**: 운영 오류 `LogError`(`Io`/`InvalidPath`/`RotationFailed`) · `HistoryError`(`Io`/`Serde`/`Corrupt`)를 `thiserror` 파생. push 표면에 전파되지 않고 `query`/`reload` 경로에서만 반환(§1/§8). 지속 값 타입(`UploadHistoryRecord`, U0 FROZEN 3필드)은 별개 부류로 순수 serde 유지.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §8(**U6-NFR-MNT-01**, U0 Q4=A 관례 상속).

**ObservabilityConfig (federated 주입 뷰)**
- **책임**: U8이 원시 config를 파싱·해소해 주입하는 RESOLVED TYPED 값(`log_file: PathBuf`, `log_max_size_bytes: ByteCount`, `log_max_retained: usize`, `history_file: PathBuf`, `tray_enabled: bool`)의 생성자 주입 뷰 + `OBSERVABILITY_CONFIG_KEYS` const 선언(U8 federated union 집계용, R-CFG-U6-01). 코어 필드(`log_level`/`notify_consecutive_failures`)는 `ConfigProvider` 경유(비코어는 아님).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §0(federated-config 해소) + §7(코어 필드만 live-reload).

---

## 3. U6 내부 의존 엣지 (비순환 확인)

> 규약: `A -> B` = "A가 B에 의존한다(B의 타입/함수/계약을 사용)". U6 논리 컴포넌트는 서로 대체로 독립이며 모두 U0 값 계층/트레이트를 소비한다. U6 내부에 순환 없음.

### 3.1 의존 엣지 표

| 논리 컴포넌트 | 소속 애그리게이트 | 의존 대상 |
|---|---|---|
| LogEmitter | `StructuredLogger` | LogWriter · U0(`LogRecord`/`Timestamp`/`TokenSecret`/`Logger` 계약) · Clock seam |
| LogWriter | `StructuredLogger` | LogRotator · U0(`ByteCount`) · `std::fs`/`std::sync` · FileIo seam |
| LogRotator | `StructuredLogger` | `std::fs` · ObservabilityConfig(rotation 파라미터) |
| LogReloadAdapter | `StructuredLogger` | LogEmitter/LogWriter(재적용) · U0(`ConfigReloadObserver`/`ConfigProvider` 코어 필드) |
| StatusAggregator | `StatusService` | U0(`StatusSink`/상태 어휘 값 타입) · `std::sync::Mutex` |
| StatusSnapshotDeriver | `StatusService` | StatusAggregator · U0(`StatusSnapshot`) |
| HealthOracle | `StatusService` | StatusAggregator · U0(`Health`/`HealthReason`) |
| LivenessProbe | `StatusService` | StatusAggregator · U0(`Liveness`/`LivenessSignal`) |
| HistoryFramer | `UploadHistoryStore` | U0(CBOR `encode`/`decode`·`UploadHistoryRecord` 값 타입) |
| HistoryAppender | `UploadHistoryStore` | HistoryFramer · `std::fs` · FileIo seam · U0(`HistorySink` 계약) |
| HistoryScanner | `UploadHistoryStore` | HistoryFramer · `std::fs` · ErrorTypes(`HistoryError`) |
| HistoryQueryFilter | `UploadHistoryStore` | HistoryScanner · U0(`HistoryQuery`) · ErrorTypes(`HistoryError`) |
| CriticalRouter | `CriticalErrorNotifier` | FailureEscalator · `Arc<dyn Logger>` · `Arc<dyn StatusSink>` · NoOpTrayBackend · U0(`CriticalEventSink` 계약·입력 타입) |
| FailureEscalator | `CriticalErrorNotifier` | `std::sync` · ObservabilityConfig(`N` = U0 코어 필드) |
| NoOpTrayBackend | `TrayIndicator` | (없음 — 무연산 리프) |
| ErrorTypes | 횡단 | `thiserror`(U0 핀) |
| ObservabilityConfig | 횡단 | U0(`ByteCount`/`Timestamp` 등 값 타입) — U8 주입 |

### 3.2 ASCII 의존 스케치 (박스 미사용)

```
공개 컴포넌트 (U8 조립 루트가 생성·주입, U1~U5 역참조 없음):

  StructuredLogger:   LogEmitter -> LogWriter -> LogRotator
                      LogReloadAdapter -> (LogEmitter/LogWriter 재적용)
  StatusService:      StatusAggregator <- {SnapshotDeriver, HealthOracle, LivenessProbe}
  UploadHistoryStore: HistoryFramer <- {Appender, Scanner -> QueryFilter}
  CriticalErrorNotifier: CriticalRouter -> FailureEscalator
                         CriticalRouter -> {Arc<dyn Logger>, Arc<dyn StatusSink>, NoOpTrayBackend}
  TrayIndicator:      NoOpTrayBackend (리프, 무연산)

  방향 요약:  U6 논리 컴포넌트  ->  foundation(U0) 값 타입/코덱/트레이트/코어 config
             U6  ->  (U1~U5 없음)                    => 크레이트 그래프 비순환
             U6 내부  ->  (순환 없음)
```

### 3.3 비순환 및 sub-unit 무-import 확인

- **크레이트 그래프**: `observability` 의 `[dependencies]` 는 `foundation`(U0) 만이다(R-PUSH-01). U1~U5를 의존에 넣지 않으므로 역참조가 코드 리뷰/컴파일에 드러난다. U8 조립 루트만 U6를 하위 단위에 주입하며 U6는 자신이 주입되는 대상을 import하지 않는다.
- **U6 내부 비순환**: 모든 내부 엣지는 (파사드 -> writer/rotator, aggregator <- 판정자, framer <- appender/scanner, router -> escalator/싱크) 단방향이며 역방향이 없다.
- **fan-out 대상은 계약 타입 주입**: `CriticalRouter` 는 `Arc<dyn Logger>`/`Arc<dyn StatusSink>` 를 U8 조립 시 주입받아 U6-내부(`StructuredLogger`/`StatusService`)에 배선되며 구체 타입 역참조가 아니다.

---

## 4. Trait seam (인터페이스 이음매)

> "trait seam"은 구현을 교체·모킹 가능케 하는 인터페이스 경계다. U6의 seam은 아래 4종이며, 다른 단위의 seam(예: `HttpTransport`=U3, watch-backend=U2)은 **U6 관심사가 아니다**.

| Seam | 소유/방향 | MVP 구현 | 향후/테스트 교체 |
|---|---|---|---|
| **U0 SinkContracts** (`Logger`/`StatusSink`/`ReadJudgment`/`HistorySink`/`CriticalEventSink`/`ConfigReloadObserver`) | U0 정의 -> U6 구현, U8 하향 주입 | U6 4개 실체 싱크가 구현 | 소비 단위는 계약 타입(`Arc<dyn ...>`)으로만 소비(구현 교체 자유) |
| **TrayBackend** | U6 고유 계약 | NoOpTrayBackend(`start()->Ok(None)`, D1) | 향후 confirm 시 실체 데스크톱 트레이 어댑터로 교체(타 컴포넌트 불변) |
| **Clock/now-source** | U6 내부 seam | `std::time::SystemTime` -> U0 `Timestamp` stamping(D4) | 테스트에서 결정적 clock 주입(타임스탬프 stamping 테스트 가능성) |
| **FileIo** | U6 내부 seam | `std::fs`/`std::io`(로그 append·히스토리 프레임 I/O) | IO 오류 주입 어댑터로 REL-01 비차단 속성(PROP-U6-BL-02) 검증 |

---

## 5. PBT 타깃 정렬 (PBT-01 / 확장 Full)

> 이 단계는 신규 PBT 결정을 내리지 않는다(PBT-09 프레임워크 = `proptest`, U0 상속으로 충족). FD 확정 속성(PROP-U6-DE-*/BR-*/BL-*)을 명명 논리 컴포넌트에 정렬해 추적성을 강화한다. NFR ID -> 속성 매핑은 `nfr-design-patterns.md` §12/§13과 일관된다.

### 5.1 컴포넌트 -> Testable Property 정렬표

| 논리 컴포넌트 | 정렬 속성(FD 소유) | 카테고리 | 실현 NFR / 규칙 |
|---|---|---|---|
| HistoryFramer | PROP-U6-DE-01 (`decode(encode(r)) == r`) | Round-trip | U6-NFR-REL-02 / U6-NFR-MNT-01(R-HIST-02) |
| HistoryAppender · HistoryScanner | PROP-U6-BR-08 (append-only 불변) · PROP-U6-BR-09 (truncated-tail 관용) | Invariant | U6-NFR-REL-02(R-HIST-01/04) |
| HistoryQueryFilter | PROP-U6-BR-08 (query AND 오라클) | Oracle | U6-NFR-PERF-02(R-HIST-05) |
| LogEmitter | PROP-U6-DE-02 (방출 라인 파싱 무결) · PROP-U6-BR-01 (레벨 필터) | Round-trip · Oracle | U6-NFR-USE-01 / U6-NFR-PERF-01(R-LOG-01/02) |
| StatusAggregator · StatusSnapshotDeriver | PROP-U6-BR-02 (2축 독립) · PROP-U6-BR-03 (consent/offline 파생) | Invariant · Oracle | U6-NFR-REL-03(R-STATUS-01/02/03) |
| HealthOracle | PROP-U6-BR-04 (health 오라클) | Oracle | U6-NFR-REL-03(R-STATUS-04) |
| LivenessProbe | PROP-U6-BR-05 (update_probe 격리) | Invariant(핵심) | U6-NFR-REL-03(R-STATUS-05) |
| CriticalRouter | PROP-U6-BR-06 (닫힌 집합) · PROP-U6-BL-01 (fan-out 정합) | Invariant · Oracle | U6-NFR-REL-04(R-CRIT-01/03) |
| FailureEscalator | PROP-U6-BR-07 (연속실패 임계 상태머신) | Induction · Oracle | U6-NFR-REL-04(R-CRIT-02) |
| 전 실체 싱크(공통) | PROP-U6-BL-02 (관측 실패 비차단) | Invariant | U6-NFR-REL-01(R-PUSH-02, FileIo seam 주입) |

> **속성 없음(No PBT properties identified)**: NoOpTrayBackend(동작 부재, D1) · LogWriter/LogRotator(rotation IO = 결정적 예제테스트, `business-rules.md` §8.6) · LogReloadAdapter(리로드 멱등은 U0 config 흐름 PROP-BL-04에 흡수) · ObservabilityConfig(config round-trip은 U0 PROP-BR-02에 흡수) · ErrorTypes(반환 형상, 타입 시스템)는 독립 PBT 속성이 없다(FD §8.6/§6.3 판정과 일관).

### 5.2 ProptestGenerators — 런타임 그래프 밖 test-support 논리 단위 (PBT-07)

- **배치**: 도메인 제너레이터(`proptest`)는 **런타임 의존 그래프(§3) 밖**의 별도 test-support 논리 단위다. 비-default Cargo feature(`proptest-support`)로 게이트되어 릴리스 런타임 바이너리에 포함되지 않는다(U0 mirror).
- **책임**: `UploadHistoryRecord`(3필드: `error_detail` `None`(성공)/유니코드·개행/빈 문자열(실패)·0/경계/대값 `bytes_transferred`·임의 `timestamp`), `LogRecord` 방출 라인(전 `LogLevel`·`cycle_id` 유무·유니코드/개행/빈 `message`·다수/빈 `fields`), 상태 명령 시퀀스(`set_operational`/`raise_condition`/... 인터리빙·중복 raise/clear), `report_*` 시퀀스(4 case 혼합·Success/Failure + 임의 `N`), IO 오류 주입 어댑터(FileIo seam) 제너레이터 제공. **U0 재사용**: `Timestamp`·`ManifestDigest`·`LogLevel`·`ActiveCondition`/`OperationalState`/`LivenessSignal`·자유형 `detail` 제너레이터는 U0 `proptest-support` 를 dev에서 켜 재사용(재작성 금지).
- **의존 방향**: `ProptestGenerators -> {observability 런타임 타입, foundation `proptest-support`}`. 런타임 컴포넌트는 이 단위에 의존하지 않으므로 §3 런타임 DAG를 오염시키지 않는다.
- **이월**: 구체 구현·shrinking·고정 시드·CI 통합(PBT-08)은 Code Generation / Build-and-Test 이월.

---

## 6. RESILIENCY-01 매핑 — U6 = Supporting (best-effort 비차단 관측)

- **분류**: U6 `observability`는 **Supporting(관측·감사)** 이다 — 동기화 코어(U1~U5/U8)가 U6 트레이트를 통해 값을 push하지만 push 표면이 **infallible·비차단**이라(§1) **U6 실패가 코어를 정지시키지 않는다**(코어의 단일 장애점이 아님). 반대로 U6는 U0에만 의존하므로 U6 실패의 영향 반경은 관측 표면(로그 유실·히스토리 append 유실)에 국한된다.
- **회복력 기여(순수 lib 범위)**:
  - **best-effort infallible 비차단(U6-NFR-REL-01, 전 실체 싱크)** — 관측 실패가 코어를 막지 않음(RESILIENCY-05/15 관측·사고 표면화의 안전 전제).
  - **append-only 무손실 + truncated-tail 관용(U6-NFR-REL-02, HistoryFramer/Appender/Scanner)** — 크래시 후 재기동 query가 정상 prefix를 잃지 않음(RESILIENCY-02 로컬 재구성). 단, **RPO=0은 U4 `SyncStateStore` 소관**이며 U6 히스토리는 비권위 감사 로그.
  - **단일 집계 원자 관측 + update_probe 격리(U6-NFR-REL-03, StatusService)** — health/status의 일관 관측 + `update_probe` 격리가 **RESILIENCY-04 자동 롤백 게이트** 안정성 근거(일시 조건 오판 롤백 방지).
  - **트레이 비의존 3중 fan-out(U6-NFR-REL-04, CriticalRouter)** — GUI-less 데몬에서 사고가 항상 로그+health+CLI status로 표면화(RESILIENCY-15).
- **N/A 범위**: RTO/RPO 수치·DR/HA·서킷브레이커·auto-scaling·배포/롤백·카오스(RESILIENCY-02/05~14)는 순수 lib에 부적용. 배포/롤백은 U7a `AutoUpdater`(U6는 `update_probe` 판정만 제공). U6 신규 인프라 통제 없음.

---

## 7. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 산출물 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **준수 — blocking 없음** | §5가 각 논리 컴포넌트를 확정 속성(PROP-U6-DE-*/BR-*/BL-*)에 정렬(PBT-01), ProptestGenerators를 비-default `proptest-support` test-support 논리 단위로 문서화(PBT-07). 신규 PBT 결정 없음(PBT-09는 U0 상속 충족). PBT-08은 Code Generation/Build-and-Test 이월. |
| **Resiliency Baseline** | ON | **준수(부분 적용 + 대체로 N/A) — blocking 없음** | §6이 U6 = Supporting(best-effort 비차단) 분류와 U0 의존 매핑을 컴포넌트 입도로 확정. REL-01~05 계약의 컴포넌트 안착 명시. RTO/RPO/DR/HA/서킷브레이커/카오스는 순수 lib에 N/A(RPO=0은 U4). 신규 resiliency 결정 없음. |
| **Security Baseline** | OFF | **N/A — 미로딩·미강제** | 확장 OFF, RISK-01 수용(로그/히스토리 로컬 평문). LogEmitter/HistoryFramer의 토큰 redaction(U6-NFR-SEC-01)은 U0 `TokenSecret` 소비 계약 준수일 뿐 신규 강제 통제 아님. |

**블로킹 판정**: 이 산출물에 blocking finding 없음. §2 컴포넌트 안착은 `nfr-design-patterns.md` §13 맵과 1:1이며, §3 의존 그래프는 비순환(`observability -> foundation` 단일 엣지, U1~U5 무-import)이다.

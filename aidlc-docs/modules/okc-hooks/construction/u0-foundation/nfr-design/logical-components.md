# U0 Foundation — Logical Components (논리 컴포넌트 분해 + 추적성 맵)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U0 Foundation** -> NFR Design -> 산출물 2/2 (`logical-components.md`)
**작성일**: 2026-09-08
**크레이트**: `foundation` (lib) · **소속 컴포넌트(공개 애그리게이트)**: `CoreTypes`, `ConfigProvider`
**입력 아티팩트**: 자매 산출물 `nfr-design/nfr-design-patterns.md`(§9 패턴 -> 논리 컴포넌트 안착 맵이 이 문서의 권위 근거) · `plans/u0-foundation-nfr-design-plan.md` §3 확정 답변(Q1=A 입도, Q2~Q7) · `functional-design/`(domain-entities §1~§5 · business-rules · business-logic-model) · `inception/application-design/`(components.md 공개 2 컴포넌트 · component-methods.md · component-dependency.md)
**규칙**: `construction/nfr-design.md` Step 6 · `common/content-validation.md`(ASCII 화살표 `A -> B`, 박스 문자 미사용, Rust 제네릭 백틱) · `common/ascii-diagram-standards.md` · 활성 확장 `property-based-testing.md`(ON, Full) · `resiliency-baseline.md`(ON)

> **문서 성격 (Q1=A 입도 가드)**: 이 문서는 Application Design이 확정한 **공개 2 애그리게이트**(`CoreTypes`·`ConfigProvider`)를 상위 애그리게이트로 유지한 채, 그 내부의 **이미 확정된 기능을 명명된 논리 컴포넌트로 전개**한다. 이는 **발명이 아니라 확정 기능(FD 타입·규칙·흐름)의 명명·정렬**이며, FD 흐름에서 near-free로 파생된다. 각 논리 컴포넌트는 (a) 책임, (b) 공개 인터페이스 표면, (c) 안착하는 확정 NFR 패턴(자매 산출물 `nfr-design-patterns.md` §9 안착 맵과 1:1)으로 기술한다.
>
> **이것은 추적성 문서 맵이며 물리 모듈/크레이트 증식 강제가 아니다.** 논리 컴포넌트 -> 물리 파일/모듈 레이아웃 매핑은 **Code Generation에서 최소로** 결정된다(다수 논리 컴포넌트가 한 모듈에 상주할 수 있음). 이 문서의 목적은 NFR·규칙·PBT 속성 -> 명명 단위 추적성 극대화와 RESILIENCY-01(U0 = Critical, DAG 루트) 의존 매핑 강화다.
>
> **표기 규약**: 화살표는 ASCII `A -> B`만 사용(유니코드 화살표 금지). 박스/선-그리기 문자 미사용. Rust 제네릭/타입/식별자(예: `Vec<u8>`, `Arc<WatcherConfig>`, `dyn ConfigReloadObserver`, `ArcSwap`, `CodecError`)는 백틱으로 감싼다.

---

## 1. 논리 컴포넌트 전개 개요

| 공개 애그리게이트 | 성격 | 논리 컴포넌트(Q1=A 확정 명명) |
|---|---|---|
| `CoreTypes` | 순수 값 타입 + 순수 함수 + 계약 트레이트(상태·I/O 없음) | Codec · PathNormalizer · ErrorTaxonomy · TokenSecret · SyncStateModel · SinkContracts · StatusVocab |
| `ConfigProvider` | config 로드/검증 + 상태 보유(active snapshot) + 관찰자 팬아웃 | ConfigLoader · ConfigValidator · UrlValidator · ConfigStore · ObserverRegistry |
| (런타임 그래프 밖) | 테스트 전용 — 비-default `proptest-support` feature | ProptestGenerators (test-support 논리 단위, §5.2) |

---

## 2. `CoreTypes` 애그리게이트 — 논리 컴포넌트

> `CoreTypes`는 U0(및 전 단위)의 DAG 루트 값 계층이다. 순수 값 타입·순수 함수·계약 트레이트만 보유하며 상태·스레딩·I/O가 없다.

### 2.1 Codec
- **책임**: 내부 지속 상태의 CBOR 인코딩/디코딩(FD Q1=B 포맷 분리; 크레이트=`ciborium`[NFR-Req Q1=A]). R-CODEC-01 무손실 round-trip 불변식의 실행 지점. 절단·손상·적대적 CBOR 바이트열을 받는 진입점으로 "유효 CBOR인가 / 대상 타입으로 역직렬화되는가"만 판정하고 손상 복구는 U4 소관.
- **공개 인터페이스 표면**: `encode<T: Serialize>(v: &T) -> Result<Vec<u8>, CodecError>` · `decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, CodecError>`. 무손실 대상 값 모델(`Manifest`, `ManifestEntry`, `Sha256Digest`, `ManifestDigest`, `Timestamp`, `ByteCount`, `ChangeSet`, `SyncState`, `ErrorClass`, `TransportError`, `ClassifiedError`, `TransferResult`)은 선언적 값 타입으로 Codec의 round-trip 페이로드이며(§판단-노트 참조) 별도 명명 컴포넌트로 승격하지 않는다.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §1 panic-free-total(**U0-NFR-REL-02**, 순수 모듈 컴파일타임 clippy lint-gate `deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)`) + §1 무손실 코덱(**U0-NFR-REL-01**, R-CODEC-01). 인코딩/디코딩 실패는 `CodecError`(Fatal)로만 표면화.

### 2.2 PathNormalizer
- **책임**: `RelativePath` 정규화 — POSIX 구분자 · 상대 · `..`/`.` 제거, 정규화된 값만 존재한다는 구성 불변식 보증. 순수 함수.
- **공개 인터페이스 표면**: `RelativePath`(참고 형상 `struct RelativePath(String)`) 및 그 정규화 생성 경로(`RelativePath::normalize`). 불변식: `normalize(normalize(p)) == normalize(p)`(멱등).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §1 panic-free-total(**U0-NFR-REL-02**) — 순수 모듈 lint-gate 대상. 모든 입력에 대해 패닉 없이 `Result`(또는 total 구성)로 표면화.

### 2.3 ErrorTaxonomy
- **책임**: 오류 **분류(classify)** 값 계층 소유. 전송 오류 -> `ErrorClass` 매핑(R-CLASS-01)과 `is_retryable()` total 판정(R-CLASS-02/03). 운영 오류 타입(`CodecError`·`ConfigError`) 정의. 백오프·재시도 스케줄은 U4 소관(U0는 분류만).
- **공개 인터페이스 표면**: `enum ErrorClass { Retryable, AuthAborted, Backpressure, Fatal }` · `ErrorClass::is_retryable() -> bool`(total) · `struct TransportError { class, http_status: Option<u16>, detail }` · `enum TransportErrorClass { AuthFailed, ServerError, Backpressure, Timeout, Network }` · `struct ClassifiedError { class, code: Option<String>, detail }` · `enum TransferResult { Success{bytes}, Partial{bytes, resume_offset}, Failed{error} }` · 운영 오류 `CodecError`(순수 serde 계열) · `ConfigError`(`thiserror` 파생, 이슈 `Vec` 운반).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §1 panic-free-total(**U0-NFR-REL-02**, R-CLASS-03 `is_retryable` total function — 미정의·패닉 경로 없음) + §4 config 검증의 오류 운반체(**U0-NFR-MNT-01**, `ConfigError` = `thiserror` 파생; 분류 값 타입 `ErrorClass`는 순수 serde 유지 — 별개 부류).

### 2.4 TokenSecret
- **책임**: `token`을 감싸는 redacting newtype. `Debug`/`Display` 표면에서 실제 값 대신 `***`를 노출하고 실제 값은 `.expose()`로만 해소. 로그 유출 위생의 U0 측 실행 지점.
- **공개 인터페이스 표면**: redacting newtype(참고: `Debug`/`Display` -> `***`, `.expose() -> &str`). `Serialize`/`Deserialize`는 **실제 값 보존**(round-trip/지속 경로 전용, PROP-BR-02 무손실).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §6 토큰 redaction(**U0-NFR-SEC-02**, R-TOKEN-01/02) — Debug/Display 한정 redaction + **Q7=A U0 no-Serialize-to-log 계약**(`WatcherConfig`/`token`은 serde `Serialize`로 로그에 방출되지 않음; 값-보존 `Serialize`는 round-trip 전용 예약). Security Baseline OFF·RISK-01 수용 하의 저비용 잔존 위생.

### 2.5 SyncStateModel
- **책임**: 동기화 상태 머신 타입과 전이 규칙 **모델**(선형 + dirty 재진입, Q8=A) 정의. 전이 명령 시퀀스의 **실행**은 U4 `SyncStateStore`가 수행(U0는 모델만 소유, stateful 실행 속성 없음).
- **공개 인터페이스 표면**: `enum SyncState { Idle, Dirty, Uploading, Committed }` + 전이 표(가드·실패 처리·dirty 재진입 모델, business-logic-model.md 소유).
- **안착 NFR 패턴**: 신규 NFR 패턴 안착 없음(순수 값 + 모델). REL-01 무손실 round-trip 대상 값 타입으로 Codec 페이로드에 포함(PROP-DE-01). PBT 정렬은 §5(PROP-DE-04/PROP-BL-05, U4 실행 위임).

### 2.6 SinkContracts
- **책임**: U0가 계약으로 소유하고 U6가 구현하는 push-only 관측 싱크 트레이트 계열 정의(back-reference 없음, U8 하향 주입). SEC-02 no-Serialize 계약의 소비-측 준수 주체(`Logger`)를 계약으로 명문화.
- **공개 인터페이스 표면**: `trait Logger`(`log(record)` / `event(level, event, cycle_id, fields)`) · `trait StatusSink`(`set_operational` / `raise_condition` / `clear_condition` / `record_sync_success` / `set_dirty` / `set_resume_progress` / `set_liveness`) · `trait HistorySink`(`append(record)`) · `trait CriticalEventSink`(`report_auth_failure` / `report_cycle_result` / `report_preflight_exceeded` / `report_update_rollback`). `LogRecord` 값 스키마는 U6 이월.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §6 토큰 redaction 소비 계약(**U0-NFR-SEC-02**, R-OBSERVER-02: `log_level` 변경 -> U6 `StructuredLogger`) — `Logger` 계약이 config 파생 필드를 `Debug`/`Display`(redacted) 또는 명시적 per-field 프로젝션으로만 방출한다는 U0 계약의 인터페이스 경계.

### 2.7 StatusVocab
- **책임**: Q9 2축 상태 어휘(운영 라이프사이클 단일 상태 + 동시 성립 활성 조건 집합) 값 어휘 정의. 조건 -> 운영상태 결합 판정 규칙은 U6 소유(U0는 어휘만).
- **공개 인터페이스 표면**: `enum OperationalState { Idle, Syncing, Offline, Paused }` · `enum ActiveCondition { AuthFailed, ConsentBlocked, OverLimit, VaultUnavailable, UpdateRolledBack }` · `enum LivenessSignal { IdleReached, CredentialReadable }` · `struct StatusSnapshot { operational, conditions, last_success: Option<Timestamp>, dirty, resume: Option<(ByteCount, ByteCount)>, consent, offline }`.
- **안착 NFR 패턴**: 신규 NFR 패턴 안착 없음(유한 enum 값 어휘). REL-01 round-trip 대상으로 Codec 페이로드에 포함(직렬화 시 PROP-DE-01). 독립 PBT 속성 없음(domain-entities §5.5).

---

## 3. `ConfigProvider` 애그리게이트 — 논리 컴포넌트

> `ConfigProvider`는 U0의 유일한 사람-대면 표면(JSON config)이자 유일한 상태 보유·팬아웃 컴포넌트다. 공개 메서드: `load(path)` · `current() -> ConfigSnapshot` · `reload() -> Result<(), ConfigError>` · `subscribe(observer: &dyn ConfigReloadObserver)`.

### 3.1 ConfigLoader
- **책임**: config 파일 발견(R-DISCOVER-01: `--config` > `OKC_WATCHER_CONFIG` > 플랫폼 기본 경로)과 원시 로드 + `serde_json` 1차 파스(`serde_json::Value`) 오케스트레이션. 검증은 ConfigValidator에 위임. 최초 로드 실패 = abort(R-RELOAD-04), 실행 중 로드 = keep-last-good 경로(R-RELOAD-03)로 결과 전달.
- **공개 인터페이스 표면**: `load(path) -> Result<ConfigSnapshot, ConfigError>`(2-pass 오케스트레이션 진입점). 주입된 known-key set(§3.2, Q6=A)을 검증 경로로 전달.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §4 config 검증(**U0-NFR-USE-01**, R-CFG-STRICT-01) + §2 reload 쓰기 경로 직렬화(**U0-NFR-REL-03**의 `load -> validate -> swap -> notify` 시퀀스 중 load 단계) + §5 federated known-key set 주입 계약(**Q6=A**, `watcher-bin`이 union 집계 후 주입).

### 3.2 ConfigValidator
- **책임**: 검증 로직 소유 — (1) 미지 키 **전수 나열**(1차 `serde_json::Value`에서 주입된 known-key set 밖 **모든** 키 수집, 첫 키에서 중단 없음, R-CFG-STRICT-01), (2) per-field 위반은 타입드 deserialize의 **FIRST 구조화 오류**로 보고(Q5=C 스코프 트림 — 전-field 누적 Value 검증 패스 미채택). 미지 키 clean 판정 후에만 타입드 deserialize(2-pass 순서). SafetyLimits 키는 known-key set 밖이므로 미지 키로 거부(R-LIMIT-01).
- **공개 인터페이스 표면**: 검증 결과 `Result<WatcherConfig, ConfigError>`(내부 표면). `ConfigError`는 (다수) 미지 키 이슈 + 단일 첫 field 위반을 담는 이슈 `Vec` 운반. per-field 규칙: `vault_path` 절대경로 · `server_endpoint` https-only(UrlValidator 위임) · `log_level` 열거 · `notify_consecutive_failures >= 1` 등(business-rules §4.1).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §4 config 검증(**U0-NFR-USE-01 / U0-NFR-MNT-01**, R-CFG-STRICT-01 + business-rules §4.1) + §5 federated known-key set의 **주입된 known-key-set 소비 계약**(**Q6=A**, U0는 하위 단위 import 없이 주입만 소비).

### 3.3 UrlValidator
- **책임**: `server_endpoint`의 URL 형식 파싱(`url` 크레이트)과 **스킴 https-only 강제**(TLS 강제, `http`/무스킴 거부). SEC-01 잔존 통제의 실행 지점.
- **공개 인터페이스 표면**: URL 파싱 + 스킴 assert 판정(ConfigValidator가 per-field 검증 중 호출). 위반은 `ConfigError` field 위반으로 표면화.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §6 관련 잔존 통제(**U0-NFR-SEC-01**, TLS/`https` 강제 = 유일 잔존 강제 통제) + §4 config 검증(business-rules §4.1 `server_endpoint` 규칙).

### 3.4 ConfigStore
- **책임**: active config snapshot 보유 + reload 쓰기 경로 오케스트레이션. `arc_swap::ArcSwap<Arc<WatcherConfig>>`로 lock-free 읽기(`current()`), 내부 `reload_mutex`로 `load -> validate -> swap -> notify` 전체 시퀀스 직렬화. 검증 실패 시 스왑 미도달 = keep-last-good(REL-05). 성공 스왑 후에만 ObserverRegistry 팬아웃 호출(R-RELOAD-05).
- **공개 인터페이스 표면**: `current() -> ConfigSnapshot`(lock-free `Arc` 로드) · `reload() -> Result<(), ConfigError>`(쓰기 경로, `reload_mutex` 하 직렬화). 불변식: 동시 읽기 스레드는 항상 완결된 이전 또는 완결된 새 스냅샷 하나만 관측(중간 상태 없음).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §2 reload 쓰기 경로 직렬화(**U0-NFR-REL-03**, `reload_mutex` + `ArcSwap`; R-RELOAD-01/02/05, R-OBSERVER-03, R-TRIGGER-01) + **U0-NFR-REL-05** keep-last-good(검증 실패 시 스왑 미도달).

### 3.5 ObserverRegistry
- **책임**: 리로드 관찰자를 기동 시 1회 등록되는 **불변 순서 컬렉션**으로 저장하고, 성공 스왑 직후 순서대로 순회하며 **per-observer `catch_unwind`** 로 격리해 팬아웃(Q4=A + Q7=A). 런타임 뮤테이션 없음 -> 결정적 순서 자연 충족 + 레지스트리 락 불필요.
- **공개 인터페이스 표면**: `subscribe(observer: &dyn ConfigReloadObserver)`(데몬 스레드 기동 전 U8 조립 시 1회) · 내부 저장 `immutable ordered Vec<Arc<dyn ConfigReloadObserver>>` · 성공 스왑 후 팬아웃 통지(`on_config_reload()` 호출, push-only·단방향, back-reference 없음). 팬아웃 대상 매핑: `token`/`secure_store_enabled` -> U5 `CredentialProvider`/`ConsentGate`, `log_level` -> U6 `StructuredLogger`(R-OBSERVER-02).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §3 관찰자 팬아웃 격리(**U0-NFR-REL-04**, 불변 순서 `Vec` + `catch_unwind`; R-OBSERVER-01/02/03, R-RELOAD-05). 불변식: 스왑은 팬아웃 이전 이미 원자 커밋 -> 패닉 관찰자가 `current()`/타 관찰자를 오염 못함(롤백 없음).

---

## 4. U0 내부 의존 엣지 (비순환 확인)

> 규약: `A -> B` = "A가 B에 의존한다(B의 타입/함수/계약을 사용)". CoreTypes 컴포넌트는 값 계층 리프이고, ConfigProvider 컴포넌트가 CoreTypes 방향으로만 의존한다(역방향 없음).

### 4.1 의존 엣지 표

| 논리 컴포넌트 | 소속 애그리게이트 | 의존 대상(U0 내부) |
|---|---|---|
| Codec | `CoreTypes` | ErrorTaxonomy(`CodecError`) + 무손실 대상 값 모델(선언적 값 타입) |
| PathNormalizer | `CoreTypes` | (없음 — 순수 리프) |
| ErrorTaxonomy | `CoreTypes` | (없음 — 순수 리프) |
| TokenSecret | `CoreTypes` | (없음 — 순수 리프) |
| SyncStateModel | `CoreTypes` | (없음 — 순수 리프) |
| StatusVocab | `CoreTypes` | (없음 — 순수 리프) |
| SinkContracts | `CoreTypes` | ErrorTaxonomy(`ClassifiedError`) · StatusVocab(`StatusSnapshot`/`OperationalState`) |
| UrlValidator | `ConfigProvider` | ErrorTaxonomy(`ConfigError`) |
| ConfigValidator | `ConfigProvider` | UrlValidator · ErrorTaxonomy(`ConfigError`) · TokenSecret(`token` 필드) |
| ConfigLoader | `ConfigProvider` | ConfigValidator · ErrorTaxonomy(`ConfigError`) |
| ConfigStore | `ConfigProvider` | ConfigLoader · ObserverRegistry · ErrorTaxonomy(`ConfigError`) · TokenSecret(`WatcherConfig`) |
| ObserverRegistry | `ConfigProvider` | (없음 — `Arc<WatcherConfig>` 스냅샷을 `dyn ConfigReloadObserver`에 전달만) |

### 4.2 ASCII 의존 스케치 (박스 미사용)

```
ConfigProvider 계층 (상태 보유 + 팬아웃):

  ConfigStore  ->  ConfigLoader  ->  ConfigValidator  ->  UrlValidator
       |                                    |                  |
       +-> ObserverRegistry                 +--> TokenSecret    +
       |                                    |                  |
       v                                    v                  v
  ---------------------------------------------------------------------
CoreTypes 값 계층 (순수 리프 - 역방향 의존 없음):

  ErrorTaxonomy    PathNormalizer    TokenSecret    SyncStateModel
       ^                                 ^
       |                                 |
  Codec (-> ErrorTaxonomy, 값 모델)   SinkContracts (-> ErrorTaxonomy, StatusVocab)
                                                          |
                                                          v
                                                      StatusVocab

  방향 요약: ConfigProvider.*  ->  CoreTypes.*   (한 방향)
            CoreTypes 리프    ->  (없음)         => 사이클 없음
```

### 4.3 비순환 및 sub-unit 무-import 확인

- **비순환(acyclic)**: 모든 엣지는 ConfigProvider -> CoreTypes 또는 CoreTypes 내부의 (Codec/SinkContracts) -> 리프 방향으로만 흐르며 역방향 엣지가 없다. 순환 없음(component-dependency.md의 `CoreTypes` = 루트, `ConfigProvider -> CoreTypes` 매트릭스와 정합).
- **U0는 어떤 하위 단위(U1~U7)도 import하지 않음 (Q6=A 주입 계약)**: 미지 키 판정 기준(federated known-key set)은 각 단위가 컴파일타임 `const &[&str]`로 노출하고 `watcher-bin`이 union을 집계해 `ConfigProvider::new(known_keys = union_keys)`로 **주입**한다. U0는 "주입받는 계약"만 정의하므로 `U0 -> 하위 단위` 엣지가 생기지 않아 DAG 루트 비순환이 유지된다(`nfr-design-patterns.md` §5).

---

## 5. PBT 타깃 정렬 (PBT-01 / 확장 Full)

> 이 단계는 신규 PBT 결정을 내리지 않는다(PBT-09 프레임워크 = NFR-Req에서 `proptest`로 SATISFIED). 확정 속성(FD의 PROP-DE-*/PROP-BR-*/PROP-BL-*)을 명명 논리 컴포넌트에 정렬해 추적성을 강화한다. NFR ID -> 속성 매핑은 `nfr-design-patterns.md` §8과 일관된다.

### 5.1 컴포넌트 -> Testable Property 정렬표

| 논리 컴포넌트 | 정렬 속성(FD 소유) | 카테고리 | 실현 NFR / 규칙 |
|---|---|---|---|
| Codec | PROP-DE-01 · PROP-BR-01 · PROP-BL-01 (`decode(encode(v)) == v`) | Round-trip | U0-NFR-REL-01(R-CODEC-01) |
| Codec | REL-02 no-panic(적대적/절단 바이트열 -> `CodecError`, 패닉 없음) | Invariant | U0-NFR-REL-02(§1 lint-gate) |
| PathNormalizer | PROP-DE-03 (`normalize(normalize(p)) == normalize(p)`) | Idempotence | U0-NFR-REL-02(no-panic) |
| ErrorTaxonomy | PROP-BR-04 (`is_retryable` total; `TransportErrorClass` -> `ErrorClass` 전수 매핑) | Invariant + Easy verification | U0-NFR-REL-02(R-CLASS-01/02/03) |
| ConfigValidator | PROP-BR-05 (미지 키 STRICT reject, 삽입 키 전수 나열) | Invariant | U0-NFR-USE-01(R-CFG-STRICT-01) |
| ConfigStore | PROP-BR-02 · PROP-BL-04 (config 파싱 round-trip) | Round-trip | U0-NFR-SEC-02(토큰 값 보존) · U0-NFR-USE-01 |
| ConfigStore | PROP-BR-03 · PROP-BL-04 (reload 멱등성 "2회 == 1회") | Idempotence | U0-NFR-REL-03(R-RELOAD-02) |
| TokenSecret | PROP-BR-02(round-trip 시 실제 토큰 값 보존) | Round-trip | U0-NFR-SEC-02(Debug/Display redaction + Serialize 보존) |
| SyncStateModel | PROP-DE-04 · PROP-BL-05 (SyncState 전이 모델) | Induction / 상태 기반 | 모델=U0, **명령 시퀀스 실행=U4 `SyncStateStore`(PBT-06)** |
| (참고) ManifestDigest 값 | PROP-DE-02 · PROP-BL-02 (순서 무관 결정성) | Invariant + Oracle | 값 타입=CoreTypes, **다이제스트 계산·실행=U1 `ContentAddressing`** |

> **속성 없음(No PBT properties identified)**: SinkContracts(순수 push-only 계약), StatusVocab(유한 enum 어휘), UrlValidator/ConfigLoader(검증 규칙은 PROP-BR-05에 흡수), R-TRIGGER-01 트리거 권한(구성 규칙)은 독립 PBT 속성이 없다(domain-entities §5.5 · business-rules §9.6 · business-logic-model 판정과 일관).

### 5.2 ProptestGenerators — 런타임 그래프 밖 test-support 논리 단위 (PBT-07)

- **배치**: 도메인 제너레이터(`proptest`)는 **런타임 의존 그래프(§4) 밖**의 별도 test-support 논리 단위다. 비-default Cargo feature(`proptest-support`)로 게이트되어 릴리스 런타임 바이너리에 포함되지 않는다(NFR-Req 확정).
- **책임**: 유효/무효 `WatcherConfig`(절대 `vault_path`, https `server_endpoint`, `log_level` 열거, `notify_consecutive_failures` 경계, 미지 키/오타 주입), 유니코드·개행 포함 `detail` 문자열, `ErrorClass`/`TransportErrorClass` 전 변이 열거, 경계·대값 `ByteCount`/오프셋, 빈/단일/다수 엔트리 `Manifest`, 정규화 규칙 만족 `RelativePath` 제너레이터 제공.
- **의존 방향**: `ProptestGenerators -> {CoreTypes.*, ConfigProvider.*}`(테스트가 런타임 타입을 참조). 런타임 컴포넌트는 이 단위에 의존하지 않으므로 §4 런타임 DAG를 오염시키지 않는다.
- **이월**: 구체 구현·shrinking·고정 시드·CI 통합(PBT-08)은 Code Generation / Build-and-Test 이월.

---

## 6. RESILIENCY-01 매핑 — U0 = Critical (DAG 루트)

- **분류**: U0 `foundation`은 **Critical**이다 — 전 단위(U1~U7 + `watcher-bin`)가 U0 값 타입·코덱·config·싱크 계약에 의존하는 **의존 DAG 루트**다(component-dependency.md 빌드 순서 Foundation -> U1 -> U4 -> U5 -> U2 -> U3 -> U6 -> U7). U0의 회복력 계약이 무너지면 전 데몬이 무너진다.
- **회복력 기여(순수 lib 범위)**:
  - **무손실 코덱(U0-NFR-REL-01, Codec)** — U4 zero-loss 지속(RPO=0)의 기반. 크래시/재시작 후 디스크 복구 상태가 원값과 정확히 동일함을 보증(R-CODEC-01).
  - **panic-free-total(U0-NFR-REL-02, Codec/PathNormalizer/ErrorTaxonomy)** — `decode`가 U4 크래시 복구 핫 경로에서 패닉하면 데몬이 재시작마다 재-크래시해 keep-last-good/zero-loss를 무력화하므로, 순수 표면 lint-gate가 REL-05 회복력을 지탱.
  - **reload 원자성 + keep-last-good(U0-NFR-REL-03/REL-05, ConfigStore)** — 검증 실패 시 last-good 유지, 부분 적용 없음(nginx식 fail-safe).
  - **관찰자 팬아웃 격리(U0-NFR-REL-04, ObserverRegistry)** — 관찰자 K개 중 하나가 패닉해도 `catch_unwind`로 격리, 나머지 K-1개 계속.
- **N/A 범위**: RTO/RPO 수치·DR/HA·서킷브레이커·auto-scaling·배포/롤백·카오스/DR 테스팅(RESILIENCY-02/05~14)은 순수 lib에 부적용. U0 신규 인프라 통제 없음(`nfr-design-patterns.md` §7/§8과 일관).
- **fine-grained 맵이 추적성을 극대화하는 방식(Q1=A 근거)**: 공개 2 애그리게이트를 12개 명명 논리 컴포넌트로 전개함으로써, Critical 공유 크레이트에 대해 (NFR ID -> 규칙 ID -> **명명 컴포넌트** -> Testable Property)의 4단 추적 사슬이 성립한다. 하류 단위가 U0의 어느 논리 컴포넌트에 의존하는지(예: U4 -> Codec/SyncStateModel, U5 -> TokenSecret/ErrorTaxonomy/SinkContracts, U6 -> SinkContracts/StatusVocab)를 컴포넌트 단위로 지목할 수 있어, 회복력 계약 변경의 영향 반경(blast radius)이 애그리게이트가 아니라 컴포넌트 입도로 추적된다.

---

## 7. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 산출물 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **준수 — blocking 없음** | §5가 각 논리 컴포넌트를 확정 속성(PROP-DE-*/PROP-BR-*/PROP-BL-*)에 정렬(PBT-01), ProptestGenerators를 test-support 논리 단위로 문서화(PBT-07). 신규 PBT 결정 없음(PBT-09는 NFR-Req 충족). PBT-08은 Code Generation/Build-and-Test 이월 |
| **Resiliency Baseline** | ON | **준수(부분 적용 + 대체로 N/A) — blocking 없음** | §6이 U0 = Critical·DAG 루트 분류와 하류 의존 매핑을 컴포넌트 입도로 확정. REL-01~05 계약의 컴포넌트 안착 명시. RTO/RPO/DR/HA/서킷브레이커/카오스는 순수 lib에 N/A. 신규 resiliency 결정 없음 |
| **Security Baseline** | OFF | **N/A — 미로딩·미강제** | 확장 OFF, RISK-01/02 수용. TokenSecret redaction(SEC-02)·UrlValidator https(SEC-01)는 이미 채택된 잔존 위생/통제의 컴포넌트 배치일 뿐 신규 강제 통제 아님 |

**블로킹 판정**: 이 산출물에 blocking finding 없음. §2~§3 컴포넌트 안착은 `nfr-design-patterns.md` §9 맵과 1:1이며, §4 의존 그래프는 비순환(U0 = DAG 루트, sub-unit 무-import)이다.

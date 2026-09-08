# U0 Foundation — Tech Stack Decisions (기술 스택 결정)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U0 Foundation** -> NFR Requirements -> 산출물 2/2 (`tech-stack-decisions.md`)
**작성일**: 2026-09-08
**크레이트**: `foundation` (lib) · **소속 컴포넌트**: `CoreTypes`, `ConfigProvider`
**입력 아티팩트**: `domain-entities.md` · `business-rules.md` · `business-logic-model.md`(U0 Functional Design, 승인 2026-09-08T08:10:00Z) · `u0-foundation-nfr-requirements-plan.md` §3 확정 답변(Q1~Q13) · `requirements.md`(§5 NFR / §12·§13 오버레이) · 활성 확장 `property-based-testing.md`(**PBT-09**) · `resiliency-baseline.md`
**규칙**: `construction/nfr-requirements.md` Step 6 · `common/content-validation.md`(Unicode 박스 문자 미사용, 화살표 표기 `A -> B`, 표/코드블록 검증)

> **문서 성격**: 이 문서는 **구체 크레이트/툴체인 선택을 기록하는 유일한 산출물**이다. 자매 산출물 `nfr-requirements.md`가 카테고리별 NFR(기술중립)을 정의하고, 이 문서는 그 NFR을 실현할 **구체 기술 결정**과 각 결정의 **근거·전파 범위**를 확정한다. English 식별자(크레이트명·타입명·config 키·rule ID·NFR ID)는 원문 그대로 유지한다. Rust 제네릭/타입은 렌더링을 위해 백틱으로 감싼다(예: `Arc<WatcherConfig>`).

---

## 0. 전파 원칙 (U0 = Wave-1 DAG 루트)

U0 `foundation`은 의존성 DAG의 **루트(Wave-1)** 이며, U1~U8 전 크레이트가 U0에 의존한다. 따라서 이 문서에서 확정하는 **언어·edition·MSRV·크레이트·PBT 프레임워크 선택은 워크스페이스 전역 기본값**이 되고, 가능한 경우 워크스페이스 `Cargo.toml`의 `[workspace.package]`(edition/rust-version) 및 `[workspace.dependencies]`(공용 크레이트 버전 핀)를 통해 **상속(inherit)** 된다. 하위 단위는 `crate.workspace = true`로 버전을 물려받아 **드리프트 없이** 동일 스택을 재사용한다.

- **전파 표기 규약**: 각 결정 절 말미에 "전파 범위"를 명시한다(`U0 내부 한정` / `U1~U8 전역` / `특정 소비 단위`).
- **버전 정책**: 내부 크레이트는 단일 배포 바이너리 구성(UQ-2=B, RESILIENCY-01)이므로 `publish=false` + path 의존이며 semver 의례가 없다. 외부 크레이트는 `[workspace.dependencies]`에서 한 곳에 핀하고 CI가 MSRV 대비 검증한다(Build-and-Test 이월).
- **N/A 카테고리(참고, 결정 없음)**: 확장성/가용성/성능 수치 목표/DR·RTO·RPO/config 외 사용성/보안 강제 통제는 U0에서 N/A로 확정(근거는 `nfr-requirements.md`). 이 문서는 이들에 대한 기술 선택을 만들지 않는다.

---

## 1. 언어 / 툴체인 (Q5=A, NFR-17)

| 항목 | 결정 | 근거 |
|---|---|---|
| 언어 | **Rust** | NFR-17 확정(결정적 코어를 Rust로 직접 구현; okc-core 의존성 0, FQ-1=A/requirements §12.1). PBT-09 기본값 `proptest`의 전제. |
| Edition | **2024** | Q5=A. Rust 1.85부터 안정. greenfield라 레거시 back-compat 부담이 없어 2021 대비 개선을 포기할 이유가 없음. |
| MSRV | **고정(pinned)** — `[workspace.package].rust-version` | Q5=A. 고정 MSRV가 NFR-05 크로스플랫폼(macOS/Windows/Linux) **재현 가능 빌드**를 보장. 부동 툴체인은 하위 단위가 부지불식 하한을 올려 3-OS 패키징 드리프트를 유발. |

**메커니즘**: 워크스페이스 루트 `Cargo.toml`의 `[workspace.package]`에 `edition = "2024"`와 `rust-version = "<pinned MSRV>"`를 두고, 모든 멤버 크레이트가 `edition.workspace = true` / `rust-version.workspace = true`로 상속한다. Edition 2024는 MSRV 하한을 1.85 이상으로 강제하므로 둘은 정합해야 한다.

**이월**: MSRV 대비 CI 검증(예: MSRV 툴체인으로 `cargo check`)의 구체 파이프라인 통합은 **Build-and-Test 단계 이월**. 이 문서는 정책만 확정한다.

**전파 범위**: **U1~U8 전역** — edition/MSRV는 워크스페이스 상속으로 전 크레이트에 동일 적용.

---

## 2. 직렬화 (Q1=A, 설정 부분은 확정 사항)

U0에는 **두 개의 완전히 별개인 직렬화 경로**가 있으며 혼용하지 않는다(`business-logic-model.md` §1.1, `domain-entities.md` §3, Q1=B DECIDED-EARLIER):

- **경로 A — 사람이 편집하는 설정 파일**: 포맷 = **JSON**(고정, nginx식). 크레이트 = **`serde_json`**.
- **경로 B — 데몬 내부 지속 상태**: 포맷 = **CBOR**(serde 호환 바이너리). 크레이트 = **`ciborium`**(Q1=A).

### 2.1 내부 지속 상태 CBOR 코덱 = `ciborium` (Q1=A)

`CoreTypes`의 코덱 시그니처는 이미 serde 기반으로 확정되어 있다(`business-logic-model.md` §1.3):

```
fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, CodecError>
fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, CodecError>
```

R-CODEC-01(무손실 round-trip `decode(encode(v)) == v`, `business-rules.md` §1)이 핵심 계약인 이 코덱의 CBOR 백엔드로 **`ciborium`** 을 채택한다.

**후보 비교 (Q1 근거):**

| 크레이트 | serde 호환 | 유지보수 | 시그니처 적합성 | 판정 |
|---|---|---|---|---|
| **`ciborium`** | 예(`serde::Serialize`/`DeserializeOwned` 그대로) | 활발히 유지, 순수 Rust | 확정된 `encode<T: Serialize>`/`decode<T: DeserializeOwned>` 시그니처에 무변경 적합 | **채택** |
| `minicbor` | 아니오(비-serde 파생) | 유지됨 | 확정 serde 시그니처를 폐기하고 타입마다 `#[cbor]` 주석 필요 | 기각(시그니처 재작성) |
| `serde_cbor` | 예 | **유지 중단(archived/deprecated)** | 시그니처는 맞으나 공급망 리스크 | 기각(DAG 루트 크레이트에 부적합) |

**채택 근거(Q1=A)**: (a) 확정된 serde 시그니처에 그대로 맞아 타입 재작성이 없고, (b) 순수 Rust로 크로스플랫폼(NFR-05) 재현 빌드에 유리하며, (c) 자기기술적(self-describing) CBOR라 자동 업데이트 후 상태파일 **스키마 진화**(새 필드 추가/무시)에 강하다(Q1=B 근거와 정합, `domain-entities.md` §3). (d) 유지보수 중단된 `serde_cbor`의 공급망 리스크를 회피한다.

**코덱 지속 대상 타입**(R-CODEC-01 적용, `business-rules.md` §1 / `business-logic-model.md` §1.2): `Manifest`(+`ManifestEntry`·`RelativePath`·`Sha256Digest`·`ManifestDigest`) · `SyncState` · `Timestamp` · `ClassifiedError`/`TransportError`/`TransferResult` · 하위 단위 지속 레코드 `ConsentGrant`[U5] / `UploadHistoryRecord`[U6] · 재개 오프셋.

**성능 계약(정성)**: 코덱 최대 페이로드는 100k 엔트리 `Manifest`이며, 처리 비용은 **엔트리 수에 선형·유계**라는 정성 계약만 둔다(수치 throughput/latency/peak-memory 게이트 없음 — N/A 근거는 `nfr-requirements.md`). 스트리밍 해시 메모리 바운드(NFR-02)는 U1 소관.

**전파 범위**: 코덱 자체는 **U0 소유**. `ciborium` 의존은 **U4/U5/U6**(지속 파일 I/O 수행 단위)로 전파된다. 실제 파일 열기/원자적 쓰기/복구는 U4/U5/U6 소관이고 U0는 순수 바이트 변환만 담당(I/O 없음).

### 2.2 설정 파일 = 사람 편집 JSON via `serde_json` (확정 사항)

설정 파일은 사람이 읽고 편집해야 하므로 CBOR가 아니라 **JSON**을 유지한다(Q1=B DECIDED-EARLIER, 재결정 아님). 파싱/역직렬화 크레이트는 **`serde_json`** 이다. strict 미지 키 검증 메커니즘은 §3 참조.

**전파 범위**: 런타임 JSON 파싱은 **U0/`watcher-bin` 한정**(설정 로드 시점). 단, 설정 스키마는 federated(§3)이므로 필드 스키마 정의 자체는 U1~U7 각 소비 섹션으로 확장된다.

---

## 3. config 검증 파싱 = `serde_json` 2-패스 Value (Q2=A)

Q4=B(알 수 없는 config 키 = strict reject, DECIDED-EARLIER)이고 R-CFG-STRICT-01(`business-rules.md` §4.2)은 **"발견된 모든 알 수 없는 키를 나열(첫 키에서 중단하지 않음)"** 을 명시한다. 이 전수 나열을 구현하는 메커니즘으로 **`serde_json` 2-패스**를 채택한다(Q2=A).

**2-패스 알고리즘:**

```
1) 1차 파스: JSON 텍스트 -> serde_json::Value
2) 키 수집:  알려진 키 집합(known-key set) 밖의 키를 전부 수집·나열(위치 포함)
3) 분기:
   - 미지 키 존재 -> reject (수집한 전체 목록 + JSON 위치를 구조화 메시지로 반환)
   - clean       -> 2차 파스: 타입드 WatcherConfig 로 역직렬화
```

### 3.1 `#[serde(deny_unknown_fields)]`가 불충분한 이유

- **전수 나열 불가**: `deny_unknown_fields`는 **첫 미지 필드에서 중단**한다 -> R-CFG-STRICT-01의 "모든 미지 키 나열"(사용자가 한 번에 여러 오타 교정)을 위반한다.
- **federated 스키마 비호환**: 설정은 하위 단위 소유 섹션(`debounce_ms`[U2] · `reconciliation_interval_s`[U2] · `exclude_patterns`[U1] · `chunk_threshold_bytes`[U3] · `backoff`[U4] · `request_timeout_s`[U5] · `log_rotation`[U6] · `service`[U7a] 등, `domain-entities.md` §2 각주)을 가진 통합 스키마다. 단일 모놀리식 struct에 `deny_unknown_fields`를 걸면 이들 섹션과 맞지 않는다.

### 3.2 federated known-key-set 메커니즘

알 수 없는 키 판정은 **전체 통합 스키마 기준**으로 적용된다(`business-rules.md` §4.1 각주, `domain-entities.md` §2.1). 따라서 known-key set은 **연합(federated)** 구조다 — U0가 자기 핵심 키(`vault_path` · `server_endpoint` · `token` · `secure_store_enabled` · `log_level` · `notify_consecutive_failures`)를 등록하고, **각 하위 단위가 자기 섹션 키를 known-key set에 등록**한다. 2-패스는 이 통합 집합 밖의 키를 전부 수집한다. SafetyLimits는 config 필드가 **아니므로** 대응 키가 오면 미지 키로 거부된다(R-LIMIT-01, `business-rules.md` §4.3).

### 3.3 구조화 오류 메시지 (Q12=A, 범위 절제)

검증 실패 메시지는 **구조화 메시지만** 채택한다 — **필드명 + 기대 형식/범위 + JSON 위치 포인터**. 2-패스가 이미 위반 키와 위치(`serde_json::Value` 파싱 시)를 알고 있으므로 near-free로 파생된다. **did-you-mean 편집거리 제안(예: `vault_paht` -> `vault_path`?)은 범위 절제를 위해 이월**한다(§12 참조) — fuzzy-match 크레이트/Levenshtein 코드를 지금 도입하지 않는다.

**전파 범위**: 2-패스 검증기는 **U0 `ConfigProvider` 소유**. known-key set 등록 계약은 **U1~U7 전역**(각 단위가 자기 섹션 키를 등록). `serde_json` 의존은 §2.2와 동일 범위.

---

## 4. URL 검증 = `url` 크레이트 (Q3=A)

`server_endpoint`는 "유효 URL 형식이며 스킴은 `https`만"이어야 한다(`business-rules.md` §4.1, `domain-entities.md` §2). **https-only reject는 이미 확정된 검증 규칙**(NFR-06 TLS 강제, §13 오버레이)이며 재결정 대상이 아니다 — 이 절이 확정하는 것은 **"유효 URL 형식"을 검증하는 파서/깊이**다.

**결정(Q3=A)**: **`url` 크레이트**로 완전 파싱한 뒤 스킴 `== "https"`를 단언한다.

```
config load 시점:
  server_endpoint(문자열) -> url::Url::parse -> (파싱 성공?) -> scheme == "https" 단언
    - 파싱 실패 또는 scheme != https -> 검증 실패(reject)
```

**채택 근거(Q3=A)**: (a) WHATWG 준수 완전 파싱으로 기형 authority/포트를 **로드 시점에 정밀 거부**해 오류 국소성을 확보한다(수기 `starts_with("https://")`는 기형 호스트를 통과시켜 실패를 U5 연결 시점으로 미룸). (b) 파싱된 `url::Url` 객체를 **U5 `AuthTransport`의 base-URL join에 재사용**하므로 하위 단위가 재파싱하지 않는다.

**전파 범위**: 검증은 **U0 `ConfigProvider` 소유**. 파싱된 base URL은 **U5**로 전파(AuthTransport base-URL join 재사용). `url` 의존은 워크스페이스 공용 의존.

---

## 5. 오류 처리 (Q4=A)

U0는 워크스페이스 전역 오류 계약을 정의한다. 오류 타입을 두 부류로 나눠 다르게 파생한다(Q4=A):

| 부류 | 타입 | 파생 전략 | 근거 |
|---|---|---|---|
| **운영 오류(반환용)** | `CodecError` · `ConfigError` · `CredentialError` | **`thiserror`** 파생(`Display`/`std::error::Error`) | `Result`로 반환되는 라이브러리 오류. `thiserror`가 라이브러리 관용·보일러플레이트 최소·워크스페이스 전역 일관을 제공. |
| **분류 값 타입(직렬화용)** | `ErrorClass` · `ClassifiedError` · `TransportError`(+`TransportErrorClass`) | **순수 serde enum/struct 유지**(thiserror 미사용) | 이들은 **CBOR round-trip 대상**(R-CODEC-01, `domain-entities.md` §1.4)이며 throw되지 않는 **값**이다. U4/U6가 소비. |

**`anyhow`를 쓰지 않는 이유(Q4=A)**: `anyhow`식 타입소거는 R-CLASS-01/02(`business-rules.md` §3)의 분류가 의존하는 **구조화 변이**(`ErrorClass::{Retryable, AuthAborted, Backpressure, Fatal}`)를 소실시킨다. `is_retryable()`가 전(total) 함수로 성립하려면(R-CLASS-03) 변이가 타입 시스템에 보존돼야 하므로 파운데이션 오류 계약에 부적합하다.

**패닉-프리 total 계약과의 연계(Q8=A)**: U0 순수 표면(`encode`/`decode`·정규화·오류 분류)은 절단·손상·적대적 CBOR를 포함한 **모든 입력에 대해 패닉 없이 `Result`로 표면화**한다(unwrap/슬라이스 패닉 금지). `decode`는 U4 크래시 복구 핫 경로에 놓이므로(`business-logic-model.md` §1.4) 여기서의 패닉은 재시작 시 데몬을 재-크래시시켜 keep-last-good/zero-loss를 무력화한다. 검증은 U0 신뢰성 불변식 + PBT no-panic 속성(§8)으로 수행한다. 이 계약은 `thiserror`/serde 파생과 무관한 **구현 규율**이며 코덱 백엔드(`ciborium`) 오류를 반드시 `CodecError`(Fatal)로 매핑한다.

**전파 범위**: 운영 오류의 `thiserror` 파생 관례와 분류 값 타입 계약은 **U1~U8 전역**(하위 단위가 자기 운영 오류를 `thiserror`로 파생, 분류 값 타입은 U0 것을 소비). `thiserror` 의존은 워크스페이스 공용.

---

## 6. 동시성 프리미티브 = `arc_swap::ArcSwap<Arc<WatcherConfig>>` (Q6=A, Q7=A)

`current()`는 데몬의 여러 스레드(U2 `FilesystemWatcher`, U8 `ControlPlane` IPC, 직렬 동기화 사이클)가 동시에 읽고, 그 사이 CLI `reload`가 R-RELOAD-02(원자적 all-or-nothing 스왑, `business-rules.md` §5)를 수행한다. Functional Design은 스레딩 메커니즘을 기술중립으로 이월했다(`business-logic-model.md` §4.3).

### 6.1 스냅샷/스왑 = `ArcSwap` (Q6=A)

**결정**: 활성 config 스냅샷을 **`arc_swap::ArcSwap<Arc<WatcherConfig>>`** 로 보관한다.

- **`current()`**: 락-프리 읽기 — 값싼 `Arc` 로드(구독/읽기 스레드가 블로킹되지 않음).
- **`reload()` 성공 분기**: 검증 완료된 새 `Arc<WatcherConfig>`로 **원자적 포인터 스왑**. 포인터 교체가 곧 all-or-nothing이므로 R-RELOAD-02(부분 적용 금지)를 **직접 충족**한다. 구독자는 항상 **완결된 NEW 스냅샷만** 관측한다(`business-logic-model.md` §4.3 원자성).

**후보 대비(Q6 근거)**: `RwLock<Arc<WatcherConfig>>`는 스왑 동안 읽기 스레드 블로킹 및 writer starvation 여지가 있고, `Mutex<...>`는 핫 읽기 경로에서 동시 읽기가 경합한다. `ArcSwap`은 락-프리 읽기로 이 둘을 회피한다.

### 6.2 관찰자 팬아웃 격리 = per-observer `catch_unwind` (Q7=A)

성공 스왑 직후(R-OBSERVER-01) `ConfigProvider`는 등록된 관찰자(U5 `CredentialProvider`, U6 `StructuredLogger`)에 결정적으로 팬아웃한다(R-OBSERVER-03, `business-rules.md` §8). 스왑은 **이미 원자적으로 커밋**된 상태다.

**결정(Q7=A)**: 관찰자별 **격리(`catch_unwind` + 로그)** 후 나머지 관찰자에 계속 팬아웃한다.

- 스왑이 이미 커밋됐으므로 패닉 관찰자가 `current()`나 다른 관찰자를 오염시킬 수 없다.
- 관측의 **best-effort·infallible** 성격(`domain-entities.md` §1.6, `business-logic-model.md` §5.2)과 R-OBSERVER-03 결정성·격리에 정합한다.

**전파 범위**: 스냅샷/스왑·팬아웃 격리는 **U0 `ConfigProvider` 내부 한정**. 다만 락-프리 `current()` 읽기 패턴의 이점은 **U2/U8**(핫 읽기 스레드)에 파급되고, 관찰자 계약(`ConfigReloadObserver`)은 **U5/U6**가 구현한다. `arc_swap` 의존은 U0 내부이며 하위 단위는 `current()`가 반환하는 스냅샷만 소비한다.

---

## 7. 속성 기반 테스트 (PBT-09 강제 결정) = `proptest` (Q9=A, Q10=A)

> **이 절은 PBT-09 강제(blocking) 의무의 정식 기록이다.** PBT-09(프레임워크 선택)는 Functional Design에서 이 NFR Requirements 단계로 명시적으로 이월된 강제 의무이며(property-based-testing.md "Enforcement Integration" 표 · requirements §5.5/§2.3 · Q3=A Full), tech-stack-decisions에 문서화하고 dev-dependency로 추가해야 한다.

### 7.1 프레임워크 = `proptest` (Q9=A)

**결정**: 워크스페이스 PBT 프레임워크로 **`proptest`** 를 채택한다.

PBT-09 요구(custom generators·automatic shrinking·seed-based reproducibility·test runner 통합)를 모두 충족한다:

| PBT-09 요구 | `proptest` 충족 |
|---|---|
| 도메인 타입 커스텀 제너레이터 | 매크로 기반 `Strategy` 제너레이터 — `RelativePath`(정규화 만족)·상관 매니페스트 쌍·유니코드 `detail`·`ErrorClass`/`TransportErrorClass` 전 변이 등 제약 도메인에 적합 |
| 자동 shrinking | 강한 자동 shrinking(실패 입력을 최소 재현 케이스로 축소) |
| 시드 재현성 | 실패 시 시드 로깅으로 정확 재현 가능 |
| 테스트 러너 통합 | `cargo test` 네이티브 통합 |

**`quickcheck` 대비(Q9 근거)**: `quickcheck`는 `Arbitrary` 트레이트 기반 경량이나 shrinking이 약하고, 제약 있는 도메인 제너레이터 작성이 번거로우며, 시드 재현/CI 통합 편의가 낮다. `proptest`가 NFR-17의 기본값("core가 Rust로 확정되면 proptest")과도 정합한다.

### 7.2 제너레이터 노출 = `proptest-support` 비기본 cargo feature (Q10=A, PBT-07)

PBT-07(제너레이터 재사용성)에 따라 U0가 정의하는 도메인 타입(`RelativePath`·`Manifest`·`Timestamp` 등)의 제너레이터를 **단일 출처**로 두고 하위 단위가 재사용한다.

**결정(Q10=A)**: `foundation`에 **비기본(non-default) cargo feature `proptest-support`** 로 제너레이터 모듈을 노출한다. 하위 크레이트는 자기 dev-dependency에서 이 feature를 켜 재사용한다.

- **드리프트 방지**: 제너레이터 정의가 한 곳(U0)에만 존재 -> 단위 간 정의 갈라짐 없음.
- **feature 게이트**: 비기본 feature이므로 **proptest가 비테스트/프로덕션 빌드에 유출되지 않는다**(`proptest`는 `proptest-support` feature 하의 optional dependency).

재사용 소비 단위(각 산출물의 PBT 섹션 근거): **U1**(`ContentAddressing` 다이제스트 결정성 / `ManifestDiffer` diff 오라클, `business-logic-model.md` §6.2) · **U4**(`SyncStateStore` stateful PBT, `domain-entities.md` §5.4 / PBT-06) · **U5/U6**(지속 레코드 round-trip).

### 7.3 시드 재현성 및 이월(PBT-08)

**시드 재현성**: `proptest`는 실패 시 시드를 기록하고 회귀 파일(`proptest-regressions`)로 최소 실패 케이스를 고정할 수 있다 — 정확 재현을 보장한다.

**이월(PBT-08 상세)**: 케이스 수·shrink 튜닝·고정 시드 vs 시드 로깅 정책·CI 파이프라인 통합은 **Code Generation / Build-and-Test 단계 이월**이다(property-based-testing.md "Enforcement Integration": PBT-08은 Build and Test 적용). 이 문서는 프레임워크 선택(PBT-09)과 제너레이터 노출 방식(PBT-07, Q10)만 확정한다.

**전파 범위**: **U1~U8 전역** — `proptest`는 워크스페이스 공용 **dev-dependency**, `proptest-support` feature는 제너레이터를 필요로 하는 하위 단위(U1/U4/U5/U6)가 dev에서 활성화.

---

## 8. 보안 위생 타입 = 자체 redacting newtype (Q11=A)

`WatcherConfig.token`은 평문 String이 **1차·기본 저장 위치**다(Q7=A/§13 오버레이, DECIDED-EARLIER). Security Baseline은 **OFF**이고 RISK-01은 **수용**됐다(requirements §7/§13). 그럼에도 U6 `StructuredLogger`가 config/오류를 직렬화하거나 `WatcherConfig`가 우발적으로 Debug-print될 때 토큰이 로컬 평문 로그에 새는 것은 **별개의 저비용 위생 문제**다.

**결정(Q11=A)**: `token` 필드를 **자체 구현 redacting newtype**로 감싼다.

- `Debug`/`Display`: 실제 값 대신 `***`를 출력(로그·오류 캡처 유출 차단).
- 실제 값 접근: `.expose()` 메서드로만.
- `Serialize`/`Deserialize`: **실제 값을 보존**하여 config round-trip(PROP-BR-02, `business-rules.md` §9.2)을 깨뜨리지 않음.
- 규모: 약 15~20줄, **외부 의존 0**.

**`secrecy` 크레이트를 쓰지 않는 이유(Q11 근거)**: `secrecy`의 `SecretString`은 검증된 패턴이나 (a) 외부 의존을 추가하고 (b) 기본 Serde를 구현하지 않아 config round-trip 글루가 별도로 필요하다. 자체 newtype이 더 싸고 round-trip 요구에 직접 맞는다.

**성격 명시**: 이는 강제 보안 통제가 아니라 **저비용 방어적 위생**이다. Security Baseline OFF / RISK-01 수용 하에서 유일한 잔존 통제는 TLS(NFR-06)이며(§4의 https-only 검증), 이 redaction은 그와 별개의 로그-유출 완화일 뿐이다.

**전파 범위**: newtype 정의는 **U0 소유**. **U5**(토큰 해소 `resolve_token`) · **U6**(로그/오류 직렬화)로 전파 — 두 단위가 이 타입 계약을 준수한다.

---

## 9. 문서 / 품질 린트 (Q13=A)

`foundation`은 전 단위가 의존하는 **공유 계약 크레이트**다.

**결정(Q13=A):**

| 항목 | 결정 | 근거 |
|---|---|---|
| 커버리지 게이트 | **전역 커버리지 % 게이트 없음** | 순수 값 타입 크레이트에 정량 % 게이트는 과함. 실질 검증은 PBT + 예제 앵커(PBT-10)로 확보. |
| 공개 API 문서 | **`#![deny(missing_docs)]` on foundation public items** | 공유 계약 크레이트의 공개 표면 문서화를 강제(rustdoc). |

**이월**: CI 커버리지 통합의 구체 방식은 **Build-and-Test 이월**. PBT-10(예제 기반 테스트 보완)은 Code Generation에서 property test와 함께 example 테스트를 병행하는 형태로 적용.

**전파 범위**: `#![deny(missing_docs)]`는 **U0 `foundation` 공개 항목 한정**(공유 계약 크레이트). 하위 단위는 자기 정책을 따르되 U0 계약 문서를 소비. 커버리지 정책(게이트 없음)은 워크스페이스 기본값 참고.

---

## 10. 의존성 요약표

**범례**: version-policy = 외부 크레이트는 `[workspace.dependencies]`에서 핀 후 상속(inherit), 내부는 `[workspace.package]` 상속 + `publish=false` + path 의존. kind = normal(런타임) / dev(테스트). feature = 특기 사항. propagation = U1~U8 전파 범위.

| crate | version-policy | kind | feature | propagation (U1~U8) | grounding |
|---|---|---|---|---|---|
| `serde` | workspace-inherited (핀: `1`) | normal | `derive` | **U1~U8 전역**(전 지속/설정 타입이 파생) | Q1=A/Q4=A serde 시그니처 · `business-logic-model.md` §1.3 |
| `ciborium` | workspace-inherited (핀: `0.2`) | normal | — | **U4/U5/U6**(내부 상태 지속 I/O) | Q1=A · R-CODEC-01 (`business-rules.md` §1) |
| `serde_json` | workspace-inherited (핀: `1`) | normal | — | **U0/`watcher-bin`**(설정 파싱) + federated 키 등록 U1~U7 | Q1=B(설정=JSON) · Q2=A 2-패스 · R-CFG-STRICT-01 |
| `url` | workspace-inherited (핀: `2`) | normal | — | **U5**(파싱된 base URL 재사용) | Q3=A · NFR-06 (`business-rules.md` §4.1) |
| `thiserror` | workspace-inherited (핀: `2`) | normal | — | **U1~U8 전역**(운영 오류 파생 관례) | Q4=A · R-CLASS-01/02 |
| `arc_swap` | workspace-inherited (핀: `1`) | normal | — | **U0 내부 한정**(읽기 이점은 U2/U8) | Q6=A · R-RELOAD-02 |
| `proptest` | workspace-inherited (핀: `1`) | **dev** | `proptest-support`(비기본, U0 제너레이터 노출) | **U1~U8 전역**(dev), 제너레이터 재사용 U1/U4/U5/U6 | **PBT-09**/Q9=A · PBT-07/Q10=A |
| 내부 크레이트(`foundation` 등) | `[workspace.package]` 상속, `publish=false`, path 의존 | normal | 단위별 lib | **U1~U8 전역**(단일 배포 바이너리) | UQ-2=B / RESILIENCY-01 · Q5=A |

> **버전 핀 주석**: 위 major/minor 핀은 `[workspace.dependencies]`에 기재하는 대표 계열이며, 정확한 patch 핀과 MSRV(Edition 2024, Rust 1.85+) 정합 검증은 Build-and-Test에서 수행한다. `secrecy`·fuzzy-match(Levenshtein) 크레이트는 **의도적으로 미채택**(§8, §12).

**내부 크레이트 버전 관리(참고)**: 워크스페이스는 단위별 lib 크레이트 + 얇은 `watcher-bin`으로 구성되고 **단일 배포 바이너리**로 산출된다(UQ-2=B / RESILIENCY-01, DECIDED-EARLIER). 내부 크레이트는 `publish=false` + path 의존 + `[workspace.package]` 버전 상속이므로 **semver 의례가 없다**.

---

## 11. 범위 절제 기록 (Scope Trims / Deferrals)

이 문서에서 **의도적으로 도입하지 않은** 항목과 그 이월처:

| 절제 항목 | 결정 | 근거 / 이월처 |
|---|---|---|
| **did-you-mean 편집거리 제안** (Q12) | **이월(도입 안 함)** | 사용자 지시 "구현범위가 너무 커지지 않는 한". 구조화 메시지(필드명+기대형식+JSON 위치)만 채택(§3.3). fuzzy-match 크레이트/Levenshtein 코드 미도입. **Code Generation에서 선택적(optional) 추가 가능**. PROP-BR-05(`business-rules.md` §9.4)는 편집거리 제너레이터로 향후 테스트 가능. |
| **코덱 streaming API** | **buffered 확정(fork 없음)** | 참고 시그니처가 buffered(`encode -> Vec<u8>` / `decode(&[u8])`)로 확정(`business-logic-model.md` §1.3). U4 원자적 temp+rename가 완결 파일을 요구하므로 실질적 streaming fork 부재(DECIDED-EARLIER). |
| **PBT-08 상세**(케이스 수/shrink 튜닝/시드 정책/CI) | **이월** | Code Generation / Build-and-Test(property-based-testing.md Enforcement Integration). 이 문서는 PBT-09(프레임워크)+PBT-07(제너레이터 노출)만 확정(§7). |
| **MSRV CI 검증 파이프라인** | **이월** | 정책만 확정(§1). 구체 CI 통합은 Build-and-Test. |
| **커버리지 % 게이트 / CI 커버리지 통합** | **게이트 없음 확정 + 통합 이월** | Q13=A(§9). PBT + 예제 앵커(PBT-10)로 대체. 통합은 Build-and-Test. |
| `secrecy` 크레이트 | **미채택** | 외부 의존 + 기본 Serde 미구현(§8, Q11). 자체 newtype으로 대체. |

---

## 12. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 문서 적용 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제·준수 — PBT-09 충족** | §7이 PBT-09 프레임워크(`proptest`) 정식 결정 + dev-dependency 등재 + PBT-07 제너레이터 노출(`proptest-support` feature, Q10=A). PBT-08 상세는 Build-and-Test 이월(명시). blocking 해소. |
| **Resiliency Baseline** | ON | **부분 적용 + 대체로 N/A** | RESILIENCY-01: U0=Critical(전 단위 의존). §2.1 무손실 `ciborium` 코덱이 U4 zero-loss 지속(RPO=0, NFR-03)의 기반, §6 `ArcSwap` keep-last-good/원자 스왑이 config 회복력 실현. RTO/RPO 수치·DR·HA·배포/롤백은 U0(순수 lib)에 **N/A**(RESILIENCY-02, requirements §6). |
| **Security Baseline** | OFF | **N/A(잔존 표면화)** | 미로딩·미강제. RISK-01/02 수용. 유일 잔존 통제 TLS(https)는 §4 검증(파서/깊이만 이 문서 결정, https-only는 재결정 아님). 저비용 in-scope 잔존 결정은 토큰 redaction 위생(§8) 하나. |

**N/A NFR 카테고리(기술 선택 없음)**: 확장성 · 가용성 · 성능 수치 목표(코덱은 엔트리 수 선형·유계 정성 계약만, §2.1) · Resiliency DR/RTO/RPO · config 외 사용성 · 보안 강제 통제. 근거 상세는 자매 산출물 `nfr-requirements.md` §N/A 판정표.

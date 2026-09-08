# U5 Auth & Consent — Logical Components (논리 컴포넌트 분해 + 추적성 맵)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U5 Auth & Consent** -> NFR Design -> 산출물 2/2 (`logical-components.md`)
**작성일**: 2026-09-08
**크레이트**: `auth-consent` (lib) · **소속 컴포넌트(공개 애그리게이트)**: `AuthTransport`, `CredentialProvider`, `ConsentGate`
**입력 아티팩트**: 자매 산출물 `nfr-design/nfr-design-patterns.md`(§16 패턴 -> 논리 컴포넌트 안착 맵이 이 문서의 권위 근거) · U5 FD 3종(`domain-entities.md` §1~§5 · `business-rules.md` · `business-logic-model.md`) · `nfr-requirements/{nfr-requirements.md,tech-stack-decisions.md}` · `construction/u0-foundation/nfr-design/logical-components.md`(스타일 템플릿)
**규칙**: `construction/nfr-design.md` Step 6 · `common/content-validation.md`(ASCII 화살표 `A -> B`, 박스 문자 미사용, Rust 제네릭 백틱) · `common/ascii-diagram-standards.md` · 활성 확장 `property-based-testing.md`(ON, Full) · `resiliency-baseline.md`(ON)

> **문서 성격**: 이 문서는 FD가 확정한 **공개 3 컴포넌트**(`AuthTransport`·`CredentialProvider`·`ConsentGate`)를 상위 애그리게이트로 유지한 채, 그 내부의 **이미 확정된 기능(FD 타입·규칙·흐름)을 명명된 논리 컴포넌트로 전개**한다. 이는 발명이 아니라 확정 기능의 명명·정렬이며 FD 흐름에서 near-free로 파생된다. 각 논리 컴포넌트는 (a) 책임, (b) 공개/내부 인터페이스 표면, (c) 안착하는 확정 NFR 패턴(`nfr-design-patterns.md` §16 맵과 1:1), (d) U0 소비/seam 관계로 기술한다.
>
> **이것은 추적성 문서 맵이며 물리 모듈/크레이트 증식 강제가 아니다.** 논리 컴포넌트 -> 물리 파일/모듈 레이아웃 매핑은 **Code Generation에서 최소로** 결정된다(다수 논리 컴포넌트가 한 모듈에 상주 가능). 목적은 NFR·규칙·PBT 속성 -> 명명 단위 추적성 극대화와 크레이트 의존 엣지(`foundation`에만 의존) 비순환 확인이다.
>
> **표기 규약**: 화살표는 ASCII `A -> B`만 사용(유니코드 화살표 금지). 박스/선-그리기 문자 미사용. Rust 제네릭/타입/식별자(예: `Result<OkcResponse, TransportError>`, `Arc<dyn HttpTransport>`, `Mutex<ConsentInner>`, `Duration`, `TokenSecret`)는 백틱으로 감싼다.

---

## 1. 논리 컴포넌트 전개 개요

| 공개 애그리게이트 | 성격 | 논리 컴포넌트(확정 명명) |
|---|---|---|
| `AuthTransport` | 유일 아웃바운드 HTTP 경로 — 무상태 `Send + Sync` 전송 파이프라인 | TransportPipeline · RequestAssembler · ResponseClassifier · HttpTransportPort · UreqAdapter |
| `CredentialProvider` | 토큰 해소 — 라이브 스냅샷(캐시 없음) | TokenResolver · SecureStorePort · UnavailableSecureStore |
| `ConsentGate` | 동의 상태머신 + 원자 지속 + 게이트 | ConsentStateMachine · ConsentStore · DisclosureProvider |
| (런타임 그래프 밖) | 테스트 전용 — 비-default `proptest-support` feature | ProptestGenerators (test-support 논리 단위, §5.2) |

---

## 2. `AuthTransport` 애그리게이트 — 논리 컴포넌트

> `AuthTransport`는 Watcher -> 서버 **유일 아웃바운드 HTTP 경로**다. 자체 가변 상태가 없고 주입 의존만 보관하는 `Send + Sync` 파이프라인이다(nfr-design-patterns §4). 공개 메서드: `send(req: OkcRequest) -> Result<OkcResponse, TransportError>`.

### 2.1 TransportPipeline
- **책임**: `send`의 오케스트레이션 — 토큰 해소(`CredentialProvider`) -> URL 확정 -> TLS 가드 -> `RawHttpRequest` 조립 -> `HttpTransport.execute`(1회 시도) -> 응답/실패 분류 순서를 구동한다(business-logic-model §1.2). 재시도/백오프·프로토콜 해석은 하지 않는다(경계: 재시도 U4, 프로토콜 U3).
- **공개/내부 인터페이스 표면**: `send(OkcRequest) -> Result<OkcResponse, TransportError>`. 주입 보관: `Arc<CredentialProvider>` · `Arc<dyn HttpTransport>` · `Arc<dyn ConfigProvider>`(server_endpoint 조회) · `Duration`(U8 주입 데드라인).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §2 요청 데드라인 부착 + graceful degrade(**U5-NFR-REL-02**, R-AT-TO) + §4 무상태 `Send + Sync` 동시성(락 불필요, 재진입 안전) + §11 관측 push 경계(**U5-NFR-USE-02**, 값 반환 1차).

### 2.2 RequestAssembler
- **책임**: 논리 `OkcRequest`를 저수준 `RawHttpRequest`로 확정 — `server_endpoint` base + `path` -> 절대 URL, **TLS 가드**(스킴 `https` 아니면 seam 호출 전 거부), **전송 시점 토큰 헤더 주입**(`OkcRequest.headers` 아님), 주입 데드라인을 `timeout` 필드에 부착. 데드라인 없는 요청·토큰 원문 유출을 구조적으로 차단.
- **공개/내부 인터페이스 표면**: 내부 조립 `(OkcRequest, TokenSecret, Duration) -> RawHttpRequest`(+ TLS 가드 판정). base URL은 U0 `validate_https_url`가 파싱한 `url::Url` 재사용.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §8 TLS/`https` 강제 이중 방어(**U5-NFR-SEC-01**, R-AT-TLS 전송 계층 가드) + §9 토큰 유출 위생(**U5-NFR-SEC-02**, R-AT-TOKEN 전송 시점 주입) + §2 데드라인 부착(**U5-NFR-REL-02**, R-AT-TO).

### 2.3 ResponseClassifier
- **책임**: `RawHttpResponse`(상태코드) 또는 `HttpError`를 U0 `TransportErrorClass`로 **분류만** 하는 순수 매칭 함수(2xx -> `Ok(OkcResponse)`, 그 외 -> 단일 `TransportErrorClass`). 범위 기반 total 매칭으로 미정의·미래 상태코드 흡수. `TransportErrorClass -> ErrorClass` 매핑·재시도 판정은 U0/U4 소유(비관여).
- **공개/내부 인터페이스 표면**: 순수 `classify(status: u16) -> TransportClassifyResult` · `classify_error(HttpError) -> TransportErrorClass`(참고 형상). 컴파일타임 clippy lint-gate 대상(`deny(unwrap_used/expect_used/indexing_slicing/panic)`).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §1 분류 total-function + panic-free(**U5-NFR-REL-01**, R-AT-CLASS-01/02/03 — clippy lint-gate + PBT no-panic, taxonomy 폐쇄성).

### 2.4 HttpTransportPort
- **책임**: 구체 HTTP/TLS 스택 결합을 회피하는 최소 전송 **포트 트레이트**. TLS 위에서만 전송하고 분류 이전 원시 결과만 반환한다. U3/PBT가 목 어댑터로 서버 없이 검증하는 seam.
- **공개/내부 인터페이스 표면**: `trait HttpTransport { fn execute(&self, req: RawHttpRequest) -> Result<RawHttpResponse, HttpError>; }`(`Send + Sync`). `RawHttpRequest`/`RawHttpResponse`/`HttpError`(`Timeout`/`Connect`/`Tls`/`Io`) 값 타입 동반.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §5 `HttpTransport` seam 포트/어댑터(**U5-NFR-MNT-01**, PROP-BL-U5-01 목 관측). seam 계약이 TLS 불변식·목 테스트 가능성의 경계.

### 2.5 UreqAdapter
- **책임**: `HttpTransport` 포트의 **프로덕션 어댑터** — `ureq`(blocking) + rustls TLS로 실제 전송. blocking이 seam 동기 시그니처에 정합(Q4=A). rustls 순수 Rust로 native-tls/OpenSSL 시스템 C 의존 회피(NFR-05). 평문 http fallback 없음.
- **공개/내부 인터페이스 표면**: `impl HttpTransport for UreqAdapter`(참고 형상). `ureq`/rustls 의존은 이 어댑터에만 유입(U3 목 경로 미유입). 구체 배선(`Cargo.toml`·feature·crypto provider 확정)은 Code Generation 이월.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §5 rustls 이식성(**NFR-05** 재현 빌드) + §8 rustls-only TLS(**U5-NFR-SEC-01**).

---

## 3. `CredentialProvider` 애그리게이트 — 논리 컴포넌트

> `CredentialProvider`는 토큰 출처(secure-store -> config -> env)를 해소한다. **캐시 없이 매 호출 U0 `ConfigProvider.current()`(core 필드 `token`/`secure_store_enabled`)를 라이브 조회**해 config 리로드(토큰 회전)를 구조적으로 반영한다(nfr-design-patterns §4). 공개 메서드: `resolve_token() -> Result<TokenSecret, CredentialError>` · `token_status() -> TokenStatus` · `on_config_reload()`(no-op).

### 3.1 TokenResolver
- **책임**: R-CP-01 우선순위 실행 — `secure_store_enabled` 시 `SecureStore.read_token()` 최우선, 그 후 config `token` -> env `OKC_WATCHER_TOKEN` 폴백. secure-store 실패/불가/미저장은 비치명 폴백 신호. 부재 -> `Missing`, 공백 -> `Empty`. `token_status()`는 부수효과 없이 존재/소스만 반환(원문 미노출).
- **공개/내부 인터페이스 표면**: `resolve_token() -> Result<TokenSecret, CredentialError>` · `token_status() -> TokenStatus{present, source: Option<TokenSource>}`. 주입 보관: `Arc<dyn ConfigProvider>` · `Arc<dyn SecureStore>` · env 접근.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §4 라이브 스냅샷 동시성(**U5-NFR-USE-02**, R-CP-04 회전 반영 자동, `on_config_reload` no-op) + §9 토큰 위생(**U5-NFR-SEC-02**, R-CP-03 원문 미노출) + §11 관측 표면(**U5-NFR-USE-02**, US-E4-01 actionable 사전 실패).

### 3.2 SecureStorePort
- **책임**: OS 보안 저장소 조회를 감싸는 **포트 트레이트**(US-E4-02). `Ok(None)` = 세션 가용하나 미저장, `Err(Unavailable)`/`Err(Backend)` = 폴백 신호. 실 백엔드는 이 경계 내 교체 가능(seam).
- **공개/내부 인터페이스 표면**: `trait SecureStore { fn read_token(&self) -> Result<Option<TokenSecret>, SecureStoreError>; }`(`Send + Sync`). `SecureStoreError{Unavailable, Backend(String)}`.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §6 `SecureStore` seam(**U5-NFR-MNT-02**, R-CP-01 폴백 계약).

### 3.3 UnavailableSecureStore
- **책임**: `SecureStore`의 **null-object 기본 어댑터** — 항상 `Err(Unavailable)`. MVP 기본 주입체로 config `token` -> env 안전 폴백을 성립시킨다(실효 순서 = config -> env). 실 keyring 백엔드는 미채택(NFR-05 부담·헤드리스 데몬).
- **공개/내부 인터페이스 표면**: `struct UnavailableSecureStore; impl SecureStore -> 항상 Err(Unavailable)`(참고 형상). 외부 keyring 크레이트 미유입.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §6 null-object 어댑터(**U5-NFR-MNT-02** MVP 트림, keyring DEFER).

---

## 4. `ConsentGate` 애그리게이트 — 논리 컴포넌트

> `ConsentGate`는 동의 4-상태 라이프사이클(`ConsentLifecycle`)의 권위 소스이자 업로드 게이트다. 인메모리 상태 + 지속을 **`Mutex<ConsentInner>` 쓰기 직렬화**로 all-or-nothing 보장한다(nfr-design-patterns §4). 공개 메서드: `acknowledge` · `grant` · `withdraw` · `view` · `is_upload_permitted` · `ensure_acknowledged` · `disclosure_text` · `on_config_reload`(no-op).

### 4.1 ConsentStateMachine
- **책임**: R-CG-TRANSITION 전이 합법성(불법 전이는 상태 불변 + `ConsentError`, `acknowledge` 멱등, `Withdrawn -> Granted` 재부여), R-CG-GATE 게이트 판정(`Granted <-> Permitted`, 그 외 -> 대응 `BlockReason`), R-CG-PROJECT projection(`ConsentLifecycle -> ConsentState` 3변이), R-CG-FORWARD-ONLY(회수 없음). 순수 판정 + 지속은 성공 시에만.
- **공개/내부 인터페이스 표면**: `acknowledge()/grant()/withdraw() -> Result<(), ConsentError>` · `is_upload_permitted() -> ConsentDecision` · `view() -> ConsentStatus` · `ensure_acknowledged() -> Result<(), ConsentError>`. 상태 `ConsentLifecycle` + `Option<ConsentGrant>`는 `ConsentInner`(뮤텍스 보호)에 상주.
- **안착 NFR 패턴**: `nfr-design-patterns.md` §4 게이트 쓰기 직렬화 뮤텍스(**U5-NFR-REL-03** all-or-nothing, R-CP-04 reload 후 상태 보존) + §11 관측 push(**U5-NFR-USE-02**, `ConsentGate`가 `StatusSink.raise_condition(ConsentBlocked)` 직접 push).

### 4.2 ConsentStore
- **책임**: `ConsentRecord`의 **원자 지속(temp+rename) + 무손실 round-trip + 기동 로드 fail-safe 강등**. 성공 전이만 `encode`(U0 CBOR) -> temp write + `fsync` -> `rename` 커밋; 실패 시 `PersistFailed` + 인메모리 롤백. 로드 시 `decode` 실패/일관성 위반은 차단-우선 기본으로 강등(패닉 없이 `Result`). 파일 I/O는 U5 소유(`std`), 바이트 변환만 U0 코덱.
- **공개/내부 인터페이스 표면**: 내부 `persist(&ConsentRecord) -> Result<(), ConsentError::PersistFailed>` · `load() -> ConsentRecord`(fail-safe). 지속 경로 = U0 `ConfigProvider` 해소 데이터 디렉토리 하위. `ciborium` 직접 의존 없음(U0 `encode`/`decode` 경유).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §3 동의 지속 원자성 + crash-safety + fail-safe 강등(**U5-NFR-REL-03**, R-CG-PERSIST; PROP-U5-08/PROP-DE-U5-01 round-trip).

### 4.3 DisclosureProvider
- **책임**: RISK-01 고지 문안(`disclosure_text()`) 정적 반환 — **4대 필수 내용**(연속성·비가역/forward-only·클라 필터 없음·로컬 평문 산출물[config 토큰 포함]) 전량 포함. 최종 법적/UX 문안 이월이나 4개 요소 부재는 회귀로 차단.
- **공개/내부 인터페이스 표면**: `disclosure_text() -> &'static str`. 신규 크레이트 없음(정적 문자열).
- **안착 NFR 패턴**: `nfr-design-patterns.md` §10 RISK-01 고지 텍스트(**U5-NFR-USE-01**, R-CG-DISCLOSURE; PROP-U5-09 키워드 존재 검사).

---

## 5. 논리 컴포넌트 의존 엣지 (크레이트 비순환 확인)

> 규약: `A -> B` = "A가 B에 의존한다(B의 타입/함수/계약을 사용)". U5 크레이트 전체는 **`foundation`(U0)에만** 의존한다(FD `domain-entities.md` §5). U6(구현) 역참조 없음 -> 빌드 순서 역전 없음.

### 5.1 크레이트-레벨 의존 엣지

```
의존 엣지 (크레이트):
  auth-consent (U5)  ->  foundation (U0)   [유일]
  auth-consent       ->  ureq(rustls)       [UreqAdapter 어댑터 한정, normal dep]
  auth-consent       ->  serde/thiserror/url [workspace-inherited]
  auth-consent(dev)  ->  proptest + foundation[features=proptest-support]

  U5는 U6/U3/U4/U8 을 import 하지 않음  => DAG 비순환 유지
  상위 소비: U3 -> AuthTransport.send ; U8 -> ConsentGate.is_upload_permitted (+ 데드라인 주입)
```

### 5.2 논리 컴포넌트 -> U0 소비 매핑

| 논리 컴포넌트 | 소속 애그리게이트 | U0(foundation) 소비 대상 | seam/포트 |
|---|---|---|---|
| TransportPipeline | `AuthTransport` | `ConfigProvider`(server_endpoint) · `TransportError` | `HttpTransport`(주입) · `CredentialProvider`(주입) |
| RequestAssembler | `AuthTransport` | `validate_https_url`(파싱 `url::Url`) · `TokenSecret` · `Duration`(U8 주입) | — |
| ResponseClassifier | `AuthTransport` | `TransportErrorClass`(5변이) · `TransportError` | — (순수) |
| HttpTransportPort | `AuthTransport` | — (U5 소유 seam, `Send + Sync`) | 포트 |
| UreqAdapter | `AuthTransport` | — (`ureq`/rustls 어댑터) | 포트 구현 |
| TokenResolver | `CredentialProvider` | `ConfigProvider`(token/secure_store_enabled) · `TokenSecret` · `CredentialError`(`thiserror`) | `SecureStore`(주입) |
| SecureStorePort | `CredentialProvider` | `TokenSecret` | 포트 |
| UnavailableSecureStore | `CredentialProvider` | — (null-object) | 포트 구현 |
| ConsentStateMachine | `ConsentGate` | `Timestamp` · `ConsentState`(projection 타깃) · `ActiveCondition` · `StatusSink`(주입, ConsentBlocked push) · `ConsentError`(`thiserror`) | — |
| ConsentStore | `ConsentGate` | `encode`/`decode`(CBOR) · `ConfigProvider`(데이터 디렉토리) | — |
| DisclosureProvider | `ConsentGate` | — (정적 문자열) | — |
| ProptestGenerators | (test-support) | `proptest-support`(U0 제너레이터, `Timestamp` 등) | (dev, 런타임 밖) |

### 5.3 비순환 및 sub-unit 무-import 확인
- **크레이트 비순환**: `auth-consent -> foundation` 단방향뿐. U3/U4/U6/U8 방향 엣지 없음(component-dependency.md 빌드 순서 Foundation -> U1 -> U4 -> U5 -> ... 와 정합).
- **관측 push 비순환**: `ConsentGate`/`AuthTransport`는 U0 `StatusSink`/`CriticalEventSink` **계약 트레이트**(U6 구현)만 주입받아 소비하므로 U5 -> U6 엣지가 생기지 않는다(빌드 순서 역전 없음, nfr-design-patterns §11).
- **federated 값 무-import**: `request_timeout_s`는 U8이 `Duration`으로 하향 주입하므로 U5는 U0 `ConfigSnapshot`에 federated 접근자를 추가하지 않고 U8도 import하지 않는다(생성자 주입만).

---

## 6. PBT 타깃 정렬 (PBT-01 / 확장 Full)

> 이 단계는 신규 PBT 결정을 내리지 않는다(PBT-09 = U0 §7 상속으로 SATISFIED). 확정 속성(FD의 PROP-U5-*/PROP-DE-U5-*/PROP-BL-U5-*)을 명명 논리 컴포넌트에 정렬해 추적성을 강화한다. NFR ID -> 속성 매핑은 `nfr-design-patterns.md` §15와 일관된다.

### 6.1 컴포넌트 -> Testable Property 정렬표

| 논리 컴포넌트 | 정렬 속성(FD 소유) | 카테고리 | 실현 NFR / 규칙 |
|---|---|---|---|
| ResponseClassifier | PROP-U5-01(분류 total + 오라클) · PROP-U5-02(taxonomy 폐쇄성) | Invariant + Oracle | U5-NFR-REL-01(R-AT-CLASS-01/02/03) |
| RequestAssembler | PROP-U5-03(TLS 강제 + 토큰 1회 + 타임아웃 부착) | Invariant | U5-NFR-SEC-01/SEC-02/REL-02(R-AT-TLS/TOKEN/TO) |
| TransportPipeline | PROP-BL-U5-01(send 파이프라인 불변식, 목 관측) | Invariant | U5-NFR-MNT-01 · REL-02 |
| TokenResolver | PROP-U5-04(우선순위 오라클, 4축 조합 전수) | Oracle + Easy verification | U5-NFR-MNT-02/USE-02(R-CP-01/02/03) |
| ConsentStateMachine | PROP-U5-05(상태 전이 Induction) · PROP-U5-06(게이트 불변식) · PROP-U5-07(forward-only) · PROP-DE-U5-02(projection total) | Induction + Invariant | U5-NFR-REL-03/USE-02(R-CG-TRANSITION/GATE/FORWARD-ONLY/PROJECT) |
| ConsentStore | PROP-U5-08 / PROP-DE-U5-01(`ConsentRecord`/`ConsentGrant` round-trip) | Round-trip | U5-NFR-REL-03(R-CG-PERSIST, NFR-13) |
| DisclosureProvider | PROP-U5-09(고지 4대 내용 포함) | Invariant + Easy verification | U5-NFR-USE-01(R-CG-DISCLOSURE) |

> **속성 없음(No PBT properties identified)**: HttpTransportPort/SecureStorePort(순수 seam 계약 — 값이 아니라 주입 동작), UnavailableSecureStore(상수 동작 -> PROP-U5-04 폴백에 흡수), UreqAdapter(실전송 어댑터 — 통합/네트워크 테스트, Build-and-Test), on_config_reload no-op(§4)은 독립 PBT 속성이 없다(domain-entities §6.3 · business-rules §4.7 · business-logic-model §6.3 판정과 일관).

### 6.2 ProptestGenerators — 런타임 그래프 밖 test-support 논리 단위 (PBT-07)
- **배치**: 도메인 제너레이터(`proptest`)는 **런타임 의존 그래프(§5) 밖**의 별도 test-support 논리 단위다. 비-default Cargo feature(`proptest-support`)로 게이트되어 릴리스 런타임 바이너리에 포함되지 않는다(U0 패턴 mirror, tech-stack §6).
- **책임**: `OkcRequest` 변형(메서드/경로/헤더/본문) · 목 `HttpTransport` 응답 조합(상태코드 100~599 전수 + `HttpError` 전 변이 + PROJECT_BUSY/queue-full 코드) · `TokenSource` 4축 조합(secure_store on/off x seam 결과 x config x env) · `ConsentLifecycle`/`ConsentRecord`(일관성 불변식 §3.5 만족) + 연산 시퀀스 + 손상 레코드 제너레이터 제공. U0 제너레이터(`Timestamp` in `ConsentGrant` 등)는 `proptest-support` dev feature로 재사용.
- **의존 방향**: `ProptestGenerators -> {auth-consent 런타임 타입, foundation(proptest-support)}`(테스트가 런타임 타입 참조). 런타임 컴포넌트는 이 단위에 의존하지 않으므로 §5 런타임 DAG를 오염시키지 않는다.
- **이월**: 구체 구현·shrinking·고정 시드·CI 통합(PBT-08)은 Code Generation / Build-and-Test 이월.

---

## 7. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 산출물 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **준수 — blocking 없음** | §6이 각 논리 컴포넌트를 확정 속성(PROP-U5-*/PROP-DE-U5-*/PROP-BL-U5-*)에 정렬(PBT-01), ProptestGenerators를 test-support 논리 단위로 문서화(PBT-07). 신규 PBT 결정 없음(PBT-09는 U0 §7 상속 충족). PBT-08은 Code Generation/Build-and-Test 이월. |
| **Resiliency Baseline** | ON | **준수(부분 적용 + 대체로 N/A) — blocking 없음** | §2 TransportPipeline 타임아웃 + graceful degrade(RESILIENCY-10 전송 절반; 백오프 U4), §4 ConsentStore 원자 지속 + fail-safe 강등. §5 크레이트 비순환(U5 = Wave-내 소비 단위, U0 = Critical DAG 루트). RTO/RPO/DR/HA/서킷브레이커는 순수 lib에 N/A. RESILIENCY-14 resilience testing은 Build-and-Test 이월. |
| **Security Baseline** | OFF | **N/A — 미로딩·미강제** | RISK-01 수용(config 평문 `token`). RequestAssembler TLS 가드(SEC-01)·`TokenSecret` 위생(SEC-02)·UnavailableSecureStore(secure-store DEFER)는 이미 채택된 잔존 위생/통제의 컴포넌트 배치일 뿐 신규 강제 통제 아님. |

**블로킹 판정**: 이 산출물에 blocking finding 없음. §2~§4 컴포넌트 안착은 `nfr-design-patterns.md` §16 맵과 1:1이며, §5 의존 그래프는 비순환(`auth-consent -> foundation` 단방향, sub-unit 무-import)이다.

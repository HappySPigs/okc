# U5 Auth & Consent — NFR Requirements (비기능 요구사항 / 품질 속성)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U5 Auth & Consent** -> NFR Requirements -> 산출물 1/2 (`nfr-requirements.md`)
**작성일**: 2026-09-08
**크레이트**: `auth-consent` (lib) · **소속 컴포넌트**: `AuthTransport`, `CredentialProvider`, `ConsentGate`
**입력 아티팩트**: `functional-design/domain-entities.md`·`business-rules.md`·`business-logic-model.md`(U5 FD 3종) · `plans/u5-auth-consent-functional-design-plan.md`(§3 AUTOPILOT 결정 DEC-U5-01..17 · §4 확장 컴플라이언스 · §5 드롭리스트) · `inception/requirements/requirements.md`(FR-13/14/15 · NFR-04/06/07 · §7 RISK-01 · §8 DEP-01/02/05/06/07 · §13 토큰 저장 오버레이) · `construction/u0-foundation/nfr-requirements/{nfr-requirements.md,tech-stack-decisions.md}`(스타일 템플릿 + 전파된 결정)
**전제(AUTOPILOT 확정, 재오픈 금지)**: DEC-U5-01(HttpTransport seam) · DEC-U5-02(TLS 이중화) · DEC-U5-03(상태코드 매핑) · DEC-U5-04(request_timeout_s=30, U8 하향 주입) · DEC-U5-06/07(우선순위 + secure-store DEFER) · DEC-U5-11(ConsentRecord 단일 지속) · DEC-U5-13(게이트 범위) · DEC-U5-14(고지 텍스트) · DEC-U5-15(1회 시도) · Q4=A · Q7=A · NFR-04/06/07 · FR-13/14/15 · RISK-01
**전파 상속(U0 = Wave-1 DAG 루트)**: Rust / Edition 2024 / 고정 MSRV 1.85 / `proptest`(PBT-09) / `proptest-support` 비기본 feature / `thiserror` 운영오류 파생 관례 / `TokenSecret` redaction / U0 CBOR 코덱(`ciborium`) — 모두 U0 `tech-stack-decisions.md`에서 확정, U5는 소비·준수한다.

> **문서 성격**: 이 문서는 U5가 소유하는 타입(`domain-entities.md`)·규칙(`business-rules.md`)·흐름(`business-logic-model.md`) **위에 얹는 NFR(품질 속성) 계층**이다. FD의 규칙을 재기술하지 않고 **규칙 ID로 참조**하며, 각 NFR에 근거(아티팩트 + 섹션 + NFR/FR/규칙/PBT ID)와 수용 기준을 붙인다. 구체 크레이트·툴체인 선택의 정본은 자매 산출물 `tech-stack-decisions.md`이며, 이 문서는 각 품질 속성을 **실현하는 메커니즘**으로 그 결정을 참조한다.
>
> **표기 규약**: 기술중립을 지향하되(품질 속성 중심), tech-stack-decisions.md가 위임한 곳에서만 구체 크레이트를 인용한다. 다이어그램은 ASCII 박스 없이 **화살표 표기(`A -> B`)** 와 표/목록으로 기술한다. English 식별자(크레이트·타입·config 키·규칙/NFR ID)는 원문 유지하고, Rust 타입/제네릭(예: `Result<OkcResponse, TransportError>`)은 백틱으로 감싼다.

---

## 1. NFR 개요 및 U5 특성

U5 `auth-consent`는 **Watcher -> 서버 유일 아웃바운드 HTTP 경로**(`AuthTransport`) + **토큰 해소**(`CredentialProvider`) + **동의 게이트/지속**(`ConsentGate`)을 소유하는 lib다. `foundation`(U0)에만 의존해 DAG 비순환을 유지하며(순수 lib, 자체 런타임/스레드/처리량 축 없음), 서버측 동작(DEP-01/02/05/06/07)은 전부 `[blocked-on-server]` 목(mock) 계약으로만 검증한다. 따라서 확장성·가용성·성능 수치 목표의 상당수가 **N/A**다(§7). 반대로 U5가 실현하는 **핵심 품질 계약**은 (a) 응답 분류의 total-function panic-free 성질, (b) 모든 요청의 타임아웃 데드라인 + 오프라인 graceful degrade(NFR-04), (c) 유일 잔존 통제 TLS 강제(NFR-06)와 토큰 유출 위생, (d) 동의 레코드의 원자적·무손실 지속(NFR-13), (e) RISK-01 informed-consent 고지다.

**본 문서가 확정하는 U5 NFR 카탈로그(카테고리별 ID)**:

| 카테고리 | NFR ID | 요약 |
|---|---|---|
| 신뢰성 | U5-NFR-REL-01 | 응답/실패 분류 = total-function + panic-free(모든 상태코드/`HttpError`) |
| 신뢰성 | U5-NFR-REL-02 | 모든 요청 타임아웃 데드라인 + 오프라인 graceful degrade(NFR-04) |
| 신뢰성 | U5-NFR-REL-03 | `ConsentRecord` 원자적(temp+rename) 지속 + round-trip 무손실 + 손상 fail-safe 강등(NFR-13) |
| 유지보수성 | U5-NFR-MNT-01 | `HttpTransport` seam(구체 전송 스택 결합 회피 + 목 테스트 가능성) |
| 유지보수성 | U5-NFR-MNT-02 | `SecureStore` seam + `UnavailableSecureStore`(keyring 백엔드 DEFER) |
| 유지보수성 | U5-NFR-MNT-03 | 오류 타입 파생 전략(운영오류 = `thiserror`, 분류/값 타입 = U0 serde) |
| 유지보수성 | U5-NFR-MNT-04 | PBT 도메인 제너레이터(U0 `proptest-support` 재사용 + U5 자기 제너레이터) |
| 보안-잔존 | U5-NFR-SEC-01 | TLS/`https` 전송 강제(유일 잔존 통제, NFR-06 — 이중 방어) |
| 보안-잔존 | U5-NFR-SEC-02 | 토큰 유출 위생(`TokenSecret` redaction + 전송 시점 헤더 주입) |
| 사용성 | U5-NFR-USE-01 | RISK-01 고지 텍스트 informed-consent 4대 필수 내용(NFR-07/FR-15) |
| 사용성 | U5-NFR-USE-02 | credential/consent 관측 표면(actionable 오류 표면화, US-E4-01/06) |

---

## 2. 신뢰성 (Reliability)

### 2.1 U5-NFR-REL-01 — 응답/실패 분류 = total-function + panic-free (DEC-U5-03)

- **요구**: `AuthTransport`의 응답/전송실패 -> `TransportErrorClass` 분류는 **모든** 입력(2xx / 정의된 상태코드 / 미정의·미래 상태코드(100~599) / `HttpError` 전 변이)에 대해 정확히 하나의 결과(`Ok(OkcResponse)`(2xx) 또는 단일 `TransportErrorClass`)를 반환하는 **전(total) 함수**이며, `unwrap`/슬라이스 인덱싱/정수 변환 패닉을 내지 않는다. 미정의 상태코드는 범위 규칙(기타 4xx -> `ServerError`, 5xx -> `ServerError`, 그 외 비정상 -> `Network`)으로 흡수한다. 분류 결과 클래스는 항상 U0 `TransportErrorClass` 5변이 중 하나이며, U5는 새 오류 계열을 만들지 않는다(taxonomy 폐쇄성).
- **선정 근거(왜 명시적 NFR인가)**: `AuthTransport`는 매 서버 왕복마다 호출되는 핫 경로이고, 그 분류 결과는 U0 `to_error_class()`(U4 재시도 게이트) 입력이다. 여기서 패닉하거나 누락 분기가 있으면 데몬 동기화 사이클 전체가 크래시하거나 재시도 판정이 불능이 되어 오프라인 회복력(NFR-04)을 정면으로 무력화한다. 따라서 "라이브러리 기본 의존"이 아니라 U5의 **명시적 신뢰성 불변식**으로 승격한다(U0-NFR-REL-02 panic-free total 계약의 U5 소비 측면).
- **근거**: `business-rules.md` §1 R-AT-CLASS-01(전체 매핑)·R-AT-CLASS-02(total function)·R-AT-CLASS-03(PROJECT_BUSY 인식) · `business-logic-model.md` §1.2(6·7단계 분류) · requirements.md §3.2(응답 분류)·§5.2 NFR-04 · U0 `TransportErrorClass`/`to_error_class()` 소유(재정의 금지). 실현 = 순수 매칭 로직(신규 크레이트 없음).
- **수용 기준**:
  - (AC-1) 전 상태코드(100~599 전수) + `HttpError` 전 변이 제너레이터 기반 PBT(PROP-U5-01, 카테고리 Invariant+Oracle)가 매핑 표(참조 오라클)와 일치하며 반례·패닉 없이 통과한다.
  - (AC-2) 분류 결과는 항상 U0 5변이 중 하나이고 `to_error_class()`로 재시도 판정이 total하게 이어진다(PROP-U5-02, taxonomy 폐쇄성).
  - (AC-3) 429 및 알려진 코드 문자열(PROJECT_BUSY/queue-full)은 `Backpressure`로 승격되며, 깊은 body 스키마 파싱은 U3 이월(R-AT-CLASS-03, `[blocked-on-server]` 목 계약으로 검증).

### 2.2 U5-NFR-REL-02 — 모든 요청 타임아웃 데드라인 + 오프라인 graceful degrade (NFR-04, DEC-U5-04)

- **요구**: `AuthTransport.send`가 조립하는 **모든** `RawHttpRequest`는 주입된 요청 데드라인(`request_timeout_s` 유래, 기본 30초, `>= 1`)을 `timeout` 필드에 부착한다 — 데드라인 없는 요청은 존재할 수 없다(RESILIENCY-10 전송 절반). 단일 전체 데드라인(connect + 응답 포함; connect/read 분리는 이월). 데드라인 초과(`HttpError::Timeout`)는 `Timeout`으로, 연결/TLS/IO 실패(`HttpError::Connect`/`Tls`/`Io`)는 `Network`로 분류되어 U4 graceful degrade(백오프·오프라인 재시도)에 공급된다. `AuthTransport`는 재시도하지 않는다(1회 시도, DEC-U5-15) — 백오프는 U4 소유.
- **federated-config 해소(교차관심 확정)**: `request_timeout_s`는 U0 core 6필드가 아니므로 U0 `ConfigSnapshot`에서 읽지 않는다. **U8 조립루트(watcher-bin)가 원본 config를 파싱해 타입 값(`Duration`)으로 `AuthTransport`에 생성자 하향 주입**한다(DEC-FEDERATED-KEYS Q6=A). U5는 `AUTH_CONSENT_CONFIG_KEYS`에 `request_timeout_s`를 등록해 **미지-키 거부만 방지**하고 검증 규칙(양의 정수)을 소유하나, 값 읽기 경로는 갖지 않는다. federated 파라미터 live-reload는 MVP 범위 밖(재시작으로 변경).
- **근거**: `business-rules.md` §1 R-AT-TO(모든 호출 데드라인)·R-AT-CLASS-01(Timeout/Network 분류) · `business-logic-model.md` §1.1(요청 데드라인 주입)·§1.2 · `domain-entities.md` §4(`request_timeout_s` federated known-key) · requirements.md §5.2 NFR-04·§6 RESILIENCY-10. 실현 클라이언트(`ureq` blocking + rustls) 정본 = tech-stack-decisions.md.
- **수용 기준**:
  - (AC-1) 목 `HttpTransport`로 관측 시, `execute`에 도달하는 모든 `RawHttpRequest.timeout`은 주입된 데드라인과 동일한 양의 값이다(PROP-U5-03 타임아웃 불변식, 카테고리 Invariant).
  - (AC-2) 타임아웃/연결실패가 각각 `Timeout`/`Network`로 분류되어 `is_retryable()==true`로 U4에 이어진다(오프라인 graceful degrade 공급, R-AT-CLASS-01).
  - (AC-3) `AuthTransport`는 자체 재시도·백오프를 수행하지 않는다(1회 시도; 재시도 판정·스케줄은 U4 경계, DEC-U5-15).

### 2.3 U5-NFR-REL-03 — `ConsentRecord` 원자적 지속 + round-trip 무손실 + 손상 fail-safe 강등 (NFR-13, DEC-U5-11)

- **요구**: `acknowledge`/`grant`/`withdraw` 성공은 `ConsentRecord`를 **U0 CBOR 코덱(`encode`/`decode`)으로 무손실 직렬화 + temp+rename 원자적 쓰기**로 지속한다(부분쓰기 무손상). 지속 실패는 `ConsentError::PersistFailed`이고 **인메모리 상태도 롤백**(all-or-nothing — 지속되지 않은 전이는 관측되지 않음). 로드 시 `decode(encode(record)) == record`(round-trip 무손실, NFR-13)가 성립해 재시작 후 상시 동의가 원값 그대로 복구된다. 일관성 위반(손상) 레코드는 안전 기본(`AcknowledgedNotGranted`/`NotAcknowledged`)으로 **강등(fail-safe: 의심 시 업로드 금지)** 한다.
- **선정 근거**: 동의 상태는 업로드 게이트(R-CG-GATE)의 권위 소스다. 지속이 부분 적용되거나 손상 복구가 낙관적(fail-open)이면, 철회했는데 재시작 후 업로드가 재개되는 등 RISK-01 통제(informed consent + forward-only)를 위반한다. 따라서 원자성 + fail-safe 강등을 명시적 신뢰성 불변식으로 승격한다. round-trip 코덱 자체는 U0 소유(U0-NFR-REL-01)이며 U5는 자기 지속 타입(`ConsentRecord`/`ConsentGrant`) 제너레이터로 재확인한다.
- **근거**: `business-rules.md` §3 R-CG-PERSIST(원자적 지속·round-trip·일관성 강등) · `domain-entities.md` §3.5(`ConsentRecord` 일관성 불변식)·§3.2(`ConsentGrant`) · `business-logic-model.md` §3.2(기동 로드 fail-safe)·§3.3(성공 시에만 지속) · requirements.md §5.5 NFR-13(직렬화 round-trip 무손실). 실현 코덱(`ciborium` via U0) + 파일 I/O(U5 소유) 정본 = tech-stack-decisions.md.
- **수용 기준**:
  - (AC-1) 일관성 불변식 만족 `ConsentRecord`/`ConsentGrant`(전 `ConsentLifecycle` 변이·`grant` 부재/존재·유니코드 `GrantId`)에 대해 `decode(encode(v)) == v`가 반례 없이 통과한다(PROP-U5-08 / PROP-DE-U5-01, 카테고리 Round-trip, PBT-02).
  - (AC-2) 지속 쓰기 실패는 `PersistFailed`로 표면화되고 인메모리 상태가 롤백된다(부분 적용 없음).
  - (AC-3) 손상/일관성 위반 레코드는 로드 시 차단-우선 기본으로 강등되며 패닉 없이 `Result`로 종결한다(fail-safe Invariant).

---

## 3. 유지보수성 (Maintainability)

### 3.1 U5-NFR-MNT-01 — `HttpTransport` seam (DEC-U5-01)

- **요구**: `AuthTransport`는 구체 HTTP/TLS 스택에 직접 결합하지 않고 최소 전송 seam(`HttpTransport` 트레이트, `execute(&self, RawHttpRequest) -> Result<RawHttpResponse, HttpError>`, `Send + Sync`) 뒤에서 동작한다. 구체 blocking 클라이언트(rustls 기반)는 seam 구현체로 주입되고, U3 `UploadProtocolDriver`는 이 트레이트의 **목(mock) 구현**으로 프로토콜을 서버 없이 테스트한다(`[blocked-on-server]` 검증 경로의 핵심).
- **선정 근거**: seam이 없으면 U3 테스트가 실서버·실네트워크에 결합되고 U5 분류/타임아웃/토큰 불변식(PROP-U5-01/03)을 목으로 관측할 수 없다. 트레이트 경계는 구체 클라이언트 교체(blocking <-> async, 크레이트 변경)를 U5 내부로 국소화한다.
- **근거**: `domain-entities.md` §1.2(`HttpTransport` seam·`RawHttpRequest`/`RawHttpResponse`/`HttpError`) · `business-logic-model.md` §1.2(seam 1회 호출)·§6.1(목 관측 PROP-BL-U5-01) · plans §3 DEC-U5-01. 구체 클라이언트(`ureq`) 정본 = tech-stack-decisions.md.
- **수용 기준**:
  - (AC-1) `AuthTransport`는 `HttpTransport` 트레이트만 참조하고 구체 클라이언트 타입에 정적 결합하지 않는다(구현체는 생성자 주입).
  - (AC-2) 목 `HttpTransport`로 전송된 `RawHttpRequest`를 캡처해 TLS/토큰/타임아웃 불변식(PROP-U5-03)을 서버 없이 검증할 수 있다.

### 3.2 U5-NFR-MNT-02 — `SecureStore` seam + `UnavailableSecureStore` (DEC-U5-07)

- **요구**: OS 보안 저장소 조회를 `SecureStore` 트레이트(`read_token(&self) -> Result<Option<TokenSecret>, SecureStoreError>`, `Send + Sync`) seam으로 감싸고, **MVP는 실 keyring 백엔드를 구현하지 않고** `UnavailableSecureStore`("항상 `Err(Unavailable)`") 기본 구현을 주입한다. secure-store 실패/불가는 **비치명**이며 반드시 config `token` -> env 순으로 안전 폴백한다(US-E4-02). 실효 MVP 순서 = config -> env.
- **선정 근거(MVP 트림)**: §13 오버레이가 config 평문 `token`을 1차·기본 저장으로 확정했고 Watcher는 헤드리스/데몬으로 실행되어 데스크톱 세션 보안 저장소가 대개 불가하다(watcher-daemon-model). keyring 크레이트(플랫폼별 C/네이티브 결합)를 MVP에 도입하면 NFR-05 크로스플랫폼 재현 빌드 부담이 커지므로, seam + Unavailable 기본만 두고 실 백엔드는 code-gen 이월한다.
- **근거**: `domain-entities.md` §2.2(`SecureStore` seam·`SecureStoreError`·`UnavailableSecureStore`) · `business-rules.md` §2 R-CP-01(폴백) · `business-logic-model.md` §2.2 · requirements.md §13(config 평문 1차·기본, secure-store opt-in) · plans §3 DEC-U5-07. 백엔드 미채택(keyring 없음) 정본 = tech-stack-decisions.md.
- **수용 기준**:
  - (AC-1) `secure_store_enabled == false`이면 seam을 호출하지 않고, `true`라도 `Err(Unavailable)`/`Err(Backend)`/`Ok(None)`이면 config -> env로 결정적 폴백한다(PROP-U5-04 우선순위 오라클에 흡수).
  - (AC-2) MVP 빌드 그래프에 keyring/네이티브 보안저장소 크레이트가 유입되지 않는다(seam + Unavailable 기본만).

### 3.3 U5-NFR-MNT-03 — 오류 타입 파생 전략 (U0-NFR-MNT-01 관례 계승)

- **요구**: U5가 정의하는 오류를 U0 관례에 맞춰 두 부류로 파생한다:
  - **운영 오류(반환용, `Result`)**: `CredentialError`·`ConsentError`(+ 전송 계층의 U5 소유 오류가 있다면)는 **`thiserror`** 파생으로 `Display`/`std::error::Error`를 얻는다(라이브러리 관례, 워크스페이스 전역 일관).
  - **분류/값 타입(직렬화·round-trip 대상, throw 아님)**: `TransportError`/`TransportErrorClass`/`ErrorClass`는 **U0 소유 serde 값 타입**을 그대로 소비한다(U5가 재파생하지 않음). U5의 지속 값 타입(`ConsentRecord`/`ConsentGrant`/`ConsentLifecycle`)은 순수 serde 파생으로 CBOR round-trip 대상(U5-NFR-REL-03)이 된다.
- **선정 근거**: `anyhow`식 타입소거는 U4 재시도 분류가 의존하는 구조화 변이를 소실시키므로 거부(U0-NFR-MNT-01과 동일 근거). U5는 신규 오류 계약을 만들지 않고 U0 taxonomy를 소비한다.
- **근거**: `domain-entities.md` §2.1(`CredentialError`)·§3.4(`ConsentError`)·§0(U0 소비 값 타입) · U0 `tech-stack-decisions.md` §5(오류 처리 전파 U1~U8) · plans §5 드롭리스트. 실현 크레이트(`thiserror` workspace-inherited) 정본 = tech-stack-decisions.md.
- **수용 기준**:
  - (AC-1) `CredentialError`/`ConsentError`는 `thiserror` 파생으로 `Display`/`Error`를 제공한다.
  - (AC-2) `ConsentRecord`/`ConsentGrant`는 serde 파생만 하고 CBOR round-trip 무손실(U5-NFR-REL-03 AC-1)을 만족한다.

### 3.4 U5-NFR-MNT-04 — PBT 도메인 제너레이터 재사용 (U0 `proptest-support` 계승, PBT-07)

- **요구**: U5의 PBT는 워크스페이스 프레임워크 `proptest`(PBT-09, U0 확정)를 dev-dependency로 사용하고, U0 도메인 타입 제너레이터(`Timestamp` 등)는 U0의 **비기본 `proptest-support` feature**를 dev에서 켜 재사용한다(단일 출처 -> 드리프트 방지). U5 고유 도메인 제너레이터(`OkcRequest`·목 `HttpTransport` 응답 조합·`TokenSource` 4축 조합·`ConsentLifecycle`/`ConsentRecord`/연산 시퀀스)는 U5가 정의한다.
- **선정 근거**: U0-NFR-MNT-02가 하위 단위(U5 포함) 지속 레코드 round-trip을 명시적 재사용 소비자로 지목했다. U5는 U0 패턴을 그대로 계승해 `proptest`가 프로덕션 빌드 그래프에 유입되지 않도록(non-default/dev) 유지한다.
- **근거**: `business-rules.md` §4·`domain-entities.md` §6·`business-logic-model.md` §6(제너레이터 총괄, PBT-07) · U0 `tech-stack-decisions.md` §7(`proptest`/`proptest-support`, 재사용 소비 U5) · property-based-testing.md PBT-07/PBT-09 · plans §4.2. feature 게이팅 정본 = tech-stack-decisions.md.
- **수용 기준**:
  - (AC-1) `auth-consent`의 `proptest` 의존은 dev-dependency이며 기본 빌드 그래프에 나타나지 않는다(U0 패턴 mirror).
  - (AC-2) U0 제너레이터가 필요한 지점(예: `Timestamp` in `ConsentGrant`)은 U0 `proptest-support`를 dev에서 켜 재사용하고, U5 고유 제너레이터는 문서화된 도메인 제약(일관성 불변식 §3.5)을 존중한다.

---

## 4. 보안-잔존 (Security residual)

**전제**: Security Baseline 확장 = **OFF**(requirements §2.3 Q1=B). 암호화 저장·키관리·시크릿 스캐닝 등 강제 통제는 신설하지 않으며 RISK-01(소스 노출)은 **문서화된 수용 위험**이다(config 평문 `token` 포함, §13). 아래 두 항목은 신설 통제가 아니라 (a) 유일 잔존 통제(TLS)의 U5 실현과 (b) 저비용 방어적 토큰 위생이다.

### 4.1 U5-NFR-SEC-01 — TLS / `https` 전송 강제 (유일 잔존 통제, NFR-06, DEC-U5-02)

- **요구(신규 통제 아님 — 이중 방어의 전송 계층 실현)**: `AuthTransport`는 **TLS(https) 위에서만** 전송한다. 이중 방어:
  1. **config 계층**: `server_endpoint`는 U0 `validate_https_url`로 https만 허용(비-https는 최초 로드 실패) — U5는 재사용.
  2. **전송 계층**: 요청 조립 시 확정 절대 URL 스킴이 `https`가 아니면 `HttpTransport.execute` **호출 전 거부**(seam 미도달; 정상 경로는 config 검증으로 이미 https이므로 회귀 방어). 구체 클라이언트도 rustls 기반 TLS만 사용하고 평문 http를 하지 않는다.
- **잔존 통제 명문화**: Security OFF 하에서 TLS는 **in-transit 가로채기만 완화**한다(RISK-01: 서버로의 공개 자체는 미완화 — 이 사실은 `disclosure_text`(U5-NFR-USE-01)가 고지한다). 토큰은 who-may-upload 통제일 뿐 소스 노출 완화가 아니다. 그 외 강제 통제는 신설하지 않는다.
- **근거**: `business-rules.md` §1 R-AT-TLS(이중 방어) · `business-logic-model.md` §1.2(3단계 TLS 가드) · `domain-entities.md` §1.2(seam TLS 불변식)·§0(`validate_https_url` 재사용) · requirements.md §5.4 NFR-06·§7 RISK-01(잔존 통제)·§13(TLS 강제 불변) · U0 `nfr-requirements.md` §5.1 U0-NFR-SEC-01(파싱된 base URL을 U5가 재사용) · plans §3 DEC-U5-02. 실현 클라이언트 TLS 백엔드(rustls) 정본 = tech-stack-decisions.md.
- **수용 기준**:
  - (AC-1) 목 `HttpTransport`로 관측 시, `execute`에 도달하는 모든 `RawHttpRequest.url` 스킴은 `https`다(비-https는 seam 도달 전 거부, PROP-U5-03 TLS 불변식).
  - (AC-2) 구체 전송 클라이언트는 rustls 기반 TLS만 사용하고 평문 http fallback을 하지 않는다(native-tls/OpenSSL 시스템 의존 미채택 — tech-stack-decisions.md).
  - (AC-3) TLS가 유일 잔존 통제이며 소스 노출(RISK-01)을 완화하지 않음을 문서화한다(그 외 강제 통제 신설 없음).

### 4.2 U5-NFR-SEC-02 — 토큰 유출 위생 (`TokenSecret` redaction + 전송 시점 주입, R-AT-TOKEN)

- **요구(수용 위험 완화용 저비용 위생)**: 토큰은 U0 redacting newtype `TokenSecret`로만 다루며(`Debug`/`Display` = `***`, 실제 값은 `.expose()`로만), 로그·오류 `detail`·`TransportError`에 원문을 넣지 않는다(U0-NFR-SEC-02 소비 측면). 토큰 헤더는 `OkcRequest.headers`가 아니라 **전송 시점 `RawHttpRequest` 조립 단계에서 주입**해 호출자(U3)가 토큰을 다루지 않게 유출면을 축소한다(FR-13). `token_status()`는 존재/소스만 반환하고 원문을 노출하지 않는다(R-CP-03).
- **선정 근거**: config 평문 `token`은 RISK-01 수용 산출물이나, 그것이 로그/오류/응답 캡처로 **추가** 유출되는 것은 별개의 저비용 위생 문제다. U0가 이미 `TokenSecret` newtype을 제공하므로 U5는 이를 준수·경유하기만 하면 되며 신규 크레이트/코드가 거의 없다(외부 의존 0).
- **근거**: `business-rules.md` §1 R-AT-TOKEN(전송 시점 주입·로그 유출 금지)·§2 R-CP-03(원문 미노출) · `domain-entities.md` §0(`TokenSecret`)·§1.1(헤더에 토큰 미포함) · requirements.md §7 RISK-01(로컬 평문 산출물)·§13 · U0 `tech-stack-decisions.md` §8(redaction newtype 전파 U5) · plans §3 DEC-U5-08. 
- **수용 기준**:
  - (AC-1) 토큰은 `TokenSecret`로만 이동하며 로그/오류/`TransportError.detail`에 원문이 나타나지 않는다(`Debug`/`Display` = `***`).
  - (AC-2) 해소가 `Ok`인 전송은 토큰 헤더를 정확히 1회 포함하고, `Missing`/`Empty`이면 `execute` 미호출(요청 미발송, PROP-U5-03 토큰 불변식).
  - (AC-3) 이는 RISK-01 수용 하 저비용 위생일 뿐 Security Baseline 통제를 켜는 것이 아니다(확장 여전히 OFF).

---

## 5. 사용성 (Usability)

### 5.1 U5-NFR-USE-01 — RISK-01 고지 텍스트 informed-consent (DEC-U5-14, FR-15/NFR-07)

- **요구**: `disclosure_text()`가 반환하는 고지 문안은 **4대 필수 내용**을 모두 포함해야 한다(하나라도 결여 시 US-E4-04 위반):
  1. **연속성**: 상시 동의 + 자동 동기화 하에 이후 모든 볼트 변경(나중에 추가된 비밀 포함)이 추가 확인 없이 자동 업로드됨.
  2. **비가역성 + 실효적 철회 부재**: 업로드된 콘텐츠는 회수 불가; 서버 전까지 철회는 forward-only(DEP-02/DEP-06 `[blocked-on-server]`).
  3. **클라이언트 측 민감 콘텐츠 필터 없음**: raw 볼트 전체(비밀·개인정보 포함)가 전송됨(FR-15/NFR-07).
  4. **로컬 평문 산출물**: 히스토리·매니페스트·SyncState·로그 **및 config 평문 토큰**이 OS 보안 저장소 밖에 존재함(RISK-01).
- **선정 근거**: config 파일 + CLI가 U5의 유일 사람-대면 표면이고, informed consent는 RISK-01을 수용 위험으로 만드는 전제다(고지 없이는 "수용"이 성립하지 않음). 최종 법적/UX 문안은 이월하되, 4개 내용 요소의 **부재는 회귀로 차단**한다.
- **근거**: `business-rules.md` §3 R-CG-DISCLOSURE · `domain-entities.md` §3.6(4대 필수 내용) · `business-logic-model.md` §3.3 · requirements.md §3.4 FR-15·§5.4 NFR-07·§7 RISK-01 · plans §3 DEC-U5-14. 신규 크레이트 없음(정적 문자열).
- **수용 기준**:
  - (AC-1) `disclosure_text()`가 4대 필수 내용 요소를 모두 포함한다(키워드/문구 존재 검사, PROP-U5-09, 카테고리 Invariant+Easy verification).
  - (AC-2) 최종 법적 문안은 code-gen 이월이나, 4개 요소 중 하나라도 결여하면 테스트가 실패한다(회귀 방지).

### 5.2 U5-NFR-USE-02 — credential/consent 관측 표면 + actionable 오류 (US-E4-01/06)

- **요구**: 인증·동의 상태는 CLI `status`/`consent view`가 소비할 수 있게 관측 가능해야 한다:
  - `token_status() -> TokenStatus{ present, source }`(원문 미노출)로 토큰 존재·소스를 표면화(US-E4-01).
  - 토큰 부재/공백(`CredentialError::Missing`/`Empty`)은 **요청 미발송 + actionable 사전 실패**로 표면화하고(US-E4-01 AC3: 업로드 미시작 + CLI/로그에 조치 가능 오류), 필요 시 `StatusSink.raise_condition(AuthFailed)` 성격 조건을 push(구체 배선은 코디네이터/U8 경계).
  - `view() -> ConsentStatus{ acknowledged, grant, state }`로 부여 참조·상태를 조회(US-E4-06), 게이트 차단 시 `raise_condition(ConsentBlocked)`.
- **선정 근거**: RISK-01/동의 모델의 신뢰성은 사용자가 "현재 인증/동의가 어떤 상태인지"를 확인 가능해야 성립한다. U5는 값/판정을 반환하고 실제 push 소유자 확정은 U8 배선(계약은 양쪽 지원, `business-logic-model.md` §5)이다.
- **근거**: `business-rules.md` §2 R-CP-03(`token_status`)·§3 R-CG-GATE(차단 관측) · `domain-entities.md` §2.1(`TokenStatus`)·§3.3(`ConsentStatus`) · `business-logic-model.md` §2.3(liveness)·§5(관측 push 경계) · requirements.md §3.4 FR-13/FR-14·§3.5(관측). 신규 크레이트 없음(U0 `StatusSink` 계약 소비).
- **수용 기준**:
  - (AC-1) `token_status()`/`view()`는 부수효과 없이 관측 표면을 반환하고 토큰 원문을 노출하지 않는다.
  - (AC-2) 토큰 부재/공백은 요청 미발송 + actionable 오류로 표면화된다(데몬 정지 없음 — 인증 시점 사전 판정, PROP-U5-04에 흡수).

---

## 6. NFR to 요구사항 추적표

각 U5 NFR을 소스 NFR/FR ID(requirements) + 규칙 ID(FD) + PBT 속성 ID로 매핑한다.

| U5 NFR ID | 요약 | 소스 NFR/FR ID | 규칙 ID | PBT ID |
|---|---|---|---|---|
| U5-NFR-REL-01 | 분류 total-function + panic-free | 응답 분류(§3.2), NFR-04(U4 기반) | R-AT-CLASS-01/02/03 | PROP-U5-01/02 (Invariant+Oracle) |
| U5-NFR-REL-02 | 모든 요청 타임아웃 + graceful degrade | NFR-04, RESILIENCY-10 | R-AT-TO, R-AT-CLASS-01 | PROP-U5-03(타임아웃), PROP-BL-U5-01 |
| U5-NFR-REL-03 | 동의 원자 지속 + round-trip + fail-safe | NFR-13, FR-14 | R-CG-PERSIST | PROP-U5-08 / PROP-DE-U5-01 (Round-trip) |
| U5-NFR-MNT-01 | `HttpTransport` seam | (테스트 가능성), DEP-01/03(목 계약) | (seam 계약) | PROP-BL-U5-01(목 관측) |
| U5-NFR-MNT-02 | `SecureStore` seam + Unavailable | FR-13, NFR-05(포터빌리티) | R-CP-01 | PROP-U5-04(폴백 흡수) |
| U5-NFR-MNT-03 | 오류 파생 전략 | NFR-17 | R-AT-CLASS, R-CG-* | PROP-U5-08(값 타입 round-trip) |
| U5-NFR-MNT-04 | PBT 제너레이터 재사용 | NFR-17, NFR-08..14(기반) | (제너레이터 계약) | PBT-07 |
| U5-NFR-SEC-01 | TLS/`https` 전송 강제 | NFR-06, RISK-01(잔존) | R-AT-TLS | PROP-U5-03(TLS 불변식) |
| U5-NFR-SEC-02 | 토큰 유출 위생 | NFR-06(잔존), FR-13, RISK-01 | R-AT-TOKEN, R-CP-03 | PROP-U5-03(토큰 불변식) |
| U5-NFR-USE-01 | RISK-01 고지 텍스트 | FR-15, NFR-07, RISK-01 | R-CG-DISCLOSURE | PROP-U5-09 (Invariant) |
| U5-NFR-USE-02 | credential/consent 관측 표면 | FR-13, FR-14, US-E4-01/06 | R-CP-03, R-CG-GATE | PROP-U5-04(흡수) |
| (포터빌리티 연계) | rustls-only(네이티브 TLS 미채택)로 NFR-05 실현 | NFR-05 | (tech-stack 결정) | N/A(빌드 재현성) |

---

## 7. N/A 카테고리 근거표

아래 카테고리는 U5에 요구를 신설하지 않는다(발명 금지).

| 카테고리 | 판정 | 근거 |
|---|---|---|
| **확장성(Scalability)** | N/A | U5는 순수 lib(전송 + 게이트 로직). 자체 런타임·스레드 풀·처리량 축 없음. `send`는 요청당 1회 시도(DEC-U5-15). 100k 파일 / 20 GiB 스케일은 U1, 청크 전송은 U3 소관. |
| **가용성(Availability)** | N/A | lib 크레이트라 SLA 없음. 전체 Watcher는 단일 사용자·사용자 재시작 로컬 프로세스로 RTO/availability-SLA가 requirements RESILIENCY-02(§6)에서 이미 N/A 확정. |
| **성능 수치 목표(Performance numeric)** | N/A(정성 계약만) | `AuthTransport`는 얇은 전송 소유자로 throughput/latency/peak-memory 수치 게이트 근거 없음. 유일 정성 계약 = 모든 요청 단일 데드라인 유계(U5-NFR-REL-02). MVP는 요청/응답 body를 인메모리 버퍼로 취급(스트리밍 요청 body는 이월); 대용량 blob 청킹은 U3 소관. |
| **Resiliency DR / RTO / RPO** | N/A(신규 결정 없음) | DR/RTO/availability = requirements RESILIENCY-02(§6)에서 N/A. RPO = zero-loss는 U4 durable state 소관이며 U5 동의 지속(U5-NFR-REL-03)은 그 부분집합(동의 레코드 무손실·원자성)일 뿐. 배포/HA/롤백은 U7/상위 소관. |
| **사용성(config/CLI 표면 외)** | N/A | U5는 트레이/GUI 표면 없는 lib(watcher-daemon-model). 사람-대면 표면 = config 검증(U0)·CLI(U7b)·고지 텍스트(U5-NFR-USE-01)·관측 표면(U5-NFR-USE-02). GUI/트레이 UX는 범위 밖. |
| **보안 강제 통제(Security enforced)** | N/A | Security Baseline OFF, RISK-01 수용(config 평문 `token` 포함). 암호화 저장·키관리·시크릿 스캐닝 신설 없음. 유일 잔존 통제 = TLS(U5-NFR-SEC-01). secure-store 실 백엔드는 선택적 강화(DEFER, U5-NFR-MNT-02). in-scope 잔존 위생 = 토큰 유출 위생(U5-NFR-SEC-02) 하나. |

---

## 8. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 단계 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제·준수** | 프레임워크 `proptest`(PBT-09)는 U0에서 워크스페이스 전역 확정 — U5는 dev-dependency로 채택 + `proptest-support`(U0 제너레이터) 재사용(U5-NFR-MNT-04, PBT-07). 속성은 FD에서 식별 완료(PROP-U5-01..09 + PROP-DE-U5-01/02 + PROP-BL-U5-01) 후 이 단계 NFR에 매핑(§6). 핵심: 분류 total-function 오라클(REL-01), 타임아웃/TLS/토큰 불변식(REL-02/SEC-01/SEC-02), 동의 지속 round-trip(REL-03), 고지 내용(USE-01). shrinking/시드/CI(PBT-08) 상세는 Code Generation / Build-and-Test 이월. **blocking 없음.** |
| **Resiliency Baseline** | ON | **부분 적용** | RESILIENCY-10(모든 네트워크 호출 타임아웃 + graceful degrade) = U5-NFR-REL-02(전송 절반; 백오프는 U4). 오프라인/연결실패 -> `Timeout`/`Network` 분류(REL-01)로 U4 graceful degrade 공급. 동의 레코드 temp+rename 원자적 지속 + fail-safe 강등(REL-03). RPO/RTO 수치·배포/HA/DR은 U5(순수 전송+게이트 로직)에 **N/A**(RESILIENCY-02, §7). RESILIENCY-14 resilience testing(오프라인/타임아웃/resume 시뮬레이션)은 NFR Design/Build-and-Test 이월. |
| **Security Baseline** | OFF | **N/A (잔존만 표면화)** | 미로딩·미강제. RISK-01 수용(config 평문 `token`). 유일 잔존 통제 = TLS/`https`(U5-NFR-SEC-01, NFR-06) — §13 검증 규칙의 전송 계층 실현일 뿐 신규 통제 아님. 저비용 in-scope 잔존 위생 = 토큰 유출 위생(U5-NFR-SEC-02). secure-store는 선택적 강화(DEFER). 암호화 저장·키관리·시크릿 스캐닝 신설 없음. |

**블로킹 판정**: PBT-09는 U0에서 확정되어 U5가 상속·채택하므로 Property-Based Testing 확장의 blocking finding 없음. Resiliency는 부분 적용(N/A 항목 명시), Security는 N/A(잔존 표면화)로 blocking 없음.

---

## 9. 후속 단계 이월 항목 (참고)

- 구체 크레이트 결정(`ureq` blocking + rustls TLS 백엔드 · rustls crypto provider 선택 · `SecureStore` keyring 미채택 · 타임아웃 단일 데드라인 · `GrantId` 생성 방식)의 정본 -> 자매 산출물 `tech-stack-decisions.md`.
- connect/read 분리 타임아웃 -> 이월(단일 데드라인이 MVP 충분, U5-NFR-REL-02).
- secure-store 실 keyring 백엔드 -> Code Generation(선택적 강화).
- 스트리밍 요청 body(대용량 blob) -> 이월(MVP는 인메모리 버퍼; 청킹은 U3).
- RESILIENCY-14 resilience testing(오프라인/타임아웃/resume 시뮬레이션) -> NFR Design / Build-and-Test.
- PBT 케이스 수·shrinking·고정 시드·CI 통합(PBT-08) · 최종 고지 법적 문안(DEC-U5-14) -> Code Generation / Build-and-Test.
- federated `request_timeout_s` 값 전달 방식(U8 하향 주입) 최종 확정 -> NFR Design 교차관심(이 문서 §2.2에서 U8 주입으로 확정).

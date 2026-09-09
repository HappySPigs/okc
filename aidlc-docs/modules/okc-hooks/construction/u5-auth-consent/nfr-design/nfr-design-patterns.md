# U5 Auth & Consent — NFR Design Patterns (NFR 실현 설계 패턴)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U5 Auth & Consent** -> NFR Design -> 산출물 1/2 (`nfr-design-patterns.md`)
**작성일**: 2026-09-08
**크레이트**: `auth-consent` (lib) · **소속 컴포넌트**: `AuthTransport`, `CredentialProvider`, `ConsentGate`
**입력 아티팩트**: `nfr-requirements/nfr-requirements.md`(U5-NFR-REL/MNT/SEC/USE 카탈로그) · `nfr-requirements/tech-stack-decisions.md`(`ureq`+rustls · secure-store 미채택 · 단일 데드라인 · 오류 파생 · PBT) · U5 FD 3종(`domain-entities.md`·`business-rules.md`·`business-logic-model.md`) · `construction/u0-foundation/nfr-design/{nfr-design-patterns.md,logical-components.md}`(스타일 템플릿 + 상속 계약) · 활성 확장 `property-based-testing.md`(ON, Full)·`resiliency-baseline.md`(ON)
**규칙**: `construction/nfr-design.md` Step 6 · `common/content-validation.md`(ASCII 화살표 `A -> B`, 박스 문자 미사용, Rust 제네릭 백틱) · `common/ascii-diagram-standards.md`

> **문서 성격**: 이 문서는 U5가 확정한 **품질 속성(NFR Requirements)** 을 **어떤 설계 패턴으로 실현하는가** 를 기록한다. 여기서 크레이트를 새로 결정하지 않는다 — 구체 크레이트의 정본은 `tech-stack-decisions.md`, 규칙 ID는 FD, 카테고리별 NFR은 `nfr-requirements.md`가 소유하며 이 문서는 그것들을 **설계 패턴으로 정렬·실현**한다. 각 패턴은 (a) 패턴/결정 진술, (b) 실현하는 NFR·규칙 근거, (c) 구조 노트/불변식, (d) 명시적 트레이드오프로 기술한다. 논리 컴포넌트 분해의 상세 맵은 자매 산출물 `logical-components.md`가 소유하며, 이 문서는 각 패턴이 어느 논리 컴포넌트에 안착하는지만 §12 추적표에서 참조한다.
>
> **AUTOPILOT 전제(재오픈 금지)**: DEC-U5-01(HttpTransport seam) · DEC-U5-02(TLS 이중화) · DEC-U5-03(상태코드 매핑) · DEC-U5-04(request_timeout_s=30, U8 하향 주입) · DEC-U5-07(secure-store DEFER) · DEC-U5-11(ConsentRecord 단일 지속) · DEC-U5-15(1회 시도) · Q4=A(얇은 전송) · Q7=A · NFR-04/06/07 · RISK-01. tech-stack: `ureq`(blocking, rustls) · U0 CBOR 코덱 경유 · `thiserror` · `proptest`(U0 §7 상속).
>
> **표기 규약**: 화살표는 ASCII `A -> B`만 사용한다(유니코드 화살표 금지). 박스/선-그리기 문자를 쓰지 않는다. Rust 제네릭/타입/식별자(예: `Result<OkcResponse, TransportError>`, `Arc<dyn HttpTransport>`, `Mutex<ConsentInner>`, `Duration`, `TokenSecret`)는 백틱으로 감싼다.

---

## 1. 분류 total-function + panic-free (REL-01) — 순수 매칭 모듈 clippy lint-gate + PBT no-panic

**(a) 패턴/결정**: 응답/전송실패 -> `TransportErrorClass` 분류를 **부수효과 없는 순수 매칭 함수**로 격리하고(입력 = `RawHttpResponse.status: u16` 또는 `HttpError` 변이, 출력 = `Ok(OkcResponse)` 또는 단일 `TransportErrorClass`), 그 순수 분류 모듈 상단에 U0 §1 패턴을 mirror한 **컴파일타임 clippy lint-gate**(`deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)`)를 건다. 상태코드 -> 클래스 사상은 **범위 기반 total 매칭**(2xx 정상 / 401·403 -> `AuthFailed` / 429·알려진 코드 -> `Backpressure` / 5xx·기타 4xx·3xx -> `ServerError` / 그 외 -> `Network`)으로 미정의·미래 상태코드까지 흡수한다. 런타임 방어 래퍼(`catch_unwind` 등)는 두지 않는다(비용 0).

**(b) NFR/규칙 실현**:
- **U5-NFR-REL-01**(분류 total-function + panic-free)의 "모든 입력(100~599 + `HttpError` 전 변이)에 정확히 하나의 결과, 패닉 없음"을 코드 규율이 아니라 **컴파일타임 lint-gate로 승격**한다. `AuthTransport`는 매 서버 왕복 핫 경로이고 분류 결과는 U0 `to_error_class()`(U4 재시도 게이트) 입력이므로, 여기서 패닉/누락 분기가 나면 동기화 사이클이 크래시하거나 오프라인 회복력(NFR-04)이 무력화된다.
- **규칙 R-AT-CLASS-01**(전체 매핑) · **R-AT-CLASS-02**(total function, 미정의/패닉 경로 없음) · **R-AT-CLASS-03**(429/코드 힌트만으로 Backpressure 승격, 깊은 body 파싱은 U3 이월).
- **taxonomy 폐쇄성(REL-01 AC-2)**: 분류 결과 클래스는 항상 U0 `TransportErrorClass` 5변이 중 하나이며 U5는 새 오류 계열을 만들지 않는다 — U0 `to_error_class()`로 재시도 판정이 total하게 이어진다.

**(c) 구조 노트/불변식**: lint-gate는 **순수 분류 모듈에만** 적용한다(전송 I/O를 수행하는 `ureq` 어댑터 경계는 순수 표면이 아니므로 대상 아님, §5). 불변식 — 분류의 모든 경로는 `Ok(OkcResponse)`(2xx) 또는 단일 `TransportErrorClass`로 나가며 `panic!`/`unwrap`/정수·슬라이스 인덱싱 패닉 경로가 타입·lint 수준에서 부재한다. `u16` 상태코드 범위 매칭은 슬라이스 인덱싱 없이 range guard로 표현한다.

**(d) 트레이드오프**: (1) `catch_unwind` 방어 래퍼는 **미채택** — U0 §1과 동일 근거로 total 순수 함수에 런타임 unwind-catch는 개념 상충이며 버그를 은폐한다. (2) 앱-레벨 코드 정밀 분류(기타 4xx를 permanent-client-error로 세분)는 **미채택(MVP 트림)** — U0 taxonomy에 permanent-client 클래스가 없어 ServerError로 흡수하되 U4 연속실패 카운트+에스컬레이션(FR-19)이 오분류 루프를 상한한다(R-AT-CLASS-01 트림). 비용은 lint 규율 부담뿐, 런타임 비용 0.

---

## 2. 요청 데드라인 부착 + graceful degrade (REL-02) — federated-config 하향 주입 `Duration` [DEC-U5-04 / DEC-FEDERATED-KEYS Q6=A]

**(a) 패턴/결정**: `AuthTransport`는 생성자에서 **U8 조립루트가 주입한 요청 데드라인(`Duration`, `request_timeout_s` 유래, 기본 30초, `>= 1`)** 를 보관하고, `send`가 조립하는 **모든** `RawHttpRequest.timeout` 필드에 이 값을 예외 없이 부착한 뒤 `HttpTransport.execute`(1회 시도)에 넘긴다. 데드라인 초과(`HttpError::Timeout`) -> `Timeout`, 연결/TLS/IO 실패(`HttpError::Connect`/`Tls`/`Io`) -> `Network`로 §1 분류가 사상해 **U4 graceful degrade(백오프·오프라인 재시도)에 공급**한다. `AuthTransport`는 자체 재시도/백오프를 하지 않는다(얇은 전송, DEC-U5-15).

**(b) NFR/규칙 실현**:
- **U5-NFR-REL-02**(모든 요청 타임아웃 데드라인 + 오프라인 graceful degrade) · **규칙 R-AT-TO**(데드라인 없는 요청 부재, RESILIENCY-10 전송 절반) · **R-AT-CLASS-01**(Timeout/Network 분류 -> U4 공급).
- **FEDERATED-CONFIG 해소(§ federated 섹션 참조)**: `request_timeout_s`는 U0 core 6필드가 아니므로 `AuthTransport`가 `ConfigProvider.current()`로 읽지 **않는다**. U8이 원본 config를 파싱해 타입 값(`Duration`)으로 **생성자 하향 주입**한다.

**(c) 구조 노트/불변식**: 단일 전체 데드라인(connect + 응답 포함; connect/read 분리는 이월). 불변식 — 목 `HttpTransport`로 관측 시 `execute`에 도달하는 모든 `RawHttpRequest.timeout`은 주입된 데드라인과 동일한 양의 값이다(PROP-U5-03 타임아웃 불변식). U5는 `AUTH_CONSENT_CONFIG_KEYS = &["request_timeout_s"]` 상수로 **미지-키 거부만 방지**하고 검증 규칙(양의 정수 `>= 1`)을 소유하나, 값 읽기 경로는 갖지 않는다(주입만 소비).

**(d) 트레이드오프**: (1) connect/read 분리 타임아웃은 **이월(MVP 트림)** — 단일 데드라인이 얇은 전송에 충분(U5-NFR-REL-02). (2) `AuthTransport`가 U0 `ConfigSnapshot`에 federated 접근자를 추가해 스스로 읽는 대안은 **미채택** — U0 동결 + core 6필드 한정 계약 위반. (3) federated 파라미터 live-reload는 **범위 밖(MVP 트림)** — 재시작으로 변경(U0 core-field 토큰 reload만 U0 관찰자 팬아웃).

---

## 3. 동의 지속 원자성 + crash-safety + fail-safe 강등 (REL-03) — U0 CBOR 코덱 + `std` temp+rename [DEC-U5-11]

**(a) 패턴/결정**: `acknowledge`/`grant`/`withdraw` 성공만이 `ConsentRecord`를 **U0 `encode`(CBOR/`ciborium` 경유) -> `std` temp+rename 원자적 쓰기**로 지속한다. 순서는 **인메모리 전이 준비 -> `encode` -> temp 파일 쓰기 + `fsync` -> `rename`(원자 커밋) -> 인메모리 상태 확정**이다. 어느 단계든 실패하면 `ConsentError::PersistFailed`를 반환하고 **인메모리 상태를 롤백**한다(all-or-nothing — 지속되지 않은 전이는 관측되지 않음). 기동 로드 시 `decode` 실패/일관성 위반(손상) 레코드는 안전 기본(`AcknowledgedNotGranted`/`NotAcknowledged`)으로 **강등(fail-safe: 의심 시 업로드 금지)** 하며 패닉 없이 `Result`로 종결한다.

**(b) NFR/규칙 실현**:
- **U5-NFR-REL-03**(원자 지속 + round-trip 무손실 + fail-safe 강등) · **규칙 R-CG-PERSIST**(원자적 지속·all-or-nothing·round-trip·손상 강등) · **R-CG-GATE**(동의 상태 = 업로드 게이트 권위 소스).
- **U0 상속**: round-trip 무손실(NFR-13)은 U0 `encode`/`decode`(U0-NFR-REL-01 R-CODEC-01)가 보증하고 U5는 자기 지속 타입(`ConsentRecord`/`ConsentGrant`) 제너레이터로 재확인(PROP-U5-08 / PROP-DE-U5-01). U0 §2 reload 원자성 패턴(temp 커밋 후에만 관측)의 파일 계층 대응이다.

**(c) 구조 노트/불변식**: 불변식 — (1) rename 이전 실패는 활성 상태 파일을 오염하지 않는다(부분쓰기 무손상, nginx식 fail-safe). (2) `state == Granted`이면 로드 후에도 `grant.is_some()`; 위반 레코드는 차단-우선 기본으로 강등된다. (3) `Withdrawn`에서도 `acknowledged == true` 유지(고지 확인은 철회로 취소되지 않음). **전체-파일 스트리밍 비대상**: `ConsentRecord`는 최신 상태만 담는 소형 단일 레코드(부여 이력 누적 없음)라 whole-buffer `encode`가 정합하며, 대용량 스트리밍/청킹은 U5 범위 밖(U3 소관).

**(d) 트레이드오프**: (1) 이력 누적(append-only 로그)은 **미채택(MVP 트림)** — 최신 상태 단일 레코드로 충분(FR-14 부여 참조). (2) WAL/저널링은 **미채택** — temp+rename가 단일 소형 레코드에 원자성을 이미 보증(추가 복잡도 불요). (3) 낙관적 fail-open 복구는 **거부** — RISK-01 통제(informed consent + forward-only) 위반(철회 후 재시작 업로드 재개 위험).

---

## 4. 동시성 모델 (Concurrency) — 무상태 `Send + Sync` 전송 + 라이브-스냅샷 자격증명 + 게이트 쓰기 직렬화 뮤텍스

**(a) 패턴/결정**: 데몬 멀티스레드 공유 하에서 U5 세 컴포넌트의 동시성 형상을 다음으로 확정한다:
- **`AuthTransport` = 무상태 `Send + Sync`**: 주입된 `Arc<dyn HttpTransport>` + `Arc<CredentialProvider>` + `Arc<dyn ConfigProvider>`(server_endpoint 조회) + `Duration`(데드라인)만 보관하고 자체 가변 상태가 없다. `send`는 재진입 안전하며 락이 불필요하다.
- **`CredentialProvider` = 라이브 스냅샷, 캐시 없음**: `resolve_token()`이 매 호출 U0 `ConfigProvider.current()`(core 필드 `token`/`secure_store_enabled`, lock-free `Arc` 로드) + env를 **그때그때 재해소**한다. 토큰 캐시를 두지 않으므로 config 리로드(토큰 회전)가 **구조적으로 즉시 반영**된다.
- **`ConsentGate` = 내부 쓰기 직렬화 뮤텍스**: 인메모리 상태 + 지속을 `Mutex<ConsentInner>`(`state`/`grant`/`record` + 파일 경로)로 감싸, 전이+지속(§3 all-or-nothing 시퀀스)을 임계구역 안에서 직렬화한다. `view()`/`is_upload_permitted()` 읽기도 동일 락을 짧게 잡는다.

**(b) NFR/규칙 실현**:
- **U0 싱크/seam `Send + Sync` 근거 계승**(domain-entities §1.2/§2.2): `HttpTransport`/`SecureStore` 트레이트가 `Send + Sync`이므로 `AuthTransport`/`CredentialProvider`가 스레드 간 공유 가능.
- **U5-NFR-REL-03 all-or-nothing**: `ConsentGate` 뮤텍스가 전이+지속 시퀀스를 직렬화해 동시 CLI 명령(U7b) 인터리빙과 게이트 읽기 중간상태 관측을 차단한다(U0 §2 `reload_mutex` 시퀀스 직렬화 패턴의 U5 대응).
- **규칙 R-CP-04**(`on_config_reload` 재해석): 라이브 스냅샷이라 회전 반영이 자동. `ConfigReloadObserver` 훅은 U0 팬아웃 배선 계약 유지를 위해 **등록된 no-op**으로 남긴다.

**(c) 구조 노트/불변식**: 불변식 — (1) 어떤 동시 게이트 읽기도 **완결된(지속 커밋된) 상태만** 관측한다(미지속 전이 미관측). (2) 토큰 해소는 스냅샷 시점의 config를 반영하며 부분 갱신이 없다(U0 `ArcSwap` 원자 로드). `ConsentGate` 쓰기는 CLI-only·드문 연산(U7b 디스패치)이고 게이트 읽기는 사이클당 1회(U8 단일 코디네이터)라 락 경합이 미미하다.

**(d) 트레이드오프**: (1) `CredentialProvider` 토큰 캐시 + 명시적 무효화는 **미채택(MVP 트림)** — 라이브 스냅샷 재해소가 값싸고(Arc 로드 + env read) 회전 반영이 자동이라 캐시/무효화 기계가 불필요. (2) `ConsentGate`에 U0식 `ArcSwap` lock-free 읽기는 **미채택(MVP 트림)** — 게이트 읽기가 핫 경로가 아니고(사이클당 1회) 전이+지속 원자성이 뮤텍스로 더 단순하게 성립. 채택안은 최대 단순성 대가로 드문 쓰기 경합 시 짧은 블로킹을 수용한다.

---

## 5. `HttpTransport` seam — 포트/어댑터 + rustls 이식성 (MNT-01 / SEC-01 / NFR-05) [DEC-U5-01]

**(a) 패턴/결정**: `AuthTransport`는 구체 HTTP/TLS 스택에 정적 결합하지 않고 **포트 트레이트 `HttpTransport`**(`execute(&self, RawHttpRequest) -> Result<RawHttpResponse, HttpError>`, `Send + Sync`) 뒤에서 동작한다. 프로덕션 **어댑터**는 `ureq`(blocking + rustls) 구현체이고, U3 `UploadProtocolDriver`와 U5 PBT는 **목(mock) 어댑터**로 서버·네트워크 없이 검증한다. blocking 클라이언트가 seam 동기 시그니처에 정합(Q4=A). rustls는 순수 Rust TLS라 native-tls/OpenSSL 시스템 C 의존을 피한다.

**(b) NFR/규칙 실현**:
- **U5-NFR-MNT-01**(seam으로 구체 스택 결합 회피 + 목 테스트 가능성) · **U5-NFR-SEC-01**(rustls-only TLS) · **NFR-05**(크로스플랫폼 재현 빌드 — 단일 배포 바이너리, RESILIENCY-01).
- **PROP-BL-U5-01**(목 관측): seam이 캡처한 `RawHttpRequest`로 TLS/토큰/타임아웃 불변식(PROP-U5-03)을 서버 없이 검증(`[blocked-on-server]` 경로 핵심).

**(c) 구조 노트/불변식**: seam은 **분류 이전**의 원시 결과(`RawHttpResponse` 또는 `HttpError`)만 반환하고 상태코드->클래스 분류(§1)는 `AuthTransport`가 소유한다. 어댑터 교체(blocking<->async, 크레이트 변경)는 U5 내부로 국소화된다. `ureq`/rustls 의존은 프로덕션 어댑터에만 유입되고 U3 목 경로에는 유입되지 않는다.

**(d) 트레이드오프**: (1) async 스택(reqwest+tokio)은 **미채택(MVP 트림)** — 얇은 전송에 async 런타임 과잉. (2) native-tls/OpenSSL은 **미채택** — NFR-05 재현 빌드 마찰(시스템 C 의존). (3) rustls crypto provider(ring vs aws-lc-rs) 고정은 **이월** — MVP는 `ureq` 기본 수용, 재현 빌드 우선 시 `ring` 선호(빌드 배선 시점 결정).

---

## 6. `SecureStore` seam — null-object 어댑터 (MNT-02) [DEC-U5-07]

**(a) 패턴/결정**: OS 보안 저장소 조회를 포트 트레이트 `SecureStore`(`read_token(&self) -> Result<Option<TokenSecret>, SecureStoreError>`, `Send + Sync`)로 감싸고, **MVP는 실 keyring 백엔드를 두지 않고** `UnavailableSecureStore`(항상 `Err(Unavailable)`) **null-object 어댑터**를 주입한다. secure-store 실패/불가는 **비치명**이며 반드시 config `token` -> env로 안전 폴백한다(실효 MVP 순서 = config -> env).

**(b) NFR/규칙 실현**:
- **U5-NFR-MNT-02**(seam + `UnavailableSecureStore`, keyring DEFER) · **규칙 R-CP-01/02**(폴백 우선순위, `SecureStoreUnavailable`은 내부 신호로 소비, 최종 오류 미승격) · **NFR-05**(keyring 네이티브/C 결합 회피).

**(c) 구조 노트/불변식**: 불변식 — `secure_store_enabled == false`이면 seam 미호출; `true`라도 `Err(Unavailable)`/`Err(Backend)`/`Ok(None)`이면 config -> env로 결정적 폴백(PROP-U5-04에 흡수). MVP 빌드 그래프에 keyring/네이티브 보안저장소 크레이트가 유입되지 않는다.

**(d) 트레이드오프**: 실 keyring 백엔드는 **미채택(MVP 트림)** — config 평문 1차·기본(§13) + 헤드리스 데몬(watcher-daemon-model)이라 데스크톱 세션 보안 저장소가 대개 불가하고, 플랫폼별 결합이 NFR-05 부담을 키운다. seam 경계 유지로 실 백엔드는 API 변경 없이 code-gen 이월 주입 교체 가능.

---

## 7. 오류 파생 전략 (MNT-03) — 운영오류 `thiserror` + U0 taxonomy 소비 [U0 §5 상속]

**(a) 패턴/결정**: U5 오류를 U0 관례에 맞춰 두 부류로 파생한다:
- **운영 오류(반환용, `Result`)**: `CredentialError`·`ConsentError`는 **`thiserror`** 파생으로 `Display`/`std::error::Error`를 얻는다(workspace-inherited).
- **분류/값 타입(소비)**: `TransportError`/`TransportErrorClass`/`ErrorClass`는 **U0 소유 serde 값 타입을 그대로 소비**(U5 재파생 없음). U5 지속 값 타입(`ConsentRecord`/`ConsentGrant`/`ConsentLifecycle`)은 순수 serde derive(CBOR round-trip 대상, §3).

**(b) NFR/규칙 실현**: **U5-NFR-MNT-03** · U0 §5(오류 처리 전파). `anyhow`식 타입소거는 U4 재시도 분류가 의존하는 구조화 변이를 소실시키므로 거부(U0 관례 계승). U5는 신규 오류 계약을 만들지 않고 U0 taxonomy를 소비한다.

**(c) 구조 노트/불변식**: 불변식 — `CredentialError`/`ConsentError`는 `Display`/`Error`를 제공하고, `ConsentRecord`/`ConsentGrant`는 serde derive만 하여 CBOR round-trip 무손실(REL-03)을 만족한다. `HttpError`(전송 seam 원인)는 어댑터 내부에서 `TransportError`로 사상되며 `Result`로만 나간다.

**(d) 트레이드오프**: `anyhow` 타입소거 **거부**(구조화 변이 보존). 이 결정은 U0 전역 관례라 U5 고유 대안 검토 없음(재결정 아님).

---

## 8. TLS/`https` 전송 강제 (SEC-01) — 이중 방어 [DEC-U5-02 / NFR-06]

**(a) 패턴/결정**: `AuthTransport`는 **TLS(https) 위에서만** 전송한다(이중 방어):
1. **config 계층**: `server_endpoint`는 U0 `validate_https_url`로 https만 허용(비-https는 최초 로드 abort) — U5는 재사용.
2. **전송 계층**: 요청 조립 시 확정 절대 URL 스킴이 `https`가 아니면 `HttpTransport.execute` **호출 전 거부**(seam 미도달, 내부 오류 표면화). 정상 경로는 config 검증으로 이미 https이므로 이 가드는 회귀 방어. 구체 어댑터(`ureq`)도 rustls만 사용하고 평문 http fallback을 하지 않는다.

**(b) NFR/규칙 실현**: **U5-NFR-SEC-01**(유일 잔존 통제) · **규칙 R-AT-TLS** · U0 `validate_https_url` 재사용(U0-NFR-SEC-01 소비 측면). Security Baseline OFF 하에서 TLS는 **in-transit 가로채기만 완화**하며 RISK-01(서버로의 공개 자체)은 미완화 — 이 사실은 `disclosure_text`(§10)가 고지한다.

**(c) 구조 노트/불변식**: 불변식 — 목 `HttpTransport`로 관측 시 `execute`에 도달하는 모든 `RawHttpRequest.url` 스킴은 `https`다(PROP-U5-03 TLS 불변식). 그 외 강제 통제는 신설하지 않는다.

**(d) 트레이드오프**: native-tls/OpenSSL 경로는 **미채택**(§5와 동일). TLS가 소스 노출을 완화하지 않음을 문서화(신규 강제 통제 없음, Security OFF 유지).

---

## 9. 토큰 유출 위생 (SEC-02) — `TokenSecret` redaction + 전송 시점 헤더 주입 [R-AT-TOKEN]

**(a) 패턴/결정**: 토큰은 U0 redacting newtype `TokenSecret`로만 이동한다(`Debug`/`Display` = `***`, 실제 값은 `.expose()`로만). 토큰 헤더는 `OkcRequest.headers`가 아니라 **전송 시점 `RawHttpRequest` 조립 단계에서 주입**해 호출자(U3)가 토큰을 다루지 않게 유출면을 축소한다. 로그·오류 `detail`·`TransportError`에 원문을 넣지 않으며 `token_status()`는 존재/소스만 반환한다.

**(b) NFR/규칙 실현**: **U5-NFR-SEC-02**(저비용 위생) · **규칙 R-AT-TOKEN**(전송 시점 주입·로그 유출 금지)·**R-CP-03**(원문 미노출) · U0 §6 redaction 계약 소비. U0가 이미 `TokenSecret`을 제공하므로 U5는 준수·경유뿐(외부 의존 0).

**(c) 구조 노트/불변식**: 불변식 — (1) 토큰이 로그/오류/`TransportError.detail`에 원문으로 나타나지 않는다. (2) 해소가 `Ok`인 전송은 토큰 헤더를 정확히 1회 포함하고, `Missing`/`Empty`이면 `execute` 미호출(요청 미발송, PROP-U5-03 토큰 불변식). 이는 RISK-01 수용 하 저비용 위생일 뿐 Security Baseline을 켜는 것이 아니다.

**(d) 트레이드오프**: 없음(외부 의존/신규 코드 거의 없음, U0 newtype 준수). config 평문 `token`은 RISK-01 문서화된 수용 위험이며 그 추가 유출(로그 등)만 위생으로 차단.

---

## 10. RISK-01 고지 텍스트 informed-consent (USE-01) — 정적 문안 4대 필수 내용 [DEC-U5-14]

**(a) 패턴/결정**: `disclosure_text() -> &'static str`는 정적 문안으로 **4대 필수 내용**(연속성 · 비가역성/forward-only · 클라 필터 없음 · 로컬 평문 산출물[config 토큰 포함])을 모두 포함한다. 최종 법적/UX 문안은 code-gen 이월이나, 4개 내용 요소의 **부재는 회귀로 차단**한다(PROP-U5-09 키워드/문구 존재 검사).

**(b) NFR/규칙 실현**: **U5-NFR-USE-01**(NFR-07/FR-15) · **규칙 R-CG-DISCLOSURE** · domain-entities §3.6. config + CLI가 U5 유일 사람-대면 표면이고 informed consent는 RISK-01을 수용 위험으로 만드는 전제(고지 없이는 "수용" 불성립). 신규 크레이트 없음(정적 문자열).

**(c) 구조 노트/불변식**: 불변식 — 4개 요소 중 하나라도 결여하면 테스트 실패(US-E4-04 위반). 게이트(§ R-CG-ACK-GATE)는 고지 확인(`acknowledged == true`) 전 어떤 업로드도 시작하지 않는다.

**(d) 트레이드오프**: 최종 법적 문안 확정은 **이월** — MVP는 4대 내용 요소 존재만 회귀 방지.

---

## 11. 관측 push 경계 + 관측 표면 (USE-02) — 값 반환 우선 + `StatusSink` 계약 소비 [business-logic-model §5]

**(a) 패턴/결정**: U5 컴포넌트는 **판정/결과를 반환**하는 것을 1차로 하고 실제 상태 push는 U0 `StatusSink`/`CriticalEventSink` 계약(push-only, back-reference 없음)을 통해 이뤄진다:
- **`AuthTransport`**: 401/403을 `TransportError{AuthFailed}`로 **반환만** 하고 인증 실패 조건 표면화는 코디네이터/U4 경로가 수행(얇은 전송 소유자 원칙, Q4=A) — 주입된 `CriticalEventSink`가 있으면 직접 report도 계약상 허용.
- **`ConsentGate`**: 동의 상태는 게이트가 권위 보유하므로 주입된 `StatusSink`로 `raise_condition(ConsentBlocked)`/`clear_condition`을 직접 push(자연 소유).
- **관측 표면**: `token_status() -> TokenStatus{present, source}`(원문 미노출) · `view() -> ConsentStatus{acknowledged, grant, state}` — 부수효과 없이 CLI `status`/`consent view`(U7b)가 소비.

**(b) NFR/규칙 실현**: **U5-NFR-USE-02**(actionable 오류 표면화, US-E4-01/06) · **규칙 R-CP-03**(`token_status`)·**R-CG-GATE**(차단 관측). 토큰 부재/공백(`Missing`/`Empty`)은 요청 미발송 + actionable 사전 실패로 표면화(데몬 정지 없음). 신규 크레이트 없음(U0 계약 소비).

**(c) 구조 노트/불변식**: 불변식 — U5는 U6(구현)를 역참조하지 않고 U0 트레이트만 의존한다(빌드 순서 역전 없음, DAG 비순환). 최종 push 소유자 확정(전송 실패 조건)은 U8 조립 배선으로 결정되며 계약은 양쪽 다 지원한다.

**(d) 트레이드오프**: `AuthTransport` 직접 push를 1차 소유로 삼는 대안은 **미채택(MVP)** — 얇은 전송 소유자 원칙(Q4=A)에 따라 값 반환을 1차로, push 소유는 U8 배선에 위임(계약은 양쪽 지원). `ConsentGate`만 자연 소유로 직접 push.

---

## 12. PBT 속성 실현 (MNT-04) — `proptest`(dev) + U0 `proptest-support` 재사용 [U0 §7 상속]

**(a) 패턴/결정**: U5 PBT는 워크스페이스 `proptest`(PBT-09, U0 확정)를 **dev-dependency**로 사용하고, U0 도메인 제너레이터(`Timestamp` 등)는 U0 **비기본 `proptest-support` feature**를 dev에서 켜 재사용한다(단일 출처 -> 드리프트 방지). U5 고유 제너레이터(`OkcRequest`·목 `HttpTransport` 응답 조합·`TokenSource` 4축 조합·`ConsentLifecycle`/`ConsentRecord`/연산 시퀀스)는 U5가 정의한다. U5가 하위에 제너레이터를 노출할 경우 U0 패턴대로 비기본 `proptest-support` feature 뒤에 게이트한다.

**(b) NFR/규칙 실현**: **U5-NFR-MNT-04**(PBT-07 제너레이터 재사용) · U0-NFR-MNT-02(하위 단위 지속 레코드 round-trip 재사용 소비자로 U5 지목). 확정 속성(FD): PROP-U5-01/02(분류 total+오라클), PROP-U5-03(TLS/토큰/타임아웃 불변식), PROP-U5-04(토큰 우선순위 오라클), PROP-U5-05/06/07(상태머신 Induction/게이트/forward-only), PROP-U5-08/PROP-DE-U5-01(지속 round-trip), PROP-U5-09(고지 내용), PROP-BL-U5-01(파이프라인 목 관측).

**(c) 구조 노트/불변식**: 불변식 — `proptest`가 프로덕션 빌드 그래프에 유입되지 않는다(dev/non-default). U5 고유 제너레이터는 문서화된 도메인 제약(§domain-entities §3.5 일관성 불변식)을 존중한다. 제너레이터 논리 단위는 런타임 DAG 밖(§logical-components §5).

**(d) 트레이드오프**: 없음(U0 §7 상속). PBT-08 상세(케이스 수·shrink·시드·CI)는 Code Generation / Build-and-Test 이월.

---

## FEDERATED-CONFIG 해소 (wave-level 일관 적용 — 이 유닛의 실현)

U5가 config 값을 받는 방식을 명시적으로 확정한다(federated_config_handled = **true**):

| config 값 | 성격 | U5 수신 방식 |
|---|---|---|
| `token` | **U0 core 6필드** | `CredentialProvider`가 U0 `ConfigProvider.current()`(라이브 스냅샷, lock-free `Arc` 로드)로 조회 — reload 팬아웃 반영(§4) |
| `secure_store_enabled` | **U0 core 6필드** | 동상(`ConfigProvider.current()`) |
| `server_endpoint` | **U0 core 6필드** | `AuthTransport`가 `ConfigProvider.current()`로 조회 + U0 `validate_https_url` 재사용(§8) |
| `request_timeout_s` | **federated(비-core)** | **U8 조립루트가 원본 config 파싱 -> `Duration` 타입 값으로 `AuthTransport` 생성자 하향 주입**(§2). U0 `ConfigSnapshot`에서 읽지 않음 |

- **원칙(DEC-FEDERATED-KEYS Q6=A)**: 비-core 값은 U0 `ConfigSnapshot`(core 6필드만 노출)에서 읽지 않고 U8 composition root가 원본 config를 파싱해 **RESOLVED TYPED 값**(예: `Duration`)으로 생성자 주입(downward injection)한다. U0는 FROZEN 유지.
- **U5 소유분**: `AUTH_CONSENT_CONFIG_KEYS = &["request_timeout_s"]` 상수(U0 R-CFG-STRICT-01의 union 집계 대상 — **미지-키 수용 전용, 값 읽기 아님**) + 검증 규칙(양의 정수 `>= 1`).
- **live-reload 범위**: core-field(토큰 등) reload만 U0 관찰자 팬아웃으로 반영(§4 `CredentialProvider` 라이브 스냅샷). federated 파라미터(`request_timeout_s`) live-reload는 **MVP 범위 밖**(재시작으로 변경).

```
config 파일
  -> [core 6필드: token/secure_store_enabled/server_endpoint]
        -> U0 ConfigProvider.current()  -> CredentialProvider / AuthTransport (라이브 조회)
  -> [federated: request_timeout_s]
        -> U8 watcher-bin 파싱 -> Duration -> AuthTransport 생성자 주입 (하향)
        (U0 ConfigSnapshot 은 이 값의 소스가 아님)
```

---

## 13. MANDATORY 카테고리 N/A 판정표

| 카테고리 | 판정 | 근거 |
|---|---|---|
| **Scalability** | N/A | U5는 순수 lib(전송 + 게이트 로직). 자체 런타임·스레드풀·처리량 축 없음. `send`는 요청당 1회 시도(DEC-U5-15). 100k 파일 스케일은 U1, 청크 전송은 U3 소관. |
| **Availability** | N/A | lib 크레이트라 SLA 없음. 전체 Watcher availability는 requirements RESILIENCY-02에서 N/A 확정. |
| **Performance(수치)** | N/A(정성 계약만) | 얇은 전송 소유자 — throughput/latency/peak-memory 수치 게이트 근거 없음. 유일 정성 계약 = 모든 요청 단일 데드라인 유계(§2). MVP는 요청/응답 body 인메모리 버퍼(스트리밍 이월; 청킹 U3). |
| **Resiliency DR/RTO/RPO** | 부분 적용 + 대체로 N/A | §2 타임아웃 + graceful degrade(RESILIENCY-10 전송 절반; 백오프 U4), §3 원자 지속 + fail-safe 강등. RTO/RPO 수치·DR/HA·서킷브레이커·배포/롤백은 순수 lib에 N/A(RESILIENCY-02). RESILIENCY-14 resilience testing(오프라인/타임아웃 시뮬레이션)은 Build-and-Test 이월. |
| **Security(강제 통제)** | N/A(잔존만 표면화) | Security Baseline OFF, RISK-01 수용(config 평문 `token`). 암호화 저장·키관리·시크릿 스캐닝 신설 없음. 유일 잔존 통제 = TLS(§8). 저비용 잔존 위생 = 토큰 유출 위생(§9). secure-store 실 백엔드는 선택적 강화(DEFER, §6). |
| **Logical Components** | 다뤄짐(N/A 아님) | 공개 3 컴포넌트(`AuthTransport`/`CredentialProvider`/`ConsentGate`)의 내부 논리 분해·seam·U0 소비 매핑은 자매 산출물 `logical-components.md`가 소유(추적성 문서 맵). |

---

## 14. Autopilot Decisions (주제 / 선택 / MVP-트림? / 근거)

| id | 주제 | 선택(chosen) | MVP 트림 | 근거 |
|---|---|---|---|---|
| ND-U5-01 | 분류 panic-free 강제 방식 | 순수 매칭 모듈 clippy lint-gate(U0 §1 mirror) + PBT no-panic; `catch_unwind` 없음 | 아니오 | REL-01 total-function을 컴파일타임 강제(§1). 런타임 방어 래퍼는 버그 은폐로 거부. |
| ND-U5-02 | 요청 데드라인 형상 + federated 주입 | 단일 전체 데드라인(`Duration`), U8 하향 주입; connect/read 분리·live-reload 이월 | 예 | REL-02 보존, U0 core 6필드 미침범(§2). connect/read 분리는 과잉. |
| ND-U5-03 | 동의 지속 crash-safety | `std` temp+rename 원자 쓰기 + all-or-nothing 롤백 + fail-safe 강등; WAL/이력 없음 | 예 | 소형 단일 레코드에 temp+rename로 원자성 충분(§3). WAL/append-only 과잉. |
| ND-U5-04 | 동시성 모델 | 무상태 `Send+Sync` 전송 + 캐시-없는 라이브 스냅샷 자격증명 + `ConsentGate` 쓰기 직렬화 `Mutex` | 예 | 토큰 캐시/무효화·`ArcSwap` 게이트 미도입(§4). 라이브 재해소가 회전 반영 자동 + 게이트 읽기 저빈도. |
| ND-U5-05 | HTTP 전송 어댑터 | `HttpTransport` 포트 + `ureq`(blocking, rustls) 어댑터 + 목; async·native-tls 미채택 | 예 | 얇은 전송에 blocking 정합(§5). async 런타임/시스템 C 의존 과잉. |
| ND-U5-06 | secure-store 어댑터 | `SecureStore` seam + `UnavailableSecureStore` null-object(keyring 없음) | 예 | config 평문 1차 + 헤드리스 데몬(§6). keyring 네이티브 결합 NFR-05 부담. |
| ND-U5-07 | 오류 파생 | 운영오류=`thiserror`, 분류/값=U0 serde 소비; `anyhow` 거부 | 아니오 | U0 §5 관례 상속(§7). 구조화 변이 보존. |
| ND-U5-08 | TLS 강제 | 이중 방어(U0 `validate_https_url` + 전송 계층 가드) + rustls-only | 아니오 | SEC-01 유일 잔존 통제(§8). |
| ND-U5-09 | 토큰 위생 | `TokenSecret` redaction 준수 + 전송 시점 헤더 주입 | 아니오 | SEC-02 저비용 위생(§9). 외부 의존 0. |
| ND-U5-10 | 관측 push 소유 | 값 반환 1차 + `ConsentGate`만 직접 push; 전송 실패 push는 U8 배선 위임 | 예 | 얇은 전송 소유자 원칙(§11, Q4=A). 계약은 양쪽 지원. |
| ND-U5-11 | PBT | `proptest`(dev) + U0 `proptest-support` 재사용 + U5 자기 제너레이터 | 아니오 | U0 §7 상속(§12). 프로덕션 그래프 미유입. |

---

## 15. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 단계 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **준수 — blocking 없음** | PBT-09(프레임워크)는 U0에서 워크스페이스 전역 확정, U5 상속(§12) — 신규 blocking PBT 결정 없음. PBT-01 속성 식별은 FD 완료. 각 패턴이 확정 속성에 정렬: §1 -> PROP-U5-01/02, §2 -> PROP-U5-03(타임아웃)/PROP-BL-U5-01, §3 -> PROP-U5-08/PROP-DE-U5-01, §8 -> PROP-U5-03(TLS), §9 -> PROP-U5-03(토큰), §10 -> PROP-U5-09, §4 -> PROP-U5-05(reload 후 상태 보존). PBT-08(케이스/시드/CI)은 Code Generation/Build-and-Test 이월. |
| **Resiliency Baseline** | ON | **준수(부분 적용 + 대체로 N/A) — blocking 없음** | RESILIENCY-10(모든 네트워크 호출 타임아웃 + graceful degrade) = §2(전송 절반; 백오프 U4). 오프라인/연결실패 -> `Timeout`/`Network` 분류(§1)로 U4 공급. 동의 레코드 temp+rename 원자 지속 + fail-safe 강등(§3). RESILIENCY-01: U5는 Wave-내 소비 단위(U0 = Critical DAG 루트). RTO/RPO/DR/HA/서킷브레이커/auto-scaling(RESILIENCY-02/05~13)은 순수 lib에 N/A. RESILIENCY-14 resilience testing은 Build-and-Test 이월. 신규 인프라 통제 없음. |
| **Security Baseline** | OFF | **N/A — 미로딩·미강제** | RISK-01 수용(config 평문 `token`). 유일 잔존 통제 = TLS/`https`(§8, U5-NFR-SEC-01) — §13 검증 규칙의 전송 계층 실현일 뿐 신규 통제 아님. 저비용 잔존 위생 = 토큰 유출 위생(§9). secure-store는 선택적 강화(DEFER, §6). 암호화 저장·키관리·시크릿 스캐닝 신설 없음. |

**블로킹 판정**: 이 단계에 blocking finding 없음. PBT-09는 U0 상속으로 충족, Resiliency는 부분 적용 + 대체로 N/A, Security Baseline은 OFF로 N/A다.

---

## 16. 추적표 (패턴 -> NFR ID -> 규칙 ID -> 논리 컴포넌트)

| 설계 패턴 | NFR ID | 규칙 ID | 안착 논리 컴포넌트 |
|---|---|---|---|
| §1 분류 total-function + panic-free (clippy lint-gate + PBT no-panic) | U5-NFR-REL-01 | R-AT-CLASS-01/02/03 | `AuthTransport` -> ResponseClassifier |
| §2 요청 데드라인 부착 + federated 주입 + graceful degrade | U5-NFR-REL-02 | R-AT-TO, R-AT-CLASS-01 | `AuthTransport` -> RequestAssembler · TransportPipeline |
| §3 동의 지속 원자성 + crash-safety + fail-safe 강등 | U5-NFR-REL-03 | R-CG-PERSIST | `ConsentGate` -> ConsentStore |
| §4 동시성 모델 (무상태 전송 · 라이브 스냅샷 · 게이트 뮤텍스) | U5-NFR-REL-03, U5-NFR-USE-02 | R-CP-04, R-CG-PERSIST | `AuthTransport`·`CredentialProvider`·`ConsentGate` -> ConsentStore |
| §5 `HttpTransport` seam (포트/어댑터 + rustls 이식성) | U5-NFR-MNT-01, U5-NFR-SEC-01 | (seam 계약), R-AT-TLS | `AuthTransport` -> HttpTransportPort · UreqAdapter |
| §6 `SecureStore` null-object 어댑터 | U5-NFR-MNT-02 | R-CP-01 | `CredentialProvider` -> SecureStorePort · UnavailableSecureStore |
| §7 오류 파생 전략 (`thiserror` + U0 taxonomy 소비) | U5-NFR-MNT-03 | R-AT-CLASS, R-CG-* | (전 컴포넌트) -> ErrorSurface |
| §8 TLS/`https` 강제 (이중 방어) | U5-NFR-SEC-01 | R-AT-TLS | `AuthTransport` -> TlsGuard(RequestAssembler) |
| §9 토큰 유출 위생 (`TokenSecret` + 전송 시점 주입) | U5-NFR-SEC-02 | R-AT-TOKEN, R-CP-03 | `AuthTransport` -> RequestAssembler · `CredentialProvider` -> TokenResolver |
| §10 RISK-01 고지 텍스트 | U5-NFR-USE-01 | R-CG-DISCLOSURE | `ConsentGate` -> DisclosureProvider |
| §11 관측 push 경계 + 관측 표면 | U5-NFR-USE-02 | R-CP-03, R-CG-GATE | `CredentialProvider` -> TokenResolver · `ConsentGate` -> ConsentStateMachine |
| §12 PBT 속성 실현 (`proptest` + `proptest-support`) | U5-NFR-MNT-04 | (제너레이터 계약) | (test-support) -> ProptestGenerators |

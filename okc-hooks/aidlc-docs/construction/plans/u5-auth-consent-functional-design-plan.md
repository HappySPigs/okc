# U5 Auth & Consent — Functional Design 계획 및 결정(AUTOPILOT)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U5 Auth & Consent** -> Functional Design (Part: 계획 + 자동결정 게이트)
**작성일**: 2026-09-08
**크레이트**: `auth-consent` (lib) · **소속 컴포넌트**: `AuthTransport`, `CredentialProvider`, `ConsentGate`
**입력 아티팩트**: `unit-of-work.md`(§U5 책임), `component-methods.md`(§U3·U5 시그니처·이월 항목), `unit-of-work-story-map.md`(U5 = US-E4-01~06), `stories.md`(US-E4-*), `requirements.md`(FR-13/FR-14/FR-15 · NFR-04/06/07 · RISK-01/02 · DEP-01/02/05/06/07 · §13 부록), `crates/foundation/src/**`(소비하는 U0 실제 타입/트레이트), U0 Functional Design 3종(스타일·규칙/속성 id 템플릿)
**규칙**: `construction/functional-design.md` · `common/content-validation.md` · 활성 확장 `property-based-testing.md`(PBT-01 강제) · `resiliency-baseline.md`

> **AUTOPILOT 고지**: 사용자 승인(2026-09-08, 게이트 waived)에 따라 이 단계의 모든 미결 질문은 **권장안·MVP 편향**으로 저자가 직접 확정한다. 아래 §3은 통상의 질문 목록을 **자동결정 표**로 대체한다. 이미 확정된 상위 결정(§5 드롭리스트)은 재개봉하지 않는다.

---

## 1. 단위 컨텍스트 (Step 1 — 요약)

U5는 Watcher에서 서버로 나가는 **유일한 아웃바운드 HTTP 경로**와 **동의 게이트**를 소유한다. P2(프라이버시 민감 사용자) 주담당 스토리(US-E4-04/06)의 소재지이며, 서버측 동작(DEP-01/02/05/06/07)은 모두 `[blocked-on-server]` 목(mock) 계약으로만 검증한다.

### 1.1 `AuthTransport` — 얇은 전송 소유자 (Q4=A)
- TLS만 사용(NFR-06), 매 요청 토큰 첨부(FR-13), config 타임아웃 적용(NFR-04)
- 응답/실패를 U0 오류 taxonomy(`TransportError`/`TransportErrorClass`)로 분류: 401->AuthFailed · 5xx->ServerError · PROJECT_BUSY/queue-full(429)->Backpressure · 타임아웃->Timeout · 연결실패->Network
- 프로토콜 의미(U3)·재시도(U4) 미소유. 2xx 본문은 상위(U3)에 그대로 반환

### 1.2 `CredentialProvider` — 토큰 소스 결정
- 1차·기본 = config JSON `token`(U0 소유 `TokenSecret`) + env 폴백(keyring-less 평문, §13)
- 선택적 강화 = OS secure-store(US-E4-02); 세션 불가(헤드리스/데몬) 시 config/env로 안전 폴백
- `resolve_token()` / `token_status()` / `on_config_reload()`(US-E4-03)

### 1.3 `ConsentGate` — RISK-01 고지 + 상시 동의 + 조회/철회
- 최초 실행 고지 확인(US-E4-04) + 상시 동의 부여/참조 저장(US-E4-05) + 조회/철회(US-E4-06)
- 유효 동의 없으면 U8 `SyncCycleCoordinator`의 업로드/커밋 단계 차단(감시·상태·감지는 유지)
- 철회는 전진 방향(forward-only); 동의 부여 참조는 U0 CBOR 코덱으로 로컬 지속

### 1.4 소비하는 U0(파운데이션) 실제 타입/트레이트 (재정의 금지)
- 값/오류: `TokenSecret`, `TransportError`, `TransportErrorClass`, `ErrorClass`, `Timestamp`, `ConsentState`(placeholder projection 대상), `ActiveCondition`(`ConsentBlocked`/`AuthFailed`)
- 코덱: `encode`/`decode`(무손실 round-trip, NFR-13)
- 트레이트: `StatusSink`(`raise_condition`/`clear_condition`), `CriticalEventSink`(`report_auth_failure`), `Logger`, `ConfigReloadObserver`
- config: `ConfigSnapshot`/`WatcherConfig`(`token`/`secure_store_enabled`/`server_endpoint`), `validate_https_url`

---

## 2. Functional Design 실행 계획 (산출물 체크박스)

아래 3종을 `aidlc-docs/construction/u5-auth-consent/functional-design/` 에 생성한다(프론트엔드 파일 없음 — 이 단위는 UI 무소유). 기술중립(Rust스러운 시그니처는 참고용, 인프라 관심사 배제).

- [x] **`domain-entities.md`** — U5가 도입/특화하는 값·엔티티: `AuthTransport`의 `OkcRequest`/`OkcResponse`/`HttpMethod`/`Headers`/`Body` + HTTPS-client 트레이트 seam(`HttpTransport`, `RawHttpRequest`/`RawHttpResponse`); `CredentialProvider`의 `TokenSource`/`TokenStatus`/`CredentialError` + `SecureStore` seam; `ConsentGate`의 `ConsentLifecycle`(U0 `ConsentState` 충돌 회피 명명)/`ConsentGrant`/`GrantId`/`ConsentStatus`/`ConsentDecision`/`BlockReason`/`ConsentError`/`ConsentRecord`(지속); U5 config 하위 스키마(`request_timeout_s`)·`AUTH_CONSENT_CONFIG_KEYS`. U0 타입은 이름으로 참조만(재정의 금지).
  - 커버 컴포넌트: AuthTransport · CredentialProvider · ConsentGate
- [x] **`business-rules.md`** — R-* 규칙(검증·제약·불변식·오류 처리·MVP 트림 엣지케이스) + PROP-* Testable Properties: 상태코드->클래스 매핑(R-AT-*), TLS 강제(R-AT-TLS), 토큰 첨부(R-AT-TOKEN), 타임아웃(R-AT-TO), 토큰 해소 우선순위(R-CP-*), 동의 상태전이·게이트·forward-only(R-CG-*), 지속 round-trip(R-CG-PERSIST). PBT: 분류 total-function 오라클, 토큰 우선순위 오라클, 게이트 불변식, 동의 상태머신(Induction), 지속 round-trip.
  - 커버 NFR/FR: NFR-06 · NFR-04 · FR-13 · 응답 분류 · FR-14 · RISK-01
- [x] **`business-logic-model.md`** — 컴포넌트별 알고리즘/워크플로/데이터 흐름 + 합성(전송 파이프라인: resolve token -> build request -> TLS guard -> dispatch(seam) -> classify -> 2xx body up / TransportError up), 크로스유닛 경계(무엇이 주입/반환되고 무엇이 다른 단위 소유인지) 명시, 컴포넌트별 Testable-Properties 노트 + 확장 컴플라이언스 요약.
  - 커버: 3개 컴포넌트 조립 + U3/U4/U8 경계
- [x] **PBT-01 "Testable Properties" 섹션**(각 산출물 내, 확장 강제): PROP-U5-* — 카테고리 라벨 + 도메인 제너레이터(PBT-07) 요구 기재. 속성 없는 요소는 "No PBT properties identified" 명시.
- [x] **확장 컴플라이언스 요약**(완료 게이트용): PBT / Resiliency / Security 준수·N/A 표(§4).
- [x] 산출물 작성 전 `content-validation.md` 검증(ASCII 화살표만, 박스드로잉 0건, Rust 타입 백틱, 특수문자 이스케이프).

---

## 3. AUTOPILOT 결정 표 (질문 대체)

각 행: 주제 / 선택 옵션 / MVP-트림? / 근거 + 인용. 모두 저자 확정(권장안·MVP 편향).

| id | 주제 | 선택(chosen) | MVP 트림 | 근거 + 인용 |
|---|---|---|---|---|
| DEC-U5-01 | HTTPS 클라이언트 추상화 | 최소 `HttpTransport` 트레이트 seam으로 전송을 감싸고, 구체 async HTTP 스택은 NFR/code-gen 이월 | 예 | U3가 목 대비 테스트 가능해야 하고 FD는 기술중립(비동기 스택 확정은 NFR 관심사). MVP 가이던스 명시 |
| DEC-U5-02 | TLS 강제 지점 | U0 config https-only 검증(`validate_https_url`) + `AuthTransport` 디스패치 직전 non-https URL 거부(방어적 이중화) | 예(이중화 최소) | NFR-06; U0 R-CFG(server_endpoint https만) 재사용, 전송 계층 런타임 가드만 추가 |
| DEC-U5-03 | 상태코드 -> `TransportErrorClass` 매핑(엣지 포함) | 401/403->AuthFailed · 429(+PROJECT_BUSY/queue-full 코드)->Backpressure · 5xx->ServerError · 기타 4xx->ServerError · 3xx(미추적)->ServerError · 타임아웃->Timeout · 연결실패->Network | 예 | U0 taxonomy는 5변이 고정이라 전용 permanent-client-error 클래스 없음. 앱-레벨 코드 body 파싱은 U3 이월, 상태코드+최소 헤더/코드 힌트만 사용. U4 bounded 재시도+에스컬레이션(FR-19)이 기타-4xx 오분류 루프를 상한 |
| DEC-U5-04 | 요청 타임아웃 소스 | U5가 `request_timeout_s` config 키를 `AUTH_CONSENT_CONFIG_KEYS`에 등록(미지-키 수용 전용)하고 검증 규칙(기본 30초, `>=1`)을 소유. 값은 U8 조립루트가 원본 config에서 해소해 요청당 단일 전체 데드라인(타입 값)으로 `AuthTransport`에 하향 주입 — U0 타입드 `ConfigSnapshot`(core 6필드)에서 읽지 않음. 전달 방식 확정은 NFR-Design 교차관심 | 예 | NFR-04 보존; connect/read 분리 타임아웃은 이월(단일 데드라인이 MVP 충분) |
| DEC-U5-05 | 응답 반환 계약 | 2xx만 `OkcResponse`(raw body)로 상위(U3) 반환; 비-2xx는 `TransportError`. 리다이렉트 자동추종 없음 | 아니오 | component-methods §AuthTransport("2xx만 Ok"); 고정 https base 엔드포인트라 리다이렉트 불필요 |
| DEC-U5-06 | 토큰 해소 우선순위 | `SecureStore`(enabled+세션 가용) > config `token` > env 폴백 | 아니오 | U0 R-TOKEN-01(Q7=A) 재사용 — 재개봉 아님, U5는 실행만 |
| DEC-U5-07 | OS secure-store 구현 | `SecureStore` 트레이트 seam + `UnavailableSecureStore`("항상 불가") 기본 impl -> config/env 안전 폴백. 실 keyring 백엔드 DEFER | 예 | MVP 가이던스 명시(§13 config 평문 1차·기본); 헤드리스/데몬 폴백 계약(US-E4-02) 보존. 효과적 MVP 순서 = config > env |
| DEC-U5-08 | env 폴백 변수명 / 토큰 타입 | 환경변수 `OKC_WATCHER_TOKEN`; 해소 토큰 타입 = U0 `TokenSecret`(redacting) | 아니오 | U0 `OKC_WATCHER_CONFIG` 명명 관례 정합; `TokenSecret`는 로그 유출 방지(SEC-02 계약) |
| DEC-U5-09 | 동의 상태 enum 명명 | U5 내부 라이프사이클 = `ConsentLifecycle`(NotAcknowledged/AcknowledgedNotGranted/Granted/Withdrawn); U0 placeholder `ConsentState`(Granted/Blocked/Unknown)로 projection해 `StatusSnapshot.consent`에 노출 | 아니오 | U0가 `ConsentState`를 이미 소유(placeholder)하므로 이름 충돌 회피(U0의 ErrorClass->TransportErrorClass 정정과 동일 패턴). component-methods 4-상태 의미 보존 |
| DEC-U5-10 | 동의 부여 식별자 | 불투명 `GrantId` newtype; 구체 생성(UUID vs 난수)은 code-gen 이월 | 예 | FD 기술중립; component-methods의 `Uuid`는 참고용 |
| DEC-U5-11 | 동의 지속 형태 | 단일 로컬 `ConsentRecord` 파일(U0 CBOR 코덱, temp+rename 원자적 쓰기), 최신 상태만 보존 | 예 | "동의는 단순 로컬 레코드"(MVP 가이던스); FR-14 "참조 저장"; 부여 이력 누적 불필요 |
| DEC-U5-12 | 동의 입력 표면 | CLI(`consent acknowledge/grant/view/withdraw`, ControlPlane 경유) 1차; config-플래그 grant/ack 경로 DEFER | 예 | US-E4-04/05는 "config 플래그 또는 CLI" 허용 — CLI가 최소 완결 경로. config-플래그는 이월(관찰자 팬아웃 복잡도 절감) |
| DEC-U5-13 | 업로드 게이트 범위 | `is_upload_permitted()` == `Permitted` iff `Granted`; 업로드/커밋만 차단, 감시/감지/status는 유지; 재동의 시 다음 사이클 재개 | 아니오 | US-E4-06(Q6=B); U8 코디네이터가 게이트 소비 |
| DEC-U5-14 | 고지 텍스트(disclosure) | RISK-01 4대 필수 내용(연속성·비가역+forward-only·클라 필터 없음·로컬 평문 산출물[config 토큰 포함]) 한국어 초안; 최종 법적 문안은 이월 | 예 | US-E4-04 고지 불변식 체크리스트; NFR-07/FR-15/RISK-01 |
| DEC-U5-15 | 전송 재시도/프로토콜 경계 | `AuthTransport.send`는 1회 시도만, 재시도 미소유(U4), 프로토콜 해석 미소유(U3) | 아니오 | Q4=A 얇은 전송 소유자; component-methods 경계 재확인 |
| DEC-U5-16 | 동의 순서 강제 | `grant()`는 `AcknowledgedNotGranted`/`Withdrawn`에서만 성공; 미확인 상태 grant는 `ConsentError::NotAcknowledged` | 아니오 | US-E4-05 Given(고지 확인 기록되어 있고); component-methods `ConsentError` 변이 |
| DEC-U5-17 | `on_config_reload` 동작 | `CredentialProvider.on_config_reload()`는 토큰/secure_store_enabled 재해석; `ConsentGate.on_config_reload()`는 MVP no-op(config-grant 이월이므로 재해석 대상 없음) | 예 | US-E4-03; U0 R-OBSERVER-02(token 변경 -> U5 재해석). config-grant DEFER(DEC-U5-12)로 ConsentGate 재해석 불필요 |

---

## 4. MANDATORY-카테고리 N/A 표 + 확장 컴플라이언스

### 4.1 MANDATORY 워크플로 카테고리 적용/판정
| MANDATORY 카테고리 | 이 산출물 적용 | 판정 근거 |
|---|---|---|
| Content Validation(`content-validation.md`) | **적용** | 전 산출물 ASCII 화살표(`A -> B`)만·박스드로잉 0건·Rust 타입 백틱·특수문자 이스케이프 준수 |
| Question Format(`question-format-guide.md`) | **N/A** | AUTOPILOT — 질문 미발행, §3 결정 표로 대체 |
| Rule Details 로딩 | **적용** | `functional-design.md` + 활성 확장(PBT/Resiliency) 로드 후 설계 |
| Audit/State 기록 | **N/A(쓰기 범위 외)** | STRICT WRITE SCOPE — `aidlc-state.md`/`audit.md` 미접촉(상위 오케스트레이터 소관) |

### 4.2 확장 컴플라이언스
| 확장 | 활성 | 이 단계(U5 FD) 적용 | 계획/판정 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제(PBT-01)** | 각 산출물 "Testable Properties" 섹션 필수. 핵심: 응답 분류 total-function 오라클(PROP-U5-01), 토큰 우선순위 오라클(PROP-U5-04), 업로드 게이트 불변식(PROP-U5-06), 동의 상태머신 Induction(PROP-U5-07), `ConsentRecord` round-trip(PROP-U5-08, U0 코덱 재사용). 도메인 제너레이터(PBT-07) 요구 기재. 프레임워크(PBT-09)는 NFR 이월(Rust=proptest 유력). 미준수 시 blocking |
| **Resiliency Baseline** | ON | **부분 적용** | NFR-04 요청 타임아웃(모든 네트워크 호출 데드라인) = RESILIENCY-10 전송 절반(백오프는 U4). 오프라인/연결실패 -> `Network`/`Timeout` 분류로 U4 graceful degrade 공급. 동의 레코드 temp+rename 원자적 지속(부분쓰기 무손상). RPO/RTO 수치·배포/롤백·HA/DR은 U5(순수 전송+게이트 로직)에 **N/A**(인프라/상위 소관) |
| **Security Baseline** | OFF | **N/A** | 미로딩·미강제. config 평문 `token`(RISK-01)은 문서화된 수용 위험. 잔존 통제 = TLS 강제(NFR-06) + `TokenSecret` 로그 리댁션만. secure-store는 선택적 강화(DEFER) |

---

## 5. 드롭리스트 (이미 확정 — 재설계/재개봉 금지)

| 항목 | 확정 내용 | 출처 |
|---|---|---|
| U0 공유 값/오류/코덱 | `TransportError`/`TransportErrorClass`/`ErrorClass`/`TokenSecret`/`Timestamp`/`ConsentState`(placeholder)/코덱 `encode`·`decode` 모두 U0 소유(컴파일+테스트 검증) | `crates/foundation/src/**`, U0 FD |
| U0 싱크 계약 트레이트 | `StatusSink`/`CriticalEventSink`/`Logger`/`ConfigReloadObserver`(모두 `Send+Sync`) U0 소유, U5는 소비만 | `core_types/sink.rs` |
| 토큰 우선순위(Q7=A) | secure-store > config > env (U0 R-TOKEN-01) | U0 `business-rules.md` §6 |
| 토큰 저장(§13) | config 평문 1차·기본 + env 폴백 + secure-store opt-in | requirements §13 부록 |
| FQ-1=A | okc-core 콘텐츠 주소 재현 없음, 권위 `vault_content_id` 서버 소유 | requirements FQ |
| FQ-2=A | 최신 상태 대체(재스냅샷 diff) | requirements FQ |
| 안전 한도 상수 | 총<=20 GiB, 파일당<=2 GiB, 수<=100k (U1 소관) | requirements; U0 R-LIMIT-01 |
| Wire 포맷 | CBOR(내부 지속), config는 JSON | Q1=B, U0 FD |
| Q4=A | AuthTransport = 얇은 전송 소유자 | component-methods §AuthTransport |
| Security Baseline | OFF (RISK-01 수용) | 확장 설정 |
| 서버측 DEP | DEP-01/02/05/06/07 = `[blocked-on-server]` 목 계약 | stories US-E4-*, requirements §8 |

---

## 6. 다음 단계(참고)
U5 Functional Design 산출물 3종 완료 -> **U5 NFR Requirements**(per-unit 루프 다음 스테이지: 요청 타임아웃 수치·백오프 상호작용·secure-store 백엔드·async HTTP 스택·proptest 프레임워크 확정). 이후 U5 NFR Design -> (Infrastructure Design 해당 시) -> U5 Code Generation.

# U5 auth-consent — Code Summary (코드 요약)

**크레이트**: `auth-consent` (lib) · **단계**: CONSTRUCTION -> Per-Unit Loop -> U5 -> Code Generation
**성격**: 인증 전송 + 자격증명 해소 + 동의 게이트. `foundation`(U0)에만 의존(비순환). U0 소유 타입(`TokenSecret`/`TransportError(Class)`/`ConsentState`/`StatusSink`/`ConfigProvider`/`validate_https_url`/코덱)을 재정의 없이 소비.

## 1. 구현된 컴포넌트 + 공개 API 표면

3개 공개 애그리게이트 + seam/값 계층(`lib.rs` 재-export):

- `AuthTransport` (무상태 `Send + Sync`) — `new(credential, transport, config, deadline)`, `send(req: OkcRequest) -> Result<OkcResponse, TransportError>`. 재시도/프로토콜 해석 없음. seam `HttpTransport`(`execute`, 1회 시도), 프로덕션 어댑터 `UreqAdapter`. 봉투 `OkcRequest`/`OkcResponse`/`RawHttpRequest`/`RawHttpResponse`/`Headers`/`Body`/`HttpMethod`/`HttpError`. 순수 분류 `classify`/`classify_with_code`/`classify_error` + `Classification`. 조립 `assemble`(TLS 가드 + 토큰 헤더 1회 + 데드라인).
- `CredentialProvider` — `new(...)`/`with_defaults(config)`, `resolve_token() -> Result<TokenSecret, CredentialError>`, `token_status() -> TokenStatus`(원문 미노출). seam `SecureStore`(+ `UnavailableSecureStore` null-object)/`EnvReader`(+ `ProcessEnv`). 값 `TokenSource`/`TokenStatus`/`CredentialError`(`Missing`/`Empty`/`SecureStoreUnavailable`).
- `ConsentGate` — `open(...)`/`open_with_system_clock(...)`, `acknowledge()`/`grant()`/`withdraw()`/`view()`/`is_upload_permitted() -> ConsentDecision`/`ensure_acknowledged()`/`disclosure_text()`/`consent_state() -> ConsentState`. seam `Clock`(+`SystemClock`), `ConsentStore`(원자 지속 + fail-safe 로드), 순수 `decide`/`project`/`sanitize`. 값 `ConsentLifecycle`(4-상태)/`ConsentRecord`/`ConsentGrant`/`GrantId`/`ConsentDecision`/`BlockReason`/`ConsentStatus`/`ConsentError`.
- seam `ConfigSource`(U0 `ConfigProvider` blanket impl), federated 상수 `AUTH_CONSENT_CONFIG_KEYS`/`DEFAULT_REQUEST_TIMEOUT_S`, `validate_request_timeout_s(i64) -> Result<Duration, TimeoutConfigError>`.

## 2. 모듈 레이아웃

`src/lib.rs`(재-export + federated key + `#![deny(missing_docs)]`), `config_source.rs`, `consent/{mod,types,store,disclosure}.rs`, `credential/{mod,secure_store}.rs`, `transport/{mod,types,classifier,assembler,ureq_adapter}.rs`, `testing.rs`(mock/stub, `cfg(any(test, feature="proptest-support"))`), `generators.rs`(`proptest-support`). 순수 표면(`config_source`/`types`/`disclosure`/`classifier`/`assembler`/`secure_store`, credential/mod)은 `#![deny(clippy::unwrap_used, expect_used, indexing_slicing, panic)]` lint-gate. I/O 수행 모듈(`consent/mod`/`consent/store`/`transport/mod`/`ureq_adapter`)은 비적용.

## 3. 외부 의존성

- `foundation` (path, U0): 위 소비 타입 전부.
- `serde` — `ConsentRecord`/`ConsentGrant`/`ConsentLifecycle`/`GrantId` 파생(CBOR round-trip).
- `thiserror` — 오류 파생. `url` — base+path join(U0 `validate_https_url` 재사용). `ureq` — 프로덕션 `HttpTransport`(rustls TLS, native-tls 아님).
- `proptest` — optional, `proptest-support` 하 dev-only(`default = []`).

## 4. 적용된 MVP 축소

- `AuthTransport` 는 재시도/백오프를 하지 않고(재시도 -> U4), 2xx body 를 해석하지 않는다(-> U3). `HttpTransport.execute` 는 1회 시도(DEC-U5-15).
- 실 keyring 백엔드 없음 — `UnavailableSecureStore` null-object 기본 주입, 외부 keyring 크레이트 미유입(Code Generation 이월). 실효 순서 secure-store -> config -> env 에서 secure 실패/불가/미저장은 비치명 폴백 -> config -> env.
- `CredentialProvider` 는 캐시 없이 매 호출 라이브 해소(config 리로드/토큰 회전 자동 반영). config-grant 리로드 훅은 no-op(DEC-U5-17).
- `GrantId` = `grant-{unix_nanos}-{seq}`(UUID 는 code-gen 이월, DEC-U5-10).
- classifier: permanent-client 클래스 부재 — 기타 4xx/3xx -> `ServerError`(U4 가 상한). Backpressure 는 상태코드 + 알려진 코드 문자열(`BACKPRESSURE_CODES`) 힌트로만 승격, body 스키마 깊은 파싱 없음. 소스 헤더/코드 계약은 잠정(`[blocked-on-server]` 목 검증).
- `ureq` crypto provider(ring vs aws-lc-rs) 확정은 Build-and-Test 이월(TS-U5-03). http fallback 없음(비-https 는 `assemble` TLS 가드가 거부).
- U5 는 `request_timeout_s` 값을 직접 읽지 않음 — U8 이 파싱·검증해 `Duration` 하향 주입(`AUTH_CONSENT_CONFIG_KEYS` 는 미지-키 수용 전용).
- SEC-02: `Headers`/`Body`/`RawHttpRequest` `Debug` 는 토큰/본문 리댁션, 오류 `detail` 에 토큰 원문 미포함.

## 5. 테스트 커버리지 (35건)

- 단위(`src/**` 인라인, 26건): lib(timeout 검증/키 상수) 2; consent(전체 라이프사이클/재-open 지속/손상 fail-safe/ack 게이트/projection/StatusSink push/고지 4요소) 7; credential(config>env, env 폴백, 공백 -> Empty, 전부 부재 -> Missing, secure 승리, secure 불가 폴백, 기본 unavailable) 7; transport(2xx 토큰+타임아웃 부착, 토큰 부재 미발송, 상태코드 분류, 전송 실패 매핑) 4; classifier(오라클 예제/전 코드 total no-panic/코드 승격/전송실패 매핑) 4; assembler(https+토큰1회+타임아웃/비-https 거부·토큰 미유출) 2.
- Property(`tests/prop_auth_consent.rs`, `proptest-support` 게이트, 9건): PROP-U5-01/02(classify 오라클 total + `to_error_class` total), PROP-U5-03/PROP-BL-U5-01(파이프라인 불변식 https+토큰1회+데드라인·부재시 미발송), PROP-U5-04(토큰 우선순위 4축 오라클), PROP-U5-05/06/07(동의 상태머신 참조모델 대조 + reload + `Permitted iff Granted` + forward-only), PROP-U5-08/PROP-DE-U5-01(`ConsentRecord`/`ConsentGrant` round-trip), PROP-DE-U5-02(projection/decide total) + PROP-U5-09(고지 4대 필수 내용 회귀 방지).
- 제너레이터(`generators.rs`) + 테스트 더블(`testing.rs`): `arb_okc_request`/`arb_http_error`/`arb_consent_lifecycle`/`arb_consistent_record`/`arb_consent_grant`/`Op` 등; `MockHttpTransport`(요청 캡처), `StaticConfig`/`StaticEnv`/`StaticSecureStore`, `FixedClock`, `RecordingStatusSink`.

## 6. 검증 사실 (확정)

전체 워크스페이스가 빌드되며 모든 크레이트 테스트가 통과한다 (foundation 32 / content-core 17 / change-detect 26 / sync-state 15 / auth-consent 35 / observability 24 = 총 149건, 실패 0). 그리고 `cargo clippy --all-targets --features proptest-support -- -D warnings` 가 모든 크레이트에서 CLEAN 이다.

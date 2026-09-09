# U5 Auth & Consent — Business Rules (결정 규칙 / 검증 로직 / 제약)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U5 Auth & Consent** -> Functional Design -> 산출물 2/3 (`business-rules.md`)
**작성일**: 2026-09-08
**크레이트**: `auth-consent` (lib) · **소속 컴포넌트**: `AuthTransport`, `CredentialProvider`, `ConsentGate`
**전제(AUTOPILOT 확정, 계획 §3)**: DEC-U5-02(TLS 이중화) · DEC-U5-03(상태코드 매핑) · DEC-U5-04(request_timeout_s=30) · DEC-U5-06(우선순위) · DEC-U5-07(secure-store DEFER) · DEC-U5-13(게이트 범위) · DEC-U5-16(순서 강제) · Q4=A · Q7=A · NFR-04/06/07 · FR-13/14/15 · RISK-01

> **문서 성격**: U5가 소유하는 **결정 규칙·검증 로직·제약**을 정의한다. 타입 정의는 `domain-entities.md`가 소유하며 이 문서는 그 타입명을 **그대로 재사용**한다. 알고리즘/흐름은 `business-logic-model.md`가 소유한다. U0 소유 규칙(R-TOKEN-01 우선순위·R-CLASS-01 매핑·R-CFG 검증 등)은 **재정의하지 않고 참조**한다.
>
> **표기 규약**: 기술중립 설계. Rust스러운 시그니처는 참고용. ASCII 화살표(`A -> B`)만. English 식별자명 유지.

---

## 1. AuthTransport — 응답 분류 규칙 (U0 taxonomy)

**소유 경계**: `AuthTransport`는 응답/실패를 U0 `TransportErrorClass`로 **분류(classify)** 만 한다. `TransportErrorClass -> ErrorClass` 매핑과 재시도 판정은 U0(`to_error_class`)/U4 소유이며 U5는 관여하지 않는다(재시도 미소유, DEC-U5-15).

### R-AT-CLASS-01 (상태코드/실패 -> `TransportErrorClass` 전체 매핑, DEC-U5-03)

| 입력(HTTP 상태 또는 전송 실패) | -> `TransportErrorClass` | 근거 |
|---|---|---|
| 2xx | (오류 아님 — `Ok(OkcResponse)` 반환) | 정상 응답, body는 U3에 반환 |
| 401 | `AuthFailed` | 인증 실패 — 재시도 제외, 인증 흐름 위임(US-E4-03). `CriticalEventSink.report_auth_failure` + `StatusSink.raise_condition(AuthFailed)` |
| 403 | `AuthFailed` | 인가 실패 — 401과 동일 abort+위임 의미(토큰 문제 계열) |
| 429 (또는 응답 코드 PROJECT_BUSY / queue-full) | `Backpressure` | 서버 혼잡 — 오류 아닌 정상 지연(U4 백오프) |
| 5xx | `ServerError` | 서버측 일시 오류 — 재시도 대상 |
| 기타 4xx (400/404/409/422 등) | `ServerError` | **MVP 트림**: U0 taxonomy에 permanent-client-error 클래스 없음. ServerError로 분류(재시도 가능)하되, U4 연속실패 카운트+에스컬레이션(FR-19)이 오분류 루프를 상한. 앱-레벨 코드 정밀 분류는 U3 이월 |
| 3xx (리다이렉트) | `ServerError` | **MVP 트림**: 고정 https base 엔드포인트라 리다이렉트 미추종(DEC-U5-05), 예기치 않은 3xx는 서버측 이상으로 취급 |
| 요청 타임아웃(`HttpError::Timeout`) | `Timeout` | 데드라인 초과(NFR-04) — 재시도 대상 |
| 연결 실패(`HttpError::Connect`/`Tls`/`Io`) | `Network` | 네트워크 단절/TLS 핸드셰이크 실패 — 오프라인/재시도(U4 graceful degrade) |

### R-AT-CLASS-02 (전역성 / total function)
- `AuthTransport`의 분류는 **모든** 입력(2xx / 정의된 상태코드 / 미정의 상태코드 / 전송 실패)에 대해 정확히 하나의 결과(`Ok` 또는 단일 `TransportErrorClass`)를 반환하는 **전(total) 함수**다 — 미정의·패닉 경로 없음. 미정의/미래 상태코드는 R-AT-CLASS-01의 범위 규칙(4xx->ServerError, 5xx->ServerError, 그 외 비정상->Network)으로 흡수한다.

### R-AT-CLASS-03 (PROJECT_BUSY/queue-full 인식 범위, MVP 트림)
- Backpressure 인식은 **HTTP 429가 1차 신호**다. 서버가 PROJECT_BUSY/queue-full을 다른 상태(예: 503 + 코드) 또는 헤더/얕은 well-known 필드로 표현하면, `AuthTransport`는 **최소 힌트(상태코드 + 알려진 코드 문자열)** 만으로 Backpressure로 승격한다. 응답 body 스키마 깊은 파싱은 U3 이월(프로토콜 의미 비소유). 이 동작은 `[blocked-on-server]` 목 계약으로 검증한다.

### R-AT-TLS (TLS 강제, NFR-06 — DEC-U5-02)
- `AuthTransport`는 **TLS(https) 위에서만** 전송한다. 이중 방어:
  1. **config 계층**: `server_endpoint`는 U0 `validate_https_url`로 https만 허용(비-https는 최초 로드 실패). U5는 이를 재사용.
  2. **전송 계층**: 요청 조립 시 확정 절대 URL의 스킴이 `https`가 아니면 `HttpTransport.execute` **호출 전 거부**(내부 오류로 표면화, seam 미도달). 정상 경로에서는 config 검증으로 이미 https이므로 이 가드는 회귀 방어다.
- **잔존 통제**: Security OFF 하에서 TLS는 in-transit 가로채기만 완화한다(RISK-01: 서버로의 공개 자체는 미완화). 이 사실은 `disclosure_text`가 고지한다.

### R-AT-TOKEN (요청당 토큰 첨부, FR-13)
- `AuthTransport.send`는 **매 요청** `CredentialProvider.resolve_token()` 결과(`TokenSecret`)를 요청 헤더에 첨부한 뒤 전송한다. 토큰 헤더 첨부는 `OkcRequest.headers`가 아니라 **전송 시점 `RawHttpRequest` 조립 단계**에서 주입한다(호출자가 토큰을 다루지 않게, 유출면 축소).
- **토큰 부재/공백 처리**: `resolve_token()`이 `CredentialError::Missing`/`Empty`이면 **요청을 보내지 않고**, 사전 실패를 상위에 표면화한다(US-E4-01 AC3: 업로드 미시작 + CLI `status`/로그에 actionable 오류). 이때 `StatusSink.raise_condition(AuthFailed)` 성격의 조건 또는 credential-missing 표면화를 push한다(구체 조건 push는 코디네이터/U6 결합 — `business-logic-model.md` §경계).
- **로그 유출 금지(SEC-02 계약 준수)**: 토큰은 `TokenSecret`로만 다루며 로그/오류 `detail`에 원문을 넣지 않는다(`Debug`/`Display` = `***`).

### R-AT-TO (요청 타임아웃, NFR-04 — DEC-U5-04)
- **모든** 요청은 `request_timeout_s`(config, 기본 30초) 데드라인을 부착해 전송한다 — 데드라인 없는 요청은 존재할 수 없다(RESILIENCY-10 전송 절반). 단일 전체 데드라인(connect+응답 포함); connect/read 분리는 이월.
- 데드라인 초과는 `HttpError::Timeout -> TransportErrorClass::Timeout`(R-AT-CLASS-01)로 분류된다.

---

## 2. CredentialProvider — 토큰 해소 규칙

**소유 경계**: 우선순위 규칙 자체는 U0 R-TOKEN-01(Q7=A)이 확정했다. U5는 그 **실행**과 secure-store seam 폴백 처리만 소유한다(재개봉 아님).

### R-CP-01 (해소 우선순위 실행, U0 R-TOKEN-01 실행)
```
1) SecureStore   — secure_store_enabled == true  AND  read_token() == Ok(Some(token))   (최우선)
2) config token  — WatcherConfig.token 필드(TokenSecret, 평문 1차·기본)
3) Env fallback  — 환경변수 OKC_WATCHER_TOKEN                                            (최후)
```
- **secure-store 스킵/폴백**: `secure_store_enabled == false`이면 1)을 건너뛴다. `true`라도 `read_token()`이 `Err(Unavailable)`/`Err(Backend)`/`Ok(None)`이면 **config `token` -> env 순으로 안전 폴백**한다(US-E4-02, `CredentialError::SecureStoreUnavailable`은 내부 신호로 소비되고 최종 오류로 승격하지 않음).
- **MVP 실효 순서(DEC-U5-07)**: `UnavailableSecureStore` 기본 주입 시 1)은 항상 실패하므로 실효 순서 = config `token` -> env.

### R-CP-02 (부재/공백 판정, U0 R-TOKEN-02 정합)
- 세 소스 모두 없으면 `CredentialError::Missing`. 소스에 존재하나 값이 공백(빈 문자열)이면 `CredentialError::Empty`. 이는 U0가 실패로 만들지 않으며(토큰은 config 선택 필드), **인증 시점(U5)에 표면화**된다(요청 미발송 + 401 이전 사전 판정).

### R-CP-03 (`token_status` 관측 표면)
- `token_status()`는 부수효과 없이 현재 해소 결과의 존재/소스를 반환한다(`present`/`source`). CLI `status`·헬스(`CredentialReadable` liveness 신호)의 근거다. **토큰 원문을 노출하지 않는다**(존재/소스만).

### R-CP-04 (`on_config_reload` 재해석, US-E4-03 — DEC-U5-17)
- config 리로드(토큰/`secure_store_enabled` 변경) 성공 후 U0 `ConfigReloadObserver.on_config_reload()`가 호출되면, `CredentialProvider`는 다음 `resolve_token()`에서 **새 config 스냅샷을 재조회**해 재해석한다(회전/재입력된 토큰 반영). 캐시된 토큰이 있으면 무효화한다. 이로써 config 편집 + CLI `reload`만으로 인증이 재개된다(별도 UI 없음).

---

## 3. ConsentGate — 동의 규칙

### R-CG-TRANSITION (상태 전이 합법성, DEC-U5-16)

| from `ConsentLifecycle` | 연산 | to | 결과/오류 |
|---|---|---|---|
| `NotAcknowledged` | `acknowledge()` | `AcknowledgedNotGranted` | 고지 확인 기록(원자적 지속) |
| `NotAcknowledged` | `grant()` | (불변) | `ConsentError::NotAcknowledged`(선확인 필수) |
| `AcknowledgedNotGranted` | `grant()` | `Granted` | `ConsentGrant`(신규 `GrantId`+`granted_at`) 생성·지속 |
| `Granted` | `grant()` | (불변) | `ConsentError::AlreadyGranted` |
| `Granted` | `withdraw()` | `Withdrawn` | 철회 기록. `grant` 참조는 보존 가능하나 게이트는 차단 |
| `Withdrawn` | `grant()` | `Granted` | **재부여** — 신규 `GrantId`, 대기 업로드 다음 사이클 재개 |
| 모든 상태 | `acknowledge()` (이미 확인됨) | (불변, 멱등) | 성공(멱등) — 재확인 무해 |
| 모든 상태 | `view()` | (불변) | `ConsentStatus` 반환(부수효과 없음) |

- **불법 전이는 상태를 바꾸지 않고 `ConsentError`만 반환**한다(순수 판정 + 지속은 성공 시에만).
- **`acknowledge` 멱등**: 이미 확인된 상태에서 재확인은 상태를 되돌리지 않는다(특히 `Withdrawn`에서 `acknowledge()`는 `Withdrawn` 유지 — 고지 확인 플래그는 이미 true).

### R-CG-GATE (업로드 게이트, DEC-U5-13 / US-E4-06)
- `is_upload_permitted() -> ConsentDecision`:
  - `Granted` -> `Permitted`
  - `NotAcknowledged` -> `Blocked(NeedsAcknowledgment)`
  - `AcknowledgedNotGranted` -> `Blocked(NotGranted)`
  - `Withdrawn` -> `Blocked(Withdrawn)`
- **차단 범위**: 업로드/커밋 단계만 차단한다. **감시(FR-01)·변경 감지·재조정·CLI `status`는 계속 동작**한다(Q6=B). U8 `SyncCycleCoordinator`가 이 판정을 사이클 업로드 직전에 소비한다.
- **차단 시 관측**: 차단 상태에서는 `StatusSink.raise_condition(ConsentBlocked)`, `Granted` 전이 시 `clear_condition(ConsentBlocked)`를 push한다(push는 게이트가 수행하거나 코디네이터가 판정을 받아 수행 — `business-logic-model.md` §경계).

### R-CG-FORWARD-ONLY (forward-only 철회, RISK-01 / US-E4-06)
- `withdraw()`는 **이후 업로드/커밋만 차단**하며 **이미 업로드된 콘텐츠를 회수하지 않는다**(불가). 서버측 강제·삭제는 `[blocked-on-server]`(DEP-02/DEP-06). 재부여(`Withdrawn -> Granted`) 시 대기 중이던(다음 재스냅샷) 업로드가 재개된다(FQ-2=A 재스냅샷 모델이므로 "큐"가 아니라 다음 사이클이 최신 상태를 재계산).

### R-CG-ACK-GATE (고지 미확인 시 업로드 거부, US-E4-04)
- 기록된 고지 확인(`acknowledged == false` 또는 상태 `NotAcknowledged`)이 없으면 `ensure_acknowledged() -> Err(ConsentError::NotAcknowledged)`이고 게이트는 `Blocked(NeedsAcknowledgment)`다. 즉 **고지 확인 전에는 어떤 업로드도 시작되지 않는다**(FR-15/NFR-07의 informed consent 전제).

### R-CG-PERSIST (동의 지속 무손실, NFR-13 — DEC-U5-11)
- `acknowledge`/`grant`/`withdraw` 성공은 `ConsentRecord`를 **U0 CBOR 코덱으로 원자적(temp+rename) 지속**한다. 지속 실패 시 `ConsentError::PersistFailed`이고 **인메모리 상태도 롤백**한다(지속되지 않은 전이는 관측되지 않음 — all-or-nothing).
- **round-trip 불변식**: 로드 시 `decode(encode(record)) == record`(NFR-13). 재시작 후 복구된 상태가 원값과 정확히 동일해 상시 동의가 지속된다.
- **일관성 불변식**(로드 검증): `state == Granted`이면 `grant.is_some()`; 위반 레코드(손상)는 안전 기본(`AcknowledgedNotGranted` 또는 `NotAcknowledged`)으로 강등해 **차단 우선(fail-safe: 의심 시 업로드 금지)** 한다.

### R-CG-PROJECT (status projection, `domain-entities.md` §3.7)
- `ConsentLifecycle -> ConsentState`(U0 3변이) 사상은 결정적·total: `Granted->Granted`, `Withdrawn->Blocked`, `NotAcknowledged`/`AcknowledgedNotGranted->Unknown`.

### R-CG-DISCLOSURE (고지 내용 필수, US-E4-04)
- `disclosure_text()`는 `domain-entities.md` §3.6의 **4대 필수 내용**(연속성·비가역/forward-only·클라 필터 없음·로컬 평문 산출물[config 토큰 포함])을 모두 포함해야 한다. 하나라도 결여하면 US-E4-04 위반.

---

## 4. Testable Properties (PBT-01) — 규칙(RULES) 계층

> **확장 강제(PBT-01, Full)**: 각 속성에 카테고리 라벨 {Round-trip, Invariant, Idempotence, Commutativity, Oracle, Induction, Easy verification}과 도메인 제너레이터(PBT-07) 요구를 기재한다. 프레임워크(PBT-09)는 NFR 이월(Rust=proptest 유력). 타입 계층 속성은 `domain-entities.md` §6, 흐름 속성은 `business-logic-model.md` §6이 각각 소유(중복 회피).

### 4.1 응답 분류 (핵심 — 대표 NFR 속성)
- **PROP-U5-01 — 상태코드 -> `TransportErrorClass` 분류는 total function이며 매핑 표 오라클과 일치** (카테고리: **Invariant** + **Oracle**; R-AT-CLASS-01/02)
  - **속성**: 모든 상태코드(100~599) 및 모든 `HttpError` 변이에 대해 분류가 정확히 하나의 결과(`Ok`(2xx) 또는 단일 `TransportErrorClass`)를 반환하고, 결과는 §1 매핑 표(참조 오라클)와 일치. 미정의 상태코드도 범위 규칙으로 흡수(누락·패닉 없음).
  - **제너레이터(PBT-07)**: 전 상태코드 열거(100~599 전수 — Easy verification), `HttpError` 전 변이, 429+PROJECT_BUSY/queue-full 코드 변형.
- **PROP-U5-02 — 분류의 U0 taxonomy 폐쇄성** (카테고리: **Invariant**)
  - **속성**: 분류 결과 클래스는 항상 U0 `TransportErrorClass` 5변이 중 하나이며, `to_error_class()`(U0)로 재시도 판정이 total하게 이어진다(U5는 새 오류 계열을 만들지 않음).

### 4.2 TLS / 토큰 / 타임아웃 불변식
- **PROP-U5-03 — TLS 강제 + 토큰 첨부 + 타임아웃 부착 불변식** (카테고리: **Invariant**; R-AT-TLS/TOKEN/TO; NFR-06/FR-13/NFR-04)
  - **속성(TLS)**: `HttpTransport.execute`에 도달하는 모든 `RawHttpRequest.url`은 스킴이 `https`다(비-https는 도달 전 거부). Oracle: URL 스킴 검사.
  - **속성(토큰)**: 토큰 해소가 `Ok`인 모든 전송은 토큰 헤더를 정확히 1회 포함한다; 해소가 `Missing`/`Empty`이면 `execute`가 **호출되지 않는다**(요청 미발송).
  - **속성(타임아웃)**: `execute`에 도달하는 모든 `RawHttpRequest.timeout`은 config `request_timeout_s`와 동일한 양의 데드라인이다(데드라인 없는 요청 부재).
  - **제너레이터(PBT-07)**: `OkcRequest` 변형(메서드/경로/헤더/본문), 토큰 상태(present/missing/empty), config `request_timeout_s` 경계(1·대값). 목 `HttpTransport`로 관측(전송된 `RawHttpRequest` 캡처).

### 4.3 토큰 우선순위
- **PROP-U5-04 — `resolve_token` 우선순위 오라클** (카테고리: **Oracle** + **Invariant**; R-CP-01/02)
  - **속성**: 임의의 (secure_store_enabled, secure-store 결과, config token 유무, env 유무) 조합에서 해소된 `TokenSource`는 §2 우선순위 표(참조 오라클)와 일치한다. secure-store 불가/미저장이면 config->env로 결정적 폴백; 셋 다 없으면 `Missing`, 공백이면 `Empty`.
  - **제너레이터(PBT-07)**: 4개 축의 조합 제너레이터(secure_store on/off x seam 결과{Ok(Some)/Ok(None)/Unavailable/Backend} x config{있음/없음/공백} x env{있음/없음/공백}). 유한 조합 전수 가능(Easy verification).

### 4.4 동의 게이트 / 상태머신
- **PROP-U5-05 — 동의 상태 전이 모델** (카테고리: **Induction** / 상태 기반)
  - **속성**: 임의의 연산 시퀀스(`acknowledge`/`grant`/`withdraw`/`view` + persist/reload)에 대해 실제 `ConsentGate` 상태가 §3 R-CG-TRANSITION 참조 모델과 관측적으로 동일. 불법 전이는 상태 불변 + `ConsentError`. `acknowledge` 멱등. 재시작(reload) 후 상태 보존(R-CG-PERSIST와 결합).
  - **제너레이터(PBT-07)**: 임의 연산 시퀀스 + 중간 persist/reload 주입. 참조 모델(순수 상태머신)을 오라클로.
- **PROP-U5-06 — 업로드 게이트 불변식** (카테고리: **Invariant** + **Easy verification**; R-CG-GATE)
  - **속성**: `is_upload_permitted() == Permitted` **iff** `state == Granted`. 그 외 상태는 정확히 대응 `BlockReason`으로 `Blocked`(상태<->사유 1:1). 유한 상태 전수 검증.
- **PROP-U5-07 — forward-only 철회** (카테고리: **Invariant**; R-CG-FORWARD-ONLY)
  - **속성**: 어떤 연산 시퀀스에서도 `withdraw()`는 이미 기록된 업로드 이력/사실을 되돌리지 않는다(회수 없음); 철회 후 게이트는 재부여 전까지 항상 `Blocked`. (서버측 삭제는 범위 밖 `[blocked-on-server]`.)

### 4.5 지속 무손실
- **PROP-U5-08 — `ConsentRecord` round-trip** (카테고리: **Round-trip**; PBT-02; NFR-13; R-CG-PERSIST)
  - **속성**: `decode(encode(record)) == record` for 모든 유효 `ConsentRecord`(§domain-entities §3.5 일관성 만족). 손상 레코드는 fail-safe 강등(차단 우선) — 이는 별도 Invariant.
  - **제너레이터(PBT-07)**: 일관성 불변식 만족 `ConsentRecord` + 손상(불일치) 레코드 주입.

### 4.6 고지 내용
- **PROP-U5-09 — 고지 텍스트 필수 내용 포함** (카테고리: **Invariant** + **Easy verification**; R-CG-DISCLOSURE)
  - **속성**: `disclosure_text()`는 4대 필수 내용 요소(연속성·비가역/forward-only·클라 필터 없음·로컬 평문 산출물)를 모두 포함한다(키워드/문구 존재 검사). 정적 문자열이라 예제 검증도 가능하나 회귀 방지로 속성화.

### 4.7 속성 없음(No PBT properties identified) 판정
| 규칙/요소 | 판정 | 근거 |
|---|---|---|
| R-AT-CLASS-03 (PROJECT_BUSY 인식) | PROP-U5-01에 흡수(429/코드 힌트 경로 포함) | 깊은 body 파싱은 U3 소유 — U5 규칙 계층 독립 속성 없음 |
| R-CP-03 (`token_status` 표면) | PROP-U5-04에 흡수 | 우선순위 해소 결과의 관측 투영 — 독립 속성 없음 |
| R-CP-04 (`on_config_reload`) | PROP-U5-04(재조회 후 우선순위 재적용)에 흡수 | 재해석은 우선순위 재실행 — 독립 속성 없음(예제 테스트로 회전 시나리오 보강) |
| R-CG-PROJECT | `domain-entities.md` PROP-DE-U5-02가 소유 | projection total성은 타입 계층에서 검증 |

> **제너레이터(PBT-07) 총괄**: 전 상태코드 열거, `HttpError`/`TransportErrorClass` 전 변이, 토큰 소스 조합, `OkcRequest`/`ConsentRecord`/연산 시퀀스 도메인 제너레이터를 요구한다. 구체 구현·shrinking·시드·CI(PBT-08)는 Code Generation/Build-and-Test 이월.

---

## 5. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 문서 적용 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **준수** | §4 Testable Properties 제공 — 분류 total-function 오라클(PROP-U5-01/02), TLS/토큰/타임아웃 불변식(PROP-U5-03), 토큰 우선순위 오라클(PROP-U5-04), 동의 상태머신 Induction + 게이트 불변식 + forward-only(PROP-U5-05/06/07), 지속 round-trip(PROP-U5-08), 고지 내용(PROP-U5-09). 제너레이터(PBT-07) 요구 기재 |
| **Resiliency Baseline** | ON | **부분 적용** | R-AT-TO(모든 호출 데드라인, RESILIENCY-10 전송 절반), R-AT-CLASS(오프라인/타임아웃 -> Network/Timeout으로 U4 graceful degrade 공급), R-CG-PERSIST(원자적 지속 + fail-safe 강등). RPO/RTO 수치·배포/HA/DR은 U5(순수 규칙)에 **N/A** |
| **Security Baseline** | OFF | **N/A** | 미로딩·미강제. config 평문 `token`(RISK-01)은 문서화된 수용 위험. 잔존 통제 = R-AT-TLS(NFR-06) + `TokenSecret` 로그 리댁션(R-AT-TOKEN). secure-store는 선택적 강화(DEFER) |

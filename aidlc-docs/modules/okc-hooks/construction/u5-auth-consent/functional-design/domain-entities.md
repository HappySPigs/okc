# U5 Auth & Consent — Domain Entities (도메인 엔티티 / 값 타입)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U5 Auth & Consent** -> Functional Design -> 산출물 1/3 (`domain-entities.md`)
**작성일**: 2026-09-08
**크레이트**: `auth-consent` (lib) · **소속 컴포넌트**: `AuthTransport`, `CredentialProvider`, `ConsentGate`
**전제(AUTOPILOT 확정, 계획 §3)**: DEC-U5-01(HttpTransport seam) · DEC-U5-03(상태코드 매핑) · DEC-U5-04(request_timeout_s) · DEC-U5-07(secure-store seam+DEFER) · DEC-U5-09(ConsentLifecycle 명명) · DEC-U5-11(ConsentRecord 단일 지속) · Q4=A · Q7=A · FQ-1=A · NFR-04/06 · RISK-01

> **문서 성격**: 이 문서는 U5가 **도입/특화하는 값 타입·엔티티**를 정의한다. U0(`foundation`)가 소유하는 타입(`TokenSecret`/`TransportError`/`TransportErrorClass`/`ErrorClass`/`Timestamp`/`ConsentState`/`ActiveCondition`/`ConfigSnapshot` 등)은 **이름으로 참조만** 하며 **재정의하지 않는다**. 규칙(검증·매핑·우선순위)은 자매 산출물 `business-rules.md`, 알고리즘/흐름은 `business-logic-model.md`가 소유한다.
>
> **표기 규약**: 비즈니스 의미 중심 **기술중립 설계**. Rust스러운 시그니처는 **참고용(reference-only)** 이며 인프라·스레딩·async·I/O 메커니즘이 아니라 개념 형상을 표현한다. 다이어그램은 ASCII 화살표(`A -> B`)와 표/목록으로 기술한다. English 식별자명은 원문 유지.

---

## 0. U0에서 소비하는 타입 (재정의 금지 — 참조만)

| U0 타입 | U5에서의 사용 |
|---|---|
| `TokenSecret` | `CredentialProvider.resolve_token()`의 반환 토큰. redacting newtype(`Debug`/`Display` = `***`), 실제 값은 `.expose()`로만. `AuthTransport`가 요청 헤더에 첨부 |
| `TransportError` { `class`, `http_status`, `detail` } | `AuthTransport.send()`의 오류 반환. `class: TransportErrorClass` |
| `TransportErrorClass` { `AuthFailed`·`ServerError`·`Backpressure`·`Timeout`·`Network` } | 응답/실패 분류의 대상 값(닫힌 5변이) |
| `ErrorClass` / `.is_retryable()` | `TransportErrorClass.to_error_class()`로 매핑(U0 소유) — U4 재시도 게이트가 소비. U5는 산출만 |
| `Timestamp` | `ConsentGrant.granted_at` 등 이벤트 시각 |
| `ConsentState` { `Granted`·`Blocked`·`Unknown` } | **U0 placeholder** — `StatusSnapshot.consent` 필드 타입. U5 내부 `ConsentLifecycle`를 이 3변이로 projection(§3.7) |
| `ActiveCondition` { `AuthFailed`·`ConsentBlocked`·... } | `AuthTransport`/`ConsentGate`가 `StatusSink.raise_condition`/`clear_condition`로 push |
| `StatusSink` / `CriticalEventSink` / `Logger` / `ConfigReloadObserver` | 주입받는 관측 싱크 계약(push-only). U5는 구현이 아니라 **소비자** |
| `ConfigSnapshot` / `WatcherConfig` (`token`·`secure_store_enabled`·`server_endpoint`) | `CredentialProvider`/`AuthTransport`가 `ConfigProvider.current()`로 조회 |
| `validate_https_url` | `AuthTransport` TLS 가드가 재사용(DEC-U5-02) |
| `encode` / `decode` (CBOR, NFR-13) | `ConsentGate`가 `ConsentRecord` 무손실 지속에 사용 |

> **명명 충돌 정정(U0 패턴 계승)**: U0는 `ConsentState`(3변이 placeholder)를 `StatusSnapshot`용으로 이미 소유한다. U5의 4-상태 라이프사이클은 별개 타입 `ConsentLifecycle`로 명명해 충돌을 회피한다(U0가 `ErrorClass` 충돌을 `TransportErrorClass`로 정정한 것과 동일 패턴). component-methods §ConsentGate의 `ConsentState{NotAcknowledged/AcknowledgedNotGranted/Granted/Withdrawn}` 의미는 `ConsentLifecycle`로 보존된다.

---

## 1. AuthTransport 값 타입

### 1.1 OkcRequest / OkcResponse (요청/응답 도메인 형상)

- **목적**: U3 `UploadProtocolDriver`가 조립한 **논리 요청**과 서버의 **응답**. `AuthTransport`는 이 요청에 토큰을 첨부하고 TLS로 전송한 뒤 2xx 응답만 상위에 반환한다(DEC-U5-05).
- **필드 스키마**

  | 타입 | 필드 | 개념 타입 | 의미 |
  |---|---|---|---|
  | `OkcRequest` | `method` | `HttpMethod` | HTTP 메서드 |
  | | `path` | 문자열(엔드포인트 상대 경로) | `server_endpoint` base에 이어붙일 경로 |
  | | `headers` | `Headers` | 요청 헤더(토큰은 여기 첨부되지 **않고** 전송 시 주입) |
  | | `body` | `Body` | 요청 본문 바이트 |
  | `OkcResponse` | `status` | 정수(`u16`) | HTTP 상태코드(반환 시 항상 2xx) |
  | | `headers` | `Headers` | 응답 헤더 |
  | | `body` | `Body` | 응답 본문 바이트(U3가 프로토콜 의미로 해석) |

  참고 형상:
  ```
  enum   HttpMethod  { Get, Post, Put, Patch, Delete }
  struct Headers(Vec<(String, String)>)
  struct Body(Vec<u8>)
  struct OkcRequest  { method: HttpMethod, path: String, headers: Headers, body: Body }
  struct OkcResponse { status: u16, headers: Headers, body: Body }
  ```

- **불변식**: `AuthTransport.send()`가 `Ok(OkcResponse)`를 반환하면 `status`는 **2xx**다(비-2xx는 `TransportError`로 변환). `body`는 raw 바이트이며 U5는 그 스키마를 해석하지 않는다(프로토콜 의미 = U3).
- **경계 주석**: `path`/`body`의 구체 프로토콜 페이로드(have/want·commit 등)는 U3 소유. U5는 전송 봉투(envelope)만 안다.

### 1.2 HttpTransport seam (DEC-U5-01 — HTTPS 클라이언트 추상화)

- **목적**: `AuthTransport`가 **구체 async HTTP 스택에 FD 단계에서 결합하지 않도록** 하는 최소 전송 seam. U3는 이 트레이트의 목(mock) 구현으로 프로토콜을 테스트한다. 구체 클라이언트(rustls/HTTP 라이브러리 등)는 NFR/code-gen 이월.
- **형상**

  | 타입/트레이트 | 멤버 | 의미 |
  |---|---|---|
  | `RawHttpRequest` | `url`, `method`, `headers`, `body`, `timeout` | 절대 URL로 확정된 저수준 요청(토큰 헤더 포함, 타임아웃 부착됨) |
  | `RawHttpResponse` | `status`, `headers`, `body` | 저수준 응답(상태코드 무관) |
  | `HttpTransport` (trait) | `execute(&self, req: RawHttpRequest) -> Result<RawHttpResponse, HttpError>` | TLS 전송 실행. 구현은 code-gen(또는 목) |
  | `HttpError` (enum) | `Timeout` · `Connect` · `Tls` · `Io(detail)` | 저수준 전송 실패(분류 전 원인). `AuthTransport`가 `TransportError`로 사상 |

  참고 형상:
  ```
  struct RawHttpRequest  { url: String, method: HttpMethod, headers: Headers, body: Body, timeout: Duration }
  struct RawHttpResponse { status: u16, headers: Headers, body: Body }
  enum   HttpError       { Timeout, Connect, Tls, Io(String) }
  trait  HttpTransport   { fn execute(&self, req: RawHttpRequest) -> Result<RawHttpResponse, HttpError>; }  // Send + Sync
  ```

- **불변식**: `HttpTransport` 구현은 **TLS 위에서만** 전송한다(비-https URL은 `AuthTransport`가 seam 호출 전 거부하므로 seam에 도달하지 않음, R-AT-TLS). `timeout`은 항상 부착되어 전달된다(R-AT-TO). `Send + Sync`(U0 싱크와 동일 근거 — 데몬 멀티스레드 공유).
- **경계 주석**: seam은 **분류 이전**의 원시 결과만 반환한다. 상태코드->클래스 분류(R-AT-CLASS)는 `AuthTransport`가 소유한다.

---

## 2. CredentialProvider 값 타입

### 2.1 TokenSource / TokenStatus / CredentialError

- **목적**: 토큰의 **출처**와 **가용성 표면**을 표현한다. CLI `status`/헬스가 토큰 존재/소스를 노출한다(US-E4-01 관측 가능성).
- **형상**

  | 타입 | 변이/필드 | 의미 |
  |---|---|---|
  | `TokenSource` (enum) | `Config` · `Env` · `SecureStore` | 해소된 토큰의 실제 출처(우선순위 결과) |
  | `TokenStatus` (struct) | `present: bool`, `source: Option<TokenSource>` | 존재 여부 + (존재 시) 소스. CLI `status` 표면 |
  | `CredentialError` (enum) | `Missing` · `Empty` · `SecureStoreUnavailable` | 부재/공백/보안저장소 세션 불가(-> config/env 폴백 신호) |

  참고 형상:
  ```
  enum   TokenSource     { Config, Env, SecureStore }
  struct TokenStatus     { present: bool, source: Option<TokenSource> }
  enum   CredentialError { Missing, Empty, SecureStoreUnavailable }
  ```

- **불변식**: `resolve_token()`가 `Ok(TokenSecret)`이면 `token_status().present == true`이고 `source`는 실제 승리 소스. `Missing`/`Empty`는 실패가 아니라 **인증 시점 표면화 대상**(U0 R-TOKEN-02) — 데몬 정지 없음. `SecureStoreUnavailable`는 내부 폴백 신호이며 최종 사용자 오류로 승격되지 않는다(config/env로 진행).
- **경계 주석**: `TokenSecret`(반환 토큰) 값 타입은 U0 소유. 우선순위 알고리즘은 `business-rules.md` R-CP-01.

### 2.2 SecureStore seam (DEC-U5-07 — secure-store DEFER)

- **목적**: OS 보안 저장소(US-E4-02) 조회를 감싸는 seam. **MVP는 실 keyring 백엔드를 구현하지 않고**, "항상 불가" 기본 구현으로 컴파일만 성립시키며 config/env로 안전 폴백한다.
- **형상**

  | 타입/트레이트 | 멤버 | 의미 |
  |---|---|---|
  | `SecureStore` (trait) | `read_token(&self) -> Result<Option<TokenSecret>, SecureStoreError>` | 보안 저장소에서 토큰 조회. `Ok(None)` = 세션은 가용하나 미저장 |
  | `SecureStoreError` (enum) | `Unavailable` · `Backend(detail)` | 세션 불가(헤드리스/데몬) / 백엔드 오류 -> 둘 다 config/env 폴백 |
  | `UnavailableSecureStore` (기본 impl) | 항상 `Err(Unavailable)` | MVP 기본 주입체. 실 keyring 백엔드는 NFR/code-gen 이월 |

  참고 형상:
  ```
  enum  SecureStoreError { Unavailable, Backend(String) }
  trait SecureStore      { fn read_token(&self) -> Result<Option<TokenSecret>, SecureStoreError>; }  // Send + Sync
  struct UnavailableSecureStore;  // impl SecureStore -> 항상 Err(Unavailable)
  ```

- **불변식**: secure-store 조회 실패/`Unavailable`은 **비치명**이다 — 반드시 config `token` -> env 순으로 폴백한다(US-E4-02 "실행 중단되지 않음"). `secure_store_enabled == false`이면 seam을 아예 호출하지 않는다(R-CP-01).

---

## 3. ConsentGate 값 타입

### 3.1 ConsentLifecycle (동의 라이프사이클 — U0 `ConsentState`와 별개)

- **목적**: 최초 고지 확인 -> 상시 동의 부여 -> 철회의 **4-상태 라이프사이클**. component-methods §ConsentGate의 상태 의미를 보존하되 U0 `ConsentState` 충돌을 피해 재명명한다(DEC-U5-09).
- **변이**

  | 변이 | 의미 | 업로드 게이트 |
  |---|---|---|
  | `NotAcknowledged` | RISK-01 고지 미확인(최초 실행) | 차단(`NeedsAcknowledgment`) |
  | `AcknowledgedNotGranted` | 고지 확인됨, 상시 동의 미부여 | 차단(`NotGranted`) |
  | `Granted` | 상시 동의 부여됨 | **허용** |
  | `Withdrawn` | 동의 철회됨(forward-only) | 차단(`Withdrawn`) |

  참고 형상: `enum ConsentLifecycle { NotAcknowledged, AcknowledgedNotGranted, Granted, Withdrawn }`

- **불변식**: 업로드 허용은 **오직 `Granted`** 에서만 성립(R-CG-GATE). 상태 전이 합법성은 `business-rules.md` R-CG-TRANSITION. 지속 표현은 무손실 round-trip 대상(NFR-13).

### 3.2 GrantId / ConsentGrant (동의 부여 참조)

- **목적**: FR-14 "동의 부여 참조" — 부여 식별자 + 부여 시각. 서버가 없어도 로컬 캡처로 상시 동의를 표현한다(DEP-02 `[blocked-on-server]`).
- **형상**

  | 타입 | 필드 | 개념 타입 | 의미 |
  |---|---|---|---|
  | `GrantId` | (내부값) | 불투명 문자열/식별자 | 부여 유일 식별(구체 생성=UUID vs 난수는 code-gen 이월, DEC-U5-10) |
  | `ConsentGrant` | `grant_id` | `GrantId` | 부여 참조 |
  | | `granted_at` | `Timestamp`(U0) | 부여 시각(UTC) |

  참고 형상:
  ```
  struct GrantId(String)
  struct ConsentGrant { grant_id: GrantId, granted_at: Timestamp }
  ```

- **불변식**: `ConsentGrant`는 `Granted` 상태에서만 유효하게 존재. round-trip 무손실(`decode(encode(g)) == g`, R-CG-PERSIST).

### 3.3 ConsentStatus / ConsentDecision / BlockReason (조회·게이트 판정)

- **목적**: `view()`(US-E4-06 조회)와 업로드 게이트(`is_upload_permitted()`)의 반환 형상.
- **형상**

  | 타입 | 변이/필드 | 의미 |
  |---|---|---|
  | `ConsentStatus` | `acknowledged: bool`, `grant: Option<ConsentGrant>`, `state: ConsentLifecycle` | CLI `consent view` 출력(부여 참조·시각·현재 상태) |
  | `ConsentDecision` | `Permitted` \| `Blocked(BlockReason)` | 코디네이터 업로드 게이트 판정 |
  | `BlockReason` | `NeedsAcknowledgment` · `NotGranted` · `Withdrawn` | 차단 사유(상태별) |

  참고 형상:
  ```
  struct ConsentStatus  { acknowledged: bool, grant: Option<ConsentGrant>, state: ConsentLifecycle }
  enum   ConsentDecision { Permitted, Blocked(BlockReason) }
  enum   BlockReason     { NeedsAcknowledgment, NotGranted, Withdrawn }
  ```

- **불변식**: `state == Granted <=> ConsentDecision == Permitted`. `state != Granted`이면 사유가 상태와 1:1 대응(`NotAcknowledged->NeedsAcknowledgment`, `AcknowledgedNotGranted->NotGranted`, `Withdrawn->Withdrawn`).

### 3.4 ConsentError (연산 오류)

- **형상**: `enum ConsentError { NotAcknowledged, AlreadyGranted, NoGrant, PersistFailed }`

  | 변이 | 발생 조건 |
  |---|---|
  | `NotAcknowledged` | `grant()`인데 상태가 `NotAcknowledged`(고지 미확인, DEC-U5-16) |
  | `AlreadyGranted` | `grant()`인데 이미 `Granted` |
  | `NoGrant` | `withdraw()`/`view()`인데 부여가 없음(선택적 — view는 상태 반환으로 대체 가능) |
  | `PersistFailed` | `ConsentRecord` 지속 쓰기 실패(원자적 쓰기 실패) |

### 3.5 ConsentRecord (로컬 지속 상태 — DEC-U5-11)

- **목적**: 상시 동의가 **재시작 후에도 지속**되도록 하는 단일 로컬 레코드. U0 CBOR 코덱으로 무손실 지속하며 temp+rename 원자적 쓰기(부분쓰기 무손상). 최신 상태만 보존(부여 이력 누적 없음).
- **필드 스키마**

  | 필드 | 개념 타입 | 의미 |
  |---|---|---|
  | `acknowledged` | `bool` | RISK-01 고지 확인 여부 |
  | `state` | `ConsentLifecycle` | 현재 라이프사이클 상태 |
  | `grant` | `Option<ConsentGrant>` | 부여 참조(있으면) |

  참고 형상: `struct ConsentRecord { acknowledged: bool, state: ConsentLifecycle, grant: Option<ConsentGrant> }`

- **불변식**:
  - 일관성: `state == Granted`이면 `grant.is_some()`; `state == NotAcknowledged`이면 `acknowledged == false`이고 `grant.is_none()`.
  - `Withdrawn`이어도 `acknowledged == true`는 유지된다(고지 확인은 철회로 취소되지 않음 — forward-only 재부여 시 재확인 불요).
  - round-trip 무손실(NFR-13): `decode(encode(record)) == record`(R-CG-PERSIST).
- **경계 주석**: 저장 파일 경로는 U0 `ConfigProvider`로 해소되는 데이터 디렉토리 하위(정리 대상 = U7a `Uninstaller`, RISK-01 로컬 평문 산출물). 파일 I/O는 U5가 수행하고 바이트 변환만 U0 코덱.

### 3.6 disclosure text (RISK-01 고지 — DEC-U5-14)

- **목적**: `disclosure_text() -> &'static str`가 반환하는 고지 문안. **4대 필수 내용**을 반드시 포함한다(US-E4-04 고지 불변식):
  1. **연속성**: 상시 동의 + 자동 동기화 하에 이후 모든 볼트 변경(나중에 추가된 비밀 포함)이 추가 확인 없이 자동 업로드됨(RISK-01).
  2. **비가역성 + 실효적 철회 부재**: 업로드된 콘텐츠는 회수 불가; 서버 전까지 철회는 전진 방향(forward-only)에 그침(RISK-01, DEP-02).
  3. **클라이언트 측 민감 콘텐츠 필터/사전검사 없음**: raw 볼트 전체(비밀·개인정보 포함)가 전송됨(FR-15, NFR-07).
  4. **로컬 평문 산출물**: 히스토리·매니페스트·SyncState·로그 **및 config 평문 토큰**이 OS 보안 저장소 밖에 존재함(RISK-01).
- **불변식**: 최종 법적/UX 문안은 이월이나, 위 4개 내용 요소의 **부재는 US-E4-04 위반**이다(테스트 대상, PROP-U5-09).

### 3.7 ConsentLifecycle -> U0 `ConsentState` projection

`StatusSnapshot.consent`(U0 소유, 3변이)에 노출하기 위한 결정적 사상(R-CG-PROJECT):

| `ConsentLifecycle` | -> U0 `ConsentState` |
|---|---|
| `Granted` | `Granted` |
| `Withdrawn` | `Blocked` |
| `NotAcknowledged` | `Unknown` |
| `AcknowledgedNotGranted` | `Unknown` |

- **근거**: U0 placeholder는 status 표면 요약용이라 3변이면 충분. 상세 4-상태는 CLI `consent view`(`ConsentStatus`)가 노출한다.

---

## 4. U5 config 하위 스키마 (federated known-keys)

U0 `WatcherConfig`는 6개 core 필드만 담고, U5가 소비하는 섹션은 federated known-key union의 일부다(U0 `domain-entities.md` §2 예고, DEC-FEDERATED-KEYS). U5가 소유·확정하는 키:

| 필드 | 타입 | 필수? | 기본값 | 의미 | 검증 규칙 |
|---|---|---|---|---|---|
| `request_timeout_s` | 정수 | 아니오 | `30` | 요청당 전체 데드라인(초), NFR-04 | 양의 정수(`>= 1`). 0/음수/비정수 = 검증 실패 |

참고 형상:
```
pub const AUTH_CONSENT_CONFIG_KEYS: &[&str] = &["request_timeout_s"];
```

- **federated union 근거**: `watcher-bin`(U8)이 `FOUNDATION_CONFIG_KEYS`(U0) + `AUTH_CONSENT_CONFIG_KEYS`(U5) + 타 단위 키를 union으로 집계해 `ConfigProvider::new(known_keys)`에 주입한다. 이 상수가 union에서 누락되면 U0 R-CFG-STRICT-01(strict unknown-key reject)이 `request_timeout_s`를 미지 키로 거부하므로 필수다.
- **등록은 미지-키 수용 전용(값 읽기 아님)**: `AUTH_CONSENT_CONFIG_KEYS`에 `request_timeout_s`를 등록하는 것은 **오직 unknown-key 거부를 막기 위함**이며, 그 자체로 값을 읽을 수 있게 하지 않는다. U0 타입드 `ConfigSnapshot`은 6개 core 필드만 노출하고 federated 값 접근자를 두지 않으므로, `AuthTransport`는 `ConfigSnapshot`/`ConfigProvider.current()`로 `request_timeout_s`를 읽지 않는다. 대신 **U8 조립루트가 원본 config에서 이 값을 해소해 요청 데드라인(타입 값)으로 `AuthTransport`에 하향 주입**한다(NFR-04 보존).
- **교차관심(NFR-Design 이월)**: federated config 값(예: `request_timeout_s`) 전달 방식 — U8 조립루트가 원본 config 를 파싱해 주입 vs U0 `ConfigSnapshot` 에 federated 접근자 추가 — 는 NFR-Design 교차관심 항목으로 확정한다. U0 는 현재 동결 상태다.
- **secure-store 토글**: `secure_store_enabled`는 **U0 core 필드**이므로 U5가 재정의하지 않고 참조만 한다.
- **토큰 env 폴백**: `OKC_WATCHER_TOKEN`(DEC-U5-08)은 config 키가 아니라 **환경변수**이므로 known-keys union 대상이 아니다.

---

## 5. 엔티티 관계 개요 (화살표 표기 — ASCII 박스 미사용)

**합성(포함) 관계 — "A -> B"는 A가 B를 필드로 포함/참조함:**

- `OkcRequest` -> `HttpMethod`(1) + `path`(1) + `Headers`(1) + `Body`(1)
- `AuthTransport` -- (전송 시 조립) --> `RawHttpRequest`(url + 토큰 헤더 + timeout) -> `HttpTransport.execute`
- `RawHttpResponse` -- (분류) --> `OkcResponse`(2xx) | `TransportError`(비-2xx/실패, U0 타입)
- `TransportError` -> `TransportErrorClass`(U0, 1) + `http_status: Option<u16>` + `detail: String`
- `TokenStatus` -> `present: bool` + `source: Option<TokenSource>`
- `CredentialProvider` -- (해소) --> `TokenSecret`(U0) | `CredentialError`
- `ConsentStatus` -> `acknowledged: bool` + `grant: Option<ConsentGrant>` + `state: ConsentLifecycle`
- `ConsentGrant` -> `GrantId`(1) + `granted_at: Timestamp`(U0)
- `ConsentDecision::Blocked` -> `BlockReason`(1)
- `ConsentRecord` -> `acknowledged` + `state: ConsentLifecycle` + `grant: Option<ConsentGrant>`

**소유/주입 방향 (비순환):**

- `AuthTransport` -- depends-on --> `HttpTransport`(seam, 주입) + `CredentialProvider`(주입) + U0 `ConfigSnapshot`(`server_endpoint` core 필드) + 주입된 요청 데드라인(U8이 `request_timeout_s`에서 해소해 하향 주입)
- `CredentialProvider` -- depends-on --> `SecureStore`(seam, 주입) + U0 `ConfigSnapshot`(`token`/`secure_store_enabled`)
- `ConsentGate` -- depends-on --> U0 코덱(`encode`/`decode`) + U0 `StatusSink`(주입, `ConsentBlocked` push)
- U5 전체 -- depends-on --> `foundation`(U0)만 (crate `[dependencies]` = foundation)
- 상위 소비: U3 `UploadProtocolDriver` -- uses --> `AuthTransport.send`(2xx body 수신) ; U8 `SyncCycleCoordinator` -- uses --> `ConsentGate.is_upload_permitted`

**포맷 경로 구분:**

- 사람 -> JSON config(`token`/`secure_store_enabled`/`server_endpoint`, U0 core 필드) -> `ConfigProvider` -> U5 조회
- 사람 -> JSON config(`request_timeout_s`, federated known-key) -> U8 조립루트 파싱 -> 요청 데드라인(타입 값)으로 `AuthTransport`에 하향 주입 (U0 `ConfigSnapshot`은 core 6필드만 노출하므로 이 값의 소스가 아님)
- 내부 지속: `ConsentRecord` -> U0 `encode` -> CBOR 바이트 -> (U5 원자적 파일 쓰기) -> `decode` -> `ConsentRecord` (round-trip 무손실)

**텍스트 설명(다이어그램 대안)**: U5의 최상위 엔티티는 세 갈래다 — (1) 전송 봉투(`OkcRequest`/`OkcResponse`)와 그 저수준 seam(`HttpTransport`/`RawHttp*`), (2) 자격증명 표면(`TokenSource`/`TokenStatus`, `SecureStore` seam), (3) 동의 도메인(`ConsentLifecycle` 상태 + `ConsentGrant` 참조 + `ConsentRecord` 지속). 모든 오류/전송 결과 분류 타입과 토큰 값 타입은 U0 소유이며 U5는 참조·산출만 한다. U5는 `foundation`에만 의존해 DAG 비순환을 유지한다.

---

## 6. Testable Properties (PBT-01) — 엔티티/타입 계층

> **확장 강제(PBT-01, Full)**: 규칙 계층 속성(분류 매핑·우선순위·게이트·상태머신)은 `business-rules.md`, 흐름 속성은 `business-logic-model.md`가 소유한다(중복 회피). 여기서는 **타입/값 계층** 속성만 식별한다.

### 6.1 지속 타입 round-trip
- **PROP-DE-U5-01 — `ConsentRecord`/`ConsentGrant` round-trip** (카테고리: **Round-trip**; PBT-02; NFR-13)
  - **속성**: `decode(encode(v)) == v` for `v` in { `ConsentRecord`, `ConsentGrant` }(모든 `ConsentLifecycle` 변이·`grant` 부재/존재·유니코드 `GrantId` 포함).
  - **제너레이터(PBT-07)**: `ConsentLifecycle` 전 변이 + 일관성 불변식(§3.5) 만족 `ConsentRecord`, 경계 `GrantId`(빈/유니코드), `Timestamp`(0·경계·대값).
  - **비고**: 코덱은 U0 소유(실행은 U5 자기 제너레이터로 재확인, U0 PROP-BR-01 목록의 "하위 단위 지속 레코드"에 해당).

### 6.2 projection 결정성
- **PROP-DE-U5-02 — `ConsentLifecycle -> ConsentState` projection total** (카테고리: **Invariant** + **Easy verification**)
  - **속성**: §3.7 사상은 모든 `ConsentLifecycle` 변이를 정확히 하나의 U0 `ConsentState`로 사상한다(누락·중복 없음, 유한 도메인 전수 검증 가능).

### 6.3 속성 없음(No PBT properties identified) 판정
| 타입 | 판정 | 근거 |
|---|---|---|
| `OkcRequest`/`OkcResponse`/`Headers`/`Body`/`HttpMethod` | **No PBT properties identified**(타입 계층) | 전송 봉투 값 래퍼 — 분류/TLS/토큰 첨부 속성은 `business-rules.md`(동작 규칙) |
| `HttpTransport`/`SecureStore` (seam) | **No PBT properties identified** | 순수 계약(인터페이스) — 값이 아니라 주입 동작. 흐름 속성은 `business-logic-model.md` |
| `TokenSource`/`TokenStatus`/`CredentialError` | 우선순위 오라클(`business-rules.md` PROP-U5-04)에 흡수 | 유한 enum 어휘 — 독립 값 속성 없음 |
| `ConsentDecision`/`BlockReason` | 게이트 불변식(`business-rules.md` PROP-U5-06)에 흡수 | 상태->판정 사상은 규칙 계층 |

> **제너레이터(PBT-07) 총괄**: 위 속성은 `ConsentRecord`/`ConsentGrant`/`ConsentLifecycle` 도메인 제너레이터를 요구한다. 구체 구현·shrinking·시드·CI(PBT-08)는 Code Generation/Build-and-Test 이월.

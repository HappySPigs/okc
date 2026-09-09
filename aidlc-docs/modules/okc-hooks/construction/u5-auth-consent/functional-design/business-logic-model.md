# U5 Auth & Consent — Business Logic Model (핵심 로직 · 알고리즘 · 데이터 흐름)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U5 Auth & Consent** -> Functional Design -> 산출물 3/3 (`business-logic-model.md`)
**작성일**: 2026-09-08
**크레이트**: `auth-consent` (lib) · **소속 컴포넌트**: `AuthTransport`, `CredentialProvider`, `ConsentGate`
**전제(AUTOPILOT 확정, 계획 §3)**: DEC-U5-01(HttpTransport seam) · DEC-U5-06/07(우선순위+secure-store DEFER) · DEC-U5-13(게이트 범위) · Q4=A · Q7=A · NFR-04/06 · FR-13/14 · RISK-01

> **문서 성격**: `domain-entities.md`(타입)와 `business-rules.md`(규칙) 위에서 동작하는 **핵심 로직·알고리즘·데이터 흐름**을 기술한다. 타입/규칙은 재정의하지 않고 참조한다. 특히 **크로스유닛 경계**(무엇이 주입되고 무엇이 반환되며 무엇이 다른 단위 소유인지)를 명시한다.
>
> **표기 규약**: 기술중립 설계. Rust스러운 시그니처는 참고용. ASCII 화살표(`A -> B`)만. English 식별자명 유지.

---

## 1. AuthTransport — 전송 파이프라인 (유일 아웃바운드 HTTP 경로)

### 1.1 주입 구성 (조립 루트 U8이 하향 주입)

- `CredentialProvider`(토큰 소스) — 주입
- `HttpTransport`(seam, 구체 클라이언트 또는 목) — 주입 (DEC-U5-01)
- `ConfigSnapshot` 접근자(U0 `ConfigProvider`) — `server_endpoint`(U0 core 필드) 조회
- 요청 데드라인(예: `Duration`) — **U8 조립루트가 config `request_timeout_s`를 파싱해 하향 주입한 타입 값**. `AuthTransport`는 U0 `ConfigSnapshot`에서 `request_timeout_s`를 읽지 않는다 — U0 타입드 `ConfigSnapshot`은 6개 core 필드만 노출하며 federated 값의 소스가 아니다(이 값의 유일 경로 = U8 하향 주입)
- (선택) `StatusSink`/`CriticalEventSink`(U0 계약) — 인증 실패 조건 push용 (또는 코디네이터가 반환값 받아 push, §5 경계)

> **교차관심(NFR-Design 이월)**: federated config 값(예: `request_timeout_s`) 전달 방식 — U8 조립루트가 원본 config 를 파싱해 주입 vs U0 `ConfigSnapshot` 에 federated 접근자 추가 — 는 NFR-Design 교차관심 항목으로 확정한다. U0 는 현재 동결 상태다.

### 1.2 `send(req: OkcRequest) -> Result<OkcResponse, TransportError>` 흐름

순서 목록:

1. **토큰 해소**: `CredentialProvider.resolve_token()`
   - `Ok(TokenSecret)` -> 계속
   - `Err(Missing|Empty)` -> **요청 미발송**, 사전 실패 표면화(R-AT-TOKEN) -> `TransportError{ class: AuthFailed, http_status: None, detail: "credential missing/empty" }` 성격으로 상위에 반환(또는 별도 credential 오류; U3/코디네이터가 업로드 미시작 처리, US-E4-01 AC3)
2. **URL 확정**: `server_endpoint` base + `req.path` -> 절대 URL
3. **TLS 가드(R-AT-TLS)**: 절대 URL 스킴이 `https`가 아니면 seam 호출 전 거부(회귀 방어; 정상 경로는 config 검증으로 이미 https)
4. **저수준 요청 조립**: `RawHttpRequest{ url, method, headers + 토큰 헤더 주입, body, timeout = 주입된 요청 데드라인(config `request_timeout_s` 유래, U8 하향 주입) }`(R-AT-TOKEN/R-AT-TO)
5. **전송**: `HttpTransport.execute(raw)` (1회 시도, 재시도 없음 — DEC-U5-15)
   - `Ok(RawHttpResponse)` -> 6
   - `Err(HttpError)` -> 7
6. **응답 분류(R-AT-CLASS-01)**:
   - 2xx -> `Ok(OkcResponse{ status, headers, body })` (body는 U3가 해석)
   - 401/403 -> `Err(TransportError{ AuthFailed })` (+ 인증 실패 조건 push, §5)
   - 429/PROJECT_BUSY/queue-full -> `Err(TransportError{ Backpressure })`
   - 5xx / 기타 4xx / 3xx -> `Err(TransportError{ ServerError })`
7. **전송 실패 분류(R-AT-CLASS-01)**: `HttpError::Timeout -> Timeout` · `Connect/Tls/Io -> Network`
8. 결과 반환(성공 body 또는 `TransportError`) — **재시도 스케줄링·프로토콜 해석은 하지 않음**

**흐름(화살표 표기):**
```
OkcRequest
  -> resolve_token (CredentialProvider)        [Missing/Empty -> 미발송, 사전 실패]
  -> URL 확정(server_endpoint + path)
  -> TLS 가드(https 아니면 거부)
  -> RawHttpRequest 조립(토큰 헤더 + timeout)
  -> HttpTransport.execute (seam, 1회)
  -> 분류: 2xx -> OkcResponse(body up, U3)
           비-2xx/실패 -> TransportError(U0 taxonomy) up
```

### 1.3 경계 (무엇이 U5 밖인가)
- **재시도/백오프**: U4 `RetryBackoffController` — `TransportError`를 받아 `to_error_class()`로 판정, U5는 재시도하지 않는다.
- **프로토콜 의미**: U3 `UploadProtocolDriver` — 2xx `body`(have/want·commit 응답 등)를 해석한다. U5는 raw 바이트만 반환.
- **`ErrorClass` 매핑**: U0 `TransportErrorClass::to_error_class()` 소유.
- **오류 taxonomy 정의**: U0 소유(U5는 산출만).

---

## 2. CredentialProvider — 토큰 해소 로직

### 2.1 주입 구성
- `ConfigSnapshot` 접근자(U0) — `token`(`TokenSecret`)·`secure_store_enabled`
- `SecureStore`(seam, MVP=`UnavailableSecureStore`) — 주입 (DEC-U5-07)
- env 접근(환경변수 `OKC_WATCHER_TOKEN`)

### 2.2 `resolve_token() -> Result<TokenSecret, CredentialError>` 흐름 (R-CP-01)

```
if secure_store_enabled:
    match SecureStore.read_token():
        Ok(Some(t))            -> return Ok(t)              [source = SecureStore]
        Ok(None)               -> 폴백 계속
        Err(Unavailable|Backend) -> 폴백 계속(비치명, US-E4-02)
config.token:
    Some(t) if !t.expose().is_empty()  -> return Ok(t)      [source = Config]
    Some(t) if t.expose().is_empty()   -> return Err(Empty)
env OKC_WATCHER_TOKEN:
    Some(v) if !v.is_empty()  -> return Ok(TokenSecret::new(v))  [source = Env]
    Some(v) if v.is_empty()   -> return Err(Empty)
otherwise                     -> return Err(Missing)
```

- **MVP 실효(DEC-U5-07)**: `UnavailableSecureStore`가 항상 `Err(Unavailable)`이므로 실효 순서 = config -> env.
- **캐시/재해석**: 해소 결과를 캐시할 수 있으나, `on_config_reload()`(R-CP-04) 시 무효화해 다음 해소가 새 스냅샷을 반영한다(US-E4-03 토큰 회전).

### 2.3 `token_status()` / liveness
- `token_status()`는 위 해소를 부수효과 없이 재판정해 `TokenStatus{ present, source }` 반환(원문 미노출, R-CP-03). 기동 시 해소 가능하면 `StatusSink.set_liveness(CredentialReadable)` push 근거를 제공(실제 push 배선은 코디네이터/U8, §5).

### 2.4 경계
- **토큰 저장 위치·우선순위 정책**: U0 소유(§13/R-TOKEN-01). U5는 실행.
- **secure-store 실 백엔드**: NFR/code-gen 이월(MVP는 seam + Unavailable 기본).
- **config 로드·검증·리로드**: U0 `ConfigProvider` 소유. U5는 스냅샷 소비 + `on_config_reload` 관찰자.

---

## 3. ConsentGate — 동의 상태머신 + 게이트

### 3.1 주입 구성
- U0 코덱(`encode`/`decode`) — `ConsentRecord` 지속
- 지속 파일 경로(U0 `ConfigProvider`로 해소되는 데이터 디렉토리 하위) + U5 소유 원자적 파일 I/O
- (선택) `StatusSink`(U0) — `ConsentBlocked` 조건 push

### 3.2 기동 시 로드
```
open + decode(ConsentRecord)
  -> 성공: 인메모리 상태 = record.state (일관성 검증 R-CG-PERSIST)
  -> 파일 없음: 초기 상태 NotAcknowledged(acknowledged=false, grant=None)
  -> 손상/디코드 실패: fail-safe 강등(차단 우선) + 오류 로그
```

### 3.3 연산 흐름 (성공 시에만 원자적 지속 + 전이)

| 메서드 | 로직 | 지속 |
|---|---|---|
| `acknowledge()` | 상태가 `NotAcknowledged`면 `AcknowledgedNotGranted`로; 이미 확인이면 멱등 성공(R-CG-TRANSITION) | 성공 시 `ConsentRecord` 원자적 쓰기 |
| `grant()` | `AcknowledgedNotGranted`/`Withdrawn`에서만: 신규 `ConsentGrant{ GrantId, granted_at }` 생성 -> `Granted`. `NotAcknowledged`->`Err(NotAcknowledged)`, `Granted`->`Err(AlreadyGranted)` | 성공 시 쓰기; 쓰기 실패 -> `PersistFailed` + 인메모리 롤백 |
| `withdraw()` | `Granted`->`Withdrawn`(forward-only, R-CG-FORWARD-ONLY) | 성공 시 쓰기 |
| `view()` | 부수효과 없이 `ConsentStatus{ acknowledged, grant, state }` 반환 | 없음 |
| `is_upload_permitted()` | R-CG-GATE 판정(`Granted`<->`Permitted`) | 없음 |
| `ensure_acknowledged()` | `acknowledged==false` -> `Err(NotAcknowledged)`, else `Ok` | 없음 |
| `disclosure_text()` | 4대 필수 내용 정적 문자열 반환(R-CG-DISCLOSURE) | 없음 |
| `on_config_reload()` | MVP no-op(config-grant DEFER, DEC-U5-17) | 없음 |

**상태 전이(화살표 표기):**
```
NotAcknowledged --acknowledge--> AcknowledgedNotGranted --grant--> Granted --withdraw--> Withdrawn
                                                                     ^                        |
                                                                     +--------grant(재부여)---+
(acknowledge 멱등: 이미 확인 상태에서 재호출은 상태 불변)
```

### 3.4 게이트 소비 (U8 경계)
- U8 `SyncCycleCoordinator`가 사이클의 **업로드 직전** `is_upload_permitted()`를 호출한다:
  - `Permitted` -> 업로드/커밋 진행
  - `Blocked(reason)` -> 업로드/커밋 **스킵**(감시·재조정·status는 계속), `StatusSink.raise_condition(ConsentBlocked)` push
- 재부여 시 다음 트리거 사이클이 재스냅샷해 최신 상태를 업로드(FQ-2=A — 이벤트 큐 없음, R-CG-FORWARD-ONLY).

### 3.5 경계
- **서버측 동의 persist/scope/enforce·철회 반영·삭제**: `[blocked-on-server]`(DEP-02/DEP-06). U5는 로컬 캡처만.
- **CLI 표면**(`consent acknowledge/grant/view/withdraw`): U7b `OperatorCli`/`ControlPlane`가 파싱·디스패치 -> `ConsentGate` 메서드 호출. U5는 CLI를 소유하지 않는다.
- **config-플래그 grant/ack**: DEFER(DEC-U5-12).
- **코덱·`Timestamp`·`ConsentState` projection 타깃**: U0 소유.

---

## 4. 세 컴포넌트 합성 + 사이클 내 위치

U5는 자체적으로 사이클을 구동하지 않는다 — U8 `SyncCycleCoordinator`가 사이클(scan->build->diff->limits->negotiate->transfer->commit, U1/U3 소유)을 구동하며 U5는 두 지점에서 개입한다:

```
[U8 사이클]
  ... consent 게이트 (ConsentGate.is_upload_permitted)  <- 업로드 직전
        Blocked -> 업로드/커밋 스킵(감시/status 유지)
        Permitted -> 계속
  ... U3 UploadProtocolDriver.negotiate/transfer/commit
        각 서버 왕복 -> AuthTransport.send  <- 유일 HTTP 경로
              토큰 첨부(CredentialProvider) + TLS + timeout + 분류
              2xx body -> U3 해석 ; TransportError -> U4 재시도 판정
```

- **AuthTransport <- CredentialProvider**: 매 `send`가 토큰을 해소·첨부(FR-13).
- **AuthTransport -> U3/U4**: 2xx body는 U3로, `TransportError`는 U4(재시도)로.
- **ConsentGate -> U8**: 업로드 게이트 판정 제공. AuthTransport와 ConsentGate는 **직접 결합하지 않는다**(둘 다 코디네이터가 배선).

---

## 5. 관측 push 경계 (누가 raise_condition을 호출하는가)

U2 순수성 노트(§0 노트2)와 동일한 원리로, U5 컴포넌트는 **판정/결과를 반환**하고 실제 상태 push의 1차 소유자는 U8 `SyncCycleCoordinator`가 될 수 있다. 본 설계는 다음을 허용한다(둘 다 U0 `StatusSink` 계약만 의존하므로 비순환 유지):

- **AuthTransport**: 401/403 분류 시 `CriticalEventSink.report_auth_failure(detail)` + `StatusSink.raise_condition(AuthFailed)`를 직접 push하거나, `TransportError`를 코디네이터에 반환하고 코디네이터가 push. **MVP 채택**: `AuthTransport`는 `TransportError`를 반환만 하고, 인증 실패 조건 표면화는 코디네이터/U4 경로가 수행(얇은 전송 소유자 원칙, Q4=A) — 단 주입된 `CriticalEventSink`가 있으면 직접 report도 계약상 허용.
- **ConsentGate**: 게이트가 `Blocked`일 때 `raise_condition(ConsentBlocked)`, `Granted` 전이 시 `clear_condition(ConsentBlocked)`를 push. **MVP 채택**: `ConsentGate`가 주입된 `StatusSink`로 직접 push(동의 상태는 게이트가 권위 보유하므로 자연 소유). 코디네이터는 게이트 판정으로 업로드 스킵만 결정.

> 어느 경우든 U5는 U6(구현)를 역참조하지 않고 U0 트레이트만 의존한다(빌드 순서 역전 없음). 최종 push 소유자 확정(전송 실패 조건)은 U8 조립 시 배선으로 결정되며 계약은 양쪽 다 지원한다.

---

## 6. Testable Properties (PBT-01) — 로직/흐름 계층

> **확장 강제(PBT-01, Full)**: 각 속성에 카테고리 라벨 + 제너레이터(PBT-07) 요구를 기재한다. 타입 계층은 `domain-entities.md` §6, 규칙 계층은 `business-rules.md` §4가 소유(중복 회피). 여기서는 **흐름/합성** 관점 속성만 재확인·추가한다.

### 6.1 전송 파이프라인 불변식 (흐름)
- **PROP-BL-U5-01 — send 파이프라인 불변식** (카테고리: **Invariant**; §1.2)
  - **속성**: 목 `HttpTransport`로 관측 시, `execute`에 도달하는 모든 요청은 (a) https URL, (b) 토큰 헤더 1회, (c) `request_timeout_s` 데드라인을 갖는다. 토큰 `Missing`/`Empty`이면 `execute` 미호출. 반환은 2xx->`OkcResponse`, 그 외->`TransportError`(§business-rules PROP-U5-01/03 흐름 관점 재확인).
  - **제너레이터(PBT-07)**: `OkcRequest` + 토큰 상태 + 목 응답(상태코드/HttpError) 조합.

### 6.2 컴포넌트별 Testable-Properties 노트
| 컴포넌트 | 대표 속성 | 소유 문서 |
|---|---|---|
| `AuthTransport` | 분류 total+오라클(PROP-U5-01), TLS/토큰/타임아웃 불변식(PROP-U5-03), 파이프라인(PROP-BL-U5-01) | business-rules / 여기 |
| `CredentialProvider` | 우선순위 오라클(PROP-U5-04) | business-rules |
| `ConsentGate` | 상태머신 Induction(PROP-U5-05), 게이트 불변식(PROP-U5-06), forward-only(PROP-U5-07), 지속 round-trip(PROP-U5-08), 고지 내용(PROP-U5-09), projection(PROP-DE-U5-02) | business-rules / domain-entities |

### 6.3 속성 없음(No PBT properties identified) 판정
| 흐름 요소 | 판정 | 근거 |
|---|---|---|
| §4 사이클 내 합성 | **No PBT properties identified**(U5 계층) | 사이클 구동은 U8 소유 — U5는 게이트 판정/전송만 제공. 통합 동작은 Build-and-Test 통합 테스트 |
| §5 관측 push 배선 | **No PBT properties identified** | push는 best-effort 부수효과(값 변환 아님) — U6 구현 속성. 예제 기반 테스트가 적합 |
| §2.3 liveness 신호 | PROP-U5-04(해소 가능성)에 흡수 | `CredentialReadable`은 해소 성공의 투영 |

> **제너레이터(PBT-07) 총괄**: `OkcRequest`, 목 `HttpTransport` 응답(상태코드/HttpError), 토큰 소스 조합, `ConsentRecord`/연산 시퀀스 제너레이터. 구체 구현·shrinking·시드·CI(PBT-08)는 Code Generation/Build-and-Test 이월.

---

## 7. 확장 컴플라이언스 요약 (이 산출물 범위)

| 확장 | 활성 | 이 산출물 적용 | 판정 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제·준수** | §6에 흐름/합성 속성(PROP-BL-U5-01) + 컴포넌트별 속성 노트. 규칙/타입 계층 속성은 자매 산출물이 소유. 속성 없는 흐름은 §6.3 판정 |
| **Resiliency Baseline** | ON | **부분 적용** | §1.2 모든 요청 timeout 부착(NFR-04/RESILIENCY-10 전송 절반), 오프라인/타임아웃 -> U4 graceful degrade 공급, §3.2/§3.3 `ConsentRecord` 원자적 지속 + 손상 fail-safe 강등. RPO/RTO 수치·배포/HA/DR은 U5(순수 로직)에 **N/A**(인프라/상위 소관) |
| **Security Baseline** | OFF | **N/A** | 미로딩·미강제. config 평문 `token`(RISK-01)은 문서화된 수용 위험. 잔존 통제 = TLS 강제(NFR-06, §1.2 3단계) + `TokenSecret` 리댁션. secure-store는 선택적 강화(DEFER) |

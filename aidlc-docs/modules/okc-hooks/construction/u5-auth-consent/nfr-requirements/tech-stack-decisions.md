# U5 Auth & Consent — Tech Stack Decisions (기술 스택 결정)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U5 Auth & Consent** -> NFR Requirements -> 산출물 2/2 (`tech-stack-decisions.md`)
**작성일**: 2026-09-08
**크레이트**: `auth-consent` (lib) · **소속 컴포넌트**: `AuthTransport`, `CredentialProvider`, `ConsentGate`
**입력 아티팩트**: U5 FD 3종(`domain-entities.md`·`business-rules.md`·`business-logic-model.md`) · `plans/u5-auth-consent-functional-design-plan.md`(§3 DEC-U5-01..17) · 자매 산출물 `nfr-requirements.md`(U5-NFR-* 카탈로그) · `requirements.md`(NFR-04/05/06/07 · RISK-01 · §13) · **U0 `tech-stack-decisions.md`**(전파된 워크스페이스 결정) · 워크스페이스 `Cargo.toml`(핀된 공용 의존)
**규칙**: `construction/nfr-requirements.md` · `common/content-validation.md`(Unicode 박스 문자 미사용, 화살표 `A -> B`, Rust 타입 백틱)

> **문서 성격**: 이 문서는 U5의 **구체 크레이트/툴체인 선택을 기록하는 유일한 산출물**이다. 자매 산출물 `nfr-requirements.md`가 카테고리별 NFR(기술중립)을 정의하고, 이 문서는 그 NFR을 실현할 구체 기술 결정과 근거·전파 범위를 확정한다. U0가 워크스페이스 전역으로 확정한 결정(언어/edition/MSRV/`proptest`/`thiserror`/`TokenSecret`/CBOR 코덱)은 **재결정하지 않고 상속·소비**하며, 이 문서는 U5 고유 결정(전송 클라이언트·secure-store 백엔드·타임아웃 형상·GrantId 생성)만 확정한다.

---

## 0. 전파 원칙 (U5 = Wave-내 소비 단위, U0에만 의존)

U5 `auth-consent`는 `foundation`(U0)에만 의존하는 lib다(FD `domain-entities.md` §5 DAG 비순환). 따라서 언어·edition·MSRV·공용 크레이트는 **워크스페이스 상속**(`crate.workspace = true`)으로 U0 결정을 물려받고, 이 문서는 **U5에서 처음 도입하는 결정**만 확정한다.

- **상속(재결정 아님)**: Rust / Edition 2024 / 고정 MSRV 1.85(`[workspace.package]`) · `serde`(derive) · `thiserror`(운영오류 파생) · `ciborium`(U0 코덱 경유 CBOR) · `url`(base-URL join 재사용) · `proptest` + `proptest-support`(PBT). 정본은 U0 `tech-stack-decisions.md`.
- **U5 신규 결정(이 문서 소유)**: (1) 아웃바운드 HTTPS 전송 클라이언트 = `ureq`(blocking + rustls), (2) `SecureStore` 실 백엔드 미채택(keyring 없음), (3) 요청 타임아웃 = 단일 전체 데드라인(U8 주입), (4) `GrantId` 생성 방식 이월.
- **버전 정책**: 신규 외부 크레이트는 `[workspace.dependencies]`에 major/minor 시리즈만 핀하고 U5가 `crate.workspace = true`로 상속한다. **정확한 patch 핀 + MSRV(1.85) 대비 CI 검증은 Build-and-Test 이월**(U0와 동일 정책).
- **쓰기 범위 주석**: 이 단계는 문서만 작성한다. `Cargo.toml`(워크스페이스/크레이트) 실제 배선 — `ureq` 추가, `auth-consent` 크레이트 생성, feature 선언 — 은 **Code Generation 이월**이며 이 문서는 그 결정을 기록한다.

---

## 1. 언어 / 툴체인 (U0 상속, 재결정 아님)

| 항목 | 결정 | 근거 |
|---|---|---|
| 언어 | **Rust** | NFR-17 · U0 확정. `auth-consent`는 순수 Rust lib. |
| Edition | **2024** (`edition.workspace = true`) | U0 §1 상속. |
| MSRV | **고정 1.85** (`rust-version.workspace = true`) | U0 §1 상속. NFR-05 크로스플랫폼 재현 빌드. 신규 크레이트(`ureq`)는 이 MSRV 하한과 정합해야 함(§2). |

**전파 범위**: **U0 상속** — U5는 상속만, 신규 결정 없음.

---

## 2. 아웃바운드 HTTPS 전송 클라이언트 = `ureq` (blocking + rustls) — U5 신규 결정 (DEC-U5-01 실현)

`AuthTransport`는 `HttpTransport` seam(`execute(&self, RawHttpRequest) -> Result<RawHttpResponse, HttpError>`, `Send + Sync`) 뒤에서 동작하고(U5-NFR-MNT-01), 그 **구체 구현체**가 실제 TLS 전송을 수행한다. FD는 구체 스택을 이 NFR 단계로 이월했다(DEC-U5-01). 여기서 확정한다.

### 2.1 결정: `ureq` (blocking, rustls TLS, async 런타임 없음)

**결정**: 구체 `HttpTransport` 구현체는 **`ureq`**(blocking HTTP 클라이언트, rustls TLS 백엔드)로 만든다. `auth-consent` 크레이트의 normal 의존이다.

**후보 비교:**

| 후보 | TLS | async 런타임 | 의존 트리 | 판정 |
|---|---|---|---|---|
| **`ureq`** | rustls(순수 Rust) | **없음(blocking)** | 작음 — tokio/hyper 스택 불필요 | **채택** |
| `reqwest` | rustls 또는 native-tls | tokio(async) 필요 | 큼(hyper + tokio + 다수) | 기각(async 런타임 도입 = 얇은 전송 역할 과잉) |
| 직접 `hyper` + rustls | rustls | tokio | 중~큼 + 저수준 배선 부담 | 기각(재구현 부담) |
| `native-tls`/OpenSSL 경로 | OS/OpenSSL | — | 시스템 C 의존 | 기각(NFR-05 재현 빌드 마찰) |

**채택 근거**:
- (a) **얇은 전송 역할 정합(Q4=A)**: `AuthTransport`는 1회 시도·재시도 미소유(DEC-U5-15)의 동기 요청/응답이다. async 런타임(tokio)을 U5에 끌어들일 이유가 없다 — blocking `ureq`가 seam `execute`의 동기 시그니처(`fn execute(...) -> Result<...>`)에 그대로 맞는다.
- (b) **rustls = 순수 Rust TLS -> NFR-05 크로스플랫폼 재현 빌드**: native-tls/OpenSSL 같은 시스템 C 라이브러리 의존을 피해 macOS/Windows/Linux 3-OS 패키징 마찰을 줄인다(단일 배포 바이너리 목표, RESILIENCY-01).
- (c) **작은 의존 트리**: reqwest+tokio 대비 빌드/공급망 표면이 작다(DAG 소비 단위에 적합).
- (d) **TLS-only 강제 정합(U5-NFR-SEC-01)**: rustls는 평문 http fallback이 없고, https 스킴은 U0 `validate_https_url` + 전송 계층 가드로 이중 보증된다.

### 2.2 TLS 백엔드 = rustls (native-tls/OpenSSL 미채택)

- `ureq`의 **rustls** feature로 TLS를 제공하고 `native-tls`는 활성화하지 않는다(시스템 C 의존 회피).
- **rustls crypto provider 선택(ring vs aws-lc-rs)**: `aws-lc-rs`는 cmake/C 컴파일러를 요구해 크로스컴파일·재현 빌드 마찰이 있고, `ring`은 더 이식성 있다. **MVP는 `ureq` 기본 rustls 구성을 수용하되, 재현 빌드(NFR-05) 우선 시 `ring` provider를 선호**한다 — provider 확정 및 정확한 feature 조합(`default-features = false` + `rustls`)은 **Code Generation 이월**(빌드 배선 시점 결정). 이 문서는 "rustls, native-tls 아님"만 확정한다.

### 2.3 URL join = `url` 크레이트 재사용 (U0 파싱 결과 재사용)

`server_endpoint` base + `OkcRequest.path` -> 절대 URL 확정은 U0가 `validate_https_url`로 파싱한 `url::Url`을 재사용한다(U0 `tech-stack-decisions.md` §4: 파싱된 base URL을 U5가 재사용). `url`은 워크스페이스 공용 핀이므로 **신규 크레이트 아님**.

**MSRV/Edition 정합 노트(`ureq`)**: `ureq`(major 3 시리즈)는 MSRV가 1.85보다 낮아 워크스페이스 고정 MSRV 1.85와 정합하며, Edition 2024 소비 크레이트에서 정상 컴파일된다. rustls 백엔드도 동일 MSRV 하한 이하. 정확한 patch 핀 + MSRV CI 검증은 Build-and-Test 이월.

**전파 범위**: **U5 내부 한정** — `ureq`/rustls 의존은 `auth-consent`의 `HttpTransport` 구체 구현체에만 유입된다. U3는 목 구현으로 테스트하므로 `ureq`를 끌어들이지 않는다.

---

## 3. `SecureStore` 백엔드 = 미채택(keyring 없음) — MVP 트림 (DEC-U5-07 실현)

`CredentialProvider`의 OS 보안 저장소 조회는 `SecureStore` 트레이트 seam으로 감싸져 있다(U5-NFR-MNT-02).

**결정**: **MVP는 실 keyring 백엔드 크레이트를 도입하지 않는다.** `UnavailableSecureStore`(항상 `Err(Unavailable)`) 기본 구현만 두고 config `token` -> env로 안전 폴백한다. 실효 MVP 우선순위 = config -> env.

**미채택 근거(MVP 트림)**:
- §13 오버레이가 config 평문 `token`을 1차·기본 저장으로 확정했고, Watcher는 헤드리스/데몬으로 실행되어 데스크톱 세션 보안 저장소가 대개 불가하다(watcher-daemon-model).
- keyring 계열 크레이트(예: `keyring`)는 플랫폼별 네이티브/C 결합(Keychain/Credential Manager/Secret Service)을 가져와 NFR-05 크로스플랫폼 재현 빌드 부담과 데몬 세션 불가 처리 복잡도를 키운다.
- seam(`SecureStore` 트레이트)만 두면 실 백엔드는 API 변경 없이 Code Generation/후속에서 주입 교체 가능(US-E4-02 폴백 계약 보존).

**전파 범위**: **U5 내부 한정** — seam은 `auth-consent` 소유. 실 백엔드 채택 시에도 트레이트 경계 내 교체.

---

## 4. 동의 지속 코덱 = U0 CBOR 코덱 경유 (`ciborium`, 신규 크레이트 아님) — DEC-U5-11 실현

`ConsentGate`는 `ConsentRecord`를 U0 `encode`/`decode`(무손실 CBOR round-trip, R-CODEC-01/NFR-13)로 직렬화하고 **temp+rename 원자적 파일 쓰기**(U5 소유 I/O)로 지속한다(U5-NFR-REL-03).

**결정**:
- 바이트 변환 = **U0 `foundation::encode`/`decode`** 호출(U0가 `ciborium` 소유). `auth-consent`는 `ConsentRecord`/`ConsentGrant`에 `serde` derive만 붙이면 되고, **`ciborium`을 직접 의존할 필요가 없다**(U0 코덱 함수 경유). serde는 워크스페이스 공용 핀이므로 신규 아님.
- 파일 I/O(open/temp write/rename/read) = `std`(외부 크레이트 없음). 지속 경로는 U0 `ConfigProvider`가 해소하는 데이터 디렉토리 하위.
- **`serde_json` 미사용(동의 지속)**: 내부 지속 상태는 CBOR 경로다(Q1=B: 내부=CBOR / config=JSON). 동의 레코드는 사람이 편집하지 않으므로 U0 CBOR 코덱을 쓰고 `serde_json`을 도입하지 않는다.

**전파 범위**: **U5 내부 한정**(파일 I/O). 코덱은 U0 소유(`ciborium` 전파는 U0 §2.1이 U5를 소비 단위로 이미 기재).

---

## 5. 오류 처리 (U0 §5 관례 상속)

| 부류 | 타입 | 파생 전략 | 근거 |
|---|---|---|---|
| **운영 오류(반환용)** | `CredentialError` · `ConsentError` | **`thiserror`** 파생(`Display`/`Error`) | U0 §5 워크스페이스 전역 관례. `thiserror`는 workspace-inherited. |
| **분류/값 타입(소비)** | `TransportError`/`TransportErrorClass`/`ErrorClass` | **U0 소유 serde 값 타입 소비**(U5 재파생 없음) | U4 재시도 분류가 의존하는 구조화 변이 보존(anyhow식 타입소거 거부, U0 §5). |
| **U5 지속 값 타입** | `ConsentRecord`/`ConsentGrant`/`ConsentLifecycle` | 순수 `serde` derive(throw 아님, CBOR round-trip 대상) | U5-NFR-REL-03 / U5-NFR-MNT-03. |

**전파 범위**: **U0 상속** — `thiserror` 의존은 workspace-inherited, 분류 값 타입은 U0 것을 소비.

---

## 6. 속성 기반 테스트 = `proptest` + `proptest-support`(U0 재사용) — U0 §7 상속

**결정**: 워크스페이스 PBT 프레임워크 `proptest`(PBT-09, U0 §7 확정)를 `auth-consent`의 **dev-dependency**로 채택하고, U0 도메인 제너레이터가 필요한 지점(예: `ConsentGrant.granted_at: Timestamp`)은 U0 **`proptest-support` 비기본 feature**를 dev에서 켜 재사용한다(U5-NFR-MNT-04, PBT-07). U5 고유 제너레이터(`OkcRequest`·목 `HttpTransport` 응답 조합·`TokenSource` 4축 조합·`ConsentLifecycle`/`ConsentRecord`/연산 시퀀스)는 `auth-consent` 내부(테스트/제너레이터 모듈)에 둔다.

**U0 패턴 mirror(feature 게이팅)**: `proptest`는 `auth-consent`의 **dev-dependency**로만 재선언하고, U5가 하위에 노출할 제너레이터가 생기면 U0와 동일하게 **비기본 `proptest-support` feature**(`proptest`를 optional dep으로 게이트) 뒤에 둔다 — `proptest`가 프로덕션 빌드 그래프에 유입되지 않게 한다. (참고 형상, Code Generation에서 `Cargo.toml`에 배선):

```
[dependencies]
foundation = { path = "../foundation" }
serde = { workspace = true }        # ConsentRecord/ConsentGrant derive
thiserror = { workspace = true }    # CredentialError/ConsentError
url = { workspace = true }          # base-URL join (U0 파싱 결과 재사용)
ureq = { workspace = true }         # HttpTransport 구체 구현체(rustls TLS)
proptest = { workspace = true, optional = true }   # proptest-support 시에만 유입

[dev-dependencies]
proptest = { workspace = true }
foundation = { path = "../foundation", features = ["proptest-support"] }  # U0 제너레이터 재사용

[features]
default = []
proptest-support = ["dep:proptest"]   # U5 제너레이터 노출 시(U0 패턴)
```

**시드 재현성/이월(PBT-08)**: `proptest`의 회귀 파일(`proptest-regressions`)·시드 로깅으로 최소 실패 케이스 재현. 케이스 수·shrink 튜닝·CI 통합은 **Code Generation / Build-and-Test 이월**(U0 §7.3과 동일).

**전파 범위**: **U0 상속(dev)** — `proptest`는 워크스페이스 공용 dev-dependency, `proptest-support`는 U0 제너레이터 재사용에 사용.

---

## 7. Autopilot Decisions (주제 / 선택 / MVP-트림? / 근거)

| id | 주제 | 선택(chosen) | MVP 트림 | 근거 |
|---|---|---|---|---|
| TS-U5-01 | 아웃바운드 HTTPS 클라이언트 | `ureq`(blocking, rustls TLS, async 런타임 없음) | 예 | 얇은 전송 역할에 async 런타임(reqwest+tokio) 과잉. blocking이 seam 동기 시그니처에 정합(§2.1). |
| TS-U5-02 | TLS 백엔드 | rustls(순수 Rust), native-tls/OpenSSL 미채택 | 아니오 | NFR-05 크로스플랫폼 재현 빌드 — 시스템 C 의존 회피(§2.2). |
| TS-U5-03 | rustls crypto provider(ring vs aws-lc-rs) | `ureq` 기본 수용, 재현 빌드 우선 시 `ring` 선호; 확정은 Code Generation 이월 | 예 | aws-lc-rs는 cmake/C 요구 -> 크로스컴파일 마찰. MVP는 provider 미고정(§2.2). |
| TS-U5-04 | `SecureStore` 실 백엔드 | 미채택(keyring 없음) — seam + `UnavailableSecureStore`만 | 예 | config 평문 1차·기본(§13) + 헤드리스 데몬. keyring 네이티브 결합은 NFR-05 부담(§3). |
| TS-U5-05 | 동의 지속 코덱 | U0 `encode`/`decode`(CBOR/`ciborium`) 경유 + `std` 원자적 파일 I/O; `ciborium` 직접 의존 불필요 | 아니오 | NFR-13 무손실 round-trip(R-CG-PERSIST). 내부=CBOR(Q1=B); `serde_json` 미도입(§4). |
| TS-U5-06 | 요청 타임아웃 형상 | 단일 전체 데드라인(`Duration`, 기본 30초, `>=1`), U8 조립루트가 원본 config에서 해소해 하향 주입; connect/read 분리 이월 | 예 | NFR-04 보존; U0 `ConfigSnapshot`(core 6필드)에서 읽지 않음(federated, DEC-U5-04)(§8). |
| TS-U5-07 | `GrantId` 생성 방식 | 불투명 `GrantId` newtype; UUID vs 난수 생성은 Code Generation 이월(신규 크레이트 미결) | 예 | FD 기술중립(DEC-U5-10). MVP는 생성 크레이트 미고정 — code-gen에서 결정. |
| TS-U5-08 | 오류 파생 | 운영오류=`thiserror`, 분류/값=U0 serde 소비 | 아니오 | U0 §5 워크스페이스 관례 상속(§5). |
| TS-U5-09 | PBT 프레임워크/제너레이터 | `proptest`(dev) + U0 `proptest-support` 재사용 + U5 자기 제너레이터 | 아니오 | U0 §7 상속(PBT-09/PBT-07)(§6). |

---

## 8. 요청 타임아웃 형상 (NFR-04, DEC-U5-04) — federated-config 해소 확정

`request_timeout_s`(기본 30초, `>= 1` 검증)는 U0 core 6필드가 **아니다**. 따라서:

- **소스 경로(확정)**: **U8 조립루트(watcher-bin)가 원본 config 파일을 파싱해 타입 값(`Duration`)으로 `AuthTransport`에 생성자 하향 주입**한다(DEC-FEDERATED-KEYS Q6=A; wave-level 일관 적용). `AuthTransport`는 U0 `ConfigSnapshot`/`ConfigProvider.current()`로 `request_timeout_s`를 읽지 않는다.
- **U5의 소유분**: `AUTH_CONSENT_CONFIG_KEYS = &["request_timeout_s"]` 상수를 등록해 U0 R-CFG-STRICT-01(strict unknown-key reject)이 이 키를 미지 키로 거부하지 않게 한다(**미지-키 수용 전용, 값 읽기 아님**). 검증 규칙(양의 정수 `>= 1`, 0/음수/비정수 = 실패)도 U5 소유.
- **형상(MVP 트림)**: 단일 전체 데드라인(connect + 응답 포함). connect/read 분리 타임아웃은 이월(단일 데드라인이 MVP 충분).
- **live-reload**: federated 파라미터(`request_timeout_s` 포함) live-reload는 MVP 범위 밖 — 재시작으로 변경. U0 core-field(토큰 등) reload만 U0 관찰자 팬아웃으로 반영된다(`CredentialProvider.on_config_reload`, DEC-U5-17).

**전파 범위**: 값 해소·주입은 **U8**(조립루트), 검증 규칙/상수는 **U5** 소유. `AuthTransport`는 주입된 `Duration`만 소비.

---

## 9. 의존성 요약표

**범례**: version-policy = 외부 크레이트는 `[workspace.dependencies]` 핀 상속(inherit), 내부는 path 의존. kind = normal(런타임) / dev(테스트). new = U5에서 처음 도입.

| crate | version-policy | kind | new? | feature | grounding |
|---|---|---|---|---|---|
| `foundation`(U0) | path 의존, `publish=false` | normal | — | dev: `proptest-support` | U5는 U0에만 의존(FD §5) |
| `serde` | workspace-inherited(핀: `1`) | normal | — | `derive` | `ConsentRecord`/`ConsentGrant` derive · U0 §2 상속 |
| `thiserror` | workspace-inherited(핀: `2`) | normal | — | — | `CredentialError`/`ConsentError` · U0 §5 상속 |
| `url` | workspace-inherited(핀: `2`) | normal | — | — | base-URL join(U0 파싱 결과 재사용) · U0 §4 |
| **`ureq`** | **`[workspace.dependencies]` 신규 핀: `3`** | normal | **예** | rustls(native-tls 아님); provider 확정 code-gen 이월 | **TS-U5-01/02** · DEC-U5-01 · U5-NFR-MNT-01/SEC-01 |
| `proptest` | workspace-inherited(핀: `1`) | **dev** | — | `proptest-support`(비기본, U5 제너레이터 노출 시) | U0 §7 상속 · PBT-09/PBT-07 |
| `ciborium` | (U0 코덱 경유 — U5 직접 의존 없음) | — | — | — | 동의 지속은 U0 `encode`/`decode` 호출(§4) |

> **버전 핀 주석**: 위 major/minor 핀은 대표 계열이며, `ureq`의 정확한 patch 핀 + MSRV(1.85) 정합 검증은 **Build-and-Test 이월**(U0 정책 동일). `ureq` 추가·`auth-consent` 크레이트 `Cargo.toml` 생성·feature 선언은 **Code Generation 이월**(이 단계는 문서만 작성, 쓰기 범위 준수). `keyring`·async 런타임(tokio/reqwest)·`serde_json`(동의 지속용)은 **의도적 미채택**(§2, §3, §4).

---

## 10. 범위 절제 기록 (Scope Trims / Deferrals)

| 절제 항목 | 결정 | 근거 / 이월처 |
|---|---|---|
| **async HTTP 스택(reqwest/tokio)** | **미채택** | blocking `ureq`가 얇은 전송 역할에 충분(§2.1, Q4=A). async 런타임 도입은 과잉. |
| **native-tls / OpenSSL** | **미채택** | rustls 순수 Rust로 NFR-05 재현 빌드(§2.2). |
| **rustls crypto provider 고정** | **이월** | MVP는 `ureq` 기본 수용, `ring` 선호 명시. 확정은 Code Generation(§2.2, TS-U5-03). |
| **keyring / OS secure-store 실 백엔드** | **미채택(seam+Unavailable만)** | config 평문 1차·기본 + 헤드리스 데몬(§3, DEC-U5-07). 실 백엔드 code-gen 이월. |
| **connect/read 분리 타임아웃** | **이월(단일 데드라인 확정)** | 단일 전체 데드라인이 MVP 충분(§8, U5-NFR-REL-02). |
| **`GrantId` 생성 크레이트(UUID 등)** | **이월** | 불투명 newtype만 확정; 생성 방식 Code Generation(§7 TS-U5-07, DEC-U5-10). |
| **스트리밍 요청 body(대용량 blob)** | **이월(MVP 인메모리 버퍼)** | 청킹은 U3 소관; U5는 봉투만(§nfr-requirements §7 N/A). |
| **`serde_json`(동의 지속용)** | **미채택** | 내부 지속 = CBOR(Q1=B). config JSON은 U0/U8 소관(§4). |
| **PBT-08 상세(케이스 수/shrink/시드/CI)** | **이월** | Code Generation / Build-and-Test(§6, U0 §7.3 동일). |
| **MSRV CI 검증 + patch 핀** | **이월** | 정책만 확정(§0). 구체 CI 통합은 Build-and-Test(U0 동일). |

---

## 11. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 문서 적용 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제·준수 — PBT-09 상속 충족** | §6이 `proptest` dev-dependency 채택 + U0 `proptest-support` 제너레이터 재사용(PBT-07) + U5 자기 제너레이터 확정. 프레임워크(PBT-09)는 U0에서 워크스페이스 전역 확정되어 U5가 상속하므로 blocking 없음. PBT-08 상세는 Build-and-Test 이월(명시). |
| **Resiliency Baseline** | ON | **부분 적용** | §2 `ureq`+rustls로 NFR-04 타임아웃 전송 실현(RESILIENCY-10 전송 절반; 백오프 U4). §4 U0 CBOR 코덱 + `std` temp+rename 원자적 지속으로 동의 회복력(U5-NFR-REL-03). RTO/RPO 수치·DR·HA·배포/롤백은 U5(순수 lib)에 **N/A**(RESILIENCY-02). |
| **Security Baseline** | OFF | **N/A(잔존 표면화)** | 미로딩·미강제. RISK-01 수용(config 평문 `token`). 유일 잔존 통제 TLS(https)는 §2.2 rustls-only 전송으로 실현(U5-NFR-SEC-01; https-only는 U0 §4 재결정 아님). 저비용 잔존 위생 = `TokenSecret` redaction 준수(§5, U5-NFR-SEC-02). 암호화 저장·키관리·keyring 신설 없음(§3). |

**N/A NFR 카테고리(기술 선택 없음)**: 확장성 · 가용성 · 성능 수치 목표(전송은 요청당 단일 데드라인 유계 정성 계약만) · Resiliency DR/RTO/RPO · config/CLI 외 사용성 · 보안 강제 통제. 근거 상세는 자매 산출물 `nfr-requirements.md` §7.

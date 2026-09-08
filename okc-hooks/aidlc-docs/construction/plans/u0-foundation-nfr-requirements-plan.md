# U0 Foundation — NFR Requirements 계획 및 질문

**단계**: CONSTRUCTION → Per-Unit Loop → **U0 Foundation** → NFR Requirements (Part: 계획 + 질문 게이트)
**작성일**: 2026-09-08
**크레이트**: `foundation` (lib) · **소속 컴포넌트**: `CoreTypes`, `ConfigProvider`
**입력 아티팩트**: `domain-entities.md`·`business-rules.md`·`business-logic-model.md`(U0 Functional Design 산출물, 승인 완료 2026-09-08T08:10:00Z), `u0-foundation-functional-design-plan.md`(확정 답변 Q1–Q8/FQ), `requirements.md`(§5 NFR / §11 open items / §12·§13 부록)
**규칙**: `construction/nfr-requirements.md` Steps 1–9 · `common/question-format-guide.md` · `common/content-validation.md` · 활성 확장 `property-based-testing.md`(**PBT-09 이 단계 강제**) · `resiliency-baseline.md`
**생성 방식(ultracode)**: 질문 세트는 워크플로 `wf_b3f00271-c54`(5 NFR-차원 분석가 + dedup/완결성 크리틱, 6/6 에이전트 0 오류)로 저작 → 크리틱이 이미-확정 항목을 DROP, PBT-09를 강제, 절제된 순서 세트로 큐레이션. `pbt09_present=true` 확인.

---

## 1. FD 분석 (Step 1 — 요약)

U0는 순수 lib(값 타입 + CBOR 코덱 + config 로더/검증/리로드)이며 **DAG 루트**다. 자체 런타임·스레드·처리량 축이 없어 전형적 NFR(확장성/가용성/성능 수치)의 상당수가 **N/A**다. 반면 U0가 여기서 정하는 **기술 스택 선택(크레이트·edition·PBT 프레임워크)은 워크스페이스 전 크레이트(U1~U8)로 전파**되므로 tech-stack 결정이 이 단계의 핵심이다.

FD에서 명시적으로 **이 NFR 단계로 이월된 미결정**:
- 코덱이 쓸 CBOR 크레이트(FD는 `encode`/`decode` serde 시그니처만 확정, 크레이트 미정)
- strict 미지 키 **전수 나열 메커니즘**(R-CFG-STRICT-01의 "여부"는 Q4=B 확정, "how"는 미정)
- `server_endpoint` URL 검증 깊이/파서(https-only reject는 §4.1 확정, "무엇으로 파싱"은 미정)
- 오류 타입 파생 전략, Rust edition/MSRV
- `ConfigProvider` 스레드 안전 스냅샷/스왑 프리미티브(FD는 원자성 R-RELOAD-02만 규정, 메커니즘 이월)
- 관찰자 팬아웃 패닉/실패 격리, 순수 표면 패닉-프리 범위
- **PBT-09 프레임워크 선택(강제 의무, blocking)** + 제너레이터 재사용 노출
- 토큰 로그/Debug 유출 방지(잔존 위생)

---

## 2. NFR Requirements 실행 계획 (Steps 2·6 — 산출물 체크박스)

답변 확정(§3) 후, 아래를 `aidlc-docs/construction/u0-foundation/nfr-requirements/` 에 생성한다.

- [ ] **`nfr-requirements.md`** — 카테고리별 NFR 확정: 신뢰성(코덱 무손실·패닉-프리 total·config 원자 스왑·관찰자 격리), 성능(코덱 = 엔트리 수에 선형·유계라는 **정성 계약**, 수치 목표 미설정 근거 명시), 유지보수성(오류 타입 전략·PBT 제너레이터 재사용·문서/커버리지 정책), 보안-잔존(TLS 유일 잔존 통제 + 토큰 로그-유출 위생), + N/A 카테고리(확장성/가용성/성능-수치/DR·RTO·RPO/config 외 사용성)의 판정 근거표
- [ ] **`tech-stack-decisions.md`** — 확정된 크레이트/툴체인: CBOR 크레이트, JSON strict-key 메커니즘, URL 파서, 오류 파생(thiserror), edition/MSRV, config 스왑 프리미티브(arc_swap), **PBT 프레임워크(PBT-09)**, 제너레이터 노출(cargo feature), 토큰 redaction 타입 — 각 결정에 근거·전파 범위(U1~U8) 기재
- [ ] 산출물 작성 전 `content-validation.md` 검증(특수문자 이스케이프, 표/코드블록 파싱, ASCII 박스 미사용·화살표 표기)
- [ ] 확장 컴플라이언스 요약(§4) 최종 판정 — PBT-09 강제 충족 여부 blocking 확인

> 지금은 **계획 + 질문 게이트**이므로 위 산출물을 생성하지 않는다. 답변 확정 후 생성한다.

---

## 3. 질문 (Steps 3–4)

**안내**: 아래 `[Answer]:` 태그에 **권장안이 미리 채워져** 있습니다(이 프로젝트의 기존 방식). 그대로 두시면 권장안으로 진행하고, 원하시면 다른 문자로 바꾸거나 `X)` 에 직접 기술해 주세요. **"전부 권장안대로"** 라고만 하셔도 됩니다. 모두 확정되면 "done"/"답변완료"라고 알려주세요.
>
> ⚠️ **Q9(PBT 프레임워크 선택)는 Property-Based Testing 확장이 이 단계로 이월한 강제(blocking) 의무**입니다 — 반드시 확정되어야 다음 단계로 진행합니다.

### Question 1 — CBOR 코덱 크레이트 선택 (내부 지속 상태 직렬화) · [Tech Stack]
Q1=B로 내부 지속 상태(마지막 커밋 `Manifest`·`SyncState`·`ConsentGrant`·`UploadHistoryRecord`·재개 오프셋)는 CBOR로 직렬화하기로 확정됐고, 코덱 시그니처는 이미 serde 기반(`encode<T: Serialize>` / `decode<T: DeserializeOwned>`)입니다. R-CODEC-01(무손실 round-trip)이 핵심 계약인 `CoreTypes` 코덱이 사용할 CBOR 크레이트로 무엇을 채택할까요?

A) `ciborium` — serde 호환·순수 Rust·활발히 유지. 이미 확정된 `T: Serialize`/`DeserializeOwned` 시그니처에 그대로 맞고, 자기기술적 CBOR라 Q1=B 스키마 진화에 강함 (권장)

B) `minicbor` — 비-serde 파생 기반, 바이트 레이아웃 제어가 정밀하나 확정된 serde 시그니처를 폐기하고 타입마다 `#[cbor]` 주석을 달아야 함

C) `serde_cbor` — serde 호환이나 유지보수 중단(archived/deprecated) — DAG 루트 크레이트에 공급망 리스크

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 2 — strict 미지 키 전수 리포트 메커니즘 (JSON 파싱) · [Tech Stack]
Q4=B(알 수 없는 config 키 = strict reject)이고 R-CFG-STRICT-01은 "발견된 모든 알 수 없는 키를 나열(첫 키에서 중단하지 않음)"을 명시합니다. serde의 `#[serde(deny_unknown_fields)]`는 첫 미지 필드에서 중단하고, config는 하위 단위 소유 섹션(`debounce_ms`[U2]·`backoff`[U4] 등)이 있어 단일 모놀리식 struct와도 맞지 않습니다. strict 미지 키 판정을 어떻게 구현할까요?

A) `serde_json` 2-패스: 먼저 `serde_json::Value`로 파싱해 알려진 키 집합 밖 키를 전부 수집·나열한 뒤, 통과하면 타입드 `WatcherConfig`로 역직렬화 — 전수 나열 요구 충족 + federated 키 집합에 자연 대응 (권장)

B) `serde_json` + `#[serde(deny_unknown_fields)]` — 가장 단순하나 첫 미지 키에서 중단해 R-CFG-STRICT-01(전수 나열) 위반, flatten 섹션과도 비호환

C) 커스텀 `Deserialize` 구현으로 단일 패스에서 미지 키 수집 — 제어력 최대이나 손수 작성 코드가 많아 DAG 루트 유지보수 부담

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 3 — server_endpoint URL 검증 깊이/파서 · [Tech Stack]
`server_endpoint`는 "유효 URL 형식이며 스킴은 `https`만"이어야 하고 이 검증은 config 로드 시점에 수행됩니다(https-only reject는 이미 확정된 규칙이므로 재질문 아님). "유효 URL 형식"을 어느 깊이·어떤 파서로 검증할까요? U0가 고른 URL 파서는 워크스페이스 공용 의존으로 전파됩니다.

A) `url` 크레이트로 완전 파싱 후 스킴 `== https` 단언 — WHATWG 준수, 기형 authority/포트를 로드 시점에 정밀 거부. 파싱된 URL은 U5 `AuthTransport` base-URL join에 재사용 (권장)

B) 수기 프리픽스 검사(`starts_with("https://")` + 비어있지 않음) — 무의존이나 기형 호스트/포트를 통과시켜 실패를 U5 연결 시점으로 미룸(오류 국소성 저하)

C) 정규식 기반 URL 검증 — 무의존 중간안이나 오류에 취약하고 U5에 파싱된 base URL을 못 줘 하위 단위가 재파싱

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 4 — 오류 타입 파생 전략 (thiserror vs 수기 vs anyhow) · [Tech Stack]
U0는 워크스페이스 전역 오류 계약을 정의합니다 — `Result`로 반환되는 운영 오류(`CodecError`·`ConfigError`·`CredentialError`)와, CBOR로 직렬화되어 U4/U6가 소비하는 분류 값 타입(`ErrorClass`·`ClassifiedError`·`TransportError`). 이 오류 타입들의 `Display`/`std::error::Error` 구현을 어떻게 제공할까요?

A) 운영 오류 타입에 `thiserror` 파생, 분류 값 타입(`ErrorClass`/`ClassifiedError`/`TransportError`)은 순수 serde enum/struct로 유지 — 라이브러리 관용, 보일러플레이트 최소, 워크스페이스 전역 일관 (권장)

B) 모든 오류 타입에 `impl Display`/`impl Error`를 수기 작성 — 무의존이나 반복 보일러플레이트를 하위 단위가 모두 손수 맞춰야 함

C) 파운데이션 오류를 anyhow식 타입소거로 통일 — 편하나 R-CLASS-01 분류가 의존하는 구조화 변이(`ErrorClass`/`ClassifiedError`)를 소실 — 파운데이션에 부적합

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 5 — Rust edition + MSRV 정책 · [Tech Stack]
NFR-17이 Rust를 확정했고 U0는 Wave-1 최초 빌드 DAG 루트라 여기서 정한 edition/MSRV가 워크스페이스 전 크레이트(U1~U8)로 전파됩니다. 어떤 edition/MSRV 정책을 채택할까요?

A) Edition 2024 + 워크스페이스 `Cargo.toml`에 `rust-version`으로 MSRV 고정 + CI 검증 — 2024는 Rust 1.85부터 안정, greenfield라 레거시 부담 없음. 고정 MSRV가 NFR-05 크로스플랫폼 재현 빌드 보장 (권장)

B) Edition 2021 + MSRV 고정 — 가장 보수적·최대 툴체인 호환이나 back-compat 제약 없는 greenfield에서 2024 개선 포기

C) latest-stable · MSRV 미고정(툴체인 부동) — 관리 최소이나 비재현적: 하위 단위가 부지불식 하한을 올리고 3-OS 패키징이 드리프트

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 6 — ConfigProvider 스레드 안전 스냅샷/스왑 프리미티브 · [Reliability]
`current()`는 데몬의 여러 스레드(U2 `FilesystemWatcher`, U8 `ControlPlane` IPC, 직렬 동기화 사이클)가 동시에 읽고, 그 사이 CLI `reload`가 R-RELOAD-02의 원자적 all-or-nothing 스왑을 수행합니다. Functional Design은 스레딩 메커니즘을 기술중립으로 이월했습니다. `current()` 스냅샷의 동시 읽기와 원자 스왑을 어떤 동시성 프리미티브로 구현할까요?

A) `arc_swap::ArcSwap<Arc<WatcherConfig>>` — 락-프리 읽기(`current()`는 값싼 Arc 로드), 리로드는 원자적 포인터 스왑. 읽기가 블로킹되지 않고 포인터 교체가 곧 all-or-nothing이라 R-RELOAD-02 직접 충족 (권장)

B) `RwLock<Arc<WatcherConfig>>`(std/parking_lot) — 읽기는 read 락, 리로드는 write 락. 표준적이나 스왑 동안 읽기 스레드 블로킹 및 writer starvation 여지

C) `Mutex<Arc<WatcherConfig>>` — 단일 락으로 모든 접근 직렬화. 가장 단순하나 동시 읽기가 핫 읽기 경로에서 경합

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 7 — 리로드 관찰자 팬아웃의 패닉/실패 격리 · [Reliability]
성공 스왑 직후(R-OBSERVER-01) `ConfigProvider`는 등록된 관찰자(U5 `CredentialProvider`, U6 `StructuredLogger`)에 결정적으로 팬아웃합니다(R-OBSERVER-03). 스왑은 이미 원자적으로 커밋됐습니다. 팬아웃 중 한 관찰자가 패닉하거나 오류를 반환하면 어떻게 처리할까요?

A) 관찰자별 격리(`catch_unwind` + 로그) 후 나머지 관찰자에 계속 팬아웃 — 스왑은 이미 커밋됐으므로 패닉 관찰자가 `current()`나 다른 관찰자를 오염시키지 못함. 관측의 best-effort·infallible 성격과 정합 (권장)

B) fail-fast: 첫 관찰자 패닉/오류에서 나머지 중단하고 `reload()` 호출자(CLI)에 오류 표면화. 단 이미 스왑된 config는 유지(롤백 없음)

C) 격리 없음: 관찰자 콜백을 인라인 실행, 패닉이 `ConfigProvider`를 관통(락 오염 위험). 관찰자가 계약상 패닉-프리라 신뢰

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 8 — 순수 표면의 패닉-프리(total) 신뢰성 요구 범위 · [Reliability]
`decode`는 U4 크래시 복구 경로에서 손상/절단 가능성 있는 온디스크 CBOR 바이트를 입력받고, `is_retryable`는 이미 total 함수로 확정(R-CLASS-03), `RelativePath::normalize`는 이미 fallible로 규정(§1.1)됐습니다. U0 순수 표면(`encode`/`decode`·정규화·오류 분류)에 대해 "패닉 없이 항상 `Result`로 표면화"를 명시적 신뢰성 요구로 둘 범위는?

A) U0 순수 표면 전체를 패닉-프리 total로 명시: `encode`/`decode`는 절단·손상·적대적 CBOR 포함 모든 입력에 `Result`(unwrap/슬라이스 패닉 없음), `normalize`는 무효 경로에 `Result`. U0 신뢰성 불변식 + PBT no-panic 속성으로 검증 (권장)

B) `decode`(크래시 복구 경로)만 패닉-프리 보장, `RelativePath` 등 생성자는 호출자 보장 전제로 패닉 허용

C) 명시적 패닉-프리 요구 없음 — serde/라이브러리 기본 + Rust 메모리 안전성에 의존, 패닉은 버그 신호로 수용

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 9 — PBT 프레임워크 선택 (PBT-09, ⚠️ 이 단계 강제 의무) · [Tech Stack]
PBT-09(프레임워크 선택)는 Functional Design에서 이 NFR Requirements 단계로 명시적으로 이월된 **강제(blocking) 의무**입니다. 언어는 Rust로 확정(NFR-17)됐으므로 남은 결정은 U0 foundation과 이를 의존하는 U1~U8 전체가 쓸 속성 기반 테스트 프레임워크의 확정입니다. 선택은 `tech-stack-decisions.md`에 문서화하고 dev-dependency로 추가합니다. 어떤 PBT 프레임워크를 채택하시겠습니까?

A) `proptest` — 매크로 기반 Strategy 제너레이터, 우수한 자동 shrinking, `cargo test` 통합, 시드 재현성. NFR-17 지정 기본값이며 `RelativePath`(정규화 만족)·상관 매니페스트 쌍·유니코드 `detail` 등 제약 도메인 제너레이터에 적합 (권장)

B) `quickcheck` — `Arbitrary` 트레이트 기반, 경량이나 shrinking이 약하고 제약 있는 도메인 제너레이터 작성이 번거로우며 시드 재현/CI 통합 편의가 낮음

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 10 — 도메인 제너레이터의 하위 단위 재사용/노출 방식 (PBT-07) · [Maintainability]
PBT-07은 도메인 제너레이터의 재사용성을 요구하고, U1(다이제스트 결정성·diff 오라클), U4(SyncState stateful PBT), U5/U6(지속 레코드 round-trip)은 모두 U0가 정의한 타입의 제너레이터(`RelativePath`·`Manifest`·`Timestamp` 등)를 필요로 합니다. U0의 도메인 제너레이터를 워크스페이스에 어떻게 노출/공유하시겠습니까?

A) `foundation`에 비기본 cargo feature(예: `proptest-support`)로 제너레이터 모듈 노출 → 하위 크레이트가 dev-dependency에서 feature를 켜 재사용. 단일 출처로 드리프트 방지, feature 게이트로 비테스트 빌드에 proptest 미유출 (권장)

B) 제너레이터를 `foundation`의 `#[cfg(test)]` 비공개로 두고 각 하위 단위가 U0 타입 제너레이터를 자체 재작성 — 공개 표면은 단순하나 단위 간 제너레이터 정의가 갈라져 일관성 저하

C) 별도 `foundation-testkit` 크레이트로 제너레이터 분리 — 관심사 분리는 깔끔하나 내부 워크스페이스에 크레이트/버전 관리 부담 추가

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 11 — 토큰 필드 로그/Debug 유출 방지 (Redacted 래퍼) · [Security-잔존]
`WatcherConfig`의 `token` 필드(평문 String)를 마스킹 래퍼(Debug/Display 시 실제 값 대신 `***` 노출)로 감쌀까요, 평문 String 그대로 둘까요? RISK-01은 수용됐지만 U6 `StructuredLogger`가 config/오류를 직렬화하거나 `WatcherConfig`가 우발적으로 Debug-print될 때 토큰이 로컬 평문 로그에 새는 것은 별개의 저비용 위생 문제이며, 이 타입 선택은 U5/U6로 전파됩니다.

A) 자체 구현 redacting newtype: Debug/Display는 `***`를 찍고 실제 값은 `.expose()`로만 접근. Serialize/Deserialize는 실제 값 보존해 config round-trip(PROP-BR-02)을 안 깨뜨림. 약 15~20줄, 외부 의존 0 (권장)

B) 평문 String 유지: "원시 config를 절대 로그하지 않는다"는 규율에만 의존. 가장 단순하나 Debug 파생이나 config를 캡처한 오류가 로그되면 토큰이 평문 노출

C) `secrecy` 크레이트의 `SecretString` 채택: 검증된 패턴이나 외부 의존 추가 + 기본 Serde 미구현이라 config round-trip 글루가 별도로 필요

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 12 — config 검증 오류 메시지 풍부도 · [Usability]
config 파일은 U0의 **유일한 사람-대면 표면**이고 최초 로드 실패는 abort(R-RELOAD-04)로 직결됩니다. 검증 실패 시 오류 메시지를 어느 수준으로 제공할까요? (크리틱이 gap으로 플래그한 항목 — 세트 절제를 위해 선택 질문으로 추가)

A) 구조화된 메시지(필드명 + 기대 형식/범위 + JSON 위치) + 미지 키에 did-you-mean 편집거리 제안(예: `vault_paht` → `vault_path`?) — 사람이 한 번에 여러 오타를 교정. §9.4 PROP-BR-05 오타-편집거리 제너레이터로 테스트 가능 (권장)

B) 필드명 + 위반 사유만(위치/제안 없음) — 최소한의 실행 가능 메시지

C) 파서 기본 오류 메시지 그대로 노출(serde_json 기본) — 무추가작업이나 사용성 최저

X) Other (please describe after [Answer]: tag below)

[Answer]: A — 단, **did-you-mean 편집거리 제안은 범위 절제 위해 이월**(구조화 메시지 = 필드명 + 기대 형식/범위 + JSON 위치만 채택; fuzzy-match 의존/코드 미도입, Code Generation에서 선택적으로 추가 가능)

### Question 13 — 테스트 커버리지 엄격도 + rustdoc 문서 정책 · [Maintainability]
`foundation`은 전 단위가 의존하는 공유 계약 크레이트입니다. 커버리지·공개 API 문서 정책을 어떻게 둘까요? (크리틱이 gap으로 플래그 — 권장 기본 채움; 상세 CI 통합은 Build-and-Test 이월)

A) 전역 커버리지 % 게이트 없음 + PBT + 예제 앵커(PBT-10) 위주 + `foundation` 공개 항목에 `#![deny(missing_docs)]` — 속성/예제로 실질 검증하고 공개 계약 문서화 강제 (권장)

B) 명시적 line/branch % 게이트(예: 90%) + `deny(missing_docs)` — 정량 기준이나 파운데이션 순수 타입에 과할 수 있음

C) 커버리지·문서 린트 정책 미설정(best-effort) — 관리 최소이나 공유 계약 크레이트에 문서/검증 드리프트 위험

X) Other (please describe after [Answer]: tag below)

[Answer]: A

---

## 4. N/A 카테고리 및 확장 컴플라이언스 계획 (완료 게이트에서 최종 판정)

### 4.1 N/A NFR 카테고리 (근거)
| 카테고리 | 판정 | 근거 |
|---|---|---|
| **확장성(Scalability)** | N/A | U0는 순수 lib(값 타입 + CBOR 코덱 + config 로더)로 자체 런타임·스레드·처리량 축이 없음. SafetyLimits는 컴파일타임 상수(Q2=A), 100k파일/20GiB 스트리밍 스케일(NFR-02)은 U1 소관 |
| **가용성(Availability)** | N/A | lib 크레이트라 SLA 없음. 전체 Watcher는 단일 사용자·사용자 재시작 로컬 프로세스라 RTO/availability-SLA가 Requirements(RESILIENCY-02)에서 이미 N/A 확정 |
| **성능 수치 목표(Performance numeric)** | N/A(정성 계약만) | 코덱 최대 페이로드 = 100k `Manifest`(엔트리 수에 선형·유계)라 throughput/latency/peak-memory 수치 게이트 근거 없음. 스트리밍 해시 메모리 바운드(NFR-02)는 U1 소관. U0는 "엔트리 수에 선형·유계"라는 정성 계약만 문서화 |
| **Resiliency DR/RTO/RPO** | N/A(신규 결정 없음) | DR=N/A(단일 리전), RTO/availability=N/A는 Requirements RESILIENCY-02에서 답변됨. RPO=zero-loss는 U4 durable state가 전달하며 U0 무손실 코덱(R-CODEC-01)은 그 기반일 뿐. keep-last-good/abort(Q3=A)도 확정 |
| **사용성(config 파일 외)** | N/A | U0는 UI/CLI/트레이 표면 없는 lib. 사용자 경험 표면은 U6/U7 소관. config 검증·전수-키 나열(R-CFG-STRICT-01)은 확정, 오류 메시지 풍부도만 Q12로 표면화 |
| **보안 강제 통제** | N/A | Security Baseline OFF, RISK-01/02 수용. 암호화 저장·키관리·시크릿 스캐닝 신설은 N/A. 유일 잔존 통제 TLS(NFR-06)는 §4.1 검증 규칙으로 확정. in-scope 잔존 결정은 토큰 로그-유출 위생(Q11) 하나 |

### 4.2 확장 컴플라이언스 계획
| 확장 | 활성 | 이 단계 판정 | 계획 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제 — PBT-09 충족** | Q9(proptest 프레임워크 선택)이 세트에 포함(`pbt09_present=true`). Q10이 PBT-07 제너레이터 재사용을, Q8이 no-panic 속성 검증을 보강. PBT 채택·속성 식별(PBT-01)은 FD 완료 — 재도출 안 함. shrinking/시드/CI(PBT-08) 상세는 Code Generation/Build-and-Test 이월. **Q9 미확정 시 blocking** |
| **Resiliency Baseline** | ON | **N/A(신규 U0 결정 없음)** | RTO/availability-SLA/DR는 RESILIENCY-02에서 U0에 N/A 확정. RPO=zero-loss는 U4 소관이며 U0 무손실 코덱(R-CODEC-01)은 그 기반 — Q1(코덱 크레이트)/Q8(decode no-panic)이 이 기반 신뢰성을 뒷받침. keep-last-good/abort(Q3=A) 확정 |
| **Security Baseline** | OFF | **N/A(잔존 표면화)** | 미로딩·미강제. RISK-01/02 수용. 유일 잔존 통제 TLS(https)는 §4.1 검증 규칙 — Q3은 그 검증의 파서/깊이 메커니즘만 다룸(https-only 재질문 아님). 저비용 in-scope 잔존 결정 토큰 로그-유출 위생(Q11)만 표면화 |

---

## 5. 재질문하지 않은 항목 (크리틱 DROP 목록 — 참고)

이미 확정되어 이 세트에서 **의도적으로 제외**:
- Q1=B(내부=CBOR / config=JSON 포맷 분리) · Q2=A(SafetyLimits 상수) · Q3=A(keep-last-good/abort) · Q4=B(strict reject "여부") · Q5=A(CLI reload만) · Q6=A(발견 경로) · Q7=A(토큰 우선순위) · Q8=A(SyncState 모델)
- FQ-1=A(표준 SHA-256) · FQ-2=A(최신상태 대체) · FQ-3=A(요구 경미 수정)
- 언어=Rust(NFR-17) · 워크스페이스=단위별 lib + 얇은 watcher-bin, 단일 바이너리(UQ-2=B/RESILIENCY-01) · 토큰 1차 저장=config 평문 + secure-store opt-in
- `server_endpoint` https-only 강제(§4.1) — Q3에 "확정 제약"으로 접어 넣음
- RTO/RPO/DR/availability-SLA = N/A(RESILIENCY-02)
- 코덱 API buffered vs streaming(참고 시그니처 buffered 확정, U4 원자 temp+rename가 완결 파일 요구 → 실질 fork 부재) · 내부 크레이트 버전 관리(단일 바이너리·`publish=false`·path 의존으로 사실상 결정)
- proptest 케이스 수/shrink/시드/CI(PBT-08) — Code Generation/Build-and-Test 이월(Q10에 부분 포섭)

---

## 6. 다음 단계(참고)
NFR Requirements 답변 확정 → Step 5 모호성 분석 → Step 6 `nfr-requirements.md` + `tech-stack-decisions.md` 생성 → 완료 게이트(🔧 변경 요청 / ✅ 승인 → **NFR Design**). 이후 U0 NFR Design → (Infrastructure Design: U7 전용이라 U0는 SKIP) → U0 Code Generation → Wave-2(U1,U2,U4,U5,U6).

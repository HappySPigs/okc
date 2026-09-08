# U0 Foundation — Functional Design 계획 및 질문

**단계**: CONSTRUCTION → Per-Unit Loop → **U0 Foundation** → Functional Design (Part: 계획 + 질문 게이트)
**작성일**: 2026-09-08
**크레이트**: `foundation` (lib) · **소속 컴포넌트**: `CoreTypes`, `ConfigProvider`
**입력 아티팩트**: `unit-of-work.md`(§U0 책임·얇은-단위 근거), `component-methods.md`(§Foundation 시그니처·이월 항목), `unit-of-work-story-map.md`(U0 = US-E7-06 소유 + 전 에픽 교차), `requirements.md`(FR-13/NFR-06/NFR-13/NFR-14 + §12/§13 부록), `unit-of-work-dependency.md`(U0 = DAG 루트, 내부 의존 0)
**규칙**: `construction/functional-design.md` Steps 1–9 · `common/question-format-guide.md` · `common/content-validation.md` · 활성 확장 `property-based-testing.md`(PBT-01 이 단계 강제) · `resiliency-baseline.md`

---

## 1. 단위 컨텍스트 (Step 1 — 요약)

U0는 모든 단위가 참조하는 **공유 파운데이션(DAG 루트, 내부 의존 0)** 이며, 조립 루트(U8)가 소유해 아래로 하향 주입한다. 특정 에픽을 "소유"하지 않는 교차 관심사이나, **US-E7-06(직렬화 무손실 round-trip, NFR-13) 1건만 1차 소유**한다.

### 1.1 `CoreTypes` — 이 단위가 정의할 대상
- **프리미티브**: `RelativePath`, `Sha256Digest`(=표준 sha256sum), `ManifestDigest`(로컬 멱등/no-op 판정용, 권위 `vault_content_id` 아님 — FQ-1=A), `Timestamp`
- **매니페스트**: `ManifestEntry`, `Manifest`
- **변경 집합**: `ChangeSet`(재스냅샷 diff 산출물, 이벤트 큐 아님 — FQ-2=A) + `is_empty()`
- **전송 결과·오류 taxonomy**(Q4=A 파운데이션 소유): `ErrorClass`/`TransportError`(AuthFailed·ServerError·Backpressure·Timeout·Network), `ClassifiedError`, `TransferResult`, `is_retryable()`
- **동기화 상태 머신 타입**: `SyncState`(Idle·Dirty·Uploading·Committed) — 지속/복구 구현은 U4
- **무손실 직렬화 코덱**: `encode`/`decode`, 불변식 `decode(encode(v)) == v`(US-E7-06/NFR-13)
- **관측 싱크 계약 트레이트**(§0 수정1): `Logger`/`StatusSink`/`HistorySink`/`CriticalEventSink` + Q9 2축 상태 어휘 `OperationalState`/`ActiveCondition`/`LivenessSignal`/`StatusSnapshot` + `update_probe()`/`health_check()` 시그니처

### 1.2 `ConfigProvider` — 이 단위가 정의할 대상
- 단일 JSON 설정 파일(nginx식, Q3=X) 로드·스키마 검증·리로드, 타입드 `WatcherConfig` 노출
- API: `load(path)` / `current()` / `reload()` / `subscribe(observer)`
- 토큰 1차 저장 = config 평문 `token` 필드(+env 폴백), 선택적 강화 = OS secure-store opt-in(§13)
- 리로드 시 관찰자 팬아웃(토큰→U5 `ConsentGate`/`CredentialProvider`, 로그레벨→U6 `StructuredLogger`)

### 1.3 Application Design에서 이월된(deferred) 미결정 항목 — §3 질문의 출처
| 이월 항목 | 출처 | 질문 |
|---|---|---|
| 구체 wire 포맷 (canonical JSON vs CBOR) | component-methods §CoreTypes | Q1 |
| `SafetyLimits` 값: 상수 vs config 노출 | component-methods §SafetyLimitsValidator | Q2 |
| config 검증 실패/리로드 실패 시 동작 | ConfigProvider "전체 스키마·기본값 이월" | Q3 |
| 알 수 없는 config 키 처리(strict/lenient) | 스키마 검증 규칙 미정 | Q4 |
| 리로드 트리거 표면 | ConfigProvider `reload()` 호출 경로 미정 | Q5 |
| config 파일 위치·발견 규칙 | `load(path)` 경로 해소 미정 | Q6 |
| 토큰 소스 우선순위 | §13 "config 1차 + env 폴백 + secure-store opt-in" | Q7 |
| `SyncState` 전이 규칙 | component-methods §CoreTypes "SyncState 전이 규칙 이월" | Q8 |

> 필드 스키마 상세·싱크 트레이트 최종 메서드 시그니처는 위 질문 답변 확정 후 산출물에서 완성한다(이월이지만 질문 불필요 — 계약이 대부분 확정됨).

---

## 2. Functional Design 실행 계획 (Steps 2·6 — 산출물 체크박스)

답변 확정(§3) 후, 아래를 `aidlc-docs/construction/u0-foundation/functional-design/` 에 생성한다. 기술중립(Rust스러운 시그니처는 참고용, 인프라 관심사 배제).

- [x] **`domain-entities.md`** — `CoreTypes` 도메인 엔티티/값 타입 상세: 각 타입의 필드 스키마·불변식·정규화 규칙(`RelativePath` POSIX 정규화, `Sha256Digest`/`ManifestDigest` 구분, `ManifestEntry` 정렬 기준), `WatcherConfig` 전체 필드 스키마 + 기본값 표(Q2/Q6/Q7 반영), 엔티티 관계도(텍스트/화살표 표기 — ASCII 박스 미사용)
- [x] **`business-rules.md`** — 결정 규칙·검증 로직·제약: 코덱 무손실 불변식(US-E7-06), `ChangeSet::is_empty` no-op 규칙, `ErrorClass::is_retryable` 매핑 표, config 스키마 검증 규칙(필수/선택 필드, 범위·형식 검증, Q4 strict/lenient, Q3 실패 동작), 토큰 해소 우선순위(Q7), `SafetyLimits` 소스(Q2), 리로드 원자성/관찰자 팬아웃 규칙
- [x] **`business-logic-model.md`** — 핵심 로직·알고리즘·데이터 흐름: `encode/decode` 코덱 흐름(선택 포맷 Q1), `SyncState` 전이 모델(Q8) 텍스트 상태전이표, config load→validate→typed→subscribe 흐름 + reload 팬아웃 시퀀스, 싱크 계약 트레이트의 push-only 상호작용 모델(하향 주입, U6 역참조 없음)
- [x] **PBT-01 "Testable Properties" 섹션**(각 산출물 내, 확장 강제): 컴포넌트별 식별 속성 — `decode(encode(v))==v` 라운드트립(PBT-02, US-E7-06/NFR-13); `manifest_digest` 결정성(불변/오라클 vs sha256sum); config 파싱 라운드트립 + 리로드 멱등성(PBT-04, "두 번 적용=한 번"); `SyncState` 전이는 U4 stateful PBT(PBT-06)의 모델 대상임을 명시(정의는 U0, 실행은 U4). 각 속성에 PBT 카테고리 라벨 + 제너레이터(도메인 타입, PBT-07) 요구를 기재. 속성 없는 요소는 "No PBT properties identified" 명시
- [x] **확장 컴플라이언스 요약**(완료 게이트용): PBT / Resiliency / Security 준수·N/A 표(§4)
- [x] 산출물 작성 전 `content-validation.md` 검증(특수문자 이스케이프, 코드블록/표 파싱, 복잡 시각요소 텍스트 대안) — 크리틱 지적 박스드로잉 글리프 3건 제거 완료(§4.2/§4.3/§5.1 화살표 표기로 정정), 전 산출물 박스드로잉 0건 확인

> **생성 방식(ultracode)**: 답변 확정 후, 3개 산출물을 병렬 설계 에이전트로 생성하고 완결성/PBT/일관성 크리틱으로 검증한 뒤 게이트를 제시할 예정(이전 Application Design/Units Generation 단계와 동일 패턴). 지금은 계획+질문 게이트이므로 생성하지 않는다.

---

## 3. 질문 (Steps 3–4)

**안내**: 아래 [Answer]: 태그에 **권장안이 미리 채워져** 있습니다(이 프로젝트의 기존 방식). 그대로 두시면 권장안으로 진행하고, 원하시면 다른 문자로 바꾸거나 X)에 직접 기술해 주세요. "전부 권장안대로"라고만 하셔도 됩니다. 모두 확정되면 "done"/"답변완료"라고 알려주세요.

### Question 1 — 내부 지속 상태의 직렬화 포맷(wire format)
config 파일 자체는 사람이 편집하는 JSON(nginx식)으로 고정입니다. 이 질문은 **데몬이 내부적으로 저장하는 상태**(마지막 커밋 매니페스트·SyncState[U4], 동의 부여[U5], 업로드 히스토리[U6])가 통과할 `CoreTypes` `encode/decode` 코덱 포맷에 관한 것입니다. 무손실 round-trip(US-E7-06/NFR-13)과 자동 업데이트 간 스키마 진화 내성이 핵심입니다.

A) Canonical JSON — 사람이 읽고 디버깅 가능하나, 무손실·결정적 보장을 위해 키 정렬/수 표현 정규화 부담

B) CBOR (serde 호환 바이너리) — 컴팩트·결정적·자기기술적이라 스키마 진화에 강함(자동 업데이트 후 상태파일 호환 유리), 사람 비가독 (권장)

C) bincode/postcard (Rust 특화 초경량 바이너리) — 최소 크기이나 스키마 진화·버전 호환 취약(롤백/업데이트 위험)

X) Other (please describe after [Answer]: tag below)

[Answer]: B

### Question 2 — `SafetyLimits` 값(총 ≤20 GiB / 파일당 ≤2 GiB / 파일 수 ≤100k)의 소스
서버(okc-core)가 권위 있게 재검증(DEP-04)하므로 클라이언트 한도는 프리플라이트 가드입니다.

A) 고정 상수로 하드코딩 (okc-core 캡과 동일, 서버 권위와 일치, 단순) (권장)

B) config 노출하되 서버 캡을 상한으로 강제 (사용자는 더 낮게만 설정 가능)

C) config로 완전 자유 설정 (서버 캡과 어긋날 수 있음 — 비권장)

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 3 — config 검증/리로드 실패 시 동작
실행 중 `reload()`로 받은 새 config가 스키마 검증에 실패할 때(또는 최초 로드 실패 시)의 동작입니다.

A) 리로드 실패 시 **마지막 정상 config 유지 + 오류 로그 후 계속 실행**(nginx식 fail-safe); 단, 최초 기동 시 로드 실패는 비정상 종료 (권장)

B) 리로드 실패를 중대 오류로 표면화하고 데몬 정지

C) 리로드 실패 시 degraded/paused 상태 진입(감시는 유지, 업로드 보류)

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 4 — 알 수 없는/여분의 config 키 처리
평문 JSON을 사람이 편집하므로 오타(예: 볼트 경로 키 오기)가 흔한 실패원입니다.

A) Lenient — 알 수 없는 키는 경고 로그 후 무시(전방호환 유리, 구버전으로 롤백 시 신필드 무해)

B) Strict — 알 수 없는 키가 있으면 검증 실패(오타 조기 발견, nginx식 엄격성) (권장)

X) Other (please describe after [Answer]: tag below)

[Answer]: B

### Question 5 — 리로드 트리거 표면(`ConfigProvider.reload()` 호출 경로)
설계에는 이미 CLI `reload`(OperatorCli → ControlPlane → ConfigProvider)가 있습니다. 크로스플랫폼(Windows엔 SIGHUP 없음) 고려사항입니다. 시그널/자동감지 처리 로직은 U8/U7b 소관이며 이 질문은 U0가 어떤 호출 경로를 전제로 설계되는지를 정합니다.

A) CLI `reload` 명령만 (이미 설계에 존재, 3-OS 이식 가능, 최소) (권장)

B) CLI `reload` + Unix SIGHUP(nginx식 시그널; Windows는 CLI만) — 시그널 핸들링은 U8 추가

C) CLI `reload` + config 파일 변경 자동 감지 후 자동 리로드(편집 중 부분쓰기 리로드 위험 존재)

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 6 — config 파일 위치·발견 규칙
`load(path)`가 경로를 어떻게 해소할지입니다.

A) 플랫폼별 표준 기본 경로 + `--config` CLI 플래그 오버라이드 + 환경변수(예: `OKC_WATCHER_CONFIG`) 오버라이드 (nginx식, 유연) (권장)

B) `--config` 플래그 필수(기본 경로 없음)

C) 고정 기본 경로만(오버라이드 없음)

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 7 — 토큰 소스 우선순위(config `token` 필드 / env / secure-store)
§13: config 1차 + env 폴백 + secure-store opt-in. 셋이 동시에 존재할 때의 우선순위입니다.

A) secure-store(opt-in 활성 & 세션 가용 시) > config `token` > env 폴백 (secure-store가 가장 안전하므로 우선; §13 "config 1차, env 폴백" 및 `CredentialProvider.resolve_token` 서술과 정합) (권장)

B) env > config `token` > secure-store (12-factor식 env 최우선)

C) config `token` > env > secure-store

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 8 — `SyncState` 전이 모델(실패·중간 변경 처리)
`SyncState`(Idle·Dirty·Uploading·Committed)의 전이 규칙입니다. FQ-2=A(최신 상태 대체, 폴더가 진실源) + Q2=B(트리거당 1회 직렬 사이클)를 전제로 합니다. 이 모델은 U4 `SyncStateStore`가 지속하고 U4 stateful PBT(PBT-06)가 검증합니다.

A) 선형 + dirty 재진입: `Idle→Dirty→Uploading→Committed→Idle`. 업로드 **실패** 시 `Dirty` 유지(재시도 대기, U4 백오프). 업로드 **중 새 변경** 감지 시 dirty 플래그를 세팅해 커밋 후 다음 사이클이 재스냅샷. 별도 Failed 상태 없음 (권장)

B) 명시적 `Failed` 상태 추가: `Uploading→Failed→(백오프)→Dirty`

X) Other (please describe after [Answer]: tag below)

[Answer]: A

---

## 4. 확장 컴플라이언스 계획 (완료 게이트에서 최종 판정)

| 확장 | 활성 | 이 단계(U0 Functional Design) 적용 | 계획 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제(PBT-01)** | 각 산출물에 "Testable Properties" 섹션 필수(§2). 핵심: `decode(encode(v))==v` 라운드트립(PBT-02, US-E7-06). 도메인 제너레이터(PBT-07) 요구 기재. 프레임워크 선택(PBT-09)은 NFR Requirements 이월(Rust=proptest 유력, 여기선 명시만). 미준수 시 blocking. |
| **Resiliency Baseline** | ON | 대체로 N/A + 코덱↔무손실 연계 | RESILIENCY-01: U0 = **Critical**(모든 단위가 의존, downstream=전 단위) 명시. 무손실 코덱(NFR-13)이 U4 무손실 지속(RPO=0/zero-loss)의 기반임을 문서화. RTO/RPO(02)·배포/롤백(04)·관측/HA/DR(05–15)은 U0(순수 타입+config)에 **N/A**(상위 단계/인프라 소관, RESILIENCY-03 면제는 Requirements에서 수용됨). |
| **Security Baseline** | OFF | N/A | 미로딩·미강제. config 평문 토큰(RISK-01) 및 RISK-02는 문서화된 수용 위험. |

---

## 5. 다음 단계(참고)
U0 Functional Design 완료 게이트 승인 후 → **U0 NFR Requirements**(per-unit 루프의 다음 스테이지). 이후 U0 NFR Design → (Infrastructure Design: U7 전용이라 U0는 SKIP) → U0 Code Generation. U0 완료 후 Wave-2(U1,U2,U4,U5,U6)로 진행.

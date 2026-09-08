# U0 Foundation — NFR Design 계획 및 질문

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U0 Foundation** -> NFR Design (Part: 계획 + 질문 게이트)
**작성일**: 2026-09-08
**크레이트**: `foundation` (lib) · **소속 컴포넌트**: `CoreTypes`, `ConfigProvider`
**입력 아티팩트**: `nfr-requirements/nfr-requirements.md`·`nfr-requirements/tech-stack-decisions.md`(U0 NFR Requirements 산출물, 승인 2026-09-08T09:35:00Z) · `functional-design/`(domain-entities·business-rules·business-logic-model) · `plans/u0-foundation-nfr-requirements-plan.md`(확정 답변 Q1~Q13 + DROP §5) · `inception/requirements/requirements.md`(§5 NFR / §7 RISK / §12·§13 오버레이) · `inception/application-design/components.md`(공개 2 컴포넌트 확정)
**규칙**: `construction/nfr-design.md` Steps 1–9 · `common/question-format-guide.md` · `common/content-validation.md` · 활성 확장 `property-based-testing.md`(ON, Full)·`resiliency-baseline.md`(ON)
**생성 방식(ultracode)**: 질문 세트는 워크플로 `wf_5d00f290-216`(4 설계-차원 분석가 + dedup/완결성 크리틱, 5/5 에이전트 0 오류, 653K 토큰)로 저작 -> 크리틱이 이미-확정 항목(NFR-Req Q1~Q13, FD Q1~Q8, tech-stack 크레이트, https-only, RTO/RPO/DR)을 DROP하고, MANDATORY 카테고리(Resilience/Scalability/Performance/Security/LogicalComponents)를 평가해 N/A 판정, 진짜 열린 설계 결정만 절제된 순서 세트로 큐레이션.

---

## 1. NFR Requirements 분석 (Step 1 — 요약)

NFR Requirements는 U0의 **품질 속성(무엇)** 과 **기술 스택(어느 크레이트)** 을 확정했다. NFR Design은 그 확정을 **어떤 설계 패턴·논리 컴포넌트 구조로 실현하는가(어떻게)** 를 정한다. 크리틱 분석 결과 U0는 대부분의 실현 메커니즘이 이미 NFR-Req에서 결정됐고, 다음 **7개 결정만 설계 수준에서 아직 열려 있다**:

- **논리 컴포넌트 분해**: Application Design은 공개 2 컴포넌트(CoreTypes·ConfigProvider)만 확정했고, FD는 타입·규칙·흐름만 정의 — 내부 논리 컴포넌트 분해(명명·책임·의존)는 NFR Design `logical-components.md`에서 처음 내려짐(Q1).
- **REL-02 panic-free 강제 방식**: "요구"는 확정, 코드 규율을 넘는 구조적 강제 방식은 열림(Q2).
- **REL-03 reload 쓰기 경로 직렬화**: ArcSwap는 포인터 스왑 원자성만 보장 — `load -> validate -> swap -> notify` 전체 시퀀스 직렬화는 열림(Q3).
- **REL-04 관찰자 레지스트리 구조/등록 생애주기**: `catch_unwind` 격리는 확정, 저장 자료구조·등록 시점은 열림(Q4).
- **USE-01/MNT-01 config 검증 오류 누적 깊이**: 미지 키 전수 나열은 확정, per-field 위반 누적 여부는 열림(Q5).
- **federated known-key 등록 형태**: "각 하위 단위가 등록" 계약만 확정, 비순환 유지 메커니즘 형태는 열림(Q6).
- **SEC-02 redaction 보증 표면**: 값-보존 Serialize는 확정이나 명시된 U6 직렬화 유출 벡터를 닫는 계약 경계는 열림(Q7).

---

## 2. NFR Design 실행 계획 (Steps 2·6 — 산출물 체크박스)

답변 확정(§3) 후, 아래를 `aidlc-docs/construction/u0-foundation/nfr-design/` 에 생성한다.

- [x] **`nfr-design-patterns.md`** — NFR별 실현 설계 패턴 확정: (REL-02) panic-free total 강제 = 순수 모듈 clippy 패닉-계열 lint-gate + PBT no-panic; (REL-03) reload 쓰기 경로 = 내부 `reload_mutex` 전체 시퀀스 직렬화 + ArcSwap lock-free 읽기; (REL-04) 관찰자 팬아웃 = 기동시 고정 불변 순서 컬렉션 + per-observer `catch_unwind`(확정); (USE-01/MNT-01) config 검증 = 미지 키 전수 나열(R-CFG-STRICT-01) + per-field 위반은 타입드 deserialize 첫 오류를 구조화 메시지로 보고(**Q5=C 스코프 트림**: 전-field 누적 Value 검증 패스 미채택); (확장성) federated known-key = 컴파일타임 per-unit const + watcher-bin 집계·주입 계약; (SEC-02) redaction 보증 = Debug/Display 한정 + 비-Serialize 로깅 계약. 각 패턴에 근거(REL/USE/SEC ID + 규칙 ID) + 확장 컴플라이언스 + N/A 카테고리표 포함
- [x] **`logical-components.md`** — 공개 2 애그리게이트 하위를 명명 논리 컴포넌트로 전개 + 각 컴포넌트에 책임 + 공개 인터페이스 표면 + U0 내부 의존 엣지 표 + REL-01..05·PROP-* PBT 타깃 정렬 + proptest-support 제너레이터를 런타임 그래프 밖 test-support 논리 단위로 문서화 + RESILIENCY-01 Critical DAG 루트 의존 매핑 (논리 분해는 추적성 문서 맵이며 물리 모듈/크레이트 증식 강제 아님 — 물리 레이아웃은 Code Generation에서 최소로 결정)
- [x] 산출물 작성 전 `content-validation.md` 검증(특수문자 이스케이프, 표/코드블록 파싱, ASCII 화살표 `A -> B`, 박스 미사용) — grep 검증 clean(유니코드 화살표/박스 0건)
- [x] 확장 컴플라이언스 요약(§4) 최종 판정 — 이 단계 blocking 없음 재확인(PBT-09는 NFR-Req에서 충족)

> 지금은 **계획 + 질문 게이트**이므로 위 산출물을 생성하지 않는다. 답변 확정 후 생성한다.

---

## 3. 질문 (Steps 3–4)

**안내**: 아래 `[Answer]:` 태그에 **권장안이 미리 채워져** 있습니다(이 프로젝트의 기존 방식). 그대로 두시면 권장안으로 진행하고, 원하시면 다른 문자로 바꾸거나 `X)` 에 직접 기술해 주세요. **"전부 권장안대로"** 라고만 하셔도 됩니다. 모두 확정되면 "done"/"답변완료"라고 알려주세요.

### Question 1 — U0 내부 논리 컴포넌트 분해 입도 (`logical-components.md` 핵심 산출) · [Logical Components]
NFR Design의 `logical-components.md`에서 Application Design이 확정한 공개 2 컴포넌트(`CoreTypes`·`ConfigProvider`)의 내부를 어느 입도로 명명된 논리 컴포넌트로 분해하고, 문서에 어디까지(책임·인터페이스 표면·U0 내부 의존 엣지·proptest-support 제너레이터 배치) 담을까요?

A) **Fine-grained**: 두 공개 컴포넌트를 상위 애그리게이트로 유지한 채 이미 확정된 기능을 명명 논리 컴포넌트로 전개(`CoreTypes` -> Codec/PathNormalizer/ErrorTaxonomy/TokenSecret/SyncStateModel/SinkContracts/StatusVocab, `ConfigProvider` -> ConfigLoader/ConfigValidator/UrlValidator/ConfigStore/ObserverRegistry). 각 컴포넌트를 REL-01..05·PROP-* PBT 타깃에 정렬하고 책임+공개 인터페이스 표면+U0 내부 의존 엣지를 표로 기재(RESILIENCY-01 Critical DAG 루트 추적성 극대화). proptest-support 제너레이터는 런타임 그래프 밖 별도 test-support 논리 단위로 문서화. 발명이 아니라 확정 기능의 명명·정렬이며 FD 흐름에서 near-free 파생 (권장)

B) **Coarse**: 공개 2 컴포넌트만 논리 단위로 유지하고 NFR 실현 메커니즘은 컴포넌트 명명 없이 NFR별 인라인으로만 기술. 중복 최소이나 NFR->컴포넌트 추적성 약하고 Critical 공유 크레이트 의존 매핑(RESILIENCY-01)이 얕음

C) **Partial**: 신뢰성 경계가 뚜렷한 곳(Codec·ConfigStore·ObserverRegistry·TokenSecret)만 명명 분해하고 선언적 값 타입·검증 헬퍼는 인라인 유지. 절충안이나 정렬 기준이 비일관

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 2 — 순수 표면 panic-free total(REL-02, Q8=A)의 구조적 강제 메커니즘 · [Reliability]
순수 표면(`encode`/`decode`·`RelativePath::normalize`·오류 분류 헬퍼)의 panic-free total 계약(Q8=A)을 코드 규율에 그치지 않고 어떤 구조적 메커니즘으로 강제하시겠습니까?

A) 순수 코덱/normalize/분류 모듈 상단에 컴파일타임 clippy lint-gate를 건다(`deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)`) + 기존 PBT no-panic 속성(§2.2 AC-1/2/3) 병행. Q8=A 신뢰성 불변식을 컴파일타임 보증으로 승격하고 이미 채택된 `deny(missing_docs)`(Q13=A)와 일관, 런타임 비용 0 (권장)

B) lint-gate + PBT에 더해 `decode` 진입 경계에 런타임 `catch_unwind` 방어 래퍼를 추가. 단 total 순수 함수에 런타임 unwind-catch는 panic-free-total 계약과 개념적으로 상충하고 버그를 은폐할 수 있음

C) 컴파일타임 lint-gate 없이 코드 규율 + PBT no-panic 속성만으로 확보(현행 §2.2 문언에 가장 가까움). 구조적 강제가 약해 하위 기여자가 unwrap/인덱싱 재도입 여지

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 3 — reload() 쓰기 경로(load -> validate -> swap -> notify) 직렬화 패턴 (REL-03) · [Reliability]
`ArcSwap`는 읽기 lock-free + 포인터 스왑 원자성(Q6=A)만 보장하고 `load -> validate -> swap -> notify` 전체 시퀀스의 원자성은 보장하지 않습니다. U0가 U8 스레딩 모델을 전제하지 않는(R-TRIGGER-01) 상황에서 동시 `reload()` 인터리빙·중복 팬아웃을 어떤 패턴으로 막으시겠습니까?

A) `ConfigProvider` 내부에 `reload_mutex`를 두어 `load -> validate -> swap -> notify` 전체 시퀀스를 직렬화한다. 읽기(`current()`)는 여전히 `ArcSwap`로 lock-free. reload가 CLI-only·드문 연산이라 비용이 무시할 만하고, R-RELOAD-02(all-or-nothing)와 R-OBSERVER-03(결정적 팬아웃)를 상위 단위 가정과 무관하게 U0 내부에서 자기완결적으로 보장 (권장)

B) U0 내부 직렬화를 두지 않고 U8 ControlPlane의 단일 스레드 명령 처리에 의존. 단 U8은 별개 단위이며 이 전제는 명문화되지 않아 U0의 원자성/결정성 계약이 상위 단위 구현에 종속(R-TRIGGER-01 "전제하지 않음"과 상충)

C) `ArcSwap`에 CAS/retry 루프만 적용. 그러나 CAS는 포인터 스왑만 직렬화할 뿐 validate 작업·팬아웃 인터리빙·중복 팬아웃을 막지 못해 R-OBSERVER-03 결정성 미보장

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 4 — 관찰자 레지스트리 자료구조 및 등록 생애주기 (REL-04) · [Reliability]
성공 스왑 직후 결정적 팬아웃(R-OBSERVER-03) + per-observer `catch_unwind` 격리(Q7=A)는 확정됐습니다. 관찰자를 `ConfigProvider`가 어떤 자료구조로 저장하고 언제 등록하도록 설계하시겠습니까?

A) 기동 시 고정 등록되는 불변 순서 컬렉션(`immutable ordered Vec<Arc<dyn ConfigReloadObserver>>`): U8 조립 시점에 데몬 스레드 기동 전 1회 등록되고 이후 변경 없음. 결정적 반복 순서(R-OBSERVER-03) 자연 충족, 런타임 뮤테이션이 없어 레지스트리 락 불필요. 관찰자(U5/U6)가 조립 시점에 이미 알려져 있고 reload가 CLI-only·드문 연산인 데몬 특성과 정합 (권장)

B) 동적 register/unregister를 지원하는 스레드안전 레지스트리(`RwLock`/`Mutex` 보호 Vec)로 런타임 구독 변경 허용. 그러나 런타임 (un)subscribe를 요구하는 규칙이 없고, 스레드안전 뮤테이션이 결정적 순서 재현(R-OBSERVER-03)에 추가 부담·비결정성 위험 도입

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 5 — config 검증 오류 누적 모델 및 검증 locus (per-field 위반 누적 여부) · [Usability / Maintainability]
config 하나에 여러 결함이 동시에 존재할 때(미지 키 다수 + 무효 `log_level` + 상대 `vault_path` + 비-https `server_endpoint` + `notify_consecutive_failures < 1`) `ConfigError` 검증 리포트는 결함을 어느 깊이까지 누적할까요? (미지 키 전수 나열은 R-CFG-STRICT-01로 확정; 여기서 여는 것은 per-field 위반의 누적 여부와 검증 locus입니다)

A) 단일 `ConfigError`에 구조화 이슈 Vec로 전량 누적 — 이미 메모리에 있는 `serde_json::Value`를 한 번 순회하며 미지 키 전수 + 모든 per-field 위반(필드명+기대형식/범위+JSON 위치)을 명시적 검증 패스로 수집해 하나의 오류로 반환. 타입드 deserialize는 clean 판정 후에만 수행. R-CFG-STRICT-01의 "첫 항목에서 중단하지 않음" 철학을 field 위반까지 확장하며 2-pass가 이미 위치를 알고 있어 누적 비용 near-free (권장)

B) 2단계 게이트(미지 키 우선 단락): 패스A에서 미지 키 전수 나열, 미지 키가 하나라도 있으면 그 목록만으로 reject하고 field 검증에 도달하지 않음. clean일 때만 패스B에서 per-field 위반 전량 누적. 단계 책임은 분명하나 사용자가 미지 키/field 위반을 두 번에 나눠 교정

C) 미지 키는 전수 나열하되 per-field 위반은 serde-native 타입드 deserialize의 첫 오류에서 중단(첫 field 위반만 보고). 코드 최소이나 사용자가 field 오타를 한 번에 하나씩만 교정

X) Other (please describe after [Answer]: tag below)

[Answer]: C

> **확정(Step 5 스코프 트림)**: 사용자 지시 "권장안대로 하되 구현 범위가 커지는 건 수정" 적용. Q5-A/B는 이미 확정된 serde_json 2-pass(NFR-Req Q2=A) 위에 **전-field 위반 누적을 위한 별도 Value 레벨 검증 패스**(타입드 모델과 제약 로직 중복 또는 validator 의존 신설)를 얹어 구현 범위를 키움 -> 트림 대상. **C 채택**: R-CFG-STRICT-01의 미지 키 전수 나열은 유지(확정 계약)하고 per-field 위반은 타입드 deserialize의 첫 구조화 오류로 보고. 앞선 Q12 did-you-mean 이월과 동일 성격의 트림. 트레이드오프: field 오타는 한 번에 하나씩 교정(미지 키 오타는 여전히 전수 표시).

### Question 6 — federated known-key-set 등록 메커니즘의 설계 형태 · [Extensibility]
미지 키 판정은 전체 통합 스키마 기준이고 각 하위 단위(U1~U7)가 자기 섹션 키를 소유합니다. U0는 DAG 루트라 하위 단위를 import할 수 없습니다(비순환). 하위 단위 키를 통합 known-key set에 등록하는 메커니즘을 어떤 형태로 설계할까요?

A) 컴파일타임 per-unit 상수 + 최상위 조립 bin(`watcher-bin`) 집계·주입: 각 단위가 자기 섹션 키를 `const &[&str]`로 노출하고, 전 단위를 의존하는 `watcher-bin`이 그 합집합을 `ConfigProvider` 생성 시 주입. U0는 known-key set을 "주입받는 계약"만 정의(하위 단위 import 없음)해 비순환 유지, 런타임 등록 순서 위험 없음 (권장)

B) 런타임 등록 레지스트리: 각 단위가 기동 시 자기 섹션 키를 `ConfigProvider` 가변 레지스트리에 등록(관찰자 패턴과 동형). 최초 load 이전 전 단위 등록 완료를 요구하는 순서 계약(first-load 위험) 추가

C) `foundation`이 전 단위 섹션 키 합집합을 자체 하드코딩. 가장 단순하나 U0가 하위 단위 키를 알아야 해 DAG 비순환(U0=루트)과 federated 소유를 위반 — 사실상 배제되는 반례 선택지

X) Other (please describe after [Answer]: tag below)

[Answer]: A

### Question 7 — SEC-02 redaction 보증 표면 vs U6 직렬화 유출 경계 · [Security-잔존]
SEC-02는 `token`을 redacting newtype으로 감싸되 `Serialize`/`Deserialize`는 실제 값을 보존(PROP-BR-02 round-trip 유지)하도록 확정됐습니다. 그런데 같은 SEC-02 위협 서술은 "U6 `StructuredLogger`가 config/오류를 직렬화"하는 경로를 유출 벡터로 명시 — 값-보존 Serialize는 이 유출을 막지 못합니다. redaction 보증 표면(Debug/Display만인지 serde Serialize까지인지)과 U6 로깅 경로의 안전 소비 방식을 설계 수준에서 어떻게 확정할까요?

A) redaction 보증을 Debug/Display 표면에 한정하고 U0가 계약으로 명문화: `WatcherConfig`/`token`은 serde Serialize로 로그에 방출되지 않는다(U6는 config 파생 로그 필드를 Debug/Display[redacted] 또는 명시적 per-field 프로젝션으로만 방출). 값-보존 Serialize는 round-trip/지속 경로 전용으로 예약. 확정된 "Serialize 보존" 결정을 재오픈하지 않으면서 명시된 직렬화 유출 경로를 계약으로 닫음(저비용 위생 성격 유지) (권장)

B) newtype에 로그-안전 redacted 프로젝션(`redacted_view()` 또는 logging 전용 `serialize_with`)을 추가해 U6가 사용하고 기본 Serialize는 값-보존 유지. 유출 경로를 코드로 차단하나 U0 표면/코드가 늘어남

C) U0 계약 없이 "원시 config를 로그하지 않는다"는 운영 규율에만 의존(현 tech-stack §8 서술 유지). 가장 단순하나 newtype이 막으려던 직렬화 유출을 설계 수준에서 방치

X) Other (please describe after [Answer]: tag below)

[Answer]: A

---

## 4. N/A 카테고리 및 확장 컴플라이언스 계획 (완료 게이트에서 최종 판정)

### 4.1 MANDATORY 카테고리 평가 (Step 3 요구)
| 카테고리 | 판정 | 근거 |
|---|---|---|
| **Scalability** | N/A | U0는 순수 lib(값 타입 + CBOR 코덱 + config 로더)로 자체 런타임·스레드풀·처리량 축 없음. SafetyLimits는 컴파일타임 상수(FD Q2=A), 100k파일/20GiB 스트리밍 스케일(NFR-02)은 U1 소관. 신규 확장성 패턴 없음 |
| **Performance** | N/A(정성 계약만) | U0-NFR-PERF-01 = "코덱=엔트리 수 선형·유계" 정성 계약, throughput/latency/peak-memory 수치 게이트 없음(§3.1). 코덱 buffered `Vec<u8>` 확정. 신규 성능 패턴 없음 |
| **Resilience** | 부분 적용 + 대체로 N/A(신규 패턴 없음) | RESILIENCY-01: U0=Critical(전 단위 의존, DAG 루트) 확정·문서화. U0 resiliency 설계는 무손실 코덱(REL-01, U4 zero-loss 기반)·keep-last-good/abort(REL-05)뿐이며 REL 계약으로 실현. RTO/RPO/DR/HA(RESILIENCY-02/11~13), 관측·서킷브레이커·auto-scaling·배포/롤백·카오스/DR 테스팅은 순수 lib에 부적용(RESILIENCY-02 DR=N/A 확정). Q2~Q4는 확정 REL 계약의 설계 실현이지 신규 인프라 통제 아님 |
| **Security** | N/A(강제 통제) — 잔존 1건만 표면화 | Security Baseline OFF, RISK-01/02 수용. 암호화 저장·키관리·시크릿 스캐닝 신설 없음. 유일 잔존 통제 TLS/https(SEC-01)는 §4.1 검증 규칙으로 확정. 저비용 잔존 설계 포인트는 토큰 redaction 보증 표면/U6 소비 계약(SEC-02) 하나뿐 -> Q7로 표면화(값-보존 Serialize 재오픈 아님) |
| **Logical Components** | 다뤄짐(N/A 아님) — 이 단계 1차 개방 결정 | Application Design은 공개 2 컴포넌트만 확정, FD는 타입·규칙·흐름만 정의 — 내부 논리 분해(명명·책임·의존)는 NFR Design에서 처음 결정 -> Q1로 표면화, `logical-components.md`로 확정 |

### 4.2 확장 컴플라이언스 계획
| 확장 | 활성 | 이 단계 판정 | 계획 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **준수 — 이 단계 blocking 없음** | PBT-09(프레임워크)는 NFR-Req에서 proptest로 충족. PBT-01(속성 식별)은 FD에서 완료. NFR Design은 신규 PBT 결정 없이 속성->논리 컴포넌트 추적성 정렬만 요구(Q1-A가 각 컴포넌트를 REL-01..05·PROP-* 타깃에 정렬). PBT-07(proptest-support)은 Q1-A가 test-support 단위로 문서화. PBT-08(케이스/시드/CI)은 Code Generation/Build-and-Test 이월 |
| **Resiliency Baseline** | ON | **준수(부분 적용 + 대체로 N/A) — blocking 없음** | RESILIENCY-01 Critical 분류·의존 매핑 확정, Q1-A(U0 내부 의존 엣지 표)가 추적성 강화. reload 직렬화(Q3)·관찰자 격리(Q4)·panic-free 강제(Q2)는 확정 REL 계약의 논리 실현. RTO/RPO/DR/failover/카오스는 순수 lib에 N/A. 신규 U0 resiliency 결정 없음 |
| **Security Baseline** | OFF | **N/A — 미로딩·미강제** | 확장 OFF. RISK-01/02 수용. 강제 통제 신설 없음. 유일 잔존 통제 TLS(SEC-01) 확정. 저비용 잔존 위생(SEC-02 redaction 보증 표면/U6 소비 계약)만 Q7로 표면화 — 확장을 켜는 것이 아니라 이미 채택된 newtype의 설계 경계 확정 |

---

## 5. 재질문하지 않은 항목 (크리틱 DROP 목록 — 참고)

이미 확정되어 이 세트에서 **의도적으로 제외**:
- **NFR-Req Q1~Q13 전부**: ciborium / serde_json 2-pass 채택 / url 크레이트 / thiserror+순수 serde 분류타입 / Edition2024+고정MSRV / `arc_swap::ArcSwap` / per-observer catch_unwind 채택 / 순수표면 panic-free-total "요구" / proptest[PBT-09] / proptest-support feature / redacting newtype 채택 / 구조화 메시지 채택+did-you-mean 이월 / no-coverage-gate+`deny(missing_docs)`
- **FD Q1~Q8**: CBOR/JSON 포맷 분리(Q1=B) / SafetyLimits 상수(Q2=A) / keep-last-good+abort(Q3=A) / strict reject "여부"(Q4=B) / CLI reload만(Q5=A) / config 발견 경로(Q6=A) / 토큰 우선순위(Q7=A) / SyncState 모델(Q8=A)
- **FQ-1/2/3=A** · 언어=Rust(NFR-17) · 워크스페이스=단위별 lib + 얇은 watcher-bin 단일 바이너리(UQ-2=B, `publish=false`+path 의존)
- `server_endpoint` https-only reject 채택(§4.1 규칙, 재결정 아님) · 코덱 buffered `Vec<u8>`(스트리밍 아님) · RTO/RPO/DR/availability-SLA = N/A(RESILIENCY-02) · PBT-08 상세(케이스/shrink/시드/CI) = Code Generation/Build-and-Test 이월 · 토큰 1차 저장 = config 평문 + secure-store opt-in(RISK-01/02 수용)

> **크리틱 완결성 자기평가**: 7개 질문으로 tight 목표(3~6) 상단을 약간 넘겼으나 각각 서로 다른 논리 컴포넌트·설계 패턴 축의 진짜 개방 결정이며 발명 없음. Q1은 doc-shape·제너레이터 배치 두 하위 후보를 옵션에 흡수(과질문 방지). 놓친 개방 결정 없음 — 코덱·SyncState·경로해소·토큰 우선순위·SafetyLimits·포맷 분리는 모두 FD/NFR-Req에서 확정.

---

## 6. 다음 단계(참고)
NFR Design 답변 확정 -> Step 5 모호성 분석 -> Step 6 `nfr-design-patterns.md` + `logical-components.md` 생성 -> 완료 게이트(🔧 변경 요청 / ✅ 승인 -> **다음 단계**). U0의 Infrastructure Design은 U7 전용이라 **SKIP**되므로 다음 단계는 **U0 Code Generation**. 이후 Wave-2(U1,U2,U4,U5,U6).

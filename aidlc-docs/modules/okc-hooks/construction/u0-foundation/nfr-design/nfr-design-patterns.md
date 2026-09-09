# U0 Foundation — NFR Design Patterns (NFR 실현 설계 패턴)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U0 Foundation** -> NFR Design -> 산출물 1/2 (`nfr-design-patterns.md`)
**작성일**: 2026-09-08
**크레이트**: `foundation` (lib) · **소속 컴포넌트**: `CoreTypes`, `ConfigProvider`
**입력 아티팩트**: `nfr-requirements/nfr-requirements.md`·`nfr-requirements/tech-stack-decisions.md`(U0 NFR Requirements 산출물, 승인 2026-09-08T09:35:00Z) · `plans/u0-foundation-nfr-design-plan.md` §3 확정 답변(Q1~Q7, Q5=C 스코프 트림) + §4.1 N/A 판정 · `functional-design/`(domain-entities·business-rules·business-logic-model) · `inception/application-design/components.md`(공개 2 컴포넌트) · `inception/requirements/requirements.md`(§5 NFR / §7 RISK) · 활성 확장 `property-based-testing.md`(ON, Full)·`resiliency-baseline.md`(ON)
**규칙**: `construction/nfr-design.md` Step 6 · `common/content-validation.md`(ASCII 화살표 `A -> B`, 박스 문자 미사용, Rust 제네릭 백틱) · `common/ascii-diagram-standards.md`

> **문서 성격**: 이 문서는 U0가 확정한 **품질 속성(NFR Requirements)** 을 **어떤 설계 패턴으로 실현하는가** 를 기록한다. 여기서 새로 결정하는 것은 없다 — NFR Requirements(카테고리별 NFR)와 tech-stack-decisions(구체 크레이트), Functional Design(규칙 ID), 그리고 NFR Design 계획 §3의 이미-확정 답변(Q1~Q7)을 **설계 패턴으로 실현**한다. 각 패턴은 (a) 패턴/결정 진술, (b) 실현하는 NFR·규칙 근거, (c) 구조 노트/불변식, (d) 명시적 트레이드오프로 기술한다. 논리 컴포넌트 분해(Q1=A)의 상세 맵은 자매 산출물 `logical-components.md`가 소유하며, 이 문서는 각 패턴이 어느 논리 컴포넌트에 안착하는지만 §9 추적표에서 참조한다.
>
> **표기 규약**: 화살표는 ASCII `A -> B`만 사용한다(유니코드 화살표 금지). 박스/선-그리기 문자를 쓰지 않는다. Rust 제네릭/타입/식별자(예: `Vec<u8>`, `Arc<WatcherConfig>`, `dyn ConfigReloadObserver`, `ArcSwap`, `ConfigError`)는 백틱으로 감싼다.

---

## 1. panic-free-total (REL-02) — 순수 모듈 컴파일타임 clippy lint-gate + PBT no-panic [Q2=A]

**(a) 패턴/결정**: U0 순수 표면(`encode`/`decode` = Codec, `RelativePath::normalize` = PathNormalizer, `ErrorClass::is_retryable`/`TransportError` 분류 = ErrorTaxonomy)이 위치한 **순수 모듈 상단에 컴파일타임 clippy lint-gate**를 건다: `deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)`. 여기에 이미 NFR Requirements가 요구한 **PBT no-panic 속성**(§2.2 AC-1/2/3)을 병행한다. 런타임 방어 래퍼는 두지 않는다(런타임 비용 0).

**(b) NFR/규칙 실현**:
- **U0-NFR-REL-02**(순수 표면 전체 panic-free total)의 "모든 입력에 대해 패닉 없이 `Result`로 표면화" 요구를, 코드 규율이 아니라 **컴파일타임 lint 게이트로 승격**해 하위 기여자가 `unwrap`/슬라이스 인덱싱/`panic!`을 재도입하지 못하게 강제한다.
- **U0-NFR-REL-01**(코덱 무손실 round-trip): `decode`는 R-CODEC-01의 무손실 대상이자 절단·손상·적대적 CBOR 바이트열을 받는 진입점이므로, panic-free가 round-trip 계약과 결합한다(인코딩/디코딩 실패는 `CodecError`(Fatal)로만 표면화).
- **규칙 R-CODEC-01**(코덱 불변식) · **R-CLASS-03**(`is_retryable` total function — 모든 `ErrorClass` 변이가 정확히 하나의 boolean, 미정의·패닉 경로 없음) · **R-RELOAD-03**(keep-last-good): `decode`가 U4 크래시 복구 핫 경로에서 패닉하면 데몬이 재시작마다 재-크래시해 keep-last-good/zero-loss를 무력화하므로, panic-free는 REL-05(U0-NFR-REL-05)의 회복력을 지탱한다.
- **U0-NFR-MNT-03**(`deny(missing_docs)` 채택, Q13=A)과 동일한 "컴파일타임 강제" 성향과 일관된다.

**(c) 구조 노트/불변식**: lint-gate는 **순수 모듈에만** 적용한다(Codec / PathNormalizer / ErrorTaxonomy). `ConfigStore` 등 `ArcSwap` 포인터 로드를 쓰는 상태 보유 모듈은 순수 표면이 아니므로 게이트 대상이 아니다. 불변식: 순수 표면의 모든 실패 경로는 `Result::Err`(각각 `CodecError`/정규화 오류/분류는 total 반환)로만 나가며 `panic!`/`unwrap`/인덱싱 패닉 경로가 타입·lint 수준에서 부재한다.

**(d) 트레이드오프**: (1) 계획 Q2 옵션 B(`decode` 경계 런타임 `catch_unwind` 방어 래퍼)는 **미채택** — total 순수 함수에 런타임 unwind-catch는 panic-free-total 계약과 개념적으로 상충하고 버그를 은폐한다. (2) 옵션 C(lint 없이 코드 규율 + PBT만)도 미채택 — 구조적 강제가 약해 재도입 여지. (3) 채택안의 비용은 기여자가 `unwrap`/인덱싱 대신 명시적 `Result` 처리를 써야 하는 규율 부담뿐이며 런타임 비용은 0.

---

## 2. reload 쓰기 경로 직렬화 (REL-03) — 내부 `reload_mutex` + lock-free `ArcSwap` 읽기 [Q3=A]

**(a) 패턴/결정**: `ConfigProvider` 내부에 **`reload_mutex`** 를 두어 `load -> validate -> swap -> notify` **전체 시퀀스를 직렬화**한다. 읽기(`current()`)는 여전히 `arc_swap::ArcSwap<Arc<WatcherConfig>>`로 **lock-free**(값싼 `Arc` 로드)를 유지한다. 직렬화는 U0 내부에서 자기완결적이며 U8 스레딩 모델을 전제하지 않는다.

**(b) NFR/규칙 실현**:
- **U0-NFR-REL-03**(원자 all-or-nothing 스왑 + 동시 읽기 안전): `ArcSwap`는 포인터 스왑 원자성만 보장하고 `load->validate->swap->notify` 전체 시퀀스 원자성은 보장하지 않는다. `reload_mutex`가 이 시퀀스를 직렬화해 동시 `reload()` 인터리빙과 중복 팬아웃을 차단한다.
- **규칙 R-RELOAD-01**(선검증 후 스왑) · **R-RELOAD-02**(원자성/all-or-nothing, 부분 적용 금지) · **R-RELOAD-05**(성공 후에만 통지): 전체 시퀀스 직렬화가 이 세 규칙을 U0 내부에서 보장한다.
- **규칙 R-OBSERVER-03**(결정적 팬아웃): 팬아웃이 스왑과 동일 임계구역 안에서 일어나 순서·대상이 상위 단위 가정과 무관하게 결정적이다.
- **규칙 R-TRIGGER-01**(리로드 트리거 = CLI-only, U8 전제 없음): 직렬화를 U0 내부에 두어 U8 `ControlPlane`의 단일 스레드 명령 처리에 의존하지 않는다.
- **U0-NFR-REL-05**(keep-last-good): 검증 실패 시 스왑에 도달하지 않으므로 `current()`가 직전 성공 스냅샷을 그대로 유지한다.

**(c) 구조 노트/불변식**: 불변식 — 어떤 동시 읽기 스레드도 항상 **완결된 이전 스냅샷 또는 완결된 새 스냅샷 하나만** 관측한다(중간 상태 없음). `reload_mutex`는 쓰기 경로만 감싸므로 읽기 경로(`current()`)에는 락 경합이 없다. 동일 유효 config를 연속 2회 적용해도 관측적으로 1회 적용과 동일(멱등, PROP-BR-03).

**(d) 트레이드오프**: (1) 계획 Q3 옵션 B(U0 내부 직렬화 없이 U8 단일 스레드 명령 처리에 의존)는 **미채택** — U0 원자성/결정성 계약이 상위 단위 구현에 종속되어 R-TRIGGER-01("전제하지 않음")과 상충한다. (2) 옵션 C(`ArcSwap` CAS/retry 루프만)도 미채택 — CAS는 포인터 스왑만 직렬화할 뿐 validate·팬아웃 인터리빙·중복 팬아웃을 막지 못해 R-OBSERVER-03 결정성 미보장. (3) 채택안의 비용은 reload 쓰기 경로 뮤텍스 획득뿐이나 reload는 CLI-only·드문 연산(R-TRIGGER-01)이라 무시할 만하다.

---

## 3. 리로드 관찰자 팬아웃 (REL-04) — 기동 시 고정 불변 순서 `Vec` + per-observer `catch_unwind` [Q4=A + Q7=A]

**(a) 패턴/결정**: 관찰자를 **기동 시 1회 등록되는 불변 순서 컬렉션 `immutable ordered Vec<Arc<dyn ConfigReloadObserver>>`** 로 저장한다. 등록은 U8 조립 시점에 데몬 스레드 기동 전 1회 이루어지고 이후 변경되지 않는다. 성공 스왑 직후 이 `Vec`를 순서대로 순회하며 **per-observer `catch_unwind`** 로 격리해 팬아웃한다(Q7=A 확정).

**(b) NFR/규칙 실현**:
- **U0-NFR-REL-04**(리로드 관찰자 팬아웃 패닉 격리): 관찰자 K개 중 하나가 패닉/오류를 내도 `catch_unwind` 경계로 격리하고 나머지 K-1개에 계속 팬아웃한다.
- **규칙 R-OBSERVER-01**(성공 후 push-only 통지, back-reference 없음) · **R-OBSERVER-03**(결정성 + 관찰자 측 부수효과 격리): 불변 순서 `Vec`는 런타임 뮤테이션이 없어 **결정적 반복 순서가 자연 충족**되고 레지스트리 락이 불필요하다.
- **규칙 R-OBSERVER-02**(팬아웃 대상 매핑): `token`/`secure_store_enabled` 변경 -> U5 `CredentialProvider`/`ConsentGate`, `log_level` 변경 -> U6 `StructuredLogger` (관찰자는 `ConfigReloadObserver` 계약 타입으로만 하향 주입).
- **규칙 R-RELOAD-05**(성공 후에만 통지): 팬아웃은 §2의 원자적 스왑 커밋 이후에만 발생하는 되돌릴 수 없는 후행 통지다.

**(c) 구조 노트/불변식**: 불변식 — 스왑은 팬아웃 이전에 **이미 원자적으로 커밋**되었으므로 패닉 관찰자가 `current()`나 다른 관찰자를 오염시킬 수 없다(롤백 없음). 등록 생애주기 제약: 관찰자는 **데몬 스레드 기동 전** 1회 등록되어야 하며 이후 불변(U8 조립 계약). 관측은 계약상 best-effort·infallible이다.

**(d) 트레이드오프**: 계획 Q4 옵션 B(동적 register/unregister를 지원하는 `RwLock`/`Mutex` 보호 `Vec`)는 **미채택** — 런타임 (un)subscribe를 요구하는 규칙이 없고, 스레드안전 뮤테이션이 결정적 순서 재현(R-OBSERVER-03)에 추가 부담·비결정성 위험을 도입한다. 채택안은 유연성(런타임 구독 변경)을 포기하는 대신 결정성·무락(lock-free 순회)·단순성을 얻는다. 이 트레이드오프는 관찰자(U5/U6)가 조립 시점에 이미 알려져 있고 reload가 CLI-only·드문 연산인 데몬 특성과 정합한다.

---

## 4. config 검증 (USE-01 / MNT-01) — 미지 키 전수 나열 + 구조화된 FIRST-field-violation [Q5=C 스코프 트림]

**(a) 패턴/결정**: 이미 확정된 `serde_json` 2-pass(NFR-Req Q2=A) 위에서 config 검증을 다음의 **최소 실현**으로 확정한다:
- **미지 키는 전수 나열** — 1차 파스(`serde_json::Value`)에서 주입된 known-key set(§5) 밖의 **모든** 키를 수집·나열한다(첫 키에서 중단하지 않음, R-CFG-STRICT-01).
- **per-field 위반은 타입드 deserialize의 FIRST 구조화 오류로 보고** — 필드명 + 기대 형식/범위 + JSON 위치(포인터)를 담아 **첫 per-field 위반만** 보고한다.
- `ConfigError`는 (다수일 수 있는) 미지 키 이슈 + 단일 첫 field 위반을 담는 **이슈 `Vec`** 을 운반한다.

**(b) NFR/규칙 실현**:
- **U0-NFR-USE-01**(구조화 config 검증 오류 메시지): config는 U0의 유일한 사람-대면 표면이며 최초 로드 실패는 abort(R-RELOAD-04)로 직결되므로, 구조화 메시지(필드명 + 기대 형식/범위 + JSON 위치)로 실현한다.
- **U0-NFR-MNT-01**(오류 타입 파생 전략): 운영 오류 `ConfigError`는 `thiserror` 파생(`Display`/`std::error::Error`)으로 이슈 `Vec`을 표면화한다(분류 값 타입 `ErrorClass`는 순수 serde 유지 — 별개 부류).
- **규칙 R-CFG-STRICT-01**(미지 키 전수 나열, 키 이름 + 위치 포함) · **business-rules §4.1**(per-field 검증 규칙: `vault_path` 절대경로, `server_endpoint` https-only, `log_level` 열거, `notify_consecutive_failures >= 1` 등).

**(c) 구조 노트/불변식**: 미지 키 판정 로직은 §5의 주입된 known-key set을 소비한다. 타입드 deserialize는 미지 키가 clean으로 판정된 후에만 수행한다(2-pass 순서). 불변식 — 미지 키가 하나라도 있으면 검증 실패이며 리포트가 삽입된 모든 미지 키를 나열한다(PROP-BR-05).

**(d) 트레이드오프 — 스코프 트림 명시**: 계획 Q5 옵션 A/B(전-field 위반 누적)는 **미채택**. 이유: A/B는 이미 확정된 serde_json 2-pass 위에 **전-field 위반 누적을 위한 별도 `serde_json::Value` 레벨 검증 패스**를 얹어야 하는데, 이는 타입드 모델과 **제약 로직을 중복**하거나 **validator 의존을 신설**해 구현 범위를 키운다 -> 스코프 트림 대상. 채택안(C)은 확정된 2-pass의 최소 실현이며 미지 키 전수 나열은 유지한다. 대가: field 오타는 한 번에 하나씩 교정(미지 키 오타는 여전히 전수 표시). did-you-mean 편집거리 제안은 NFR-Req USE-01대로 여전히 이월(OPTIONAL) 상태다.

---

## 5. federated known-key set (Extensibility) — 컴파일타임 per-unit const + `watcher-bin` 주입 [Q6=A]

**(a) 패턴/결정**: 미지 키 판정 기준이 되는 known-key set을 **연합(federated)** 으로 구성한다. 각 단위(U0 core 키 포함, U1~U7)가 자기 섹션 키를 **컴파일타임 `const &[&str]`** 로 노출하고, 전 단위를 의존하는 최상위 조립 bin(`watcher-bin`)이 그 **합집합(union)을 집계**해 `ConfigProvider` 생성 시 **주입**한다. U0는 "known-key set을 주입받는다"는 계약만 정의하고 하위 단위를 import하지 않는다.

**(b) NFR/규칙 실현**:
- **U0-NFR-USE-01 / 규칙 R-CFG-STRICT-01**: 미지 키 전수 나열(§4)이 정확히 판정되려면 known-key set이 **전체 통합 스키마 기준**이어야 한다. 연합 const를 `watcher-bin`이 합쳐 주입해 이를 만족한다.
- **규칙 R-LIMIT-01**(SafetyLimits = 컴파일타임 상수, config 필드 아님): 대응 키는 known-key set에 포함되지 않으므로 §4에서 미지 키로 거부된다.

**(c) 구조 노트/불변식 — 비순환(acyclic) DAG 루트 유지**: U0는 DAG 루트라 하위 단위를 import할 수 없다. 컴파일타임 const + 생성자 주입은 **런타임 등록 순서 위험이 없고** U0 -> 하위 단위 의존 엣지를 만들지 않아 비순환을 유지한다. 아래 엣지 스케치 참조:

```
각 단위가 자기 섹션 키를 컴파일타임 const로 노출 (U0는 하위 단위를 import하지 않음):

  U0 foundation  ->  const CORE_KEYS: &[&str]   (vault_path, server_endpoint, token, ...)
  U1             ->  const U1_KEYS:   &[&str]   (exclude_patterns, ...)
  U2             ->  const U2_KEYS:   &[&str]   (debounce_ms, reconciliation_interval_s, ...)
  ...            ->  ...
  U7             ->  const U7_KEYS:   &[&str]   (service, ...)
                              |
                              |  watcher-bin (전 단위 의존) 이 합집합 집계
                              v
        union_keys = CORE_KEYS + U1_KEYS + ... + U7_KEYS
                              |
                              |  ConfigProvider::new(known_keys = union_keys) 로 주입
                              v
   U0 ConfigProvider  ->  주입된 known-key set 으로 미지 키 전수 판정 (serde_json 2-pass)

  의존 엣지:  watcher-bin -> {U0, U1, ..., U7}
             U0 -> (없음 = DAG 루트)              => 비순환 유지
```

**(d) 트레이드오프**: 계획 Q6 옵션 B(런타임 등록 레지스트리, 관찰자 패턴과 동형)는 **미채택** — 최초 load 이전 전 단위 등록 완료를 요구하는 순서 계약(first-load 위험)을 추가한다. 옵션 C(`foundation`이 전 단위 키 합집합을 자체 하드코딩)는 **배제** — U0가 하위 단위 키를 알아야 해 DAG 비순환(U0=루트)과 federated 소유를 위반한다. 채택안은 컴파일타임 안전 + 순서 위험 0의 대가로 `watcher-bin`이라는 단일 집계 지점을 요구한다.

---

## 6. 토큰 redaction (SEC-02) — Debug/Display 한정 + U0 no-Serialize-to-log 계약 [Q7=A]

**(a) 패턴/결정**: redaction 보증을 **`Debug`/`Display` 표면에 한정**한다(NFR-Req Q11=A 확정 redacting newtype: `Debug`/`Display`는 `***`, 실제 값은 `.expose()`로만). 여기에 U0가 **계약으로 명문화**한다: `WatcherConfig`/`token`은 serde `Serialize`로 로그에 방출되지 않는다. U6 `StructuredLogger`는 config 파생 로그 필드를 `Debug`/`Display`(redacted) 또는 명시적 per-field 프로젝션으로만 방출한다. 값-보존 `Serialize`는 round-trip/지속 경로 전용으로 예약한다.

**(b) NFR/규칙 실현**:
- **U0-NFR-SEC-02**(토큰 로그/Debug 유출 위생): 같은 SEC-02 위협 서술이 명시한 "U6 `StructuredLogger`가 config/오류를 직렬화"하는 유출 벡터를, 값-보존 `Serialize`를 재오픈하지 않으면서 **계약으로 닫는다**.
- **U0-NFR-SEC-01**(TLS/`https` 강제): SEC-01(TLS)이 유일 잔존 강제 통제이며, 이 redaction 계약은 그와 별개의 저비용 로그-유출 위생이다(신규 통제 아님).
- **규칙 R-TOKEN-01/02**(토큰 해소 우선순위·부재 처리): 토큰은 config 평문이 1차 저장소(RISK-01 수용)이므로 로그 유출 표면을 Debug/Display 및 Serialize 경계에서 관리한다.
- **규칙 R-OBSERVER-02**(`log_level` 변경 -> U6 `StructuredLogger`): U6가 config 파생 필드를 소비하는 유일 로깅 관찰자이므로 계약의 준수 주체다.

**(c) 구조 노트/불변식**: 불변식 — (1) newtype `Debug`/`Display` 출력에 실제 토큰이 나타나지 않고 `***`가 나타난다. (2) `Serialize`/`Deserialize`는 실제 값을 보존해 config round-trip(PROP-BR-02)이 무손실 유지된다. (3) U6는 `WatcherConfig`/`token`을 serde `Serialize`로 로그에 방출하지 않는다(U0 계약). U0 측 신규 코드는 계약 명문화 + 기존 newtype뿐이다(추가 표면 없음).

**(d) 트레이드오프**: 계획 Q7 옵션 B(newtype에 로그-안전 `redacted_view()`/logging 전용 `serialize_with` 추가)는 **미채택** — 유출 경로를 코드로 차단하나 U0 표면/코드가 늘어난다. 옵션 C(U0 계약 없이 운영 규율에만 의존)도 미채택 — newtype이 막으려던 직렬화 유출을 설계 수준에서 방치한다. 채택안은 표면 증가 0으로 명시된 직렬화 유출 경로를 계약으로 닫는 대신, U6가 계약을 준수해야 한다는 소비-측 규율에 의존한다. Security Baseline은 여전히 OFF이며 이는 수용 위험(RISK-01/02) 완화용 위생일 뿐이다.

---

## 7. MANDATORY 카테고리 N/A 판정표 (계획 §4.1)

| 카테고리 | 판정 | 근거 |
|---|---|---|
| **Scalability** | N/A | U0는 순수 lib(값 타입 + CBOR 코덱 + config 로더)로 자체 런타임·스레드풀·처리량 축이 없음. SafetyLimits는 컴파일타임 상수(FD Q2=A, R-LIMIT-01), 100k 파일 / 20 GiB 스트리밍 스케일(NFR-02)은 U1 소관. 신규 확장성 패턴 없음 |
| **Performance** | N/A(정성 계약만) | U0-NFR-PERF-01 = "코덱 = 엔트리 수 선형·유계" 정성 계약이며 throughput/latency/peak-memory 수치 게이트 없음. 코덱은 buffered `Vec<u8>` 확정. 신규 성능 패턴 없음 |
| **Resilience** | 부분 적용 + 대체로 N/A | RESILIENCY-01: U0 = Critical(전 단위 의존, DAG 루트) 확정·문서화. 순수 lib에 적용되는 것은 무손실 코덱(U0-NFR-REL-01, U4 zero-loss 기반) + keep-last-good/abort(U0-NFR-REL-05) + REL-02/03/04의 설계 실현(§1~§3)뿐. RTO/RPO/DR/HA/서킷브레이커/auto-scaling/카오스(RESILIENCY-02/05~14)는 순수 lib에 부적용. Q2~Q4는 확정 REL 계약의 설계 실현이지 신규 인프라 통제 아님 |
| **Security** | N/A(강제 통제) — 잔존만 표면화 | Security Baseline OFF, RISK-01/02 수용. 암호화 저장·키관리·시크릿 스캐닝 신설 없음. 잔존 통제는 TLS/`https`(U0-NFR-SEC-01)뿐이며, 저비용 잔존 설계 포인트는 토큰 redaction 보증 표면/U6 소비 계약(U0-NFR-SEC-02, §6) 하나 |
| **Logical Components** | 다뤄짐(N/A 아님) | Application Design은 공개 2 컴포넌트만 확정, FD는 타입·규칙·흐름만 정의 — 내부 논리 분해(명명·책임·의존)는 NFR Design에서 처음 결정(Q1=A). 상세는 자매 산출물 `logical-components.md`가 소유(추적성 문서 맵이며 물리 모듈/크레이트 증식 강제 아님) |

---

## 8. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 단계 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **준수 — 이 단계 blocking 없음** | PBT-09(프레임워크)는 NFR-Req에서 `proptest`로 이미 SATISFIED — 이 단계 신규 blocking PBT 결정 없음. PBT-01(속성 식별)은 FD에서 완료. 이 단계의 각 패턴은 확정 속성에 정렬된다: REL-02 no-panic -> PROP-DE-03/PROP-BR-04, REL-03 -> PROP-BR-03(멱등), USE-01/§4·§5 -> PROP-BR-05(미지 키), SEC-02 -> PROP-BR-02(round-trip 보존). PBT-07(제너레이터 재사용)은 test-support 논리 단위로 `logical-components.md`가 문서화. PBT-08(케이스/시드/CI)은 Code Generation/Build-and-Test 이월 |
| **Resiliency Baseline** | ON | **준수(부분 적용 + 대체로 N/A) — blocking 없음** | RESILIENCY-01 Critical 분류·DAG 루트 의존 매핑 확정. REL 계약의 설계 실현: reload 직렬화(§2, Q3)·관찰자 격리(§3, Q4)·panic-free 강제(§1, Q2)·무손실 코덱(REL-01)·keep-last-good/abort(REL-05). RESILIENCY-02(RTO/RPO/DR)·05~10(관측/헬스/서킷브레이커/멀티존/auto-scaling)·11~13(DR)·14(카오스/DR 테스팅 질의)·15(사고 대응)는 순수 lib에 **N/A**. 신규 U0 resiliency 결정 없음 |
| **Security Baseline** | OFF | **N/A — 미로딩·미강제** | 확장 OFF. RISK-01(소스 노출)/RISK-02(재승인 thrash) 수용. 강제 통제 신설 없음. 유일 잔존 통제 TLS(SEC-01) 확정. 저비용 잔존 위생(SEC-02 redaction 보증 표면/U6 no-Serialize 계약, §6)만 설계 경계로 확정 — 확장을 켜는 것이 아니라 이미 채택된 newtype의 계약 경계 명문화 |

**블로킹 판정**: 이 단계에 blocking finding 없음. PBT-09는 NFR-Req에서 충족되었고, Resiliency는 부분 적용 + 대체로 N/A, Security Baseline은 OFF로 N/A다.

---

## 9. 추적표 (패턴 -> NFR ID -> 규칙 ID -> 논리 컴포넌트)

| 설계 패턴 | NFR ID | 규칙 ID | 안착 논리 컴포넌트(Q1=A) |
|---|---|---|---|
| §1 panic-free-total (clippy lint-gate + PBT no-panic) | U0-NFR-REL-02, U0-NFR-REL-01, U0-NFR-MNT-03 | R-CODEC-01, R-CLASS-03, R-RELOAD-03 | `CoreTypes` -> Codec · PathNormalizer · ErrorTaxonomy |
| §2 reload 쓰기 경로 직렬화 (`reload_mutex` + `ArcSwap`) | U0-NFR-REL-03, U0-NFR-REL-05 | R-RELOAD-01, R-RELOAD-02, R-RELOAD-05, R-OBSERVER-03, R-TRIGGER-01 | `ConfigProvider` -> ConfigStore · ConfigLoader · ConfigValidator |
| §3 관찰자 팬아웃 격리 (불변 순서 `Vec` + `catch_unwind`) | U0-NFR-REL-04 | R-OBSERVER-01, R-OBSERVER-02, R-OBSERVER-03, R-RELOAD-05 | `ConfigProvider` -> ObserverRegistry |
| §4 config 검증 (미지 키 전수 + FIRST-field 구조화) | U0-NFR-USE-01, U0-NFR-MNT-01 | R-CFG-STRICT-01, business-rules §4.1 | `ConfigProvider` -> ConfigValidator · ConfigLoader · UrlValidator; `CoreTypes` -> ErrorTaxonomy(`ConfigError`) |
| §5 federated known-key set (per-unit const + `watcher-bin` 주입) | U0-NFR-USE-01 | R-CFG-STRICT-01, R-LIMIT-01 | `ConfigProvider` -> ConfigValidator · ConfigLoader (주입 계약) |
| §6 토큰 redaction (Debug/Display 한정 + no-Serialize 계약) | U0-NFR-SEC-02, U0-NFR-SEC-01 | R-TOKEN-01, R-TOKEN-02, R-OBSERVER-02 | `CoreTypes` -> TokenSecret · SinkContracts(`Logger`) |

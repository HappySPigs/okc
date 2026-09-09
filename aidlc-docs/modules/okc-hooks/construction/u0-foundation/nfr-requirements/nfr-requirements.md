# U0 Foundation — NFR Requirements (비기능 요구사항 / 품질 속성)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U0 Foundation** -> NFR Requirements -> 산출물 1/2 (`nfr-requirements.md`)
**작성일**: 2026-09-08
**크레이트**: `foundation` (lib) · **소속 컴포넌트**: `CoreTypes`, `ConfigProvider`
**입력 아티팩트**: `functional-design/domain-entities.md`·`business-rules.md`·`business-logic-model.md`(U0 FD 산출물, 승인 2026-09-08T08:10:00Z) · `plans/u0-foundation-nfr-requirements-plan.md`(질문 Q1~Q13 확정 답변 §3 + N/A §4 + DROP §5) · `inception/requirements/requirements.md`(§5 NFR / §6 Resiliency 매핑 / §7 RISK-01·02 / §12·§13 오버레이) · 활성 확장 `property-based-testing.md`(PBT-09 이 단계 강제)·`resiliency-baseline.md`
**전제(NFR 확정 답변, 재오픈 금지)**: Q1=A(CBOR 크레이트 = `ciborium`) · Q2=A(strict 미지 키 = `serde_json` 2-pass 전수 나열) · Q3=A(URL = `url` 크레이트 파싱 + `https` 스킴 단언) · Q4=A(운영 오류 = `thiserror`, 분류 값 타입 = 순수 serde) · Q5=A(Edition 2024 + 고정 MSRV) · Q6=A(`ConfigProvider` = `arc_swap::ArcSwap<Arc<WatcherConfig>>`) · Q7=A(관찰자 팬아웃 = per-observer `catch_unwind` 격리) · Q8=A(순수 표면 전체 panic-free total) · Q9=A(PBT 프레임워크 = `proptest`, PBT-09 충족) · Q10=A(제너레이터 = 비기본 `proptest-support` 피처 노출) · Q11=A(토큰 = redacting newtype) · Q12=A(구조화 검증 메시지, did-you-mean 이월) · Q13=A(커버리지 %-게이트 없음 + PBT + `deny(missing_docs)`)
**이미 확정(FD/Requirements, 재오픈 금지)**: Q1=B(내부=CBOR / config=JSON) · Q2(FD)=A(SafetyLimits 상수) · Q3(FD)=A(keep-last-good/abort) · Q4(FD)=B(strict reject 정책) · Q8(FD)=A(SyncState 모델) · FQ-1/2/3=A · 언어=Rust(NFR-17) · 워크스페이스=단위별 lib + 얇은 watcher-bin 단일 바이너리(UQ-2=B / RESILIENCY-01) · RTO/RPO/DR/availability-SLA = N/A(RESILIENCY-02)

> **문서 성격**: 이 문서는 U0가 소유하는 타입(`domain-entities.md`)·규칙(`business-rules.md`)·흐름(`business-logic-model.md`) **위에 얹는 NFR(품질 속성) 계층**이다. FD의 비즈니스 규칙을 재기술하지 않고 **규칙 ID로 참조**하며, 각 NFR에 근거(아티팩트 + 섹션 + NFR/규칙/PBT ID)와 수용 기준을 붙인다. 구체 크레이트·툴체인 선택의 정본은 자매 산출물 `tech-stack-decisions.md`이며, 이 문서는 각 품질 속성을 **실현하는 메커니즘**으로 그 결정을 참조한다.
>
> **표기 규약**: 기술중립을 지향하되(품질 속성 중심), tech-stack-decisions.md가 명시적으로 위임한 곳에서만 구체 크레이트를 인용한다. 다이어그램은 ASCII 박스 없이 **화살표 표기(A -> B)** 와 표/목록으로 기술한다. English 식별자명(크레이트·타입·config 키·규칙/NFR ID)은 원문 그대로 유지하고, Rust 제네릭/타입(예: `Arc<WatcherConfig>`, `encode<T: Serialize>`)은 백틱으로 감싼다.

---

## 1. NFR 개요 및 U0 특성

U0 `foundation`은 **순수 lib**(값 타입 + CBOR 코덱 + config 로더/검증/리로드 + 관측 계약 트레이트)이며 워크스페이스 빌드 DAG의 **루트**다. 자체 런타임·스레드 풀·처리량 축이 없어 전형적 NFR(확장성/가용성/성능 수치)의 상당수가 **N/A**다(§7). 반대로 U0에서 실현하는 **신뢰성 계약(무손실 코덱·panic-free·원자 스왑·관찰자 격리)** 은 U1~U8 전 단위가 의존하는 기반이므로 이 문서의 핵심이다.

**본 문서가 확정하는 U0 NFR 카탈로그(카테고리별 ID)**:

| 카테고리 | NFR ID | 요약 |
|---|---|---|
| 신뢰성 | U0-NFR-REL-01 | CBOR 코덱 무손실 round-trip |
| 신뢰성 | U0-NFR-REL-02 | 순수 표면 전체 panic-free total |
| 신뢰성 | U0-NFR-REL-03 | `ConfigProvider` 원자 all-or-nothing 스왑 + 동시 읽기 안전 |
| 신뢰성 | U0-NFR-REL-04 | 리로드 관찰자 팬아웃 패닉 격리 |
| 신뢰성 | U0-NFR-REL-05 | keep-last-good / abort(config 회복력) |
| 성능/리소스 | U0-NFR-PERF-01 | 코덱 = 엔트리 수에 선형·유계(정성 계약, 수치 목표 없음) |
| 유지보수성 | U0-NFR-MNT-01 | 오류 타입 파생 전략(thiserror + 순수 serde 분류 타입) |
| 유지보수성 | U0-NFR-MNT-02 | PBT 도메인 제너레이터 재사용 노출 |
| 유지보수성 | U0-NFR-MNT-03 | 커버리지/문서 정책(no %-게이트 + PBT + `deny(missing_docs)`) |
| 보안-잔존 | U0-NFR-SEC-01 | TLS/`https` 강제(유일 잔존 통제) |
| 보안-잔존 | U0-NFR-SEC-02 | 토큰 로그/Debug 유출 위생(redacting newtype) |
| 사용성 | U0-NFR-USE-01 | 구조화 config 검증 오류 메시지(did-you-mean 이월) |

---

## 2. 신뢰성 (Reliability)

U0의 신뢰성은 "**하위 단위가 신뢰할 수 있는 순수 계약**"을 제공하는 것이다 — 무손실 직렬화, 어떤 입력에도 죽지 않는 순수 표면, 절대 부분 적용되지 않는 config 스왑, 관측 실패가 코어를 오염시키지 않는 격리.

### 2.1 U0-NFR-REL-01 — CBOR 코덱 무손실 round-trip

- **요구**: `CoreTypes`의 CBOR 코덱(`encode<T: Serialize>` / `decode<T: DeserializeOwned>`)을 통과하는 **모든** 지속 대상 값 `v`에 대해 `decode(encode(v)) == v`가 성립한다. 자유형 오류 문자열(`detail`)의 유니코드·개행·빈 문자열, `Option` 부재(`None`), 빈 컬렉션, 경계 수치(0·대값)까지 정보 손실 없이 복원된다.
- **근거**: `business-rules.md` §1 R-CODEC-01(구속력 있는 불변식) · `business-logic-model.md` §1.4 · requirements.md §5.5 NFR-13(직렬화 라운드트립 무손실) · US-E7-06 · FQ-3=A 오버레이(requirements §12.2: queue entries -> sync-state 정정). 실현 크레이트 = `ciborium`(Q1=A, 정본 tech-stack-decisions.md) — serde 호환·자기기술적이라 자동 업데이트 후 상태 파일 스키마 진화에 강함.
- **적용 대상**(내부 지속 상태): `Manifest`(+ `ManifestEntry`·`RelativePath`·`Sha256Digest`·`ManifestDigest`)·`Timestamp`·`SyncState`·`ChangeSet`·`ErrorClass`·`TransportError`·`ClassifiedError`·`TransferResult` + 재개 오프셋(`ByteCount`), 및 하위 단위 지속 레코드(`ConsentGrant`[U5]/`UploadHistoryRecord`[U6]는 각 단위가 자기 제너레이터로 재확인).
- **수용 기준**:
  - (AC-1) `RelativePath`·`detail`(유니코드/개행/빈 문자열 경계)·0/경계/대값 `ByteCount`·빈/단일/다수 엔트리 `Manifest`를 포함하는 도메인 제너레이터 기반 PBT no-op(PROP-DE-01 / PROP-BR-01 / PROP-BL-01, 카테고리 Round-trip, PBT-02)가 반례 없이 통과한다.
  - (AC-2) 인코딩/디코딩 실패는 `CodecError`(Fatal 계열, `ErrorClass::Fatal`, `is_retryable()==false`)로 표면화되며 패닉하지 않는다(U0-NFR-REL-02와 결합).
- **연계**: 이 불변식은 U4 `SyncStateStore`의 zero-loss 지속(RPO=0, requirements NFR-03)의 **기반**이다 — 크래시/재시작 후 디스크에서 복구된 상태가 원값과 정확히 동일해야 미반영 편집·재개 오프셋이 유실되지 않는다(§9 Resiliency 참조).

### 2.2 U0-NFR-REL-02 — 순수 표면 전체 panic-free total (Q8=A)

- **요구**: U0 순수 표면(`encode`/`decode`·`RelativePath::normalize`·오류 분류 헬퍼)은 **모든 입력에 대해 패닉 없이 항상 `Result`로 표면화**한다. 특히 `decode`는 **절단·손상·적대적 CBOR 바이트열**을 포함한 임의 `&[u8]` 입력에 대해 `unwrap`/슬라이스 인덱싱/정수 변환 패닉을 내지 않고 `Err(CodecError)`를 반환한다. `normalize`는 무효 경로(`..`·절대경로)에 대해 패닉이 아니라 `Result`의 정규화 오류를 반환한다.
- **근거**: `plans/u0-foundation-nfr-requirements-plan.md` §3 Q8=A · `business-logic-model.md` §1.3(decode 오류 처리) · `domain-entities.md` §1.1(RelativePath 정규화 fallible) · `business-rules.md` §3 R-CLASS-03(`is_retryable` total function) · requirements.md §5.2 NFR-03(zero-loss)·§5.5.
- **선정 근거(왜 명시적 NFR인가)**: `decode`는 U4 크래시 복구의 **핫 경로**에 있다 — 여기서 패닉하면 데몬이 재시작할 때마다 다시 크래시해 keep-last-good/zero-loss 목적을 정면으로 무력화한다. 따라서 "라이브러리 기본에 의존"이 아니라 U0의 **명시적 신뢰성 불변식**으로 승격한다.
- **수용 기준**:
  - (AC-1) `decode<T>`에 대한 PBT no-panic 속성: 임의 바이트열(유효 CBOR·절단·비트플립·랜덤 바이트) 제너레이터로 실행해 어떤 입력도 패닉 없이 `Ok`/`Err`로 종결한다.
  - (AC-2) `RelativePath::normalize`에 대한 PBT no-panic 속성: 혼합 구분자·`..`·절대경로·유니코드·빈 문자열을 포함하는 raw 경로 제너레이터(PROP-DE-03 제너레이터 재사용)로 실행해 패닉 없이 정규화 성공 또는 거부 `Result`를 반환한다.
  - (AC-3) `ErrorClass::is_retryable()`는 모든 변이에 대해 정확히 하나의 boolean을 반환하는 total function(R-CLASS-03, PROP-BR-04, 유한 도메인 전수 검증)이며 미정의·패닉 경로가 없다.
- **비고**: PBT no-panic 속성의 구체 케이스 수·shrinking·고정 시드·CI 통합(PBT-08)은 Code Generation / Build-and-Test 이월. 여기서는 요구·대상·수용 기준을 확정한다.

### 2.3 U0-NFR-REL-03 — `ConfigProvider` 원자 all-or-nothing 스왑 + 동시 읽기 안전 (Q6=A)

- **요구**: `current()`는 데몬의 여러 스레드(U2 `FilesystemWatcher`, U8 `ControlPlane` IPC, 직렬 동기화 사이클)가 **동시에 읽고**, 그 사이 CLI `reload`가 활성 config를 교체한다. 이 교체는 **원자적 all-or-nothing**이어야 한다 — 전체 새 config가 통째로 적용되거나(성공) 전혀 적용되지 않는다(실패). 부분 적용(일부 필드만 반영되는 중간 상태)은 금지하며, 동시 읽기 스레드는 항상 **완결된 이전 스냅샷 또는 완결된 새 스냅샷 하나만** 관측한다.
- **근거**: `business-rules.md` §5 R-RELOAD-01(선검증 후 스왑)·R-RELOAD-02(원자성/all-or-nothing)·R-RELOAD-05(성공 후에만 통지) · `business-logic-model.md` §4.3(reload 흐름) · plans §3 Q6=A. 실현 프리미티브 = `arc_swap::ArcSwap<Arc<WatcherConfig>>`(정본 tech-stack-decisions.md) — 락-프리 읽기(`current()` = 값싼 `Arc` 로드)로 읽기 스레드가 블로킹되지 않고, 리로드는 검증 완료된 새 `Arc<WatcherConfig>`로의 **원자적 포인터 스왑**이라 곧 all-or-nothing을 직접 실현한다.
- **수용 기준**:
  - (AC-1) `reload()`는 §4 규칙(R-CFG-STRICT-01 포함) 전부 검증에 성공한 뒤에만 스왑한다 — 검증은 스왑 이전에 완료된다(load -> parse -> validate -> swap).
  - (AC-2) 동일 유효 config로 `reload()`를 연속 2회 적용한 결과는 1회 적용과 관측적으로 동일하다(멱등, PROP-BR-03 / PROP-BL-04, 카테고리 Idempotence, PBT-04).
  - (AC-3) 스왑 실패 시 어떤 동시 읽기 스레드도 부분/무효 config를 관측하지 못한다(원자 포인터 교체 이전엔 이전 스냅샷만 보임).
- **비고**: 스레드 안전 프리미티브의 정확한 선택 근거·대안(`RwLock`/`Mutex`) 비교는 tech-stack-decisions.md 소유. 여기서는 원자성·동시 읽기 안전이라는 품질 속성과 수용 기준을 확정한다.

### 2.4 U0-NFR-REL-04 — 리로드 관찰자 팬아웃 패닉 격리 (Q7=A)

- **요구**: 성공 스왑 직후(R-OBSERVER-01) `ConfigProvider`는 등록된 관찰자(U5 `CredentialProvider`/`ConsentGate`, U6 `StructuredLogger`)에 결정적으로 팬아웃한다(R-OBSERVER-03). 팬아웃 중 한 관찰자가 **패닉하거나 오류를 반환**해도, **관찰자별로 격리**(`catch_unwind` + 로그)하고 **나머지 관찰자에 계속 팬아웃**한다. 패닉 관찰자가 `current()`나 다른 관찰자를 오염시키지 못한다.
- **근거**: `business-rules.md` §8 R-OBSERVER-01(성공 후 push-only 통지)·R-OBSERVER-03(결정성 + 부수효과 관찰자 측 격리) · `business-logic-model.md` §4.5(팬아웃 흐름) · §5.2(관측 push = best-effort·infallible 계약) · plans §3 Q7=A.
- **선정 근거**: 스왑은 팬아웃 이전에 **이미 원자적으로 커밋**됐으므로(U0-NFR-REL-03), 팬아웃은 되돌릴 수 없는 후행 통지다. 관측은 계약상 best-effort·비치명적이므로, 한 관찰자의 실패로 나머지 관찰자의 통지를 중단하는 것은 결정성(R-OBSERVER-03)과 관측 infallible 계약을 위반한다. 따라서 per-observer 격리 후 계속 진행이 옳다.
- **수용 기준**:
  - (AC-1) 관찰자 K개 중 임의 하나가 패닉해도 나머지 K-1개는 모두 통지받는다(관찰자별 `catch_unwind` 경계).
  - (AC-2) 패닉/오류는 로그로 남고 `current()`가 반환하는 스냅샷은 스왑된 새 값으로 불변 유지된다(롤백 없음).
  - (AC-3) 동일 성공 리로드는 동일 관찰자 집합에 결정적(순서·대상 재현 가능)으로 통지된다.

### 2.5 U0-NFR-REL-05 — keep-last-good / abort (config 회복력, Q3=A 확정)

- **요구(확정 동작의 신뢰성 계약화)**: config 로드/리로드 실패의 회복력 동작은 다음으로 이미 확정됐으며(재오픈 아님), U0 신뢰성 계약으로 명문화한다:
  - **실행 중 `reload()` 실패**(파일 부재/파싱 오류/스키마 위반/알 수 없는 키): **마지막 정상 config(last-good) 유지 + 오류 로그 + 계속 실행**(nginx식 fail-safe). 데몬은 degraded/paused로 진입하지 않는다.
  - **최초 기동 `load()` 실패**: **비정상 종료(non-zero exit)** — 유지할 last-good이 없으므로 keep-last-good이 성립하지 않는다.
- **근거**: `business-rules.md` §5 R-RELOAD-03(keep-last-good)·R-RELOAD-04(abort) · `business-logic-model.md` §4.4(실패 동작 요약) · requirements.md §5.2 NFR-03 · plans §5(Q3=A DECIDED-EARLIER).
- **수용 기준**:
  - (AC-1) 검증 실패 config로 `reload()`하면 `current()`는 직전 성공 스냅샷을 그대로 유지하며 실패는 `Err(ConfigError)`로만 표면화된다(부분 적용 없음, keep-last-good 보존 Invariant, PROP-BL-04).
  - (AC-2) 최초 기동 로드 실패 시 프로세스는 non-zero exit로 종료하고 관찰자 팬아웃은 발생하지 않는다.

---

## 3. 성능 / 리소스 (Performance)

### 3.1 U0-NFR-PERF-01 — 코덱은 엔트리 수에 선형·유계 (정성 계약; 수치 목표 N/A)

- **요구(정성 계약만)**: U0 코덱의 최대 페이로드는 SafetyLimits 상한을 가진 `Manifest`(파일 수 <= 100,000, R-LIMIT-01)이다. `encode`/`decode`의 시간·공간 비용은 **매니페스트 엔트리 수(및 자유형 `detail` 문자열 길이)에 선형이고 유계**이며, 참고 시그니처가 확정한 대로 **전량 인메모리 `Vec<u8>`** 버퍼드 변환이다(스트리밍 코덱 아님 — U4 원자 temp+rename가 완결 파일을 요구하므로 실질 fork 부재, plans §5).
- **명시적 결정 — 수치 목표 없음(근거)**: U0에는 **throughput·latency·peak-memory 수치 게이트를 두지 않는다**. 근거:
  - (a) 코덱 페이로드는 엔트리 수에 선형·유계라 절대적 수치 목표를 정의할 도메인 근거가 없다(입력 크기에 종속).
  - (b) U0는 자체 런타임/스레드/처리량 축이 없는 순수 lib이다.
  - (c) 대규모 스트리밍 해시의 메모리 바운드(requirements NFR-02: 100k 파일 / 20 GiB를 무한 메모리 없이 처리)는 **U1 `ContentAddressing`/`VaultScanner` 소관**이지 U0 값 코덱의 관심사가 아니다.
- **근거**: requirements.md §5.1 NFR-02(스트리밍 해시 메모리 바운드 = U1) · §5.5 NFR-14(SafetyLimits 경계 정확·단조성 — 검사 실행은 U1, U0는 상수·경계 계약만) · `business-rules.md` §4.3 R-LIMIT-01(SafetyLimits 상수 20 GiB / 2 GiB / 100k) · `domain-entities.md` §1.2 Manifest(canonical 정렬 시퀀스) · plans §4.1(Performance numeric = N/A, 정성 계약만).
- **수용 기준**:
  - (AC-1) `encode`/`decode`가 100k 엔트리 근접 축소본 `Manifest`에 대해 무한 재귀·비선형 폭증 없이 완결한다(선형·유계 특성은 PBT round-trip 제너레이터의 대량 엔트리 경계 케이스로 간접 검증, PROP-DE-01).
  - (AC-2) 어떤 절대 처리량/지연/피크메모리 임계도 U0 게이트로 설정되지 않는다(수치 게이트 부재를 명시적으로 문서화).

---

## 4. 유지보수성 (Maintainability)

### 4.1 U0-NFR-MNT-01 — 오류 타입 파생 전략 (Q4=A)

- **요구**: U0가 정의하는 워크스페이스 전역 오류 계약을 두 부류로 구분해 파생한다:
  - **운영 오류(throw 대상, `Result` 반환)**: `CodecError`·`ConfigError`·`CredentialError`에 `thiserror` 파생을 사용해 `Display`/`std::error::Error`를 얻는다 — 라이브러리 관용, 보일러플레이트 최소, 워크스페이스 전역 일관.
  - **분류 값 타입(CBOR round-trip 대상, throw 아님)**: `ErrorClass`·`ClassifiedError`·`TransportError`(+`TransportErrorClass`)는 **순수 serde enum/struct로 유지**한다 — 이들은 U4/U6가 소비하고 CBOR로 round-trip되는 값이다.
- **선정 근거**: `anyhow`식 타입소거는 거부한다 — R-CLASS-01/R-CLASS-02가 의존하는 구조화 변이(`ErrorClass`의 `Retryable`/`AuthAborted`/`Backpressure`/`Fatal`)를 소실시켜 재시도 분류를 불가능하게 만든다. 파운데이션에는 부적합.
- **근거**: `domain-entities.md` §1.4(오류 taxonomy 값 타입) · `business-rules.md` §3 R-CLASS-01/02/03(분류 매핑·total function) · requirements.md §5.7 NFR-17(Rust) · plans §3 Q4=A. 실현 크레이트(`thiserror`) 정본 = tech-stack-decisions.md.
- **수용 기준**:
  - (AC-1) 운영 오류 3종은 `thiserror` 파생으로 `Display`/`Error`를 제공한다.
  - (AC-2) 분류 값 타입은 `serde::Serialize`/`Deserialize`만 파생하고 코덱 round-trip 대상(U0-NFR-REL-01)에 포함되며 CBOR 왕복이 무손실이다(PROP-DE-01 대상 타입에 이미 열거됨).

### 4.2 U0-NFR-MNT-02 — PBT 도메인 제너레이터 재사용 노출 (Q10=A, PBT-07)

- **요구**: U0가 정의한 도메인 타입 제너레이터(`RelativePath`·`Manifest`·`Timestamp`·유효/무효 `WatcherConfig`·자유형 `detail` 등)는 하위 단위(U1 다이제스트 결정성·diff 오라클, U4 SyncState stateful PBT, U5/U6 지속 레코드 round-trip)가 재사용한다. 이를 위해 `foundation`은 제너레이터 모듈을 **비기본 cargo feature**(예: `proptest-support`)로 노출하고, 하위 크레이트는 dev-dependency에서 그 feature를 켜 재사용한다.
- **선정 근거**: 단일 출처 -> 단위 간 제너레이터 정의 드리프트 방지. feature 게이트 -> `proptest`가 비테스트/프로덕션 빌드에 유출되지 않음(non-default). 각 하위 단위가 U0 타입 제너레이터를 재작성하는 방식(일관성 저하)이나 별도 `foundation-testkit` 크레이트(버전 관리 부담)를 배제한다.
- **근거**: `domain-entities.md` §5(PBT-07 제너레이터 총괄)·`business-rules.md` §9·`business-logic-model.md` §6(제너레이터 요구) · property-based-testing.md PBT-07(Generator Quality: 도메인 타입 제너레이터·재사용성) · plans §3 Q10=A. feature 이름·게이팅 정본 = tech-stack-decisions.md.
- **수용 기준**:
  - (AC-1) `foundation`의 제너레이터 모듈은 non-default feature 뒤에 위치하며 기본 빌드에는 `proptest` 의존이 나타나지 않는다.
  - (AC-2) 제너레이터는 문서화된 도메인 제약을 존중한다(예: `RelativePath`는 정규화 규칙 만족, `WatcherConfig`는 §4.1 per-field 규칙 만족, `notify_consecutive_failures >= 1` 경계).

### 4.3 U0-NFR-MNT-03 — 커버리지 / rustdoc 문서 정책 (Q13=A, PBT-10)

- **요구**:
  - **전역 커버리지 %-게이트를 두지 않는다.** 실질 검증은 PBT(§9 속성들) + 예제 앵커(PBT-10: business-critical 경로에 예제 기반 테스트 병행)로 확보한다.
  - **`foundation` 공개 항목에 `#![deny(missing_docs)]`를 강제**한다 — 전 단위가 의존하는 공유 계약 크레이트이므로 공개 API 문서화를 컴파일타임에 강제한다.
- **선정 근거**: 파운데이션은 대부분 순수 값 타입이라 line/branch % 게이트가 과할 수 있다(선언적 타입에 인공적 커버리지 요구). 대신 속성/예제로 실질을 검증하고 공개 계약 문서화만 강제한다.
- **근거**: property-based-testing.md PBT-10(Complementary Testing Strategy: PBT는 예제 기반 테스트를 대체하지 않고 보완) · requirements.md §5.7 NFR-17 · plans §3 Q13=A. 상세 CI 커버리지 통합은 Build-and-Test 이월.

> **포터빌리티(NFR-05) 연계**: 크로스플랫폼 재현 빌드(requirements §5.3 NFR-05, macOS/Windows/Linux)는 U0가 워크스페이스 DAG 루트로서 **Edition 2024 + 고정 MSRV**(Q5=A, 워크스페이스 `Cargo.toml`의 `rust-version` + CI 검증)로 전파해 보장한다. U0는 순수 lib이라 플랫폼별 코드가 없으므로(플랫폼-네이티브 FS-watch/자격증명/업데이트는 U1/U5/U7 소관) 이 항목은 별도 U0 NFR을 신설하지 않고 tech-stack-decisions.md의 edition/MSRV 결정으로 실현된다.
- **수용 기준**:
  - (AC-1) business-critical 경로(코덱 round-trip·config 검증·분류)는 PBT와 예제 기반 테스트를 **모두** 가진다(PBT-10). 어떤 핵심 경로도 PBT 단독 커버리지가 아니다.
  - (AC-2) `foundation` 공개 항목에 문서 주석이 누락되면 `deny(missing_docs)`로 컴파일 실패한다.

---

## 5. 보안-잔존 (Security residual)

**전제**: Security Baseline 확장 = **OFF**(requirements.md §2.3 Q1=B). 암호화 저장·키관리·시크릿 스캐닝 등 강제 통제는 신설하지 않으며 RISK-01(소스 노출)·RISK-02(재승인 thrash)는 **문서화된 수용 위험**이다(requirements §7). 아래 두 항목은 신설 통제가 아니라 (a) 유일 잔존 통제의 실현과 (b) 저비용 방어적 위생일 뿐이다.

### 5.1 U0-NFR-SEC-01 — TLS / `https` 강제 (유일 잔존 통제, NFR-06)

- **요구(신규 통제 아님 — 기존 §4.1 검증 규칙의 실현)**: `server_endpoint`는 유효 URL 형식이어야 하고 **스킴은 `https`만 허용**한다(`http`/무스킴 거부). 이는 이미 확정된 config 검증 reject 규칙(business-rules.md §4.1)이며, 이 문서는 **검증 파서/깊이의 실현 메커니즘**만 정한다: `url` 크레이트로 완전 파싱 후 `scheme() == "https"`를 단언(Q3=A). 파싱된 URL은 U5 `AuthTransport`의 base-URL join에 재사용된다.
- **선정 근거**: 수기 프리픽스 검사(`starts_with("https://")`)는 기형 authority/포트를 통과시켜 실패를 U5 연결 시점으로 미룬다(오류 국소성 저하). WHATWG 준수 파서로 로드 시점에 정밀 거부하고 하위 단위 재파싱을 없앤다.
- **근거**: requirements.md §5.4 NFR-06(모든 전송은 TLS)·§7 RISK-01(잔존 통제 = TLS는 in-transit 가로채기만 완화) · §13 정정(TLS 강제 불변) · `business-rules.md` §4.1(server_endpoint https-only reject) · plans §3 Q3=A. 파서 크레이트(`url`) 정본 = tech-stack-decisions.md.
- **수용 기준**:
  - (AC-1) `http://`·무스킴·기형 authority/포트를 가진 `server_endpoint`는 config 검증 실패로 거부되고, 유효 `https` URL만 수락된다(무효 config 제너레이터 케이스, PROP-BR-05 / PROP-BL-04 무효 config 대상).
  - (AC-2) `https`가 유일 잔존 통제임을 명문화한다 — 토큰은 who-may-upload 통제일 뿐 소스 노출(RISK-01) 완화가 아니며, 그 외 강제 통제는 신설하지 않는다.

### 5.2 U0-NFR-SEC-02 — 토큰 로그/Debug 유출 위생 (Q11=A, RISK-01/02 수용)

- **요구(수용 위험 완화용 저비용 위생)**: `WatcherConfig`의 `token` 필드를 **자체 구현 redacting newtype**으로 감싼다 — `Debug`/`Display`는 실제 값 대신 `***`를 찍고 실제 값은 `.expose()`로만 접근한다. `Serialize`/`Deserialize`는 **실제 값을 보존**해 config round-trip(PROP-BR-02)을 깨뜨리지 않는다.
- **선정 근거**: U6 `StructuredLogger`가 config/오류를 직렬화하거나 `WatcherConfig`가 우발적으로 Debug-print될 때 토큰이 로컬 평문 로그로 새는 것은 RISK-01과 별개의 저비용 위생 문제다. 약 15~20줄·외부 의존 0으로 방어한다. `secrecy` 크레이트는 거부한다(외부 의존 추가 + 기본 Serde 미구현으로 round-trip 글루가 별도 필요).
- **근거**: requirements.md §7 RISK-01(로컬 평문 산출물에 config 토큰 포함)·§13(config 평문 token = RISK-01 수용) · `business-rules.md` §6 R-TOKEN-01/02(토큰 해소·부재 처리) · plans §3 Q11=A. redaction 타입 정본 = tech-stack-decisions.md.
- **수용 기준**:
  - (AC-1) 토큰 newtype의 `Debug`/`Display` 출력에 실제 토큰 값이 나타나지 않고 `***`가 나타난다.
  - (AC-2) `Serialize`/`Deserialize`는 실제 값을 보존해 config 파싱 round-trip(PROP-BR-02, PBT-04)이 무손실 유지된다.
  - (AC-3) 이는 수용 위험(RISK-01/02) 완화용 위생일 뿐 Security Baseline 통제를 켜는 것이 아니다(확장 여전히 OFF).

---

## 6. 사용성 (Usability)

### 6.1 U0-NFR-USE-01 — 구조화 config 검증 오류 메시지 (Q12=A; did-you-mean 이월)

- **요구**: config 파일은 U0의 **유일한 사람-대면 표면**이며 최초 로드 실패는 abort(R-RELOAD-04)로 직결된다. 검증 실패 시 오류 메시지는 **구조화된 형태**로 제공한다 — **필드명 + 기대 형식/범위 + JSON 위치(포인터)**. 알 수 없는 키는 R-CFG-STRICT-01에 따라 **발견된 모든 키를 나열**한다(첫 키에서 중단하지 않음).
- **범위 절제(명시)**: **did-you-mean 편집거리 제안(예: `vault_paht` -> `vault_path`?)은 이월(DEFERRED)** 한다 — 사용자 지시("구현범위가 너무 커지지 않는 한")에 따라 지금은 fuzzy-match 크레이트/Levenshtein을 도입하지 않는다. Code Generation에서 **선택적 후속 강화**로 추가 가능한 OPTIONAL 항목으로 남긴다.
- **선정 근거**: 구조화 메시지는 Q2=A의 2-pass 검증(먼저 `serde_json::Value`로 파싱해 알려진 키 집합 밖 키를 전부 수집·나열)에서 위반 키·위치를 이미 알고 있으므로 near-free로 산출된다. 편집거리 제안만 추가 의존·코드를 요구하므로 절제한다.
- **근거**: `business-rules.md` §4.1(per-field 검증)·§4.2 R-CFG-STRICT-01(전수 나열, 키 이름 + 위치 포함) · `domain-entities.md` §2.1 · plans §3 Q12=A(scope trim). strict 미지 키 실현 메커니즘(serde_json 2-pass) 정본 = tech-stack-decisions.md.
- **수용 기준**:
  - (AC-1) 알 수 없는 키 하나 이상을 포함한 config는 검증 실패로 판정되고, 오류 리포트는 삽입된 **모든** 알 수 없는 키를 이름 + 위치와 함께 나열한다(PROP-BR-05, Invariant; 오타-편집거리 제너레이터로 다수 오타 입력 테스트).
  - (AC-2) per-field 위반(무효 `log_level`·상대 `vault_path`·비-`https` `server_endpoint`·`notify_consecutive_failures < 1`)은 필드명 + 기대 형식/범위 + JSON 위치를 포함한다.
  - (AC-3) did-you-mean 편집거리 제안은 이 단계에서 구현하지 않으며 fuzzy-match 의존이 추가되지 않는다(이월 상태를 명시).
- **그 외 사용성 = N/A**: U0는 UI/CLI/트레이 표면이 없는 lib이므로 config 파일 외 사용성 요구는 없다(§7). 사용자 경험 표면은 U6/U7 소관.

---

## 7. N/A 카테고리 근거표

아래 카테고리는 U0에 요구를 신설하지 않는다(발명 금지). 각 판정은 확정 세트(plans §4.1, requirements RESILIENCY-02/§6)에서 온 것이다.

| 카테고리 | 판정 | 근거 |
|---|---|---|
| **확장성(Scalability)** | N/A | U0는 순수 lib(값 타입 + CBOR 코덱 + config 로더)로 자체 런타임·스레드·처리량 축이 없음. SafetyLimits는 컴파일타임 상수(Q2(FD)=A, R-LIMIT-01). 100k 파일 / 20 GiB 스트리밍 스케일(NFR-02)은 U1 소관. (plans §4.1) |
| **가용성(Availability)** | N/A | lib 크레이트라 SLA 없음. 전체 Watcher는 단일 사용자·사용자 재시작 로컬 프로세스라 RTO/availability-SLA가 requirements RESILIENCY-02(§6)에서 이미 N/A 확정. (requirements §6, plans §4.1) |
| **성능 수치 목표(Performance numeric)** | N/A(정성 계약만) | 코덱 최대 페이로드 = 100k `Manifest`(엔트리 수에 선형·유계, 전량 인메모리 `Vec<u8>`)라 throughput/latency/peak-memory 수치 게이트 근거 없음. 스트리밍 해시 메모리 바운드(NFR-02)는 U1 소관. U0-NFR-PERF-01의 정성 계약만 문서화. (§3.1) |
| **Resiliency DR / RTO / RPO** | N/A(신규 결정 없음) | DR = N/A(단일 리전), RTO/availability = N/A는 requirements RESILIENCY-02에서 답변됨(E: single-region 수용). RPO = zero-loss는 U4 durable state가 전달하며 U0 무손실 코덱(R-CODEC-01)은 그 기반일 뿐. keep-last-good/abort(Q3=A)도 확정(U0-NFR-REL-05). (requirements §6, plans §4.1) |
| **사용성(config 파일 외)** | N/A | U0는 UI/CLI/트레이 표면 없는 lib. 사용자 경험 표면은 U6/U7 소관. config 검증·전수-키 나열(R-CFG-STRICT-01)은 확정, 오류 메시지 풍부도만 U0-NFR-USE-01로 표면화. (§6, plans §4.1) |
| **보안 강제 통제(Security enforced)** | N/A | Security Baseline OFF, RISK-01/02 수용. 암호화 저장·키관리·시크릿 스캐닝 신설은 N/A. 유일 잔존 통제 TLS(NFR-06)는 §4.1 검증 규칙으로 확정(U0-NFR-SEC-01). in-scope 잔존 결정은 토큰 로그-유출 위생(U0-NFR-SEC-02) 하나. (requirements §2.3/§7, plans §4.1) |

---

## 8. NFR to 요구사항 추적표

각 U0 NFR을 소스 NFR ID(requirements §5) + 규칙 ID(business-rules) + PBT 속성/규칙 ID로 매핑한다.

| U0 NFR ID | 요약 | 소스 NFR ID | 규칙 ID | PBT ID |
|---|---|---|---|---|
| U0-NFR-REL-01 | 코덱 무손실 round-trip | NFR-13, US-E7-06 | R-CODEC-01 | PBT-01/PBT-02 (PROP-DE-01 / PROP-BR-01 / PROP-BL-01) |
| U0-NFR-REL-02 | 순수 표면 panic-free total | NFR-13(기반), NFR-03(U4 기반) | R-CODEC-01, R-CLASS-03 | PBT no-panic 속성(PBT-01 파생), PROP-BR-04, PROP-DE-03 |
| U0-NFR-REL-03 | config 원자 all-or-nothing 스왑 | (리로드 원자성) | R-RELOAD-01/02/05 | PBT-04 (PROP-BR-03 / PROP-BL-04, Idempotence) |
| U0-NFR-REL-04 | 관찰자 팬아웃 패닉 격리 | (관측 infallible) | R-OBSERVER-01/03 | PROP-BR-03 결정성 흡수 |
| U0-NFR-REL-05 | keep-last-good / abort | NFR-03 | R-RELOAD-03/04 | PROP-BL-04 (keep-last-good 보존 Invariant) |
| U0-NFR-PERF-01 | 코덱 선형·유계(수치 N/A) | NFR-02(U1 위임), NFR-14(경계 계약, 실행 U1) | R-LIMIT-01(상수) | PROP-DE-01 대량 엔트리 경계(간접) |
| U0-NFR-MNT-01 | 오류 타입 파생 전략 | NFR-17 | R-CLASS-01/02/03 | PROP-BR-04 (is_retryable total) |
| U0-NFR-MNT-02 | PBT 제너레이터 재사용 노출 | NFR-17, NFR-08..14(기반) | (제너레이터 계약) | PBT-07 |
| U0-NFR-MNT-03 | 커버리지/문서 정책 | NFR-17 | (공개 계약) | PBT-10 |
| (포터빌리티 연계) | Edition 2024 + 고정 MSRV로 실현 | NFR-05 | (tech-stack 결정) | N/A(빌드 재현성, 속성 대상 아님) |
| U0-NFR-SEC-01 | TLS/`https` 강제 | NFR-06 | §4.1 (https-only reject) | PROP-BR-05 / PROP-BL-04 (무효 스킴 config) |
| U0-NFR-SEC-02 | 토큰 로그/Debug 유출 위생 | NFR-06(잔존), RISK-01/02 | R-TOKEN-01/02 | PROP-BR-02 (config round-trip 보존) |
| U0-NFR-USE-01 | 구조화 검증 오류 메시지 | (사람-대면 표면) | R-CFG-STRICT-01, §4.1 | PROP-BR-05 (미지 키 나열 + 오타 제너레이터) |

---

## 9. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 단계 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제 충족 (PBT-09 SATISFIED)** | **PBT-09(프레임워크 선택)** = FD에서 이 NFR Requirements 단계로 이월된 blocking 의무 -> **`proptest` 확정(Q9=A)**, `tech-stack-decisions.md`에 문서화 + dev-dependency로 추가(정본 참조). PBT-07(제너레이터 재사용)은 U0-NFR-MNT-02(`proptest-support` 비기본 feature)로 실현. PBT no-panic 속성은 U0-NFR-REL-02(§2.2)로 요구화. PBT-10(예제 병행)은 U0-NFR-MNT-03. 속성 식별(PBT-01)·라운드트립(PBT-02)·멱등(PBT-04)·SyncState stateful(PBT-06, U4 실행 위임)은 FD에서 완료 — 재도출 안 함. shrinking/시드/CI(PBT-08) 상세는 Code Generation / Build-and-Test 이월. **PBT-09가 이 단계에서 확정되었으므로 blocking 없음.** |
| **Resiliency Baseline** | ON | **부분 적용 + 대체로 N/A** | RESILIENCY-01: U0 = **Critical**(전 단위 의존, 빌드 DAG 루트). 무손실 코덱(U0-NFR-REL-01 / R-CODEC-01)이 U4 zero-loss 지속(RPO=0, NFR-03)의 **기반**임을 명문화. keep-last-good/abort(U0-NFR-REL-05, Q3=A)는 config 회복력 규칙. **RTO/RPO 수치·DR 전략·배포/롤백·관측/HA는 U0(순수 lib)에 N/A** — RESILIENCY-02에서 이미 N/A 확정(requirements §6). 신규 U0 Resiliency 결정 없음. |
| **Security Baseline** | OFF | **N/A (잔존만 표면화)** | 미로딩·미강제. RISK-01(소스 노출)·RISK-02(재승인 thrash) 수용. 유일 잔존 통제 = TLS/`https`(U0-NFR-SEC-01, NFR-06) — §4.1 검증 규칙의 파서/깊이 실현일 뿐 신규 통제 아님. 저비용 in-scope 잔존 결정 = 토큰 로그-유출 위생(U0-NFR-SEC-02) 하나. 암호화 저장·키관리·시크릿 스캐닝 신설 없음. |

**블로킹 판정**: PBT-09(프레임워크 선택)가 이 단계에서 `proptest`로 확정되어 `tech-stack-decisions.md`에 문서화되므로, Property-Based Testing 확장의 blocking finding은 없다. Resiliency·Security는 N/A(또는 부분 적용)로 blocking 없음.

---

## 10. 후속 단계 이월 항목 (참고)

- PBT 케이스 수·shrinking·고정 시드·CI 통합(PBT-08) -> Code Generation / Build-and-Test.
- did-you-mean 편집거리 제안(U0-NFR-USE-01의 이월분) -> Code Generation 선택적 후속 강화(OPTIONAL).
- 상세 CI 커버리지 통합(U0-NFR-MNT-03) -> Build-and-Test.
- 구체 크레이트/툴체인 결정(CBOR=`ciborium`·JSON strict=`serde_json` 2-pass·URL=`url`·오류=`thiserror`·edition 2024 + MSRV·스왑=`arc_swap`·PBT=`proptest`·제너레이터 feature·토큰 redaction 타입)의 정본 -> 자매 산출물 `tech-stack-decisions.md`.

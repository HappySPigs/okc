# U2 Change Detect — Tech Stack Decisions (기술 스택 결정)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U2 `change-detect`** -> NFR Requirements -> 산출물 2/2 (`tech-stack-decisions.md`)
**작성일**: 2026-09-08
**크레이트**: `change-detect` (lib) · **소속 컴포넌트**: `FilesystemWatcher`, `ReconciliationScheduler`, `VaultAvailabilityGuard`
**입력 아티팩트**: `domain-entities.md`·`business-rules.md`·`business-logic-model.md`(U2 FD) · `u2-change-detect-functional-design-plan.md` §3(D1~D14) · `requirements.md`(§5 NFR-01/03/05·§9 concrete defaults) · `u0-foundation/nfr-requirements/tech-stack-decisions.md`(워크스페이스 스택 상속 정본) · 활성 확장 `property-based-testing.md`(PBT-07/09)·`resiliency-baseline.md`
**모드**: AUTOPILOT (게이트 면제) — 열린 점은 **권장안 + MVP 편향**, 최소 well-established 크레이트만 채택, 각 신규 크레이트 정당화. 이미 확정된 워크스페이스 스택(U0)은 재선택하지 않고 상속한다.
**규칙**: `construction/nfr-requirements.md` · `common/content-validation.md`(유니코드 박스/화살표 문자 미사용, 화살표 표기 `A -> B`, 상한 `<=`, Rust 타입 백틱, 표/코드블록 검증)

> **문서 성격**: 이 문서는 U2의 **구체 크레이트/툴체인 선택을 기록하는 유일한 산출물**이다. 자매 산출물 `nfr-requirements.md`가 카테고리별 NFR(기술중립)을 정의하고, 이 문서는 그 NFR을 실현할 구체 기술 결정과 근거·전파 범위를 확정한다. U2는 워크스페이스 DAG 루트 U0의 스택(언어·edition·MSRV·`serde`·`ciborium`·`serde_json`·`url`·`thiserror`·`arc_swap`·`proptest`)을 **상속**하며, 이 문서는 **U2 고유의 추가 결정**(FS-watch 백엔드·타이머·기본값·PBT 노출·config 주입)만 확정한다.

---

## 0. 전파 원칙 및 상속 (U2 = Wave-2 소비 크레이트)

U2 `change-detect`는 U0 `foundation`에만 의존하는(R-PURE-02) Wave-2 lib이다. 언어/edition/MSRV/공용 크레이트는 U0가 워크스페이스 `[workspace.package]`·`[workspace.dependencies]`로 확정한 값을 **상속**한다.

| 상속 항목 | 상속 값(U0 정본) | U2 적용 |
|---|---|---|
| 언어 | Rust (NFR-17) | 그대로 |
| Edition | 2024 | `edition.workspace = true` |
| MSRV | 고정 1.85 | `rust-version.workspace = true` |
| 오류 파생 | 운영 오류 = `thiserror` | `WatchError`를 `thiserror`로 파생(운영 오류 관례) |
| PBT 프레임워크 | `proptest`(PBT-09 확정) | dev-dependency 상속(재선택 아님) |
| 직렬화 | `serde`/`ciborium`/`serde_json` | **U2 미사용**(U2는 아무 값도 지속하지 않음 — 무지속, `business-logic-model.md` §5) |

- **U2가 지속/전송하지 않음(중요)**: U2는 상태를 디스크에 지속하지 않고 네트워크도 쓰지 않는다. 따라서 `ciborium`(CBOR round-trip)·`serde_json`·`url`은 **U2 런타임 의존이 아니다**. `TriggerSignal`/`GuardVerdict`/`ReconResult`는 round-trip 대상이 아니다(`domain-entities.md` §7.1).
- **전파 표기 규약**: 각 결정 절 말미에 전파 범위를 명시한다(`U2 내부 한정` / 상위 배선 U8 / 상속).
- **버전 정책**: 신규 외부 크레이트는 `[workspace.dependencies]`에서 major/minor로 핀하고 U2가 `workspace = true`로 상속한다. **정확한 patch 핀 + MSRV(Edition 2024 / Rust 1.85) 대비 CI 검증은 Build-and-Test 이월**(U0와 동일 정책).

---

## 1. AUTOPILOT 결정표 (질문 대체 — 권장안 + MVP 편향)

| # | 주제 | 채택안 | MVP 트림? | 근거 + 출처 |
|---|---|---|---|---|
| T1 | 크로스플랫폼 FS 감시 백엔드 | **`notify` 크레이트**(FSEvents/inotify/ReadDirectoryChangesW 추상) 뒤에 `WatchBackend` 트레이트를 얹어 어댑터화 | 아니오 | NFR-05를 트레이트 경계로 충족하는 MVP 표준 픽. 3-OS 네이티브를 직접 FFI하지 않고 검증된 크레이트에 위임. (NFR-05, plans D3, hint) |
| T2 | 디바운스 구현 | **hand-rolled**(std `Instant`/`Duration` + 채널 `recv_timeout` 정적-구간 타이머). `notify-debouncer-*` 미채택 | 예 | U2 디바운스 의미(볼트-전체 단일 타이머·버스트당 1·오버플로 정규화)가 단순·고유해 추가 크레이트 불필요. 순수 로직으로 백엔드 목 치환 PBT 가능(PROP-U2-01~05). (D1/D6, PBT) |
| T3 | 타이머·스케줄 | **std `Instant`/`Duration` + 데몬 스케줄러(상위)**. 비동기 런타임(tokio 등) 미도입 | 예 | 재조정 `tick(now)`는 단조 시점 주입 순수 함수(D7). async 런타임은 스코프 폭증·무이득. (`business-logic-model.md` §2/§5) |
| T4 | `TriggerStream` 구체 전송 | **std `std::sync::mpsc`**(순서 보존·단방향·단일 소비자). async stream 미채택 | 예 | D14 계약(순서 보존 + 단방향 + 코디네이터 유일 소비자)을 std로 충족, 신규 크레이트 0. (D14, `domain-entities.md` §1.3) |
| T5 | T_debounce 기본값 | **2000ms**(`debounce_ms`, 양의 정수 > 0, 고정 duration) | 예 | requirements §9 concrete defaults를 이 단계에서 확정. 적응형 없음(D4). 후보값 2000ms 채택. (§9, D4, `domain-entities.md` §5) |
| T6 | T_recon 기본값 | **900s = 15분**(`reconciliation_interval_s`, 양의 정수 >= 1, 고정 duration) | 예 | §9 concrete defaults 확정. 백스톱 상한(NFR-03) 기본값. 후보 900s 채택. (§9, D4, NFR-03) |
| T7 | degraded 백엔드 폴백 | **`DegradedBackend`(무이벤트) 허용** — 네이티브 어댑터가 MVP 초과 시 폴백, 정확성은 recon 상한 보장 | 예 | 네이티브 3-OS 완비를 Code Generation으로 미룰 수 있게 함. 정확성 바닥 = <= T_recon(U2-NFR-REL-02). (D3, NFR-05) |
| T8 | config 값 취득 | **U8 조립 루트가 해소한 타입드 값(`Duration`/`bool`) 생성자 주입**. U2는 `ConfigProvider` 미읽기. 라이브리로드 non-core = MVP 제외 | 예 | wave-level federated 해소(DEC-FEDERATED-KEYS Q6=A) U2 적용. U0 FROZEN 유지. (federated 해소, D4/D10) |
| T9 | PBT 제너레이터 노출 | **`change-detect`의 비기본 feature `proptest-support`**(U0 미러) + U0 제너레이터 재사용 | 아니오 | PBT-07 재사용성·단일 출처. proptest 프로덕션 유출 방지. (PBT-07, U0 §7.2) |
| T10 | `WatchError` 파생 | **`thiserror`**(운영 오류 관례 상속) | 아니오 | U0 Q4=A 관례. `source`(진단 상세)는 자유형·지속 안 함(로그 전용). (U0 §5, `domain-entities.md` §2.2) |

---

## 2. 시간 기본값 (T_debounce / T_recon) — requirements §9 concrete defaults 확정 (T5/T6)

requirements §9는 "Concrete defaults: debounce quiet-period T_debounce, reconciliation interval T_recon ... -> Functional / NFR Design"으로 이 단계 확정을 위임했다. FD D4(고정 duration·적응형 없음)에 따라 다음 기본값을 확정한다.

| config 키 | 개념 값 | 기본값 | 검증 규칙 | 근거 |
|---|---|---|---|---|
| `debounce_ms` (T_debounce) | `Duration` | **2000ms** | 양의 정수(> 0); 0/음수/비정수 = 시작 검증 실패(US-E1-01 AC) | 감지 지연 상한(NFR-01, R-DEB-02). 2s = 편집 버스트 흡수와 반응성의 MVP 균형 |
| `reconciliation_interval_s` (T_recon) | `Duration` | **900s (15분)** | 양의 정수(>= 1초); 위반 = 시작 검증 실패 | 미관측 변경 검출 지연 상한(NFR-03, R-RECON-03). 15분 = 백스톱 커버리지와 스캔 비용의 MVP 균형 |
| `confirm_empty` | `bool` | **false** | 불리언; 비불리언 = 검증 실패 | 파괴적-빈-커밋 가드(US-E1-06, R-GUARD-05/D10). 기본 안전(가드 활성) |

- **성격**: 세 값은 **고정 duration/플래그**이며 적응형 튜닝이 없다(D4). U2는 이를 `Duration`/`bool`로 소비하며 config를 지속·변경하지 않는다(read-only, U2-NFR-MNT-02).
- **전파 범위**: 기본값·검증 규칙은 **U2 config 뷰 소유**(`WatchConfig`/`ReconConfig`/`VaultConfig`, `domain-entities.md` §5). 실제 raw 파싱·검증·해소는 U0 strict 계층 + U8 조립 루트(§5).

---

## 3. 크로스플랫폼 FS 감시 = `notify` 뒤 `WatchBackend` 어댑터 (T1/T2/T3/T4/T7 — NFR-05)

`FilesystemWatcher`는 볼트 루트의 create/modify/delete/rename를 플랫폼 네이티브로 감시해야 한다(NFR-05: FSEvents/ReadDirectoryChangesW/inotify). 3-OS 네이티브 API를 직접 FFI하는 대신 검증된 추상 크레이트에 위임하고, 그 위에 U2 고유 `WatchBackend` 트레이트를 얹어 `RawFsEvent` 어휘로 정규화한다(`domain-entities.md` §2.3).

### 3.1 백엔드 크레이트 = `notify` (T1)

**결정**: 크로스플랫폼 파일시스템 감시 크레이트로 **`notify`** 를 채택한다.

**후보 비교(T1 근거):**

| 크레이트 | 3-OS 네이티브 | 유지보수 | 시그니처 적합성 | 판정 |
|---|---|---|---|---|
| **`notify`** | 예(FSEvents/inotify/ReadDirectoryChangesW 통합 백엔드) | 활발히 유지, 사실상 표준 | 이벤트 콜백/채널을 `WatchBackend` 어댑터로 `RawFsEvent`에 매핑 용이 | **채택** |
| 직접 FFI(`fsevent-sys`/`inotify`/winapi) | 예(개별) | OS별 개별 유지 | 3배 표면·플랫폼 조건부 컴파일 폭증 | 기각(스코프 폭증, MVP 초과) |
| polling 전용(직접 구현) | N/A(비네이티브) | — | NFR-05 "platform-native" 위반 | 기각(단, degraded 폴백으로만 T7) |

**채택 근거(T1)**: (a) `notify`가 세 OS 네이티브 백엔드를 단일 API로 통합해 NFR-05를 최소 코드로 충족한다. (b) Rust 생태계 사실상 표준으로 활발히 유지된다. (c) U2는 `notify`를 **직접 노출하지 않고** `WatchBackend` 트레이트 뒤에 감춰(어댑터 패턴) 상위 디바운스 로직을 백엔드 무관하게 유지한다 — 이것이 PROP-U2-05(백엔드 무관 등가) 테스트 가능성과 degraded 폴백(T7)의 전제다.

**MVP 트림(T7)**: 완전 네이티브 어댑터(rename 정규화·오버플로 매핑 세부 완비)가 MVP 초과이면 `DegradedBackend`(무이벤트) 폴백을 허용한다 — 이 경우 정확성 바닥은 재조정 백스톱(<= T_recon, U2-NFR-REL-02)이 보장한다. 3-OS 네이티브 어댑터 완비는 Code Generation 강화 대상.

### 3.2 디바운스·타이머·트리거 전송 = std (T2/T3/T4)

**결정**: 디바운스/스케줄/전송에 **추가 크레이트를 도입하지 않고 std로 실현**한다.

- **디바운스(T2)**: `notify-debouncer-full`/`notify-debouncer-mini` 등 **디바운서 크레이트 미채택**. 볼트-전체 단일 정적-구간 타이머(D1)는 std `Instant`/`Duration` + 채널 `recv_timeout(T_debounce)` 루프로 구현한다 — 이벤트 도착마다 타이머 리셋, 무이벤트 `T_debounce` 경과 시 트리거 1개(R-DEB-01). `Overflow`도 합성 change로 동일 투입(R-DEB-04). U2 디바운스 의미가 단순·고유해(파일별 디바운스·rename 추적 없음 — D2) 외부 디바운서보다 hand-rolled가 작고 순수하다.
- **타이머·스케줄(T3)**: 재조정 `tick(now)`는 **단조 `Instant`를 인자로 받는 순수 함수**이며(D7, `next_due` 발행 시각 앵커) 자체 타이머를 갖지 않는다 — 주기 구동은 데몬 스케줄러(상위)가 단조 시점을 공급한다. **비동기 런타임(tokio/async-std) 미도입** — 이 lib에 async 축이 없다.
- **트리거 전송(T4)**: `TriggerStream`은 **`std::sync::mpsc`**(순서 보존·단방향·단일 소비자 U8)로 실현한다 — D14 계약을 std로 충족, 신규 크레이트 0. async stream은 무이득.

**전파 범위**: `notify` 의존과 `WatchBackend` 어댑터는 **U2 내부 한정**. `TriggerStream` 수신단은 **U8 `SyncCycleCoordinator`**(유일 소비자). 상위 데몬 스케줄러(단조 시점 공급·직렬화 잠금)는 U8 소유.

---

## 4. 속성 기반 테스트 = `proptest` 상속 + `proptest-support` 노출 (T9 — PBT-07/09)

PBT-09(프레임워크)는 U0에서 **`proptest`** 로 확정됐다(U0 `tech-stack-decisions.md` §7). U2는 이를 **재선택하지 않고 워크스페이스 dev-dependency로 상속**한다.

### 4.1 제너레이터 노출 = `change-detect`의 비기본 feature `proptest-support` (T9)

**결정(U0 미러)**: `change-detect`에 **비기본 cargo feature `proptest-support`** 로 U2 도메인 제너레이터 모듈을 노출한다. 대상 제너레이터:

| 제너레이터 | 대상 속성 | 형상 |
|---|---|---|
| 이벤트 도착 타임라인 | PROP-U2-01/02/03/05 | `RawFsEvent` 시퀀스(단일/밀집 버스트/경계 간격/다중 버스트/`Overflow` 삽입) |
| 스케줄러 명령 시퀀스 | PROP-U2-04/06 | `(now 증가, busy/idle 전이, record_result)` 명령열(긴 busy·조밀 tick·경계 now==next_due) |
| 가드 입력 조합 | PROP-U2-07/08 | `Availability` x 매니페스트-공허성 x `last_committed`(None/빈/비빈) x `confirm_empty` 유한 조합 |
| `WatchBackend` 목 | PROP-U2-05, PROP-DE-U2-02 | 동일 타임라인을 여러 백엔드가 방출하는 목 집합 |

- **U0 제너레이터 재사용**: `Manifest`/`RelativePath`/`Timestamp` 제너레이터는 `foundation`의 `proptest-support` feature를 dev에서 켜 **재사용**한다(재작성 금지, 단일 출처 — U2-NFR-MNT-01).
- **feature 게이트**: 비기본 feature이므로 `proptest`가 U2 프로덕션 빌드에 유출되지 않는다(`proptest`는 `proptest-support` 하의 optional dev-dependency).

### 4.2 이월(PBT-08)

케이스 수·shrink 튜닝·고정 시드 vs 시드 로깅·CI 통합(PBT-08)은 **Code Generation / Build-and-Test 이월**(U0와 동일). 이 문서는 프레임워크 상속(PBT-09)과 제너레이터 노출(PBT-07)만 확정한다.

**전파 범위**: `proptest` = 워크스페이스 공용 dev-dependency(상속). `change-detect`의 `proptest-support` feature는 U2 테스트 및 (있다면) U2 제너레이터를 재사용할 상위 통합 테스트가 활성화.

---

## 5. config 취득 = U8 조립 루트의 타입드 값 생성자 주입 (T8 — federated 해소)

wave-level federated-config 해소(DEC-FEDERATED-KEYS Q6=A)를 U2에 적용한다. U2 비코어 값은 U0 `ConfigSnapshot`(코어 6필드 전용)에서 읽지 않는다.

**결정(T8)**:
- U8 조립 루트(`watcher-bin`)가 raw config 파일을 파싱해 U2 값(`debounce_ms`/`reconciliation_interval_s`/`confirm_empty`)을 검증·해소하고, **타입드 값(`Duration` T_debounce·`Duration` T_recon·`bool` confirm_empty)을 U2 세 컴포넌트 생성자에 하향 주입**한다.
- U2는 `ConfigProvider`/`ConfigSnapshot`을 직접 읽지 않으며 config를 지속·변경하지 않는다(read-only 소비, U2-NFR-MNT-02).
- U2는 자기 섹션 키(`debounce_ms`/`reconciliation_interval_s`/`confirm_empty`)를 U0 federated known-key-set에 등록한다(U0 `tech-stack-decisions.md` §3.2 계약).

**MVP 트림(라이브리로드 non-core 제외)**: 이 세 값의 **라이브 리로드는 MVP 범위 밖**이다 — 변경은 **재시작으로 반영**한다. U0 코어 필드(토큰 등) 리로드만 U0 관찰자 팬아웃으로 전파되며, U2 비코어 값은 생성자 주입 시점에 고정된다. U2는 non-core 값에 대해 `ConfigReloadObserver`를 구독하지 않는다.

**전파 범위**: 값 해소·주입은 **U8 조립 루트 소유**. U2는 타입드 값 수신자. 키 등록 계약은 U0 federated known-key-set와 연동.

---

## 6. 오류 처리 = `WatchError` via `thiserror` (T10)

**결정(U0 Q4=A 관례 상속)**: `WatchError`(감시 시작/백엔드 초기화 실패 분류: `Unsupported`/`RootMissing`/`OsWatchInit(source)`/`Backend(source)`)는 운영 오류이므로 **`thiserror`** 로 파생한다(`Display`/`std::error::Error`). `source`(`BackendCause`, 진단 상세)는 자유형이며 **지속하지 않는다**(로그 전용, `domain-entities.md` §2.2) — CBOR round-trip 대상이 아니다.

- **분류 값 타입 아님**: `WatchError`는 vault-availability 판정(`Availability`/`GuardVerdict`)과 구분되는 감시 계층 오류다. `Availability`/`GuardVerdict`/`CycleOutcome`은 판정 값(순수 enum)이며 throw되지 않고 반환된다 — thiserror 미적용.
- **전파 범위**: `WatchError`는 **U2 내부**. `thiserror` 의존은 워크스페이스 공용(상속).

---

## 7. 의존성 요약표

**범례**: version-policy = 신규 외부 크레이트는 `[workspace.dependencies]` 핀 후 U2가 `workspace = true`로 상속. kind = normal(런타임) / dev(테스트). feature = 특기 사항. propagation = 전파 범위.

| crate | version-policy | kind | feature | propagation | grounding |
|---|---|---|---|---|---|
| **`notify`** | workspace-pinned (핀: `6`) **[신규]** | normal | 기본(플랫폼 백엔드 자동) | **U2 내부 한정**(`WatchBackend` 어댑터 뒤) | T1 · NFR-05 · `domain-entities.md` §2.3 |
| `foundation` | 내부 path 의존, `publish=false` | normal | (dev) `proptest-support` | U0 -> U2(참조만, R-PURE-02) | CoreTypes(`Manifest`/`RelativePath`/`Timestamp`) 소비 |
| `thiserror` | workspace-inherited (핀: `2`) | normal | — | 상속(운영 오류 파생) | T10 · U0 §5(Q4=A) |
| `proptest` | workspace-inherited (핀: `1`) | **dev** | `proptest-support`(비기본, U2 제너레이터 노출) | 상속 + U2 제너레이터 재사용 | T9 · PBT-07/09 · U0 §7 |

> **미채택(의도적)**: `ciborium`/`serde_json`/`url`(U2 무지속·무전송) · `notify-debouncer-*`(hand-rolled 디바운스 T2) · tokio/async-std(비동기 런타임 없음 T3) · 직접 OS FFI 크레이트(스코프 폭증 T1). std `std::sync::mpsc`로 `TriggerStream` 전송(T4) — 크레이트 0.

> **버전 핀 주석**: `notify`는 major 계열 `6`으로 `[workspace.dependencies]`에 기재하며, **정확한 patch 핀과 MSRV(Edition 2024 / Rust 1.85+) 정합 검증은 Build-and-Test에서 수행**한다(U0와 동일 정책). `notify`의 플랫폼 백엔드(FSEvents/inotify/RDCW)가 3-OS MSRV에서 빌드됨을 CI가 확인한다(이월).

---

## 8. 범위 절제 기록 (Scope Trims / Deferrals)

| 절제 항목 | 결정 | 근거 / 이월처 |
|---|---|---|
| `notify-debouncer-*` 크레이트 | **미채택(hand-rolled 디바운스)** | U2 디바운스 의미(볼트-전체 단일 타이머·버스트당 1·오버플로 정규화)가 단순·고유. std `recv_timeout`로 충분(T2). 순수 로직으로 PBT 백엔드 목 치환 가능. |
| 비동기 런타임(tokio 등) | **미도입** | tick(now)는 단조 시점 주입 순수 함수, 디바운스는 std 타이머. async 축 없음(T3). |
| 3-OS 네이티브 어댑터 완비 | **MVP는 degraded 폴백 허용** | 정확성 바닥 = recon <= T_recon(U2-NFR-REL-02). 네이티브 완비는 Code Generation 강화(T7/D3). |
| non-core config 라이브 리로드 | **MVP 제외(재시작 반영)** | federated 값은 생성자 주입 시 고정. 코어 필드 리로드만 U0 팬아웃(T8). |
| U2 상태 CBOR 지속 | **없음(무지속)** | U2는 아무 상태도 디스크에 지속하지 않음(`business-logic-model.md` §5). `ciborium`/`serde_json` 미의존. |
| patch 핀 + MSRV CI 검증 | **이월** | 정책만 확정(§7). 구체 CI 통합은 Build-and-Test(U0와 동일). |
| PBT-08 상세(케이스 수/shrink/시드/CI) | **이월** | Code Generation / Build-and-Test(§4.2). 이 문서는 PBT-09 상속 + PBT-07 노출만 확정. |

---

## 9. 확장 컴플라이언스 요약 (완료 게이트용)

| 확장 | 활성 | 이 문서 적용 판정 | 근거 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제·준수 — PBT-09 상속 충족** | §4가 PBT-09 프레임워크(`proptest`)를 U0에서 상속(dev-dependency)하고 PBT-07 제너레이터를 `proptest-support` 비기본 feature로 노출(U0 미러). PBT-08 상세는 Build-and-Test 이월(명시). 재선택 아님 -> blocking 없음. |
| **Resiliency Baseline** | ON | **부분 적용** | `notify` 기반 크로스 OS 감시(§3, NFR-05) + recon 백스톱(std 타이머, T_recon 기본 900s §2) + degraded 폴백(T7)이 RESILIENCY-02 무손실 검출 지연 상한을 실현. 데이터 무결성 가드는 `nfr-requirements.md` §2.4. RTO/RPO 수치·DR·HA·배포/롤백은 U2(순수 lib)에 **N/A**(RESILIENCY-02). |
| **Security Baseline** | OFF | **N/A** | 미로딩·미강제. U2는 시크릿·토큰·네트워크 미취급 — 보안 크레이트(TLS·시크릿 저장) 선택 표면 없음. `notify`는 로컬 FS 감시만. RISK-01/02 수용. |

**N/A NFR 카테고리(기술 선택 없음)**: 확장성 · 가용성 · 성능 수치 목표(감지 지연 = T_debounce 상한만, §2) · Resiliency DR/RTO/RPO · config 외 사용성 · 보안 통제. 근거 상세는 자매 산출물 `nfr-requirements.md` §7 N/A 판정표.

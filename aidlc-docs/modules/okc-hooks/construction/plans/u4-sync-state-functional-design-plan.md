# U4 sync-state — Functional Design 계획 및 결정 (AUTOPILOT)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U4 Resilience & Retry** -> Functional Design (Part: 계획 + 결정)
**작성일**: 2026-09-08
**크레이트**: `sync-state` (lib) · **소속 컴포넌트**: `SyncStateStore`, `RetryBackoffController`
**입력 아티팩트**: `unit-of-work.md`(§U4 책임·얇은-단위 근거), `component-methods.md`(§U4 시그니처·이월 항목), `components.md`(§U4 컴포넌트), `unit-of-work-story-map.md`(U4 소유 스토리 행), `stories.md`(US-E3-01..04, US-E7-04/08/09/11), `requirements.md`(FR-03/11/12/19/23, NFR-03/04/11/13, RESILIENCY-14, RISK-01, §12.2 FQ-2 오버레이), `crates/foundation/src/**`(U0 실제 타입/코덱), U0 Functional Design 3종(스타일·rule-id/property-id 템플릿)
**규칙**: `construction/functional-design.md` · `common/content-validation.md` · 활성 확장 `property-based-testing.md`(PBT-01 Full 강제) · `resiliency-baseline.md`
**모드**: **AUTOPILOT** — 사용자 게이트 면제(2026-09-08 승인). 모든 미결정 항목은 권장안 + MVP 편향으로 저자가 확정하며, 질문을 제기하지 않는다(§3 결정 표가 질문을 대체).

---

## 1. 단위 컨텍스트 (Step 1 — 요약)

U4는 단일 **"장애 복구(failure recovery)"** 테마로 응집된 얇은 단위다. 의존은 **U0 `foundation` 하나뿐**(DAG W2 웨이브, `unit-of-work-dependency.md` §4)이며, `CoreTypes` 값 타입·오류 taxonomy·무손실 코덱과 `ConfigProvider`만 소비한다. 소유 스토리: US-E3-01(지속 상태), US-E3-02(coalesce/최신 스냅샷 합치기 — 별도 coalesce 로직 없이 최신 상태 대체 모델로 실현: PROP-U4-04 latest-state-wins + business-logic-model §3.3 재스냅샷 coalesce-to-latest), US-E3-03(백오프 재시도), US-E3-04(오프라인 판정), US-E7-04(latest-state-wins 불변식), US-E7-08(상태머신 PBT), US-E7-09(zero-loss), US-E7-11(회복탄력성 전략).

### 1.1 `SyncStateStore` — 이 단위가 정의할 대상
- **최신 상태 대체(FQ-2=A) 지속 상태**를 소유: 마지막 커밋 `Manifest` 1개 + dirty boolean + 진행 중 업로드 blob별 재개 오프셋. **이벤트별 지속 큐가 아니다.**
- 사이클 시작 시 마지막 커밋 매니페스트를 diff 기준선으로 읽어 준다(`ManifestDiffer`[U1]에 인자로 전달).
- 모든 쓰기 **crash-atomic**(temp+rename): 쓰기 도중 크래시가 상태를 손상시키지 않는다(NFR-03).
- 로드 시 부분 기록을 마지막 정상 상태(last-good)로 롤백; 직렬화는 U0 무손실 코덱(NFR-13).
- API(component-methods §U4): `open_and_recover(cfg)`, `last_committed_manifest()`, `is_dirty()`, `mark_dirty()`, `commit_manifest(m)`, `resume_offset(blob)`, `persist_resume_offset(blob, off)`, `clear_resume_offsets()`.

### 1.2 `RetryBackoffController` — 이 단위가 정의할 대상
- U0 오류 taxonomy(`TransportErrorClass`)로 전송 결과를 `RetryClass`(Transient/Offline/Backpressure/AuthFailed/Permanent)로 분류.
- 실패 시 지수 백오프 + 지터 + 상한으로 다음 재시도 시각 계산; 오프라인을 별도 서버 왕복 없이 판정(US-E3-04).
- 401/AuthFailed는 재시도 제외 -> U5 인증 흐름 위임; 연속 실패 카운트 -> FR-19 케이스 2 에스컬레이션 **신호**(실제 알림은 U6).
- API(component-methods §U4): `new(cfg)`, `classify(err)`, `on_failure(err, now)`, `on_success()`, `next_retry_at()`, `consecutive_failures()`.

### 1.3 교차 단위 경계(소유 vs 인자/위임)
- `Manifest`/`ChangeSet`/`SyncState`/오류 taxonomy/코덱 = **U0 소유**(U4는 소비·지속만).
- diff 계산 = **U1 `ManifestDiffer`**(U4가 last-committed를 인자로 전달, U4는 U1 미의존).
- 실제 blob 전송·재검증·커밋 = **U3 `UploadProtocolDriver`**(U4는 재개 오프셋 읽기/쓰기만 제공).
- HTTP/토큰/오류 산출 = **U5 `AuthTransport`**(U4는 산출된 `TransportError`를 분류만).
- 상태 push·알림·히스토리 = **U6**(U4는 escalate 신호만 반환, push는 코디네이터/U8).
- 사이클 오케스트레이션·단일-사이클 잠금·SingleInstanceLock = **U8/U2**(단일 writer 보장).

---

## 2. Functional Design 실행 계획 (산출물 체크박스)

아래를 `aidlc-docs/construction/u4-sync-state/functional-design/` 에 생성한다. 기술중립(Rust스러운 시그니처는 참고용, 인프라·스레딩·I/O 메커니즘 배제). **프런트엔드 파일 없음**(U4는 UI 없는 순수 백엔드 단위).

- [x] **`domain-entities.md`** — U4가 도입/특화하는 값 타입: `PersistedState`(마지막 커밋 매니페스트 + dirty + 재개 오프셋 맵의 지속 애그리게이트), `ResumeOffsetMap`(`BTreeMap<Sha256Digest, ByteCount>`), `StateConfig`·`BackoffConfig`(config 투영), `RetryClass`·`RetryDecision`·`StateError`. U0 타입(`Manifest`/`SyncState`/`TransportError`/`TransferResult`)은 **이름으로 참조만**, 재정의 금지. -> 컴포넌트: `SyncStateStore`(PersistedState/ResumeOffsetMap/StateConfig/StateError), `RetryBackoffController`(RetryClass/RetryDecision/BackoffConfig)
- [x] **`business-rules.md`** — R-* 규칙: crash-atomic 쓰기(temp+rename+fsync 순서), 부분쓰기 롤백/last-good 복구, 재개 오프셋 keying·클리어 시점, dirty 라이프사이클, `classify` 매핑 표, 백오프 공식·상한·지터, 오프라인 판정, AuthFailed 제외, 에스컬레이션 게이트(오프라인/백프레셔 제외). + PROP-* Testable Properties(NFR-03/13, PBT-06). -> 규칙: SyncStateStore 지속·복구 규칙 + RetryBackoffController 분류·백오프·에스컬레이션 규칙
- [x] **`business-logic-model.md`** — 알고리즘·워크플로·데이터 흐름: crash-atomic persist 파이프라인, open_and_recover 복구 시퀀스, 사이클 시작 시 last-committed 제공 흐름, 재개 오프셋 지속/클리어, 백오프 상태머신 전이, classify->decision 흐름. + 교차 단위 경계(인자 전달 vs 소유) 명시 + 컴포넌트별 Testable-Properties 노트 + 확장 컴플라이언스 요약. -> 양 컴포넌트 + scan->build->diff->limits 합성에서 U4의 위치
- [x] **PBT-01 "Testable Properties" 섹션**(각 산출물 내, 확장 강제): PROP-U4-01(상태머신 model-based, PBT-06/US-E7-08) · PROP-U4-02(crash-atomic zero-loss, NFR-03/US-E7-09) · PROP-U4-03(PersistedState 무손실 round-trip, NFR-13) · PROP-U4-04(latest-state-wins 불변식, NFR-11/US-E7-04) · PROP-U4-05(백오프 상태머신, US-E7-11) · PROP-U4-06(classify 전역성) · PROP-U4-07(에스컬레이션 임계). 각 속성에 PBT 카테고리 라벨 + 도메인 제너레이터(PBT-07) 요구 기재. 속성 없는 요소는 "No PBT properties identified" 명시.
- [x] **확장 컴플라이언스 요약**(완료 게이트용): PBT ON / Resiliency ON / Security OFF 표(§4).
- [x] 산출물 작성 전 `content-validation.md` 검증(ASCII 화살표 `->`만, 박스드로잉 0건, Rust 타입 백틱, Korean 산문).

---

## 3. AUTOPILOT 결정 표 (질문 대체 — 권장안 + MVP 편향)

| # | 주제 | 채택안 | MVP 트림? | 근거 + 인용 |
|---|---|---|---|---|
| D-01 | crash-atomic 쓰기 메커니즘 | **temp+rename**(신규 임시 파일 -> fsync -> atomic rename -> 부모 디렉터리 fsync) | **예** | WAL은 이 단위에서 자명하게 공짜가 아님. rename 원자성이 NFR-03을 충족(MVP 지침). component-methods "temp+rename vs WAL -> FD 이월" |
| D-02 | 상태 파일 레이아웃 | **단일 결합 문서** `PersistedState`{last_committed, dirty, resume_offsets}를 한 번에 인코딩·원자 교체 | **예** | 관심사별 다중 파일/부분 쓰기 대신 단일 원자 단위 -> 크래시 창 최소화. resume-offset 갱신 시 전체 재기록(문서 소형이라 허용). FQ-2=A 소형 상태 |
| D-03 | fsync 배리어 순서 | temp fsync -> rename -> parent dir fsync (풀 배리어 유지) | 아니오 | NFR-03 crash-atomic의 핵심 — 트림 대상 아님. US-E3-01 "원자적(임시+rename)" |
| D-04 | 부분쓰기 롤백/복구 | rename 원자성상 main 파일은 항상 last-good; 로드 시 잔존 temp 정리, main 디코드 실패 시 **빈 초기 상태 + dirty=true**(재조정 강제)로 복구 | **예** | 백업 사본/WAL 없이 rename 불변식에 의존. 디코드 불가 시 크래시 대신 zero-loss 재스캔으로 흡수. NFR-03/FR-04, `StateError::CorruptRecovered` |
| D-05 | 재개 오프셋 keying | `BTreeMap<Sha256Digest, ByteCount>`(blob = raw_sha256; 결정적 정렬 -> round-trip 안정) | 아니오 | U3 have/want가 `raw_sha256` 기준(Q6=C). component-methods `ResumeOffset{bytes_acked}` |
| D-06 | commit vs clear 원자성 | `commit_manifest`가 {새 매니페스트, dirty=false, resume_offsets=비움}을 **한 번의 원자 쓰기**로 반영(clear 흡수); `clear_resume_offsets`는 abort/HashMismatch 재시작용 별도 경로 | **예** | 커밋 후 재개 오프셋은 stale -> 같은 원자 쓰기로 정리해 중간 상태 없앰. component-methods 두 메서드 유지 |
| D-07 | 단일 writer 가정 | SingleInstanceLock(FR-23, U8/U2) + 트리거당 1회 직렬 사이클(Q2=B)로 단일 writer 보장 -> U4 내부 파일 잠금 없음 | 아니오(경계) | FR-23 "at most one instance operates on ... last-committed manifest" |
| D-08 | `classify` 매핑 | AuthFailed->AuthFailed · ServerError->Transient · Backpressure->Backpressure · Timeout->Offline · Network->Offline. Permanent는 Fatal-계열(CodecError/HashMismatch)에서만 도달(전송 경로 미산출) | 아니오 | US-E3-04 "오프라인 = 타임아웃/연결오류만", U0 `TransportErrorClass`·`ErrorClass` |
| D-09 | 백오프 공식 | **full jitter**: `delay = rand(0, min(cap, base * mult^attempt))`. 정책 엔진 없이 단일 공식 | **예** | MVP 지침 "지수 + 지터 + 상한, 정책 엔진 없음". 상한 무한증가 방지(FR-11) |
| D-10 | 백오프 기본값 | shape 확정(base/mult/cap/jitter) + 권장 기본(base=1s, mult=2.0, cap=300s); **정확 수치는 NFR Requirements 이월** | 아니오(이월) | component-methods "구체 스케줄 -> Functional/NFR Design 이월"; config `backoff` 섹션은 U4 소유 키 |
| D-11 | 에스컬레이션 카운터 | 단일 `consecutive_failures` 카운터(Transient/Offline/Backpressure에서 증가 -> 백오프 성장); **escalate 신호는 class==Transient && count>=N 에서만**(Offline/Backpressure/AuthFailed 제외) | **예** | US-E3-04 "오프라인 중 반복 오류 알림 없음", FR-19 케이스 2. N = U0 `notify_consecutive_failures`(기본 3) 소비 |
| D-12 | AuthFailed 처리 | `retry_after=None`, `is_offline=false`, `escalate=false`, 카운터 불변(리셋/증가 없음) -> U5 위임 | 아니오 | US-E3-03 체크리스트 "401은 재시도 대상 아님", FR-19 케이스 1은 별도(U6) |
| D-13 | 시계/결정성 경계 | `on_failure(now: Instant)` 주입; 컨트롤러는 (now, RNG seed) 주어지면 순수·결정적, 파일/네트워크 I/O 없음 | 아니오 | PBT 백오프 상태머신(US-E7-11) 결정 실행 위해 주입 시계 + 시드 지터 |
| D-14 | U4 소유 config 키 | `backoff`(초기지연/배수/상한/지터) + 상태 파일 경로 `state_path`(또는 디렉터리). watcher-bin이 federated union에 기여 | 아니오 | R-CFG-STRICT-01(U0): 선언 없으면 자기 키가 unknown으로 거부됨. US-E3-01 "큐 저장 위치 config" / US-E3-03 "백오프 스케줄 config" |

> 재확인(재-결정 아님): 위 표는 **함수 설계 수준**의 미결정만 확정한다. 인프라·스레딩·정확한 튜닝 수치는 NFR Requirements/Design으로 이월한다(D-10).

---

## 4. MANDATORY 카테고리 N/A + 확장 컴플라이언스 (완료 게이트용)

### 4.1 MANDATORY 카테고리 적용 판정

| MANDATORY 항목 | 이 단계 적용 | 판정/근거 |
|---|---|---|
| Content Validation | 적용 | ASCII 화살표(`->`)만, 박스드로잉 0건, Rust 타입 백틱, Korean 산문 — 전 산출물 강제 |
| Mermaid/ASCII 다이어그램 검증 | **N/A** | Mermaid 미사용; 흐름은 화살표 표기 + 순서 목록으로만 기술(ASCII 박스 없음) |
| Question File Format | **N/A** | AUTOPILOT — 질문 미발행(§3 결정 표가 대체) |
| Welcome Message | **N/A** | 신규 워크플로 시작 아님(진행 중 per-unit 루프) |
| 감사 로그(audit.md) | 범위 밖 | STRICT WRITE SCOPE상 U4 산출물만 작성; audit.md는 오케스트레이터 소관 |

### 4.2 확장 컴플라이언스

| 확장 | 활성 | 이 단계 적용 | 계획 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제(PBT-01)** | 각 산출물에 Testable Properties 섹션 필수. 핵심: PROP-U4-01 상태머신 model-based(PBT-06/US-E7-08), PROP-U4-02 crash-atomic zero-loss(NFR-03/US-E7-09), PROP-U4-03 무손실 round-trip(NFR-13), PROP-U4-05 백오프 상태머신(US-E7-11). 카테고리 라벨 + 제너레이터(PBT-07) 요구 기재. 프레임워크(PBT-09)는 Rust=proptest 유력, NFR Requirements 이월. 미준수 시 blocking. |
| **Resiliency Baseline** | ON | **핵심 적용** | RESILIENCY-01: U4 = **High criticality**(zero-loss 지속의 저장 계층). NFR-03 crash-atomic + last-good 롤백 = RPO=0 지속의 실행 지점. RESILIENCY-10(타임아웃+백오프) = `RetryBackoffController`. RESILIENCY-14(회복탄력성 테스트)는 크래시 주입/오프라인/재개 전략으로 PROP-U4-02/05에 씨앗을 두되 상세 하니스는 **NFR Design/Operations 이월**(US-E7-11). RTO/HA/DR 수치·배포는 인프라 소관 N/A. |
| **Security Baseline** | OFF | **N/A** | 미로딩·미강제. 로컬 평문 상태 파일(마지막커밋+dirty+재개오프셋)은 문서화된 수용 위험 RISK-01(requirements §12.2/RISK-01). TLS 강제(NFR-06)는 U5 소관. |

---

## 5. Drop-list (이미 확정 — 재설계·재질문 금지)

| 확정 항목 | 출처 |
|---|---|
| U0가 `Manifest`/`ManifestEntry`/`ChangeSet`/`Sha256Digest`/`ManifestDigest`/`RelativePath`/`ByteCount`/`Timestamp`/`SyncState`/`TransferResult`/`ClassifiedError`/`ErrorClass`/`TransportError`/`TransportErrorClass` 소유·구현(컴파일+테스트 완료) | crates/foundation/src/core_types/** |
| 무손실 CBOR 코덱 `encode`/`decode`(NFR-13) | foundation codec.rs |
| `ConfigProvider`/`WatcherConfig` + `notify_consecutive_failures`(기본 3, N 임계) | foundation config/** |
| FQ-1=A(okc 콘텐츠 주소 재현 없음, 서버가 vault_content_id 소유) | requirements §12.1 |
| FQ-2=A(최신 상태 대체, 이벤트 큐 없음) | requirements §12.2 |
| 안전 한도 상수(총<=20 GiB, 파일당<=2 GiB, 수<=100k) | foundation config/model.rs |
| TLS-only(NFR-06), wire=CBOR, 토큰 §13 저장 | requirements §12/§13 |
| `SyncState` 전이 참조 모델 T1~T8(모델은 U0 소유, U4는 실행) | U0 business-logic-model §2.3 |

---

## 6. 다음 단계(참고)
U4 Functional Design 산출물 3종 완료 -> **U4 NFR Requirements**(백오프 정확 수치·T_recon 정합·성능/회복탄력성 하니스 확정). 이후 U4 NFR Design -> (Infrastructure Design SKIP: U4는 인프라 자원 없음) -> U4 Code Generation.

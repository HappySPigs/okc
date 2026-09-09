# U2 Change Detection & Trigger — Functional Design 계획 (DECISIONS 플랜)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U2 `change-detect`** -> Functional Design -> 계획(플랜)
**작성일**: 2026-09-08
**크레이트**: `change-detect` (lib) · **소속 컴포넌트**: `FilesystemWatcher`, `ReconciliationScheduler`, `VaultAvailabilityGuard`
**대표 에픽**: E1 (모니터링 & 트리거링)
**모드**: AUTOPILOT (게이트 면제, 2026-09-08 사용자 승인) — 모든 열린 설계 질문을 **권장안 + MVP 편향**으로 저자가 직접 결정한다. 사용자 질문 미발행.

> **표기 규약**: 비즈니스 의미 중심의 **기술중립 설계**다. Rust스러운 시그니처는 **참고용(reference-only)** 이며 인프라·스레딩·I/O 메커니즘이 아니라 개념 형상을 표현한다. 다이어그램은 ASCII 박스 없이 **화살표 표기(A -> B)** 와 표/목록으로 기술한다. English 식별자명은 원문 유지, Rust 타입은 백틱으로 감싼다(`Manifest`, `ChangeSet`, `Vec<u8>`).

---

## 1. 단위 컨텍스트

U2 `change-detect`는 볼트 변경을 감지해 **동기화 사이클 트리거를 발행**하고, 런타임 이벤트 손실에 대한 **백스톱**과 파괴적 커밋에 대한 **가드**를 제공한다. 세 컴포넌트는 모두 **순수 판정자(pure judge)** 이며 관측 push를 하지 않는다 — 트리거·판정을 **반환**하고, 실제 로그·상태 표면화는 U8 `SyncCycleCoordinator`가 수행한다(components.md §0 노트2, U2 순수성).

- **책임 경계**: U2는 트리거 발행과 가드 판정만 소유한다. 실제 스캔·재해시·diff는 U1(`VaultScanner`/`ContentAddressing`/`ManifestBuilder`/`ManifestDiffer`)이, 마지막 커밋 매니페스트의 지속은 U4(`SyncStateStore`)가, 사이클 직렬화·관측 push는 U8(`SyncCycleCoordinator`)이 소유한다.
- **의존**: U0 `CoreTypes`(`Manifest`, `RelativePath`) + U0 `ConfigProvider`(`WatcherConfig`의 U2 소비 섹션). U2는 U1~U8 어느 단위도 역참조하지 않는다(크레이트 의존 = `foundation`만; unit-of-work.md §3.2).
- **KEY NFR/FR**: FR-01(감시+디바운스), FR-04(시작+주기 재조정), NFR-01(변경 감지 지연 상한=디바운스), NFR-03(미관측 변경 검출 지연 <= T_recon), NFR-05(FSEvents/inotify/ReadDirectoryChangesW 크로스 OS 이식성), NFR-12(재조정 diff 정확성 — 실제 diff는 U1 소유).
- **스토리**: US-E1-01(감시+디바운스), US-E1-03(시작 재조정), US-E1-04(주기 재조정 백스톱), US-E1-06(빈 볼트/사용불가 가드). US-E1-05(단일 인스턴스 잠금)의 **스토리 1차 소유는 U2**이나 **구현 컴포넌트 `SingleInstanceLock`은 UQ-1=A로 U8에 재배치**되어 이 단위의 Functional Design 대상이 아니다(§5 드롭리스트).

---

## 2. 산출물 목록 (체크박스 — 이 계획의 실행 대상)

프론트엔드/UI 파일 없음(이 단위는 GUI 없는 데몬 로직이며 사용자 표면은 U7b CLI·U6 관측 소관).

- [x] **`u2-change-detect/functional-design/domain-entities.md`** — U2가 도입/특화하는 값 타입: `TriggerSignal`/`TriggerKind`, `WatchState`, `WatchError`, `Availability`, `GuardVerdict`, `ReconResult`/`CycleOutcome`, U2 config 뷰(`WatchConfig`/`ReconConfig`/`VaultConfig`). U0 타입(`Manifest`/`RelativePath`/`WatcherConfig`)은 **참조만** 한다(재정의 금지). -> 컴포넌트: 3개 전부.
- [x] **`u2-change-detect/functional-design/business-rules.md`** — R-* 규칙(디바운스 버스트 합치기, 오버플로 정규화, 재조정 스케줄링·직렬화, 가드 분류·파괴적-빈-커밋 거부, confirm-empty, U2 순수성 불변식) + PROP-* Testable Properties(NFR-01/03/05, exactly-1-trigger-per-burst). -> FilesystemWatcher(디바운스/오버플로), ReconciliationScheduler(스케줄/백스톱), VaultAvailabilityGuard(가드).
- [x] **`u2-change-detect/functional-design/business-logic-model.md`** — 컴포넌트별 알고리즘·워크플로·데이터 흐름(디바운스 타이머 루프, 시작/주기 tick 흐름, check_reachable -> guard_diff 흐름) + 컴포넌트 합성(watcher/scheduler -> 트리거 스트림 -> U8 코디네이터 -> U1 scan/diff -> guard) + 크로스 단위 경계(입력 vs 타 단위 소유) 명시 + 컴포넌트별 Testable-Properties 노트 + 확장 컴플라이언스 요약. -> 3개 전부.

---

## 3. AUTOPILOT 결정 표 (질문 대체 — 권장안 + MVP 편향)

| # | 주제 | 채택안 | MVP 트림? | 근거 + 출처 |
|---|---|---|---|---|
| D1 | 디바운스 입도 | **볼트 전체 단일 정적-구간 타이머**(이벤트마다 리셋, 정적 구간 도달 시 1회 트리거) | 예 | 파일/서브트리별 디바운스는 스코프만 키우고 FR-01/NFR-01의 "버스트당 정확히 1회"에 불필요. (FR-01, NFR-01, US-E1-01 AC) |
| D2 | 이름변경(rename) 취급 | **delete + create 로 취급**(전용 rename 추적 없음) | 예 | FQ-2 재스냅샷 모델에서 rename은 다음 스캔 diff로 정확히 흡수됨. 전용 추적은 무이득. (MVP 가이던스, component-methods §ManifestDiffer 이월) |
| D3 | 감시 백엔드 | **`WatchBackend` 트레이트 1개 + per-OS 어댑터**(FSEvents/inotify/RDCW). 완전 네이티브 구현이 MVP 초과 시 **무이벤트 degraded 백엔드 폴백 허용** — 정확성 바닥은 재조정 백스톱(<= T_recon)이 보장 | 예 | NFR-05 이식성은 **트레이트 경계**로 설계 충족; 네이티브 어댑터는 best-effort, 정확성은 recon이 상한. (NFR-05, MVP 가이던스) |
| D4 | T_debounce / T_recon | **config 고정 duration**(적응형 튜닝 없음) | 예 | 적응형은 스코프 폭증·무스토리. (MVP 가이던스, requirements §"Concrete defaults -> Functional/NFR Design") |
| D5 | U2 관측 배선 | **StatusSink/Logger 주입 제거 — U2 컴포넌트는 완전 순수**(판정/트리거만 반환) | 예 | components.md §0 노트2가 status/log 주입을 "선택적"이라 명시; 제거해 U2 순수성 강제·스코프 축소. push는 U8 코디네이터 소유. (components.md §0 노트2) |
| D6 | 백엔드 오버플로(IN_Q_OVERFLOW/coalesce/버퍼 오버플로/무이벤트 마운트) | **합성 change 이벤트로 정규화 -> 디바운스에 투입 -> 트리거 발행**(recon까지 기다리지 않음) | 아니오 | 저비용·고가치: 오버플로 시 지연을 T_recon이 아닌 T_debounce로 회복. (US-E1-04 AC, FR-04 백스톱 근거와 정합) |
| D7 | 재조정 스케줄링·직렬화 | `next_due = last_recon_emit + T_recon`(발행 시각 앵커 — `tick`이 `Some`을 반환하는 순간 설정). **진행 중 사이클이면 `tick`은 `None`**(진행 사이클이 곧 재스냅샷=재조정 커버). `record_result`는 완료 통지(`last_result` 갱신)만 하며 `next_due`를 완료 시각으로 재계산하지 않음(완료 앵커는 간격을 `사이클 시간 + T_recon`으로 늘려 상한 위반) | 아니오 | 직렬화(US-E1-04 AC) + <= T_recon 상한(NFR-03) 동시 충족. 발행 시각 앵커라 PROP-U2-04 오라클(`next_due <= 마지막 발행 + T_recon`) 성립. (NFR-03, US-E1-04) |
| D8 | 시작 재조정 | **정상 감시 진입 전 1회 전체 재해시+diff 트리거 발행** | 아니오 | US-E1-03 필수(정지 중 발생 변경 복구). (US-E1-03, FR-04a) |
| D9 | 가드 분류 도메인 | **4 명명 조건만**: `Reachable` / `RootMissing` / `Unmounted` / `Inaccessible`. 부분가용·심링크·권한 세부는 미분류(best-effort 진단; 세 unavailable 변이는 동일하게 HOLD) | 예 | 세부 분류는 안전 동작(HOLD)에 영향 없음 — 진단 라벨일 뿐. (MVP 가이던스, US-E1-06) |
| D10 | confirm-empty 소비 | **영속 config 불리언 플래그 `confirm_empty`**(1회성 토큰 아님) | 예 | 추가 상태 0. 잔여 리스크(설정 시 파괴적 가드 상시 비활성)는 문서화된 수용 동작. (US-E1-06 AC, U0 `confirm_empty` 예고 필드) |
| D11 | `guard_diff` Proceed 규칙 + 자동 재개 | Proceed 허용 = (도달가능 AND (비어있지 않음 OR confirm_empty OR last_committed 없음)). 자동 재개는 **무상태**(볼트 복구 시 확인 없이 정상 diff) | 아니오 | US-E1-06 AC 그대로. 무상태라 재개가 자연히 성립. (US-E1-06) |
| D12 | 트리거 페이로드 | 트리거는 **ChangeSet 미탑재**(FQ-2 재스냅샷). `cause_summary`는 진단 문자열, `cycle_id`는 U8이 부여 | 아니오 | FQ-2=A 고정(폴더가 진실의 원천). 코디네이터가 재스냅샷. (FQ-2=A, component-methods §FilesystemWatcher) |
| D13 | pause 중 이벤트 | **paused 상태에서 이벤트 폐기(버퍼링 없음)** — recon 백스톱이 복구 | 예 | 버퍼링은 스코프 증가·무이득; recon이 정확성 보장. (US-E6-03 pause, NFR-03) |
| D14 | 트리거 스트림 형상 | **추상 순서 보존 트리거 채널**(구체 전송=mpsc/async는 NFR/Code-gen 이월) | 아니오 | 기술중립 원칙 — FD는 계약만. (component-methods §FilesystemWatcher 이월 규약) |

---

## 4. MANDATORY-카테고리 N/A 표 + 확장 컴플라이언스

### 4.1 MANDATORY 워크플로 카테고리

| 카테고리 | 이 단계 적용 | 근거 |
|---|---|---|
| Rule Details 로딩 | 적용 | `.aidlc-rule-details/` 계열 규칙을 참조해 산출물 형식·검증 규칙 준수 |
| Content Validation | 적용 | Korean prose, ASCII 화살표(`->`)만, 유니코드 다이어그램 글리프 미사용, Rust 타입 백틱 처리 — 세 산출물 전부 |
| Question Format Guide | **N/A** | AUTOPILOT — 질문 미발행(§3 결정 표가 대체) |
| Welcome Message | **N/A** | 워크플로 시작이 아닌 per-unit 산출 단계 |
| Audit 로깅 | **N/A (범위 외)** | `audit.md`는 오케스트레이터 소유이며 이 저자의 쓰기 범위 밖(STRICT WRITE SCOPE) |
| Plan-Level 체크박스 | 적용 | §2 산출물 체크박스로 진행 추적 |

### 4.2 확장 컴플라이언스

| 확장 | 활성 | 이 단계 판정 | 근거 |
|---|---|---|---|
| **Resiliency Baseline** | ON | **부분 적용** | 재조정 백스톱(FR-04/NFR-03)이 RESILIENCY-02(RPO/무손실 검출 지연) 근거. 파괴적-빈-커밋 가드(US-E1-06)가 데이터 무결성 보호. RTO/HA/DR/멀티AZ·배포·롤백은 U2(순수 감지 로직)에 **N/A**(상위/인프라 소관). |
| **Property-Based Testing** | ON (Full) | **강제·준수** | 세 산출물에 PROP-* Testable Properties + 카테고리 라벨 + 제너레이터(PBT-07) 요구 기재. exactly-1-trigger-per-burst(디바운스), <= T_recon 백스톱(NFR-03), 가드 불변식(US-E1-06), 백엔드 정규화 등가(NFR-05). |
| **Security Baseline** | OFF | **N/A** | 미로딩·미강제. RISK-01(로컬 평문 산출물)은 문서화된 수용 위험. U2는 시크릿·네트워크 미취급. |

> **적용성 판정**: 위 확장 규칙은 이 단계 산출물(순수 감지·트리거 설계)의 목적에 비추어 관련 규칙만 강제한다. 관련 없는 규칙은 N/A로 표기(비차단).

---

## 5. 드롭리스트 (이미 확정 — 재설계·재개봉 금지)

이 항목들은 U0 또는 상위 결정에서 **확정**되었으며 U2 FD에서 다시 결정하지 않는다(소비만).

| 항목 | 확정 내용 | 출처 |
|---|---|---|
| CoreTypes 값 타입 | `Manifest`/`ManifestEntry`/`ChangeSet`/`RelativePath`/`Sha256Digest`/`ManifestDigest`/`Timestamp`/`ByteCount` 등 — U0 소유, 컴파일+테스트 완료 | U0 `domain-entities.md`, `crates/foundation` |
| CBOR 코덱(NFR-13) | `encode`/`decode` 무손실 round-trip — U0 소유 | U0 §1 R-CODEC-01 |
| 싱크 계약 트레이트 | `Logger`/`StatusSink`/`HistorySink`/`CriticalEventSink`/`ConfigReloadObserver`(`Send+Sync`) — U0 소유. U2는 **주입받지 않음**(D5 순수성) | U0 `sink.rs`, components.md §0 수정1 |
| 2축 상태 어휘 | `OperationalState`/`ActiveCondition{...,VaultUnavailable,...}`/`LivenessSignal`/`StatusSnapshot` — U0 소유. U2는 `VaultUnavailable` 조건을 **push하지 않고** 판정만 반환(U8이 raise) | U0 `status.rs` |
| ConfigProvider/WatcherConfig | 단일 JSON 로드·검증·리로드·관찰자 팬아웃 — U0 소유. U2는 자기 섹션(`debounce_ms`/`reconciliation_interval_s`/`confirm_empty`) 스키마만 확정 | U0 §2, `config/model.rs` |
| FQ-1=A | okc-core 콘텐츠 주소 재현 없음; 권위 `vault_content_id`는 서버 | requirements FQ 부록 |
| FQ-2=A | 최신 상태 대체(이벤트 큐 없음, 재스냅샷) — 트리거는 ChangeSet 미탑재 | requirements §12.2 |
| SafetyLimits 상수 | 총<=20 GiB / 파일당<=2 GiB / 수<=100k — U0 상수, 검사는 U1 | U0 §4.3, requirements NFR-14 |
| TLS-only / 토큰 저장 §13 | NFR-06 TLS 전용; config 평문 1차 + env 폴백 + 선택 secure-store — U5 소관 | requirements §13, NFR-06 |
| Wire 포맷 | CBOR(내부 상태) / JSON(config) | U0 §3 |
| **`SingleInstanceLock`** | US-E1-05 스토리는 U2 1차 소유이나 **컴포넌트는 U8로 재배치**(UQ-1=A) — 이 FD의 대상 아님 | unit-of-work.md §1.3(4), story-map §3 |
| 실제 scan/hash/diff | U1 `VaultScanner`/`ContentAddressing`/`ManifestBuilder`/`ManifestDiffer` 소유 — U2는 트리거만 | components.md §U1 |
| 마지막 커밋 매니페스트 지속 | U4 `SyncStateStore` 소유 — U2 `guard_diff`에 인자로 전달 | components.md §U4 |
| 사이클 직렬화·관측 push | U8 `SyncCycleCoordinator` 소유 | services.md, components.md §0 노트2 |

---

## 6. 다음 단계

세 산출물(`domain-entities.md`, `business-rules.md`, `business-logic-model.md`) 완료 후 -> **NFR Requirements**(U2). NFR Requirements에서 T_debounce/T_recon 기본값·백엔드 기술 선택(notify crate 등)·PBT 프레임워크(proptest)·트리거 채널 구체 전송을 확정한다.

# U6 Observability — Functional Design 계획 및 결정(AUTOPILOT)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U6 Observability** -> Functional Design (Part: 계획 + 결정 게이트)
**작성일**: 2026-09-08
**크레이트**: `observability` (lib) · **소속 컴포넌트**: `StructuredLogger`, `StatusService`, `UploadHistoryStore`, `CriticalErrorNotifier`, `TrayIndicator`(선택)
**입력 아티팩트**: `unit-of-work.md`(§U6 책임 + §0 노트1/2/3 + §4 트레이 선택), `component-methods.md`(§U6 시그니처·이월 항목), `unit-of-work-story-map.md`(E5 매핑), `stories.md`(US-E5-01..05), `requirements.md`(FR-16/17/18/19, NFR-13/15/16, RISK-01, Q9=B), `crates/foundation/src/core_types/{sink.rs,status.rs,mod.rs}`(U6가 구현할 실제 U0 트레이트·타입 시그니처)
**규칙**: `construction/functional-design.md` · `common/content-validation.md` · 활성 확장 `property-based-testing.md`(PBT-01 이 단계 강제, Full) · `resiliency-baseline.md`(ON)

> **AUTOPILOT 모드 주석**: 사용자 승인(2026-09-08)에 따라 게이트를 유예하고, 모든 미결정 항목을 **권장안 + MVP 우선(bias-to-MVP)** 으로 저자가 스스로 확정한다. 통상 §3 질문 절은 **§3 AUTOPILOT 결정 표**로 대체된다.

---

## 1. 단위 컨텍스트 (Step 1 — 요약)

U6는 데몬의 활동·상태·이력·중대 오류를 **push-only** 로 표면화하는 관측 계층이다. U6 컴포넌트는 **U0가 소유한 싱크 계약 트레이트**(`Logger`/`StatusSink`/`HistorySink`/`CriticalEventSink`/`ReadJudgment`)의 **구체 구현만** 제공하며, 조립 루트(U8)가 하위 단위에 하향 주입한다 — 따라서 **U6는 U1~U5를 역참조하지 않는다**(크레이트 의존: `foundation` 만). E5(히스토리 & 관측성)를 대표 에픽으로 US-E5-01..05를 소유한다.

### 1.1 이 단위가 구현할 대상(U0 트레이트 -> U6 구현체)
- `StructuredLogger` `impl Logger` — JSON-line 구조화 로그 주 표면(`cycle_id` 상관), 방출 시각 stamping, `log_level` 필터, 파일 rotation, config 리로드 재적용(US-E5-01, FR-17)
- `StatusService` `impl StatusSink + ReadJudgment` (+ 고유 `snapshot()`) — Q9 2축 상태 모델의 **단일 집계 지점**; `snapshot()`/`health_check()`(운영 헬스)/`update_probe()`(순수 liveness, 운영 조건과 분리 — §0 노트3)(US-E5-02, NFR-15, Q9=B)
- `UploadHistoryStore` `impl HistorySink` (+ 고유 `query()`) — append-only 로컬 히스토리(수정/삭제 불가, 무손실 round-trip), CLI 질의 표면(US-E5-03, FR-16)
- `CriticalErrorNotifier` `impl CriticalEventSink` — 닫힌 4조건(401 / N회 연속 실패 / 프리플라이트 초과 / 자동 업데이트 롤백)만 능동 표면화(US-E5-04, FR-19)
- `TrayIndicator`(선택) — nullable no-op 싱크(§0 수정2, §4 confirm-or-drop)(US-E5-05, FR-18)

### 1.2 Application Design에서 이월된(deferred) 미결정 항목 — §3 결정의 출처
| 이월 항목 | 출처 | 결정 |
|---|---|---|
| TrayIndicator 채택/제거(confirm-or-drop) | unit-of-work §4 / US-E5-05 | D1 |
| 로그 저장 포맷·rotation 트리거·백프레셔·동시성 | component-methods §StructuredLogger | D2/D3 |
| `LogRecord` 타임스탬프 소재(레코드 vs 방출시각) | U0 `LogRecord` 자리표시 스키마 | D4 |
| 조건 -> 운영상태 결합 규칙, health/liveness 판정 임계 | component-methods §StatusService | D5/D6/D7 |
| 히스토리 저장 포맷(append 로그 vs DB)·인덱싱·round-trip 제너레이터 | component-methods §UploadHistoryStore | D8/D9 |
| 히스토리 레코드 최종 필드 스키마(FR-16) | U0 `UploadHistoryRecord` 자리표시 스키마 | D10 |
| N 임계값·중복 억제·표면화 디바운스 | component-methods §CriticalErrorNotifier | D12/D13 |
| U6 config 섹션(log 파일/rotation, history 파일, tray toggle) 스키마 | domain-entities §2 "다른 단위 소비 섹션" | D14 |

---

## 2. Functional Design 산출물 계획 (Steps 2·6 — 체크박스)

아래 3개 산출물을 `aidlc-docs/construction/u6-observability/functional-design/` 에 생성한다(프론트엔드/UI 파일 없음 — 이 단위는 UI를 소유하지 않는다). 기술중립(Rust스러운 시그니처는 참고용, 인프라 배제).

- [x] **`domain-entities.md`** — U6가 도입/특화하는 값 타입: 방출 로그 라인 스키마(`LogRecord` 확장 + stamped `timestamp`), `LogConfig`/rotation 파라미터, `UploadHistoryRecord` **최종 필드 스키마**(FR-16: content/snapshot hash·status·bytes·error_detail) + `HistoryQuery`/`UploadStatus`, `HealthReason`/`Health`/`Liveness` 판정 형상 사용, `StatusSnapshot` 파생 필드 규칙, U6 config 키(`ObservabilityConfig`). **U0 타입 재정의 금지** — 소비·확장 방식만 기술. (컴포넌트: 전 5개)
- [x] **`business-rules.md`** — R-* 규칙(검증·제약·불변식·오류 처리·MVP 트림 경계) + PROP-* Testable Properties: push-only 구조 불변, 로그 레벨 필터/redaction(SEC-02 잔존 통제), 2축 상태 독립성·health 도출 규칙·update_probe 격리, append-only 불변성·무손실 round-trip·truncated-tail 관용, 닫힌 4조건·연속 실패 임계(N). (컴포넌트: 전 5개)
- [x] **`business-logic-model.md`** — 컴포넌트별 알고리즘/워크플로/데이터 흐름 + 합성: 로그 방출 파이프라인, 상태 집계·판정 흐름, 히스토리 append/query 흐름(scan->decode->filter), 중대오류 fan-out(로그+status+tray), 크로스-유닛 경계(주입 vs 소유) 명시 + 컴포넌트별 Testable-Properties 노트 + 확장 컴플라이언스 요약. (컴포넌트: 전 5개)
- [x] **PBT-01 "Testable Properties" 섹션**(각 산출물 내, Full 강제): PROP-U6-* — 로그 방출 round-trip/레벨 필터, 히스토리 append-only 불변·무손실 round-trip(PBT-02, NFR-13 소비)·query 오라클·truncated-tail, 2축 독립·health 오라클·**update_probe 격리(롤백 루프 방지 핵심)**, 닫힌 4조건·연속 실패 상태머신. 각 속성에 카테고리 라벨 + 도메인 제너레이터(PBT-07) 요구 기재. 속성 없는 요소는 "No PBT properties identified"(push-only 구조는 컴파일타임 강제) 명시.
- [x] **확장 컴플라이언스 요약**(완료 게이트용): PBT / Resiliency / Security 준수·N/A 표(§4)
- [x] 산출물 작성 전 `content-validation.md` 검증(ASCII 화살표 `->`만·박스드로잉 0건, Rust 타입 백틱, 특수문자 이스케이프)

---

## 3. AUTOPILOT 결정 표 (Steps 3–4 대체 — 권장안 + MVP 우선)

| ID | 주제 | 채택 옵션 | MVP 트림? | 근거 + 인용 |
|---|---|---|---|---|
| **D1** | TrayIndicator 채택/제거 | **실제 데스크톱 트레이 DROP, nullable no-op 싱크만 구현** | **예** | 타깃은 헤드리스 nginx식 데몬. FR-18 트레이는 선택 + 어떤 스토리도 비의존(US-E5-05, unit-of-work §4). 로그/status/history/critical-notify는 트레이 없이 완전 동작. 실제 트레이 백엔드(3-OS)는 최대 구현 부피 -> 최강 트림. |
| **D2** | 로그 저장 포맷·목적지 | **config 지정 파일에 JSON-line append + 크기기반 rotation(max_size + 보존 파일 수)** | **예** | US-E5-01 "JSON line 로그 레코드", 체크리스트 "파일 경로·레벨·rotation config". 시간/나이 기반 보존은 **보존 파일 수(count)로 근사**(age-based 미구현). stderr/syslog 다중 싱크 미구현(파일 단일). |
| **D3** | 로그 동시성·백프레셔 | **mutex 하 동기 best-effort 쓰기, IO 오류는 삼킴(infallible 표면)** | **예** | U0 `Logger::log` 는 infallible 계약. 비동기 로깅 스레드·bounded 채널·배칭 미구현 -> MVP 부피 최소. 관측 실패가 동기화 코어를 막지 않음(§0 best-effort). |
| **D4** | 로그 타임스탬프 소재 | **방출 시점에 `StructuredLogger`가 stamping(레코드 필드 아님)** | 아니오 | U0 `LogRecord` 자리표시에 `timestamp` 없음(sink.rs) — U0 불변경(쓰기 범위 밖). 로거가 시계를 소유해 방출 JSON 라인에 `timestamp` 부가 -> US-E5-01 "각 레코드 timestamp" 충족, U0 트레이트 파괴변경 회피. |
| **D5** | health_check 조건 도출 | **`health-critical` 집합 = {`AuthFailed`(케이스1),`OverLimit`(케이스3),`UpdateRolledBack`(케이스4)} + 연속실패 escalation(케이스2). 교집합 비면 `Unhealthy`** | 아니오 | US-E5-04 닫힌 4조건 "4조건 -> unhealthy 전환" 과 정확히 일치(비목록 조건은 healthy 유지). `ConsentBlocked`/`Offline`/`VaultUnavailable` 은 제외 — 앞 둘은 의도된 정상 상태(알림 피로 회피, US-E3 오프라인은 표시만), `VaultUnavailable` 은 US-E5-04 닫힌 집합 밖이라 health flip 없이 status/로그에만 노출. |
| **D6** | 연속 실패(케이스2) health 반영 배선 | **`StatusService` 고유(비트레이트) escalation 입력 메서드를 `CriticalErrorNotifier`가 U6-내부 배선으로 호출** | **예** | `ActiveCondition`(U0 닫힌 enum)에 연속실패 변이 없음 + U0 불변경. 두 컴포넌트 모두 U6이므로 U6-내부 고유 메서드로 단일 health 집계 지점 유지(U1~U5 역참조 없음). 신규 U0 트레이트 메서드 미도입. |
| **D7** | update_probe 판정 | **`Alive` iff {`IdleReached`,`CredentialReadable`} 모두 관측, 아니면 `NotReady{missing}`. 운영 조건 무관** | 아니오 | §0 노트3/Q9=B: 일시 `AuthFailed`/`OverLimit`가 좋은 새 버전을 오판 롤백시키는 루프 방지. U7a `AutoUpdater`가 이 판정만 소비. |
| **D8** | 히스토리 저장 포맷 | **단일 append-only 파일, 길이-프레이밍된 CBOR 프레임 시퀀스(U0 코덱)** | **예** | component-methods "append 로그 vs DB" 이월. 임베디드 DB/보조 인덱스 미채택 -> `query()`는 전체 스캔 + 필터(선형). 무손실 round-trip(NFR-13)은 U0 코덱 재사용. |
| **D9** | 히스토리 append 오류·crash 관용 | **`append` infallible(오류는 로그로 삼킴); `query`는 Result. 디코드 실패 트레일링 프레임 = 크래시 truncated-tail로 간주해 정상 prefix 반환** | 아니오 | U0 `HistorySink::append` infallible 계약. append 도중 크래시(부분 프레임) 후에도 이전 레코드 보존(append-only 불변) + 재기동 질의 성공(Resiliency). 파일 중간 손상은 `Corrupt` 표면화. |
| **D10** | 히스토리 레코드 최종 스키마 | **U0 자리표시 확장: `{ content: HistoryContentId(snapshot manifest_digest + Option<server vault_content_id>), timestamp, status: UploadStatus{Success\|Failure\|Partial}, bytes_transferred, error_detail: Option<String> }`** | 아니오 | FR-16/US-E5-03 필수 필드. U0 note "최종 스키마는 하류(U6)가 확정". 권위 `vault_content_id`는 서버 소유(FQ-1=A)라 성공 커밋 시에만 존재 -> `Option`; 로컬 `manifest_digest`(snapshot hash)는 항상 존재. (U0 구조체 필드 반영은 Code-Gen 관심사; FD는 스키마만 확정) |
| **D11** | 히스토리 append 시점·주체 | **사이클 종료(success/failure/partial)마다 U8 `SyncCycleCoordinator`가 `HistorySink`로 1건 push** | 아니오 | U6는 U3를 호출하지 않음(push-only). 경계: 레코드 조립·전달은 코디네이터, append 지속은 U6. |
| **D12** | 중대오류 디바운스/중복 억제 | **케이스별 임계 로직 외 별도 디바운스 미구현. 케이스2는 임계 N 교차 시 1회 발화, 성공 시 리셋(후속 실패마다 재발화 없음)** | **예** | component-methods "중복 억제·디바운스 이월". MVP: 최소. 연속실패 escalation 은 활성 유지, `report_cycle_result(Success)` 시 카운터·escalation 리셋. |
| **D13** | N 임계값 소스 | **config `notify_consecutive_failures`(U0 필드, 기본 3)** | 아니오 | requirements FR-19 "N 설정 가능, 기본 3". U0 `WatcherConfig.notify_consecutive_failures` 재사용(model.rs) — 신규 키 불필요. |
| **D14** | U6 config 키 집합 | **`log_file`,`log_max_size_bytes`,`log_max_retained`,`history_file`,`tray_enabled` 를 U6 federated known-key로 선언** | 아니오 | federated-keys 패턴(U0 model.rs `FOUNDATION_CONFIG_KEYS` 참조): 각 단위가 자기 키 선언해야 strict-reject(R-CFG-STRICT-01)가 오탐 안 함. `tray_enabled`는 D1로 인해 파싱만 되고 동작은 no-op(호환 유지). |

---

## 4. 확장 컴플라이언스 계획 (완료 게이트에서 최종 판정)

| 확장 | 활성 | 이 단계(U6 Functional Design) 적용 | 계획 |
|---|---|---|---|
| **Property-Based Testing** | ON (Full) | **강제(PBT-01)** | 각 산출물에 "Testable Properties" 섹션 필수(§2). 핵심: 히스토리 무손실 round-trip(PBT-02, NFR-13 소비) + append-only 불변 + query 오라클, 로그 레벨 필터/방출 round-trip, 2축 독립·health 오라클·**update_probe 격리**, 닫힌 4조건·연속실패 상태머신. 도메인 제너레이터(PBT-07) 요구 기재. 프레임워크(PBT-09)는 NFR Requirements 이월(Rust=proptest 유력). 미준수 시 blocking. |
| **Resiliency Baseline** | ON | 부분 적용 | RESILIENCY-05(구조화 로깅, FR-17)·06(헬스/status, FR-18/NFR-15)·15(사고/오류 표면화, FR-19)를 U6가 직접 구현. 관측 push best-effort·infallible(관측 실패가 코어를 막지 않음). 히스토리 append-only + truncated-tail 관용 = 크래시 관용(D9). RESILIENCY-04(자동 업데이트 롤백) health 게이트 근거(`update_probe`)를 U6가 제공. RTO/RPO 수치·배포·HA/DR은 인프라/상위 단계 소관 -> N/A. |
| **Security Baseline** | OFF | N/A | 미로딩·미강제. RISK-01(로컬 평문 산출물: 로그·히스토리 평문, `error_detail`에 민감정보 혼입 가능)은 문서화된 수용 위험. 단, U0 sink.rs **SEC-02 계약**(토큰 원문 로그 금지, `TokenSecret` redaction)은 잔존 통제로 `StructuredLogger` 규칙에 반영. |

### 4.1 MANDATORY 카테고리 N/A 판정
| MANDATORY 카테고리 | 적용 | 근거 |
|---|---|---|
| Content Validation(ASCII 화살표·박스드로잉 0·백틱·이스케이프) | **적용** | 전 산출물 준수. `->`만 사용, U+2192 글리프 0건. |
| Question File Format | **N/A** | AUTOPILOT — 질문 미발행(§3 결정 표로 대체, 사용자 승인). |
| Frontend/UI 산출물 | **N/A** | U6는 UI 미소유(TrayIndicator도 D1로 no-op). CLI 표면은 U7b 소관. |
| Infrastructure/Tech-stack 선택 | **N/A** | FD는 기술중립. 포맷/스레딩/파일 I/O 구체 선택은 NFR/Infra 단계 이월. |
| Audit 로깅 | **N/A(저자 범위 밖)** | 쓰기 범위상 audit.md 미수정(워크플로 스크립트 소관). |

---

## 5. 드롭리스트 (이미 확정 — 재결정 금지)

| 항목 | 소스(소유 단위/결정) |
|---|---|
| CoreTypes 값 타입 전체(`Manifest`/`ChangeSet`/`Sha256Digest`/`ManifestDigest`/`Timestamp`/`ByteCount`/`SyncState`/`TransportError`/`ClassifiedError`/`StatusSnapshot`/`OperationalState`/`ActiveCondition`/`LivenessSignal`/`Health`/`Liveness` 등) | U0 `foundation`(구현·테스트 완료) — 재정의 금지, 소비만 |
| 싱크 계약 트레이트(`Logger`/`StatusSink`/`HistorySink`/`CriticalEventSink`/`ReadJudgment`/`ConfigReloadObserver`, 모두 `Send+Sync`) | U0 소유(§0 수정1) — U6는 구현만 |
| 무손실 CBOR 코덱 `encode`/`decode`(NFR-13) | U0 소유 — U6 히스토리가 소비 |
| `ConfigProvider`/`WatcherConfig`/`notify_consecutive_failures`(기본 3)/`log_level` | U0 소유 — U6는 구독·소비 |
| FQ-1=A(okc 콘텐츠 주소 미재현, `vault_content_id` 서버 소유) | 상류 확정 |
| FQ-2=A(최신 상태 대체) | 상류 확정 |
| 안전 한도 상수(총 <=20 GiB, 파일당 <=2 GiB, 수 <=100k) | U0/U1 확정 — U6는 `LimitReport` 표면화만 |
| TLS-only(NFR-06) | U5 확정 |
| 토큰 저장 §13(config 평문 1차 + env 폴백 + secure-store opt-in) | U0/U5 확정 |
| wire format = CBOR | U0 확정(Q1=B) |
| U6가 U1~U5 역참조 안 함(push-only, U0 트레이트만 의존) | unit-of-work §U6/§3.2 확정 |
| RISK-01 로컬 평문 산출물 = 수용 위험(Security OFF) | requirements §7 확정 |

---

## 6. 다음 단계(참고)
U6 Functional Design 3개 산출물 완료 -> **U6 NFR Requirements**(per-unit 루프의 다음 스테이지: 로그 포맷/rotation·히스토리 저장 백엔드·동시성 등 기술 스택 및 성능/회복력 수치 확정). 이후 U6 NFR Design -> (Infrastructure Design: U6는 대체로 SKIP) -> U6 Code Generation.

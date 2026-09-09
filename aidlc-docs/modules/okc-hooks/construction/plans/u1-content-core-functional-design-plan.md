# U1 Deterministic Content Core — Functional Design 계획 (DECISIONS 플랜)

**단계**: CONSTRUCTION -> Per-Unit Loop -> **U1 `content-core`** -> Functional Design (계획/DECISIONS)
**작성일**: 2026-09-08
**크레이트**: `content-core` (lib) · **소속 컴포넌트**: `ContentAddressing`, `VaultScanner`, `ManifestBuilder`, `ManifestDiffer`, `SafetyLimitsValidator`
**모드**: AUTOPILOT(사용자 승인 2026-09-08, 게이트 waived). 아래 §3 결정 표가 통상의 질문을 **대체**한다 — 모든 열린 항목은 권장안(RECOMMENDED)·MVP 편향으로 저자가 확정했다.

> **표기 규약**: 기술중립 비즈니스 설계. Rust 시그니처는 참고용. ASCII 화살표(`A -> B`)만 사용, 박스/유니코드 다이어그램 없음. Rust 타입/제네릭은 backtick으로 감싼다(`Vec<u8>`, `Manifest`, `ChangeSet`).

---

## 1. 단위 컨텍스트

U1은 볼트 콘텐츠를 **결정적으로 지문화(fingerprint)** 하고 매니페스트를 재계산·비교하며 전송 전 안전 한도를 검사하는 **순수 코어**다. `VaultScanner`의 파일 읽기를 제외하면 지속/네트워크 I/O가 없다.

- **대표 에픽**: E7(결정적 코어 속성 검증) — 또한 E1(매니페스트 재계산·diff, US-E1-02) · E2(스냅샷·프리플라이트, US-E2-01/02).
- **1차 소유 스토리(7)**: US-E1-02, US-E2-01, US-E2-02, US-E7-01, US-E7-05, US-E7-07, US-E7-10 (`unit-of-work-story-map.md` §확정표).
- **핵심 NFR/FR**: NFR-08(결정적 해싱) · NFR-12(정확 diff, 누락 0·오탐 0) · NFR-13(무손실 코덱 round-trip) · NFR-14(SafetyLimits 경계-정확·단조) · NFR-02(스트리밍 메모리 바운드) · FR-02/FR-22/FR-05.
- **소비 계약(U0 `foundation`, 이미 컴파일·테스트 완료 — 재정의 금지)**: 값 타입 `Manifest`/`ManifestEntry`/`ChangeSet`/`RelativePath`/`Sha256Digest`/`ManifestDigest`/`ByteCount`; `RelativePath::normalize`; `ChangeSet::is_empty`; CBOR 코덱 `encode`/`decode`(NFR-13); SafetyLimits 상수 `MAX_VAULT_TOTAL_BYTES`/`MAX_FILE_BYTES`/`MAX_FILE_COUNT`(`config::model`).
- **교차 단위 경계**: 마지막 커밋 `Manifest`는 **U4 `SyncStateStore`가 소유·지속**하고 `ManifestDiffer::diff`에 **인자로 전달**된다. 관측 push(트리거 로그·vault-unavailable·진행률)는 U1이 하지 않고 U8 `SyncCycleCoordinator`가 판정을 받아 수행한다(§0 노트2). `ContentAddressing::hash_stream`은 U3 전송 시 재검증(Q8=A/노트1)에서 재사용된다.

---

## 2. 산출물 목록 (체크박스 — 컴포넌트/규칙/속성 매핑)

이 단위에는 UI가 없으므로 frontend 산출물이 없다.

- [x] **`u1-content-core/functional-design/domain-entities.md`** — U1이 도입/특화하는 값 타입: `ScannedFile`, `VaultSnapshot`, `SafetyLimits`(상수 뷰), `LimitViolation`/`LimitVerdict`, `ScanError`/`BuildError`, 다이제스트 도메인 인코딩 규약. U0 타입(`Manifest`/`ManifestEntry`/`ChangeSet`/`RelativePath`/`Sha256Digest`/`ManifestDigest`)은 **이름으로 참조**하고 재정의하지 않는다. (컴포넌트: 전 5개)
- [x] **`u1-content-core/functional-design/business-rules.md`** — R-* 규칙(정규화·정렬·다이제스트 인코딩·diff 판정·경계/단조·심링크/오류 엣지) + PROP-U1-* Testable Properties(NFR-08/12/13/14). (컴포넌트: 전 5개; 규칙 R-CA-*/R-SCAN-*/R-BUILD-*/R-DIFF-*/R-LIMIT-*)
- [x] **`u1-content-core/functional-design/business-logic-model.md`** — 컴포넌트별 알고리즘/워크플로/데이터 흐름 + 합성(scan -> build -> diff -> limits) + 교차 단위 경계(인자 전달 vs 타 단위 소유) + 컴포넌트별 Testable-Properties 노트 + 확장 컴플라이언스 요약.

> 위 3개 산출물은 본 계획과 함께 이번 스테이지에서 작성되었다. 코드/`crates/**/*.rs`는 이 스테이지 산출물이 아니다(Code Generation 이월).

---

## 3. AUTOPILOT 결정 표 (질문 대체)

| # | 주제(topic) | 선택(chosen) | MVP-trim? | 근거 + 인용 |
|---|---|---|---|---|
| D1 | `manifest_digest` 도메인 인코딩 | canonical 정렬 엔트리에 대한 **명시적 길이-프리픽스 프레이밍**(각 필드 길이 프리픽스 후 concat) 위에 표준 SHA-256, **CBOR 지속 코덱과 분리** | 아니오 | 지속 코덱 스키마 진화(NFR-13 신규 필드)로부터 다이제스트 안정성 격리 -> no-op 판정(FR-10) 버전 간 불변. U0 `domain-entities.md` §1.2 "다이제스트 입력 인코딩 세부" 이월 확정 |
| D2 | `raw_sha256` 알고리즘 | **표준 SHA-256**(= `sha256sum`), okc 스킴 재현 없음 | 아니오 | FQ-1=A 고정(드롭리스트), NFR-08/US-E7-01. `vault_content_id` 권위 계산은 서버 |
| D3 | `hash_stream` 메모리 모델 | **고정 버퍼 스트리밍**(파일을 전량 적재하지 않음); 구체 버퍼 크기는 NFR Design 이월 | 아니오 | NFR-02 100k/20 GiB 무한정 메모리 금지. component-methods `hash_stream` 계약 |
| D4 | `ManifestDiffer` 범위 | **added/modified/deleted만**; rename/content-move 감지 없음 | **예** | rename 감지는 매칭 휴리스틱으로 구현 범위 대폭 확대. MVP 가이던스 명시 |
| D5 | modified 판정 기준 | 같은 경로에서 **`(raw_sha256, size)` 중 하나라도 다르면** modified(current 값 채택) | 아니오 | mtime 미사용(U0 `ManifestEntry` 결정 승계). component-methods "수정 판정(size+hash)" 이월 |
| D6 | diff 알고리즘 | canonical 정렬 전제 **투 포인터 merge-join** O(n+m), 결과 3목록 경로 오름차순 | 아니오 | 결정적·정확(NFR-12). 두 매니페스트 모두 U0 canonical 정렬 불변식 |
| D7 | diff no-op short-circuit | `current.manifest_digest == last.manifest_digest` 이면 **즉시 빈 `ChangeSet`** | 아니오 | FR-10 O(1) 조기 종료. U0 R-NOOP-02 동치성 승계 |
| D8 | 심링크(symlink) 처리 | **SKIP**(따라가지 않고 엔트리에서 제외) | **예** | 루프/볼트 이스케이프 방지 최단 안전. MVP 가이던스 |
| D9 | exclude/ignore 패턴 | **없음**(config 구동 제외 패턴 미지원) | **예** | 데몬 산출물(SyncState/히스토리/로그)은 볼트 외부 플랫폼 상태 경로에 저장되어 self-churn 없음. 구현 최소화. component-methods "제외 패턴" 이월 항목을 MVP에서 no-op으로 확정 |
| D10 | 숨김 파일(dotfiles) | **포함**(필터링 없음, `.obsidian` 등 정당한 볼트 콘텐츠) | 아니오 | 최소 규칙 — 특수 처리 없음 |
| D11 | 일관 시점(consistent snapshot) 확보 | **OS FS 스냅샷 미사용**; 단일 패스 walk + `captured_at` 기록. TOCTOU 폐쇄는 U3 전송 시 blob 재해시(FR-22/Q8=A)로 위임 | **예** | OS별 스냅샷 API(APFS/LVM/VSS)는 이식성·범위 부담. U3 재검증이 이미 계약됨(노트1) |
| D12 | 스캔/해시 I/O 오류 정책 | **fail-fast**: 전체 `build` 중단(`BuildError`), 파일 **스킵 안 함** | 아니오 | 파일 스킵은 diff에서 "삭제"로 오인되어 파괴적. 사이클 중단 후 다음 사이클 재시도가 안전(FQ-2=A 재스냅샷) |
| D13 | SafetyLimits 값 소스 | **U0 상수 참조**(`MAX_VAULT_TOTAL_BYTES`/`MAX_FILE_BYTES`/`MAX_FILE_COUNT`) | 아니오 | Q2=A 컴파일타임 상수, U0 소유(드롭리스트). 재정의 금지 |
| D14 | 경계 판정 규칙 | `actual <= limit` -> accept, `actual > limit` -> reject(경계값 자체는 **수용**) | 아니오 | NFR-14/US-E7-07 경계-정확(off-by-one 없음) |
| D15 | SafetyLimits 위반 처리 | **첫 위반에서 short-circuit**(모든 위반 누적 안 함) | **예** | MVP 가이던스. 단일 위반 리포트로 충분(FR-05 actionable) |
| D16 | SafetyLimits 검사 순서/오버플로 | **단일 패스**: count(O(1)) -> 엔트리 순회하며 per-file 검사 + total `saturating_add` 누적 | 아니오 | 결정적·O(n)·`u64` 오버플로 panic 방지(panic-free-total) |
| D17 | 빈 볼트(0 파일) 매니페스트 | **유효값으로 정상 산출**(파괴적 "전부 삭제" 판정은 U1 관심사 아님) | 아니오 | U0 `Manifest` 0-엔트리 유효 불변식 승계. US-E1-06 vault-unavailable 판정은 U2 `VaultAvailabilityGuard` |
| D18 | 엔트리 정렬 키 | `relative_path` 바이트 사전식 오름차순 1차, 이론적 tie-break `(raw_sha256, size)` | 아니오 | U0 `ManifestEntry` derive `Ord`와 일치(정상 볼트에서 경로 유일) |
| D19 | PBT 프레임워크 | **proptest**(Rust), U0 `proptest_support` 제너레이터 재사용 | 아니오 | US-E7-10/PBT-09 확정, NFR-17. `arb_manifest_entry`/`arb_relative_path` 등 기존 자산 활용 |

---

## 4. MANDATORY 카테고리 N/A + 확장 컴플라이언스

### 4.1 MANDATORY 산출물 카테고리 N/A

| 카테고리 | 이 단위 적용 | 판정 근거 |
|---|---|---|
| Frontend/UI 산출물 | **N/A** | U1은 순수 라이브러리 코어 — UI/화면/CLI 표면 없음(운영 표면은 U6/U7b) |
| API(외부 HTTP) 설계 | **N/A** | 네트워크 I/O 없음(전송은 U3/U5) |
| 데이터 지속 스키마 | **N/A(참조만)** | 지속은 U4 소유. U1은 값을 산출만 하고 저장하지 않음 |
| Mermaid/ASCII 다이어그램 검증 | **적용** | 화살표 표기(`A -> B`)만 사용, 박스/유니코드 글리프 없음 — content-validation 준수 |

### 4.2 확장 컴플라이언스 (완료 게이트)

| 확장 | 활성 | 이 단위 적용 판정 | 근거 |
|---|---|---|---|
| **Resiliency Baseline** | ON | **부분 적용** | (a) D12 fail-fast로 부분/오손 매니페스트가 파괴적 diff를 만들지 않음, (b) D3/NFR-02 스트리밍으로 대용량에서 OOM 크래시 회피, (c) U1 산출 `Manifest`가 U4 zero-loss 지속(RPO=0)의 입력. RTO/RPO 수치·배포/롤백·HA/DR은 U1(순수 로직)에 **N/A**(U4/인프라 소관) |
| **Property-Based Testing** | ON(Full) | **강제·준수** | `business-rules.md`/`business-logic-model.md`에 PROP-U1-01~05 식별(카테고리 라벨·제너레이터 요구 기재), proptest 채택(D19). 속성 없는 요소(VaultScanner I/O)는 예제/통합 테스트로 판정 |
| **Security Baseline** | OFF | **N/A** | 미로딩·미강제. RISK-01(로컬 평문 산출물)은 문서화된 수용 위험 — U1은 토큰/시크릿을 다루지 않음 |

**블로킹 파인딩 없음** — 모든 적용 가능한 활성 확장 규칙이 산출물에서 준수되거나 N/A로 판정됨.

---

## 5. 드롭리스트 (이미 확정 — 재설계·재질문 금지)

| 항목 | 확정값 | 출처 |
|---|---|---|
| CoreTypes 값 타입 전체 | U0 소유·구현 완료 | U0 `foundation` 크레이트(검증 완료) |
| CBOR 무손실 코덱 `encode`/`decode` | NFR-13 round-trip 보장 | U0 `core_types::codec` |
| `RelativePath` 정규화 규칙 | POSIX 슬래시·상대·`..`/`.` 거부·바이트 보존 | U0 `core_types::path` (구현됨) |
| SafetyLimits 상수 값 | 총 <=20 GiB / 파일당 <=2 GiB / 파일 수 <=100k | U0 `config::model` 상수, Q2=A |
| FQ-1=A | okc 콘텐츠 주소 스킴 재현 없음, `vault_content_id`는 서버 소유 | 상위 확정 |
| FQ-2=A | 최신-상태-대체(재스냅샷 diff, 이벤트 큐 아님) | 상위 확정 |
| 관측 push 소유 | U8 `SyncCycleCoordinator`(U1은 판정/값만 반환) | components.md §0 노트2 |
| 마지막 커밋 매니페스트 지속 | U4 `SyncStateStore`(diff에 인자 전달) | unit-of-work.md §U1/§U4 |
| 전송 시 blob 재검증(TOCTOU) | U3가 `hash_stream` 재사용 | components.md §0 노트1/Q8=A |
| PBT 프레임워크 = proptest | US-E7-10/PBT-09 | requirements NFR-17 |

---

## 6. 다음 단계

U1 Functional Design 3개 산출물 완료. 다음 스테이지는 **U1 NFR Requirements**(NFR-02 스트리밍 버퍼 수치·NFR-08/12/13/14 성능·기술 스택 proptest 확정 등)이며, 이어 NFR Design -> (Infrastructure Design은 순수 코어라 대체로 N/A) -> Code Generation 순으로 진행한다.

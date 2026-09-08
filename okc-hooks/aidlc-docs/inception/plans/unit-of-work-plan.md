# 작업 단위(Units of Work) 생성 계획 — okc-hooks "Watcher"

**단계**: INCEPTION → Units Generation (Part 1: 계획 + 명확화 질문)
**역할**: Solution Architect (단위 분해 담당)
**작성일**: 2026-09-08
**상태**: 아래 §4 질문의 `[Answer]:` 태그에 답해 주시면, 계획을 승인받은 뒤 필수 산출물 3종을 생성합니다. **답변·승인 전에는 산출물을 생성하지 않습니다.**

> 이 계획은 승인된 execution-plan.md(§4 U1..U7 분해, 스토리 매핑 44/44 검증) + Application Design 산출물(components.md 30개 컴포넌트, Foundation·Orchestration 계층 신설) + FQ-1/2/3=A/A/A 단순화를 **정합(reconcile)**한 결과입니다. Units Generation 규칙(`units-generation.md`)이 요구하는 6개 질문 카테고리를 모두 평가했고, **이미 확정·검증된 사항은 §1 기준선으로 확정**하고 **사람이 결정해야 하는 사안만** §4 질문(UQ-1/UQ-2/UQ-3)으로 남겼습니다.

---

## 1. 분해 기준선 (정합 결과 — §4 답변으로 변경 가능)

### 1.1 왜 execution-plan의 7단위를 다시 손봐야 하는가

승인된 execution-plan §4는 논리 단위 **U1..U7**(7개)로 스토리를 44/44 매핑했습니다. 그런데 그 이후 Application Design에서 두 가지가 바뀌었습니다:

1. **공유 Foundation 계층 신설** — `CoreTypes`(공유 타입·에러 taxonomy·무손실 코덱·상태 어휘·싱크 계약 트레이트) + `ConfigProvider`(단일 JSON 설정). 모든 단위가 참조하지만 U1..U7 어디에도 속하지 않음. 조립 루트가 소유해 아래로 주입 → 비순환 유지.
2. **Orchestration 계층 신설** — `WatcherDaemon`(조립 루트/단일 인스턴스 락/생명주기) + `SyncCycleCoordinator`(트리거당 1회 직렬 사이클). 이 역시 U1..U7에 없는 "접착(glue)" 컴포넌트.

또한 FQ-1/2/3=A로 아키텍처가 단순화됐습니다: okc-core 코드 의존성 0(표준 SHA-256), 이벤트별 지속 큐 제거(최신-상태-대체), U4가 `DurableQueue`→`SyncStateStore`로 축소.

→ 따라서 Units Generation의 실질은 **"30개 컴포넌트를 몇 개의 빌드 단위로 묶고, 그 코드 조직(크레이트) 전략을 확정하는 것"**입니다.

### 1.2 확정된 분해 (9개 그룹 — 30개 컴포넌트, 스토리 44/44)

| # | 단위 그룹 | 컴포넌트(components.md 기준) | 대표 스토리 | 요지 |
|---|---|---|---|---|
| **F** | **Foundation** | `CoreTypes`, `ConfigProvider` | (교차 관심사, 대표 스토리 없음)* | 공유 타입/에러 taxonomy/무손실 코덱/상태 어휘/싱크 계약 + 단일 JSON 설정 |
| **U1** | Deterministic Content Core | `ContentAddressing`, `VaultScanner`, `ManifestBuilder`, `ManifestDiffer`, `SafetyLimitsValidator` | E7(해시/매니페스트/diff) | 표준 SHA-256 지문 + 일관 스냅샷 + 매니페스트 + diff + 프리플라이트 |
| **U4** | Resilience / Retry | `SyncStateStore`, `RetryBackoffController` | E3(무손실 상태) | 마지막-커밋 매니페스트 + dirty 표시 + 재개 오프셋; 백오프/오프라인 재시도 |
| **U5** | Auth & Consent | `AuthTransport`, `CredentialProvider`, `ConsentGate` | E4(인증·동의) | TLS+토큰+응답분류, 토큰 소스(config 평문/opt-in 보안저장소), RISK-01 동의·철회 |
| **U2** | Change Detection & Trigger | `FilesystemWatcher`, `ReconciliationScheduler`, `VaultAvailabilityGuard`, `SingleInstanceLock` | E2(감지·트리거) | 크로스플랫폼 감시+디바운스, 재조정 스케줄, 빈-볼트 가드, 단일 인스턴스 |
| **U3** | Upload Protocol Client | `UploadProtocolDriver` | E1(업로드) | have/want→재개 전송→커밋→진행률 |
| **U6** | Observability | `StructuredLogger`, `StatusService`, `UploadHistoryStore`, `CriticalErrorNotifier`, `TrayIndicator`(선택) | E5(관측) | 구조화 로그(주 표면)/상태·헬스/히스토리/중대오류/트레이(선택) |
| **U7** | Lifecycle & CLI | `ServiceManager`, `AutoUpdater`, `Uninstaller`, `RunStateController`, `OperatorCli`, `ControlPlane` | E6(생명주기·운영) | 서비스 설치·자동시작, 자동 업데이트/롤백, 제거, 실행상태 제어, CLI + 로컬 IPC |
| **O** | Orchestration | `WatcherDaemon`, `SyncCycleCoordinator` | (조립/접착, 대표 스토리 없음)* | 조립 루트 + 단일 인스턴스 락 + 생명주기 + 트리거당 1회 직렬 사이클 |

*Foundation·Orchestration은 특정 사용자 스토리를 "소유"하지 않는 교차/접착 단위입니다. 모든 E1–E7 스토리가 이들을 관통하지만, 스토리→단위 매핑의 **1차 소유(primary owner)**는 U1..U7이 유지합니다 → 44/44 보존.

### 1.3 스토리 매핑 정합 (44/44 유지)

execution-plan §4 매핑(U1=8, U2=5, U3=8, U4=8, U5=6, U6=5, U7=4)을 그대로 승계하되, FQ-3=A 부록에서 조정된 3건만 반영합니다:

- **US-E7-01**: okc 스킴 재현 → 표준 SHA-256(sha256sum). 소유 U1 유지.
- **US-E3-01/02**: 큐 상태머신 → `SyncStateStore` 상태머신(최신-상태-대체). 소유 U4 유지(coalesce 개념 US-E3-02는 "대체"로 재해석).
- **US-E7-06**: components.md는 `CoreTypes`(Foundation)에 배정, execution-plan §4는 U1에 배정 — **경미한 차이**. 산출물에서는 **Foundation 소유로 통일**(무손실 코덱은 공유 타입이므로)하되, U1과의 인접성을 story-map에 주석으로 명시. (44/44 총계 불변)

### 1.4 빌드/구성 순서 — 선형 체인이 아니라 **의존성 DAG(4개 웨이브)**

독립 검토 패널(workflow wf_5cab8e63-f42)이 실제 단위 의존성 그래프를 검증한 결과, 제가 처음 적었던 **선형 체인**(`F→U1→U4→U5→U2→U3→U6→U7→O`, 9단계)은 **과도하게 보수적**이었습니다. `[Foundation-contract]` 엣지(로그/상태 싱크)를 Foundation으로 해석하면 실제 의존성은 **4개 층(웨이브)** 뿐입니다(순환 0):

```
Wave 1:  Foundation(F)
Wave 2:  U1 · U2 · U4 · U5 · U6      ← 5개 단위가 서로 독립(모두 Foundation에만 의존) = 최대 병렬 폭 5
Wave 3:  U3 (←U1,U4,U5)  ·  U7 (←U5,U6)
Wave 4:  Orchestration(O) (← 전부)
```

선형 체인의 8개 엣지 중 **5개가 가짜 의존성**입니다(U1→U4, U4→U5, U5→U2, U2→U3, U3→U6 — 각 대상은 Foundation만 있으면 시작 가능). 임계 경로 깊이는 9가 아니라 **4**(약 2.25× 단축). 즉 병렬성은 **단위 개수가 아니라 이 DAG가 결정**하며, 아래 UQ-1의 병합/분할 선택과 **독립적으로** 확보됩니다. 단일 배포 바이너리(RESILIENCY-01)는 모든 변형에서 유지 — 각 단위는 라이브러리 크레이트, `watcher-bin`이 링크(UQ-2=B).

**중요한 구분 — 무엇이 병렬화되나:**
- **런타임 병렬**: 동기화 파이프라인은 **의도적으로 단일 직렬 사이클**(Q2=B/FQ-2=A, 큐 없음). 여기에 병렬을 넣는 건 승인된 결정을 되돌리는 것이라 **하지 않습니다**. 런타임 동시성은 직렬 사이클과 공존하는 백그라운드 스레드 2개(FilesystemWatcher 감시 + ControlPlane IPC 서버)뿐 = 총 3스레드.
- **개발/빌드 병렬**: 위 DAG대로 Wave 2의 5개 단위를 **동시에 설계·구현·테스트** 가능. 다만 AIDLC는 단위마다 Functional Design→NFR→Code Gen에 **사람 승인 게이트**가 있어, 1인 개발자에겐 게이트 자체는 직렬입니다. 병렬성을 실제 작업으로 살리는 길은 (a) 빌드/작업 순서를 이 DAG로 잡는 것, (b) 원하면 독립 단위들의 Functional Design/Code Gen을 **병렬 에이전트로 팬아웃**하는 것.

### 1.5 규칙상 6개 질문 카테고리 평가 결과

`units-generation.md` Step 3은 6개 카테고리를 모두 평가하라고 요구합니다. 결과:

| 카테고리 | 상태 | 근거 |
|---|---|---|
| Story Grouping | **확정·검증됨** | execution-plan §4에서 44/44 매핑, Workflow Planning 크리틱 PASS(0 gaps/dups) |
| Dependencies | **확정·검증됨** | Application Design component-dependency.md에서 순환 0, 빌드순서 정합 크리틱 PASS |
| Business Domain | **확정·검증됨** | E1–E7 에픽 = 도메인 경계, personas P1/P2 확정 |
| Technical Considerations | **UQ-1/UQ-2 질문** | 단위 입도(granularity) + 코드 조직(크레이트) = 사람 결정 필요 |
| Code Organization | **UQ-2 질문 (필수 그린필드 전략)** | Rust 크레이트 레이아웃 = 그린필드 필수 결정 |
| Team Alignment / per-unit 배포·스케일 | **N/A** | 단일 개발자, 단일 배포 바이너리(RESILIENCY-01) → 팀 정렬·단위별 배포 무의미 |

---

## 2. 실행 체크리스트 (계획 승인 후 Part 2에서 수행)

- [x] §4 답변(UQ-1/UQ-2/UQ-3) + §1 기준선 반영해 최종 단위 목록 확정
- [x] `unit-of-work.md` 생성 — 단위 정의 + 책임 + (그린필드) 코드 조직 전략
- [x] `unit-of-work-dependency.md` 생성 — 단위 간 의존성 매트릭스 + 빌드 순서 + 통신 패턴(검증된 다이어그램 + 텍스트 대안)
- [x] `unit-of-work-story-map.md` 생성 — 스토리 → 단위 매핑(44/44 커버리지, 미매핑 0 검증)
- [x] 완전성 검증(스토리 44/44 커버, 순환 0, 빌드순서 정합) — 필요 시 완전성 크리틱
- [x] 활성 확장 컴플라이언스 요약(Resiliency ON / PBT ON / Security OFF; PBT·Security는 Units Generation에 N/A, Resiliency는 대체로 N/A)
- [x] aidlc-state.md 갱신 + 이 계획 단계 완료 표시 + 표준 2-옵션 완료 게이트 제시

## 3. 필수 산출물 (Units Generation 규칙)

- [x] `aidlc-docs/inception/application-design/unit-of-work.md`
- [x] `aidlc-docs/inception/application-design/unit-of-work-dependency.md`
- [x] `aidlc-docs/inception/application-design/unit-of-work-story-map.md`

---

# 4. 결정이 필요한 질문

각 질문의 `[Answer]:` 뒤에 보기 문자를 적어 주세요. 질문마다 제 권장안을 표시해 두었습니다. 맞는 보기가 없으면 **X) 기타**를 고르고 설명을 적어 주세요. 한 번에 몰아서(예: "UQ-1=A, UQ-2=B, UQ-3=A") 답하셔도 되고, **"전부 권장안대로"** 라고만 하셔도 됩니다.

## UQ-1 — 단위 입도: 병합? 유지? 분할? (technical-considerations)

**왜 중요한가**: "너무 잘게 쪼개졌나"를 독립 패널(블라인드 제안 3건 + 종합)로 검증했습니다. 결론은 **"대체로 적정, U7만 예외"**였고, 그 반대 방향(대폭 병합)은 병렬성·테스트 격리를 오히려 해칩니다. 아래 3개 안 중 하나를 고르세요. 각 단위는 Construction 루프(FD→NFR→CodeGen, 게이트 포함) 1회 = 1인 개발자에겐 게이트가 곧 직렬 비용입니다.

**패널 판정 요약**: 얇은 단위 F(2)/U4(2)/U3(1)/O(2)는 **원칙적 이유로 얇음**(F=임계경로 루트·싱크계약 역전 보존, U4=단일 "장애 복구" 테마, U3=의존 합류점의 복잡한 프로토콜 상태머신, O=최고 심사도 조립루트+직렬사이클). 진짜 결함은 **U7(6개)** — 두 일을 겸함: `{ServiceManager, AutoUpdater, Uninstaller}` = 바이너리 생명주기 vs `{OperatorCli, ControlPlane, RunStateController}` = 실행 중 데몬 제어.

A) **10단위 — U7 분할 (패널 권장)**: U7을 **U7a 배포/업데이트**(ServiceManager, AutoUpdater, Uninstaller) + **U7b 운영 제어**(OperatorCli, ControlPlane, RunStateController)로 분할. 덤으로 SingleInstanceLock을 U2→O로 이동(무료·응집도↑). Foundation→U0, Orchestration→U8 승격 포함. **(권장)**
   - 이유: U7의 "두 일"을 서로 다르게 변경·설계되는 두 응집 단위로 분리 → 각 게이트가 작고 독립적. 게이트 +1, 임계경로 웨이브 4→5(1인 개발자에겐 게이트 수가 관건, 웨이브 깊이는 거의 무의미).

B) **9단위 — 현행 유지**: §1.2 그대로(F + U1..U7 + O). 패널의 "대체로 적정" 그대로 수용, U7의 두-일은 Functional Design 단계에서 내부 모듈로 정리.
   - 이유: 게이트 수 최소화하면서 얇은 단위들의 원칙적 근거 유지. U7 결함은 감수(내부 정리).

C) **7단위 — 전송 클러스터 병합 (게이트 최소, 단 비용 있음)**: U3+U4+U5를 "Sync Transport & State" 1개로 병합(6개 컴포넌트). Construction 루프 7회.
   - **패널이 명시적으로 비권장**: U4·U5가 Wave 2 팬아웃에서 빠져나와 **최대 병렬 폭 5→3**으로 줄고, 서로 다른 PBT 테스트 프로파일 4종(네트워크 I/O·파일시스템 크래시 주입·백오프 상태머신·프로토콜 상태머신)이 **한 게이트에 뭉쳐** 밀도만 높아짐. 게이트 2개 아끼는 대가치고 손해. (원하시면 이 트레이드오프를 감수하고 선택 가능)

X) 기타 (아래 [Answer]: 태그 뒤에 설명)

**권장**: **A(10단위)**. "너무 잘게 쪼갰나"는 질문의 정직한 답은 **"오히려 U7이 덜 쪼개져 있었다"**입니다 — 나머지 얇은 단위는 병합하면 병렬성·PBT 격리를 잃습니다. 게이트 수를 정말 줄이고 싶으면 **B(9)**, 그것도 부족하면 **C(7)**이지만 C는 병렬 폭·테스트 격리 손실을 감수해야 합니다. **핵심: 병렬성은 §1.4 DAG가 결정하므로 어떤 안을 골라도 확보됩니다** — A/B/C는 "게이트 수 vs 응집도/격리"의 선택입니다.

[Answer]: A

## UQ-2 — 코드 조직: Rust 크레이트 레이아웃 (code-organization, 그린필드 필수 결정)

**왜 중요한가**: 규칙상 그린필드는 코드 조직 전략을 명시해야 합니다. 단일 배포 바이너리(RESILIENCY-01)라는 제약은 동일하지만, **소스를 어떻게 나누느냐**가 의존성 방향 강제·컴파일 시간·테스트 격리를 좌우합니다.

A) **단일 바이너리 크레이트 + 모듈**: 하나의 `Cargo.toml`, 단위별로 `src/u1/`, `src/u4/` … 모듈로 구분.
   - 장점: 가장 단순, 빌드 설정 1개. 단점: 모듈 간 의존 방향이 컴파일러로 강제되지 않음(실수로 U6→U1 역참조 가능), 증분 컴파일 이점 적음.

B) **Cargo 워크스페이스 + 단위별 라이브러리 크레이트 + 얇은 바이너리 크레이트**: `foundation`, `u1-content`, `u4-state` … 각 크레이트로, 최종 `watcher-bin`이 이들을 묶어 단일 실행파일 생성. **(권장)**
   - 장점: `Cargo.toml`의 `dependencies`가 **의존성 방향을 컴파일 타임에 강제**(비순환 그래프가 곧 크레이트 그래프 — U6이 U1을 import하려면 명시적 의존 추가가 필요해 역참조가 눈에 보임), 크레이트별 단위테스트·PBT 격리, 증분 컴파일. **여전히 단일 바이너리 산출**(RESILIENCY-01 충족).

C) **하이브리드**: 핵심 로직을 라이브러리 크레이트 1개(`watcher-core`)에 몰고 + 얇은 바이너리 크레이트. 단위는 core 크레이트 안의 모듈.
   - B와 A의 절충: 크레이트 2개만, 단위 경계는 모듈.

X) 기타 (아래 [Answer]: 태그 뒤에 설명)

**권장**: **B** — 이 프로젝트의 최대 리스크는 "의존성 순환·역참조"이고(크리틱이 반복 지적), 워크스페이스는 그걸 **컴파일러가 강제**하게 만듭니다. 단위 = 크레이트로 1:1 대응돼 Units Generation 산출물과 코드 구조가 정확히 일치. 단일 바이너리 제약도 `watcher-bin`이 전부 링크하므로 그대로 충족. (UQ-1=A와 자연스럽게 맞물림: F=`foundation` 크레이트, O=`watcher-bin`.)

[Answer]: B

## UQ-3 — 빌드/작업 순서 = DAG 웨이브 확인 (dependencies)

**왜 중요한가**: §1.4의 4-웨이브 DAG를 Construction 작업 순서의 기준으로 확정할지 확인합니다(선형 체인 폐기).

A) **확정**: §1.4의 4-웨이브 DAG(`F → {U1·U2·U4·U5·U6} → {U3·U7} → O`)를 작업 순서 기준으로 채택. 1인 개발자는 웨이브 순서대로 진행하되, 원하면 Wave 2의 독립 단위 설계/코드생성을 병렬 에이전트로 팬아웃. **(권장)**

B) 예전 선형 순서(`F→U1→U4→U5→U2→U3→U6→U7→O`)를 그대로 쓰고 싶음

X) 기타 (아래 [Answer]: 태그 뒤에 설명)

[Answer]: A

---

> **다음 단계**: UQ-1/UQ-2/UQ-3을 받으면 §1 기준선 + 답변으로 §3 필수 산출물 3종을 생성하고, 표준 2-옵션 완료 메시지(🔧 변경 요청 / ✅ 승인 후 CONSTRUCTION PHASE 진행)로 게이트를 제시합니다. **답변·승인 전에는 생성하지 않습니다.**

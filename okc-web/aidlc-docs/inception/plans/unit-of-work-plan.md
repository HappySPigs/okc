# Unit of Work Plan — okc-web (Obsidian Vault 통합 웹 플랫폼)

**Stage**: INCEPTION / Units Generation — Part 1 (Planning)
**Depth**: Standard (execution-plan.md 결정)
**Date**: 2026-09-08
**Grounding**: `execution-plan.md`(7-unit U0–U6 분해·시퀀스·유닛별 depth), `application-design.md`(모듈 맵·의존성·epic→unit·screen→story 매트릭스), `components.md`/`services.md`/`component-dependency.md`, `stories.md`(29 stories/5 epics), `requirements.md`

> **✅ 답변 상태**: 사용자의 기존 선호("추천값으로 채워줘", 해커톤 속도 우선)에 따라 **각 질문에 권장값을 미리 채워** 두었습니다. 유닛 분해는 **이미 승인된 execution-plan.md + application-design.md로 사실상 확정**되어 있어, 아래 질문은 대부분 그 결정의 확인입니다. 바꿀 항목의 `[Answer]:` 만 수정하시고, 그대로 좋으면 "승인"이라고 알려 주세요.

---

## 참고 — 확정된 7-유닛 (execution-plan + application-design)

| 유닛 | 모듈 | Epic | 책임 | 배포 |
|---|---|---|---|---|
| **U0** Foundation | `adapter`(+`queue`) · `shared`(authz/error/jobs/audit/state) | cross | 유일 okc-interop 링크점, 단일-writer 큐, RBAC 가드, OkcError→HTTP, JobStore, 감사, SQLite | (단일 프로세스 내) |
| **U1** Auth | `auth` | E1 | 계정·역할·세션·비밀번호 해시 | 동일 |
| **U2** Upload | `upload` | E2 | 토큰 발급/폐기, 토큰 인증 업로드, 검증, 소스 landing, ≤10 캡 | 동일 |
| **U3** Orchestration | `orchestration` | E3 | 프로젝트 수명주기·freeze·checkpoint 루프·compile·staleness·`sources` 레지스트리 | 동일 |
| **U4** Review | `review` | E4 | taxonomy/cluster 결정 표면, DecisionGate(Major/Critical→422), regenerate | 동일 |
| **U5** Serving | `serving` | E5 | 읽기전용 compiled-vault API, verify/explain, publish, mcp 계약 | 동일 |
| **U6** Frontend | `web`(Next.js) | cross | API 소비층(apiClient·okcErrorMap·queryKeys·useJobPolling), 화면 배선 | 프론트 |

**시퀀스**: U0 → U1 → U2 → U3 → U4 → U5, **U6 인터리브**(각 유닛 API가 나오면 해당 화면 배선).

---

## A. 분해 결정 질문 (Part 1)

## Question 1 — 스토리 그룹핑 전략은? (Story Grouping)
A) **Epic/역량 정렬** — U1–U5를 5개 epic(E1–E5)에 1:1, U0=플랫폼 기반(cross-cutting), U6=프레젠테이션(cross-cutting). 이미 screen→story 매트릭스가 이 경계로 정렬됨 — 권장

B) 기술 계층별(handler/service/repo) 그룹핑 — 유닛 경계와 교차, 추적성 약화

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 2 — 유닛 간 통신·의존 패턴은? (Dependencies)
A) **인-프로세스 Rust 모듈**(단일 axum 크레이트) — 유닛 간 네트워크 호출 없음; 모든 변경성 엔진 op은 U0 단일-writer 큐 통과; 공유 상태는 U0 `shared::state`(SQLite) 경유; 유닛은 트레이트/타입 경계로만 결합 — 권장

B) 유닛별 마이크로서비스 + HTTP/gRPC — 해커톤 단일 프로세스 토폴로지엔 과함, PROJECT_RESERVATIONS 직렬화와 상충

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 3 — 소유·팀 경계는? (Team Alignment)
A) **솔로 해커톤** — 팀 분할 없음; 소유 경계 = 모듈 경계(C1/C6 추적성 목적으로만 명시) — 권장

B) 유닛별 팀 배정 — 해당 없음(1인 개발)

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 4 — 배포 모델은? (Technical Considerations)
A) **단일 배포 프로세스**(axum 바이너리 1개 + Next.js 프론트) — 유닛은 논리 모듈; 유닛별 독립 배포 없음(모놀리스). okc-interop scheduler/PROJECT_RESERVATIONS가 프로세스-글로벌이라 단일 프로세스가 정합 — 권장

B) 유닛별 독립 배포 — 코어 동시성 모델과 상충, 데모 복잡도↑

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 5 — 도메인/바운디드 컨텍스트 경계는? (Business Domain)
A) **역량 기반 바운디드 컨텍스트** — identity(U1) · ingestion(U2) · integration-orchestration(U3) · curation-review(U4) · serving/provenance(U5) · platform-foundation(U0) · presentation(U6). 각 컨텍스트는 자기 데이터/규칙 소유, U0가 공유 인프라 — 권장

B) 단일 컨텍스트(경계 없음) — 모듈화·유지보수성(C6) 약화

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 6 — 코드 조직/디렉터리 구조는? (Code Organization — Greenfield)
A) **모노레포 준비형 단일 크레이트 백엔드** — `backend/src/{adapter,shared,auth,upload,orchestration,review,serving}/` (유닛↔모듈 1:1) + `frontend/`(Next.js App Router). 향후 모노레포 편입 시 `okc-web/` 하위로 이동(module-integration-guide §4와 정합). code-generation.md 패턴 준수 — 권장

B) 계층형 디렉터리(`handlers/`,`services/`,`repos/`) — 유닛 경계와 교차, 추적성↓

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 7 — 유닛 구현 순서는? (Build Sequence)
A) **U0 우선(기반) → U1 → U2 → U3 → U4 → U5 (의존 순), U6 인터리브** — 각 유닛의 API가 나오는 즉시 해당 화면 배선. 데모 스파인(로그인→업로드→통합→리뷰→서빙)을 조기에 관통 — 권장

B) 화면 우선(프론트 먼저) — 백엔드 계약 미확정 상태 배선은 재작업 유발

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

---

## B. 유닛 산출물 생성 체크리스트 (Part 2에서 실행)

> 승인 후 Part 2(Generation)에서 실행. 각 항목은 생성 시 [x]로 표시.

- [x] `aidlc-docs/inception/application-design/unit-of-work.md` — 7-유닛 정의·책임·경계 + **Greenfield 코드 조직 전략**(디렉터리 구조, 유닛↔모듈 1:1, 모노레포 준비) — 25.9KB
- [x] `aidlc-docs/inception/application-design/unit-of-work-dependency.md` — 유닛 의존성 매트릭스 + 통신 패턴(Mermaid + 텍스트 대안) + 빌드 순서 — 16.4KB
- [x] `aidlc-docs/inception/application-design/unit-of-work-story-map.md` — 29 스토리 → 7 유닛 매핑(모든 스토리 배정 보장, screen→story 매트릭스 근거) — 16.9KB
- [x] 유닛 경계·의존성 검증(순환 없음, U0 기반 단방향) — verifier acyclic_ok=true
- [x] 모든 스토리가 유닛에 배정됐는지 확인(누락 0) — verifier coverage_ok=true, 29/29

### 검증 & 정합 (standard) — 완료 (adversarial verifier wf_9cd09efe-6b8)
- [x] execution-plan 7-유닛 분해 및 application-design 모듈 맵과 일치 — consistency_ok=true
- [x] 유닛별 depth(execution-plan: U3/U4 comp · U1/U2 std · U5/U6 skip Functional Design) 반영 — 산출물에 명시
- [x] 6-criteria 게이트 self-check(C1 추적성·C6 모듈화) + state↔disk 정합 — codeorg_ok=true; 지적 1건(E1-S5 U6 지원 누락) 반영

---

_승인 시 다음_: Part 2(Generation) → 3개 유닛 산출물 생성(standard) → 완료·승인 게이트 → CONSTRUCTION PHASE.

# User Stories — Generation Plan (Part 1: Planning)

Role: product owner. This plan defines HOW we will convert the approved requirements ([`../requirements/requirements.md`](../requirements/requirements.md), REQ-001..013 + 5 success criteria) into user stories and personas.

> **AUTOPILOT DECISION (2026-09-08)**: Per the user's standing instruction (autopilot mode; recommended-but-MVP-scoped choices), all questions below are answered with the recommended defaults. **Q8=A** (accept all recommended defaults: Q1=E, Q2=A, Q3=B, Q4=A, Q5=A, Q6=A, Q7=A). No ambiguities to resolve; plan self-approved and executed. Scope discipline: generation will treat anything beyond the first-Unit MVP as a Follow-up stub, not an active story.

Assessment: [`user-stories-assessment.md`](user-stories-assessment.md) — decision = Execute (Yes).

---

## Execution Checklist (to run in Part 2, after approval)

- [x] Generate `user-stories/personas.md` — user archetypes and characteristics
- [x] Generate `user-stories/stories.md` — INVEST stories with acceptance criteria
- [x] Ensure stories are Independent, Negotiable, Valuable, Estimable, Small, Testable (adversarial INVEST audit + repair pass)
- [x] Include acceptance criteria for each story (Given/When/Then)
- [x] Map personas to relevant stories
- [x] Tag each story with its requirement IDs (REQ-xxx) and the success criterion it serves
- [x] Separate first-Unit stories (21) from deferred follow-up stubs (6)
- [x] Mark each first-Unit story with a priority (MoSCoW)

---

## Proposed default approach (my recommendation)

- **Breakdown**: **Hybrid — persona-grouped, journey-ordered within each persona** (Q1 default = E below). This fits a multi-persona, journey-driven tool.
- **Personas**: three (new installer, existing-Vault author, change-reviewer/recoverer) plus OKC as an implicit downstream consumer (Q2).
- **Granularity**: user-task sized stories (one meaningful outcome each), not epics-only and not micro-steps (Q3 default = B).
- **Format**: `As a <persona>, I want <capability>, so that <benefit>` + Given/When/Then acceptance criteria (Q4 default = A).
- **Acceptance criteria**: Given/When/Then, testable, tied to REQ IDs and success criteria (Q5 default = A).
- **Scope**: first-Unit stories only in `stories.md`, with deferred journeys listed in a clearly separated "Follow-up Units" section (Q7 default = A).

You can accept all defaults at once via Q8.

---

## Questions

### Q1 — Story breakdown approach
어떤 방식으로 스토리를 묶고 정렬할까요?

A) User Journey-Based — 사용자 워크플로우 순서대로
B) Feature-Based — 시스템 기능 단위로
C) Persona-Based — 사용자 유형별 그룹
D) Domain-Based — 업무 도메인 단위로
E) **Hybrid: 페르소나로 그룹 + 각 그룹 내 여정 순서** (권장)

[Answer]: E (autopilot 권장값)

### Q2 — Personas 범위
어떤 페르소나를 정의할까요?

A) **3개: 신규 설치자 / 기존 Vault 저작자 / 변경 검토·복구자 + OKC(암묵적 다운스트림 소비자)** (권장)
B) 위 3개만 (OKC 소비자 제외)
C) 하나의 통합 "Vault 저작자" 페르소나로 단순화
X) Other

[Answer]: A (autopilot 권장값)

### Q3 — Story granularity (크기)
스토리 세분화 수준은?

A) Epic 중심(큰 단위) + 하위 스토리 최소
B) **User-task 크기 — 하나의 의미 있는 결과 = 하나의 스토리** (권장, INVEST의 Small에 적합)
C) 매우 세분화(마이크로 스텝 단위)
X) Other

[Answer]: B (autopilot 권장값)

### Q4 — Story format
스토리 서술 형식은?

A) **`As a <역할>, I want <기능>, so that <가치>`** (표준, 권장)
B) Job Story: `When <상황>, I want <동기>, so I can <기대결과>`
C) 자유 서술 + 요약 제목
X) Other

[Answer]: A (autopilot 권장값)

### Q5 — Acceptance criteria style
각 스토리의 인수 기준 형식은?

A) **Given/When/Then (검증 가능, REQ ID·성공기준 매핑 포함)** (권장)
B) 체크리스트형 불릿
C) 서술형 문단
X) Other

[Answer]: A (autopilot 권장값)

### Q6 — 우선순위 표기
첫 Unit 스토리에 우선순위를 어떻게 표기할까요? (요구사항 D4: 첫 Unit 최우선 여정 = 노트 내부 정돈)

A) **MoSCoW (Must/Should/Could/Won't)** (권장)
B) High/Medium/Low
C) 우선순위 표기 없음 — 단순 나열
X) Other

[Answer]: A (autopilot 권장값)

### Q7 — 후속 Unit 여정 처리
후속 Unit로 미뤄진 여정(파일 이동/이름변경/병합, 다중 Vault, 실제 OKC 수집 테스트 등)을 스토리 문서에서 어떻게 다룰까요?

A) **`stories.md`에 별도 "Follow-up Units" 섹션으로 스토리 스텁만 기록(비활성)** (권장 — 추적성 유지)
B) 아예 제외(첫 Unit 스토리만)
C) 첫 Unit 스토리와 동등하게 전부 작성
X) Other

[Answer]: A (autopilot 권장값)

### Q8 — 기본값 일괄 수용
위 권장 기본값(Q1=E, Q2=A, Q3=B, Q4=A, Q5=A, Q6=A, Q7=A)을 그대로 수용하시겠습니까?

A) 예 — 전부 권장값으로 진행 (이 경우 Q1~Q7 개별 답변 불필요)
B) 아니오 — Q1~Q7을 개별적으로 답하겠음

[Answer]: A (autopilot — 전부 권장값 수용)

### Q9 — 추가로 강조할 사용자 시나리오
스토리에 반드시 포함되어야 할, 위에서 놓친 사용자 시나리오나 엣지 케이스가 있습니까? (예: 잘못된 편집 후 복구, 한국어 검색, 손상된 YAML 발견)

[Answer]: X — 다음 first-Unit 시나리오를 반드시 포함(모두 MVP 범위 내): (1) 잘못된 편집 후 외부 백업에서 수동 복구, (2) 한국어 리터럴 검색, (3) 손상된 YAML 탐지·거부, (4) 해시 불일치(동시 변경) 시 쓰기 거부, (5) 부분 frontmatter 업데이트 시 미지 키·본문·주석 보존, (6) 품질 감사의 운영 노이즈·중복·링크 문제 플래그. 범위 확장(파일 이동/이름변경/병합, 다중 Vault, 실제 OKC 수집)은 Follow-up 스텁으로만.

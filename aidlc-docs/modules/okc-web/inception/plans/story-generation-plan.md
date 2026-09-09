# Story Generation Plan — okc-web (Obsidian Vault 통합 웹 플랫폼)

**Stage**: INCEPTION / User Stories — Part 1 (Planning)
**Role**: Product Owner
**Date**: 2026-09-08
**Grounding**: `requirements.md` (FR-AUTH/UP/INT/SRV, NFR, §9 데모 시나리오, §10 추적성) + `okc-core-capability-analysis.md`

> **✅ 답변 상태**: 사용자의 기존 선호("추천값으로 채워줘", 해커톤 속도 우선)에 따라 **각 질문에 해커톤 권장값을 미리 채워** 두었습니다.
> 바꾸고 싶은 항목의 `[Answer]:` 만 수정하시면 됩니다. 그대로 좋으면 "승인"이라고 알려 주세요.

---

## A. 스토리 방법론 질문 (Part 1)

## Question 1 — 어떤 페르소나를 스토리로 다룰까요?
requirements.md §3 기준 후보: Administrator/Curator, Contributor(개인 기여자), Viewer, okc-mcp 소비자(비인간).

A) **관리자 + 기여자 + okc-mcp 소비자 3종** (Viewer는 관리자/기여자로 흡수, 데모 핵심 경로에 집중) — 권장

B) 관리자 + 기여자 + 뷰어 + okc-mcp 소비자 4종 전부 (뷰어 조회 스토리까지 별도)

C) 관리자 + 기여자 2종만 (okc-mcp는 계약만, 스토리 없음)

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 2 — 스토리 분해(breakdown) 방식은?
A) **페르소나 × 에픽 하이브리드**: 페르소나별로 묶되 원 요구사항 ①–⑤에 대응하는 에픽으로 조직 — 권장 (추적성 유지)

B) 순수 User Journey 기반 (업로드→통합→리뷰→서빙 흐름 순서로만)

C) 순수 Feature 기반 (인증/업로드/통합/서빙 기능 모듈별)

D) 순수 Persona 기반 (페르소나별로만, 에픽 없음)

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 3 — 스토리 세분성(granularity)은?
A) **중간 — 사용자 스토리 레벨** (각 스토리는 하나의 사용자 목표, INVEST 준수, 해커톤 데모에 실장 가능한 크기) — 권장

B) 굵게 — 에픽 레벨 위주 (상세 스토리 최소화)

C) 잘게 — 태스크/서브스토리까지 분해 (구현 태스크 수준)

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 4 — 수용 기준(Acceptance Criteria) 형식은?
A) **Given/When/Then (Gherkin 스타일)** — 테스트 가능성 최상, 권장

B) 불릿 체크리스트 (간단)

C) 둘 다 (핵심 스토리는 GWT, 나머지는 불릿)

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 5 — 스토리 범위(scope)는?
A) **MVP 데모 경로 중심** (requirements.md §9 end-to-end 시나리오를 커버; deferred/federation/okc-mcp 내부는 스토리화하지 않고 out-of-scope로 표기) — 권장

B) 전체 (deferred·federation·멀티테넌시까지 미래 스토리로 포함)

C) MVP + 즉시 후속(near-term) 스토리까지 (federation 설계 방향 스토리 1~2개 포함)

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 6 — 우선순위 표기(prioritization)를 넣을까요?
A) **MoSCoW 태깅** (Must/Should/Could/Won't; Must=데모 경로) — 권장, 해커톤 집중에 유용

B) 우선순위 없이 스토리만 나열

C) 단순 P0/P1/P2 숫자 우선순위

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

## Question 7 — conflict 리뷰(원 요구사항 ③) 스토리를 어떻게 표현할까요?
requirements.md에서 "승자 선택"은 승인/waive/omission/regenerate로 재해석됨.

A) **재해석된 결정 표면 그대로 스토리화** (관리자가 클러스터 승인 / minor waive / omission 제안 / regenerate 하는 각각을 스토리로; okc-core 제약을 수용 기준에 명시) — 권장

B) 단일 "conflict 해소" 스토리로 뭉뚱그리고 세부는 수용 기준에만

C) 원 요구사항 문자 그대로 "승자 선택" 스토리로 작성 (⚠️ 모순 보존 원칙 위배: 정보를 조용히 버림 — ADR-0024; 승자 선택 API·계획 부재로 okc-core 신규 API도 필요 — 범위 밖)

X) 기타 (please describe after [Answer]: tag below)

[Answer]: A

---

## B. 스토리 개발 실행 체크리스트 (Part 2에서 실행)

> 아래는 승인 후 Part 2(Generation)에서 실행할 단계입니다. 방법론 확정용이며, 각 항목은 생성 시 [x]로 표시됩니다.

### 페르소나
- [x] `personas.md` 생성 — 페르소나별: 이름/역할, 목표·동기, 권한 경계, 기술 수준, 대표 시나리오, 관련 에픽 매핑 (Q1 선택 반영)

### 스토리 구조 (Q2 하이브리드: 페르소나 × 에픽 = 원 요구사항 ①–⑤)
- [x] Epic 1 — 인증 & 권한 (원 요구사항 ①②; FR-AUTH-1..4)
- [x] Epic 2 — 개인 Vault 업로드 & 토큰 (원 요구사항 ④; FR-UP-1..4)
- [x] Epic 3 — 통합 오케스트레이션 (원 요구사항 ①; FR-INT-1,2,7)
- [x] Epic 4 — Conflict/Critic 리뷰 & 해소 (원 요구사항 ③; FR-INT-3,4,5,6; Q7 반영)
- [x] Epic 5 — 병합 Vault 서빙 & okc-mcp 계약 (원 요구사항 ⑤; FR-SRV-1..3)

### 각 스토리 필수 요소 (INVEST)
- [x] `As a <persona>, I want <goal>, so that <benefit>` 형식
- [x] Independent, Negotiable, Valuable, Estimable, Small, Testable 준수
- [x] 수용 기준 (Q4 형식: Given/When/Then)
- [x] MoSCoW 우선순위 태그 (Q6)
- [x] 추적성: 대응 FR 및 원 요구사항 ①–⑤ 링크
- [x] okc-core 제약이 걸리는 스토리는 수용 기준에 제약 명시(예: Major/Critical waive 불가)

### 산출물 & 매핑
- [x] `stories.md` 생성 (에픽별 스토리 + 수용 기준 + 우선순위 + 추적성)
- [x] 페르소나 ↔ 스토리 매핑 테이블
- [x] 데모 수용 시나리오(§9)와 스토리 커버리지 매핑
- [x] out-of-scope 스토리 목록 (deferred/federation/okc-mcp 내부)

---

## C. 스토리 분해 접근법 트레이드오프 (참고)

| 접근법 | 장점 | 단점 | 적합성 |
|---|---|---|---|
| **페르소나×에픽 (권장)** | 권한 경계 명확 + 원 요구사항 추적 | 약간의 교차 | ✅ 다페르소나+추적성 요구에 최적 |
| User Journey | 흐름 직관적 | 페르소나 권한 경계 흐려짐 | 데모 흐름 설명엔 좋음 |
| Feature | 구현 모듈 정렬 | 사용자 가치 관점 약함 | 이후 Application Design서 유용 |
| Persona-only | 사용자별 명확 | 요구사항 추적 약함 | 단순 앱에 적합 |
| Epic-only | 큰 그림 | 테스트 가능 세부 부족 | 초기 스코핑용 |

---

_승인 시 다음_: Part 2(Generation) → `personas.md` + `stories.md` 생성 → 검토 게이트.

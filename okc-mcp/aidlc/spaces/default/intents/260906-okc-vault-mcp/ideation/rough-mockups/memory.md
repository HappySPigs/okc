<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-09-07T03:07:18Z — 이 제품은 화면 GUI가 아니라 설치형 로컬 MCP 도구 표면이므로 rough-mockups를 non-UI 분기로 실행한다; pixel wireframe 대신 시스템 상호작용 흐름과 정보 구조(starter Vault 폴더·metadata·링크/MOC) 스케치를 산출한다. scope-document와 intent-backlog가 starter 구조 구체화(폴더 역할·metadata·link/MOC)와 INT-03 충돌 복구 흐름을 이 단계로 명시적으로 미뤄 두었으므로 실행 대상이 분명하다.

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-09-07T03:29:42Z — starter 폴더 구조는 사용자가 추천을 위임해 하이브리드(소수 기능 폴더 + 주제 폴더)를 택했다. 유형별 폴더(C)는 노트 유형을 이미 `type` frontmatter로 표현하므로 중복이고, 순수 평면형(A)은 OKC 지식 축적 anchor(MOC 허브·수집함)를 못 준다는 근거. 전 구조는 강제 아닌 권고로 남겨 SCP-04의 "사용자 승인·재정의" 경계를 지켰다.
- 2026-09-07T03:29:42Z — INT-03 충돌 복구는 최소 재읽기 안내(Q4=A)를 택했다. 차이 요약·선택(재적용/유지/취소)·안전 사본은 첫 릴리스에서 제외하고 수요 확인 시 재검토로 남겼다. 사용자 대면 최소 흐름만 확정하고, stale 판정 메커니즘(R-02)은 Requirements로 유예.
- 2026-09-07T03:29:42Z — MCP 도구 표면은 능력별 개별 도구(Q6=A)로 하되, 승인된 범위·market-research 학습에 따라 connect가 경계·정책을 먼저 확인시키고 모든 응답에 대상 경로를 명시하는 동작을 얹었다. 세부 API 계약은 Inception 유예.
- 2026-09-07T03:29:42Z — advisory finding은 심각도 그룹(Q5=A)으로 표현하되, 그룹 방식과 무관하게 "compiler 미실행" 배너를 상시 유지해 INT-05·제품 경계(통과 판정 금지)를 지켰다.

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-09-07T03:29:42Z — R-02: 충돌 감지 메커니즘(무엇을 기준으로 stale 판정)과 쓰기 권한 정의는 Requirements/Contract Design에서 확정. 이 단계는 사용자 대면 흐름만 정함.
- 2026-09-07T03:29:42Z — R-01/INT-05: "OKC 입력 전 구조·링크·metadata 문제 감소" 지표의 baseline·target·관찰 기간·측정 방법은 Requirements Analysis로 이월(scope·backlog Open question 유지).
- 2026-09-07T03:29:42Z — INT-07 첫 지원 host·platform matrix는 실제 환경 증거 없이 확정 불가로 이월.

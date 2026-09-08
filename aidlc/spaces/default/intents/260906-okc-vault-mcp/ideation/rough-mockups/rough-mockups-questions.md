# Rough Mockups 질문 — OKC Local MCP

**단계:** Rough Mockups & Concept Visualization (ideation)
**깊이:** Standard
**입력:** [`intent-statement`](../intent-capture/intent-statement.md), [`scope-document`](../scope-definition/scope-document.md), [`intent-backlog`](../scope-definition/intent-backlog.md)

> 이 제품은 화면 GUI가 아니라 설치형 로컬 MCP 도구 표면이다. 따라서 이 단계는 pixel wireframe이 아니라 사용자가 MCP 클라이언트에서 겪는 상호작용 흐름과 Vault 정보 구조를 스케치한다. 아래 질문은 scope-document와 intent-backlog가 이 단계로 명시적으로 미뤄 둔 열린 항목(starter 구조, 충돌 복구 흐름 등)을 확정하기 위한 것이며, 이미 승인된 범위·경계는 다시 묻지 않는다.
>
> 참고: 아래 선택지는 이 단계에서 새로 제안하는 초안이며, 보존된 과거 draft 문서의 확정 답변이 아니다. 최종 결정은 당신의 선택으로 확정된다.

## Q1. Starter Vault 폴더 구조

새 Vault를 OKC에 유리하게 시작하도록 어떤 폴더 구성을 기본 starter로 추천할까? (근거·상태·결정·open item·MOC 등 OKC 지식 축적 관점)

- A. 최소 평면형 — 폴더 강제를 최소화하고 링크·MOC로만 조직 (사용자 통제 최대, 폴더 추천을 사실상 하지 않는 선택 포함)
- B. 주제(topic) 중심 폴더 — 도메인·주제별 폴더 + 각 주제의 MOC 노트
- C. 지식 유형 중심 폴더 — evidence·decision·status·open item 등 노트 유형별 폴더
- D. 하이브리드 — 소수의 유형 폴더(MOC·inbox)만 두고 나머지는 주제 폴더로 자유롭게
- X. Other (please specify)

[Answer]: 사용자가 추천을 요청함 → **D. 하이브리드** 추천. 근거: Q2에서 노트 유형을 `type` frontmatter로 표현하기로 했으므로 유형별 폴더(C)는 중복이 되고, 소수의 기능 폴더(예: `MOC/`, `inbox/`, `attachments/`)만 두고 나머지는 주제 폴더로 자유롭게 두면 OKC 지식 축적의 anchor(MOC 허브·수집함)를 주면서도 사용자 통제를 유지한다. 최종 요약 확인 단계에서 함께 검토.

## Q2. 노트 metadata(frontmatter) 기본 필드 (select all that apply)

starter 노트에 어떤 YAML frontmatter 필드를 기본 권고로 넣을까? (일관된 형식으로 지식이 축적되도록)

- A. 최소 — title, tags 정도만
- B. 상태 관리 — status(예: draft/active/resolved), created/updated 날짜
- C. 지식 성격 — type(evidence/decision/note 등), source·근거 링크
- D. OKC 연계 — aliases, 상위 MOC 링크, review 여부 표시
- X. Other (please specify)

[Answer]: B, C, D (상태 관리 + 지식 성격 + OKC 연계). A 단독 최소형은 제외 — 다만 이 필드들은 권고이며 강제하지 않는다.

## Q3. 링크·MOC 안내 수준

wikilink와 MOC(Map of Content) 구성을 제품이 어느 수준까지 능동적으로 제안할까? (사용자 통제를 잃지 않는 선에서)

- A. 제안만 — 관련 노트·누락 링크를 advisory로 알려 주되 자동 삽입하지 않음
- B. 선택 적용 — 링크·MOC 항목 추가를 제안하고 사용자가 고른 것만 반영
- C. 템플릿 제공 — MOC 노트·링크 규약 템플릿을 만들어 주고 이후는 사용자가 관리
- D. 안내 문서만 — 링크·MOC 모범 사례를 문서로만 제공하고 노트에는 개입하지 않음
- X. Other (please specify)

[Answer]: C. 템플릿 제공 — MOC 노트·링크 규약 템플릿을 만들어 주고 이후는 사용자가 관리.

## Q4. 충돌 거부 후 복구 흐름 (INT-03)

읽은 뒤 외부(Obsidian 등)에서 노트가 바뀌어 안전하게 쓸 수 없을 때, 거부 후 사용자에게 어떤 최소 복구 흐름을 제공할까?

- A. 재읽기 안내 — 무엇이 바뀌었는지 알리고 "다시 읽고 재시도"만 안내
- B. 차이 요약 — 이전에 읽은 내용과 현재 내용의 차이 요약을 보여 준 뒤 재시도
- C. 차이 + 선택 — 차이를 보여 주고 "내 변경만 다시 적용 / 현재 상태 유지 / 취소" 중 선택
- D. 안전 사본 — 내 변경을 별도 노트·섹션으로 저장하고 원본은 건드리지 않음
- X. Other (please specify)

[Answer]: A. 재읽기 안내 — 무엇이 바뀌었는지 알리고 "다시 읽고 재시도"만 안내(최소 흐름).

## Q5. Advisory readiness finding 표현 (INT-05)

점검 결과(path·frontmatter·link·structure 등)를 사용자에게 어떻게 보여 줄까? (compiler를 실제로 호출하지 않았음이 분명히 드러나야 함)

- A. 심각도 그룹 — 문제를 심각도(경고/정보)로 묶고 각 항목에 근거·영향·수정가능 여부 표시
- B. 유형 그룹 — 문제 유형(경로/메타데이터/링크/구조)별로 묶어 표시
- C. 노트별 그룹 — 노트 단위로 발견 항목을 모아 표시
- D. 요약 + 상세 — 상단에 요약(개수·통과 여부, "compiler 미실행" 명시) 후 항목별 상세
- X. Other (please specify)

[Answer]: A. 심각도 그룹 (경고/정보로 묶고 각 항목에 근거·영향·수정가능 여부 표시). 보완: 어떤 그룹 방식이든 결과 상단에 "실제 compiler를 호출하지 않은 advisory 점검"임을 항상 명시한다(INT-05·제품 경계).

## Q6. MCP 도구 표면과 응답 형식

능력(연결·읽기·검색·생성·append·부분수정·점검)을 MCP 도구로 어떻게 노출하고, 결과를 클라이언트에 어떤 형식으로 돌려줄까? (연결 시 경계·정책 확인 방식 포함. 세부 API 계약은 이후 Inception에서 설계)

- A. 능력별 개별 도구 — connect/read/search/create/append/update/check 등 세분화된 도구, 응답은 구조화된 요약
- B. 소수의 상위 도구 — 읽기계·쓰기계·점검계로 묶은 소수 도구 + 동작 파라미터, 응답은 요약+상세
- C. 개별 도구 + 연결 확인 우선 — 연결 시 resolved Vault 경로·허용 경계·정책을 먼저 확인시키고, 이후 도구별 응답에 항상 대상 경로를 명시
- D. 대화형 안내 — 도구는 최소화하고 결과·다음 행동을 자연어 요약 중심으로 반환
- X. Other (please specify)

[Answer]: A. 능력별 개별 도구 (connect/read/search/create/append/update/check 등 세분화, 응답은 구조화된 요약). 보완: 승인된 범위대로 connect가 resolved Vault 경로·허용 경계·정책을 먼저 확인시키고, 이후 모든 도구 응답에 대상 경로를 명시한다(명시적 단일 Vault 경계). 세부 API 계약은 Inception으로 유예.

## Consolidated Summary Confirmation

위 6개 답변을 확정하고 이 답변을 바탕으로 산출물(`wireframes.md`, `user-flow.md`)을 생성할지 확인합니다.

- Looks correct
- Request changes

[Answer]: Looks correct

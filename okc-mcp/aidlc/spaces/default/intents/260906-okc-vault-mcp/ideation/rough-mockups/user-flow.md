# Rough Mockups — OKC Local MCP 사용자 흐름

**단계:** Rough Mockups & Concept Visualization (ideation)
**성격:** 설치형 로컬 MCP 도구 표면의 시간 순서 상호작용 흐름. 정보 구조·도구 표면·점검 결과 표현은 [`wireframes`](wireframes.md)에 있다.
**입력:** [`intent-statement`](../intent-capture/intent-statement.md), [`scope-document`](../scope-definition/scope-document.md), [`intent-backlog`](../scope-definition/intent-backlog.md)
**확정 답변:** [`rough-mockups-questions`](rough-mockups-questions.md)

> 다이어그램은 stage-protocol.md의 ASCII 표준을 따른다(기본 ASCII 문자만, Unicode box-drawing 미사용).

---

## 1. 핵심 사용자 흐름 (happy path)

scope-document의 사용자 가치 흐름과 walking-skeleton 순서(VS-1~VS-4)를 상호작용 단계로 구체화한 것이다.

```
[사용자 / MCP 클라이언트]
      |
      v
(1) connect
      |   서버가 resolved Vault 경로 · 허용 경계 · effective policy를 확인 요청
      v
(2) 사용자가 경계 확인 · 승인
      |
      v
(3) read / search
      |   기존 노트를 탐색하고, read는 읽은 시점 상태를 기억
      v
(4) create / append / update
      |   starter 폴더 · frontmatter · MOC 템플릿 권고 적용은 사용자가 선택
      v
(5) 정상 Markdown으로 즉시 지속
      |   Obsidian 및 후속 MCP 세션에서 그대로 확인
      v
(6) check  (advisory)
      |   심각도 그룹 결과 + "compiler 미실행" 명시
      v
(7) 사용자가 수정안을 개별 선택
      |
      v
(8) 충돌 안전 조건으로 source Vault 반영
          선택하지 않은 내용과 원문은 유지 (INT-06)
```

| 단계 | 행위자 | 동작 | 결과 방향 | 추적성 |
|---|---|---|---|---|
| 1 | 사용자 | 단일 source Vault 연결 요청 | resolved 경로·경계·정책 제시 | SCP-01, INT-01 |
| 2 | 사용자 | 경계 확인·승인 | 이후 모든 접근이 이 경계 안으로 제한 | SCP-01, INT-01 |
| 3 | 사용자 | read / search | 기존·MCP 생성 노트 탐색, 읽은 상태 기억 | SCP-02, INT-02 |
| 4 | 사용자 | create / append / update | starter 권고 적용(선택)하며 작성 | SCP-02, SCP-04, INT-02, INT-04 |
| 5 | 서버 | 정상 파일로 지속 | Obsidian·후속 세션에서 확인 | SCP-02, INT-02 |
| 6 | 사용자 | check (advisory) | 심각도 그룹 finding + compiler 미실행 명시 | SCP-05, INT-05 |
| 7 | 사용자 | 수정안 개별 선택 | 원하는 finding만 선택 | SCP-05, INT-06 |
| 8 | 서버 | 충돌 안전 반영 | 선택분만 적용, 원문·비선택분 유지 | SCP-03, INT-03, INT-06 |

**출력 명료성 노트:** 각 단계 응답은 대상 경로와 결과 상태(성공/거부/충돌)를 첫 줄에 두므로, 사용자는 흐름 어디에 있는지와 방금 무엇이 바뀌었는지를 매번 확인할 수 있다.

---

## 2. 충돌 거부 후 복구 흐름 (INT-03, Q4 = 재읽기 안내)

읽은 뒤 외부(Obsidian 등)에서 노트가 바뀌면 조용히 덮어쓰지 않고 거부한 뒤, 최소한의 재읽기 안내만 제공한다.

```
[사용자가 append / update 요청]
      |
      v
서버: 읽은 시점 상태와 현재 파일 상태를 비교
      |
      +--- 동일 (stale 아님) ---------> 지정 부분만 안전 수정 -----> 성공
      |
      +--- 다름 (외부에서 변경됨) -----> 쓰기 거부 (fail-closed)
                   |
                   v
             무엇이 바뀌었는지 알림:
               - 대상 노트 경로
               - "읽은 뒤 외부(Obsidian 등)에서 변경되었습니다"
                   |
                   v
             최소 안내: "다시 read 한 뒤 재시도하세요"
                   |
                   v
             사용자가 read 재실행 --> update 재시도 --> 성공
```

| 단계 | 행위자 | 동작 | 결과 방향 | 추적성 |
|---|---|---|---|---|
| C1 | 사용자 | append/update 요청 | 대상·의도 부분 전달 | INT-03 |
| C2 | 서버 | 읽은 시점 vs 현재 상태 비교 | stale 여부 판정 | INT-03, SCP-03 |
| C3 | 서버 | stale이면 쓰기 거부(fail-closed) | 외부 편집·원문 보존 | INT-03, SCP-03 |
| C4 | 서버 | 무엇이 바뀌었는지 + 재읽기 안내 | 재시도 가능한 설명으로 종료 | INT-03 |
| C5 | 사용자 | read 재실행 후 재시도 | 최신 상태 기준으로 안전 수정 | INT-03, INT-02 |

- **범위 결정:** 첫 릴리스는 최소 흐름(재읽기 안내)만 제공한다. 차이 요약, "내 변경만 재적용/현재 유지/취소" 선택, 안전 사본은 이번 범위에서 채택하지 않는다(Q4에서 사용자가 최소 흐름 선택). 이후 수요가 확인되면 재검토한다.
- **경계:** 이 흐름은 삭제·이동·이름 변경을 포함하지 않는다(INT-14 / Out of Scope).

**출력 명료성 노트:** 거부 응답은 "거부됨 → 대상 경로 → 바뀐 사실 → 다음 행동(재읽기·재시도)" 순으로, 사용자가 무엇을 해야 하는지 한 번에 알 수 있게 한다.

---

## 3. 후속 단계로 이월하는 열린 질문

- **R-02 충돌 감지 메커니즘** — 무엇을 기준으로 stale을 판정할지(예: 내용 해시·수정 시각·바이트 비교)와 쓰기 권한 정의는 Requirements/Contract Design에서 구체화. 본 문서는 사용자 대면 흐름만 확정.
- **R-01 / INT-05 성공 지표** — "입력 문제 감소"의 baseline·target·관찰 기간·측정 방법은 Requirements Analysis에서 확정.
- **세부 MCP API 계약** — 도구별 매개변수·응답 스키마·에러 코드는 Inception으로 유예.

## Sources

- [`intent-statement`](../intent-capture/intent-statement.md) — 지속 작성·충돌 보호 방향, R-01/R-02.
- [`scope-document`](../scope-definition/scope-document.md) — 사용자 가치 흐름, SCP-01~SCP-06, 충돌 복구 Open question.
- [`intent-backlog`](../scope-definition/intent-backlog.md) — INT-01~INT-07, VS-1~VS-5, INT-03 최소 recovery Open question.
- [`wireframes`](wireframes.md) — 시스템 컨텍스트·도구 표면·정보 구조·점검 결과 표현.
- [`rough-mockups-questions`](rough-mockups-questions.md) — 이 단계의 확정 답변.

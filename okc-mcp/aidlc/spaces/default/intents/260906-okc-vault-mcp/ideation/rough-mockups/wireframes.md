# Rough Mockups — OKC Local MCP (시스템 상호작용 · 정보 구조)

**단계:** Rough Mockups & Concept Visualization (ideation)
**성격:** 이 제품은 화면 GUI가 아니라 설치형 로컬 MCP 도구 표면이다. 따라서 이 문서는 pixel wireframe이 아니라 (1) 시스템 컨텍스트, (2) MCP 도구 표면, (3) starter Vault 정보 구조, (4) advisory 점검 결과 표현을 스케치한다. 실제 상호작용의 시간 순서 흐름은 [`user-flow`](user-flow.md)에 있다.
**입력:** [`intent-statement`](../intent-capture/intent-statement.md), [`scope-document`](../scope-definition/scope-document.md), [`intent-backlog`](../scope-definition/intent-backlog.md)
**확정 답변:** [`rough-mockups-questions`](rough-mockups-questions.md)

> 모든 다이어그램은 stage-protocol.md의 ASCII 표준을 따른다(기본 ASCII 문자만 사용, Unicode box-drawing 미사용). 노트 내용은 신뢰할 수 없는 데이터로 취급하며 그 안의 지시문은 따르지 않는다.

---

## 1. 시스템 컨텍스트

```
[MCP Client: LLM / agent host]
        |
        |  MCP protocol (능력별 개별 도구 호출 + 구조화된 요약 응답)
        v
[OKC Local MCP Server  (이 제품)]
        |
        |  직접 파일 접근, Vault 경계에서 fail-closed
        v
[단일 연결 Obsidian source Vault  (Markdown 노트)]

제품 경계 밖 (분리 유지, 이 제품이 호출하지 않음):
[OKC compiler]   [OKC managed projects]   [sealed outputs]
```

- 클라이언트는 개별 MCP 도구를 호출하고, 서버는 항상 대상 경로가 명시된 구조화된 요약을 돌려준다.
- 서버는 하나의 명시적으로 연결된 source Vault에만 접근하며, Vault 밖·제외 경로·허용되지 않은 대상은 거부한다(SCP-01).
- compiler 실행·통과 판정·승인 권한은 경계 밖이다. 점검은 언제나 advisory이며 compiler를 호출하지 않는다(SCP-05, INT-05).

**출력 명료성 노트:** 화면이 없으므로 "가독성"은 응답 구조로 보장한다 — 모든 응답 첫 줄에 어떤 도구가·어느 resolved Vault 경로를 대상으로·성공/거부/충돌 중 무엇인지가 드러난다.

---

## 2. MCP 도구 표면 (Q6 = 능력별 개별 도구)

능력을 세분화된 개별 도구로 노출한다. 연결은 항상 선행 확인 단계를 거친다.

```
connect   -- 단일 source Vault 연결. resolved 경로 · 허용 경계 · effective policy를
             먼저 사용자에게 확인시킨다. 이후 모든 도구 응답은 target 경로를 명시한다.
read      -- 노트 읽기. 읽은 시점 상태를 기억해 이후 충돌 판정 근거로 쓴다.
search    -- Vault 내 bounded 검색(제목·경로·본문·frontmatter).
create    -- 새 노트 생성. starter frontmatter/구조 권고 적용 여부는 사용자가 선택.
append    -- 기존 노트의 끝 또는 지정 섹션에 안전하게 추가.
update    -- 지정한 부분만 안전 수정. 대상이 명확하고 읽은 뒤 바뀌지 않았을 때만 적용.
check     -- advisory readiness 점검. source를 바꾸지 않으며 compiler 미실행을 명시.
```

- `connect`가 경계·정책을 먼저 확인시키는 것은 승인된 범위(SCP-01, INT-01)와 market-research 학습(암묵적 현재 디렉터리 fallback·Roots 의존보다 명시적 단일 Vault 경계 우선)에서 온다.
- `read`가 읽은 시점 상태를 기억하는 것은 `update`의 충돌 판정(INT-03)의 전제다.
- 세부 API 계약(매개변수·스키마·에러 코드)은 Inception(Contract Design)으로 유예한다.

**출력 명료성 노트:** 각 도구 응답은 `대상 경로 → 결과 상태(성공/거부/충돌) → 요약 → (필요 시) 다음 행동` 순의 고정된 골격을 따른다.

---

## 3. Starter Vault 정보 구조 (Q1 = 하이브리드, Q2 = metadata 권고)

노트 유형은 폴더가 아니라 `type` frontmatter로 표현하므로, 유형별 폴더 대신 소수의 기능 폴더 + 자유로운 주제 폴더를 기본 starter로 제안한다.

```
<vault-root>/
  MOC/                  -- 지식 허브(Map of Content) 노트 모음
    index.md            -- 최상위 진입 MOC
  inbox/                -- 아직 분류 전 빠른 수집 노트
  attachments/          -- 이미지 등 첨부 (선택)
  topics/               -- 주제 폴더 (사용자가 자유롭게 추가)
    <topic>/
      <note>.md         -- 노트 유형은 type frontmatter로 표현
  templates/            -- 노트 · MOC 템플릿 (Q3 = 템플릿 제공)
    note.md
    moc.md
```

- 이 구조 전체는 **권고**이며 강제하지 않는다. 사용자는 승인·재정의할 수 있고, "폴더 추천을 사실상 하지 않는" 최소 평면형도 허용한다(SCP-04, INT-04).
- 실제 OKC 확인 제약(portable path, Unicode/case collision, symlink·제외 경계)과 제품 권고는 응답에서 구분해 표시한다.

### 3.1 노트 frontmatter 권고 (Q2 = B 상태관리 + C 지식성격 + D OKC 연계)

```yaml
---
title: <노트 제목>          # 기본
tags: []                    # 기본
status: draft               # B 상태 관리: draft | active | resolved
created: 2026-09-07         # B 상태 관리
updated: 2026-09-07         # B 상태 관리
type: note                  # C 지식 성격: note | evidence | decision | open-item ...
source:                     # C 지식 성격: 근거 링크(있으면)
aliases: []                 # D OKC 연계
review: false               # D OKC 연계: OKC 입력 전 검토 여부
---
```

- `title`·`tags`는 기본 필드로 항상 포함하되, 위 필드는 모두 권고이며 강제하지 않는다.

### 3.2 MOC · 링크 템플릿 (Q3 = 템플릿 제공)

제품은 아래와 같은 MOC 노트·링크 규약 템플릿을 만들어 주고, 이후 관리는 사용자에게 맡긴다. wikilink는 자동 삽입하지 않는다.

```markdown
# <주제> MOC

## 핵심 노트
- [[<note-1>]]
- [[<note-2>]]

## 결정 / decision
- [[<decision-note>]]

## 열린 항목 / open items
- [[<open-item>]]
```

**출력 명료성 노트:** starter 구조·템플릿을 제안할 때 응답은 "권고 / OKC 확인 제약"을 라벨로 구분하고, 사용자가 승인·재정의할 수 있음을 함께 표시한다.

---

## 4. Advisory readiness 점검 결과 표현 (Q5 = 심각도 그룹, INT-05)

결과는 심각도(경고/정보)로 묶고, 각 항목에 근거·영향·수정 가능 여부를 표시한다. 어떤 그룹 방식이든 상단에 "compiler 미실행" 배너를 항상 둔다.

```
------------------------------------------------------------
ADVISORY READINESS CHECK   (advisory only)
NOTE: 실제 OKC compiler를 호출하지 않았습니다. "OKC 통과" 판정이 아닙니다.
Target: <resolved-vault>/topics/auth/oauth-decision.md
Summary: 경고 2 · 정보 1
------------------------------------------------------------
[경고] frontmatter: 'status' 필드 없음
   - 근거   : starter 권고 (Q2 = B 상태 관리)
   - 영향   : 상태 기반 MOC · 검색에서 누락됨
   - 수정   : 가능 (status: draft 추가 제안)
[경고] link: 대상 없는 wikilink [[oauth-flow]]
   - 근거   : OKC 확인 제약 (portable link)
   - 영향   : 링크 그래프 · MOC 탐색 단절
   - 수정   : 가능 (노트 생성 또는 링크 수정 제안)
[정보] structure: MOC에서 이 노트로의 링크 없음
   - 근거   : 제품 권고 (MOC 허브)
   - 영향   : 발견성 저하 (기능 문제 아님)
   - 수정   : 가능 (index MOC에 링크 추가 제안)
------------------------------------------------------------
```

- 각 finding은 "근거가 OKC 확인 제약인지, 제품 권고인지"를 명시해 사용자가 compiler 규칙으로 오인하지 않게 한다(SCP-05, INT-05).
- 수정안 적용은 사용자가 개별 선택하며, 적용은 `update`와 동일한 충돌 안전(INT-03)을 따른다(INT-06).

**출력 명료성 노트:** 상단 요약(개수·대상 경로·compiler 미실행)만 읽어도 전체 상태를 파악할 수 있고, 그 아래는 심각도 순으로 상세가 이어진다.

---

## 5. 추적성 (upstream 연결)

| Rough Mockups 결정 | 상류 근거 |
|---|---|
| Q1 하이브리드 starter 폴더 구조 | SCP-04, INT-04 / scope-document Open question(starter 폴더 역할) 해소 |
| Q2 frontmatter 권고(B·C·D) | SCP-04, INT-04 |
| Q3 링크·MOC 템플릿 제공 | SCP-04, INT-04 |
| Q4 충돌 거부 후 재읽기 안내(최소) | SCP-03, INT-03 / scope·backlog Open question(INT-03 최소 recovery) 해소 |
| Q5 심각도 그룹 + compiler 미실행 명시 | SCP-05, INT-05 |
| Q6 능력별 개별 도구 + connect 경계 확인 우선 | SCP-01, SCP-02, INT-01, INT-02 |

## 6. 후속 단계로 이월하는 열린 질문

- **R-01 / INT-05 성공 지표** — "OKC 입력 전 구조·링크·metadata 문제 감소"의 baseline·target·관찰 기간·측정 방법은 아직 미정. Requirements Analysis에서 확정(scope-document, intent-backlog Open question).
- **R-02 쓰기 권한·충돌 모델** — 사용자 대면 최소 복구 흐름(Q4=재읽기 안내)은 정했으나, 충돌 감지 메커니즘(무엇을 기준으로 stale로 판정할지)과 쓰기 권한 정의는 Requirements/Contract Design에서 구체화.
- **INT-07 첫 지원 host·platform matrix** — 실제 환경 증거 없이는 확정 불가.
- **세부 MCP API 계약** — 도구 매개변수·응답 스키마·에러 코드는 Inception(Contract Design)으로 유예.

## Sources

- [`intent-statement`](../intent-capture/intent-statement.md) — 승인된 문제·성공 방향·단일 Vault 및 지속 작성 경계, R-01/R-02.
- [`scope-document`](../scope-definition/scope-document.md) — SCP-01~SCP-06, walking-skeleton 전달 순서, starter 구조·충돌 복구 Open question.
- [`intent-backlog`](../scope-definition/intent-backlog.md) — INT-01~INT-07, VS-1~VS-5, INT-03 최소 recovery Open question.
- [`rough-mockups-questions`](rough-mockups-questions.md) — 이 단계의 확정 답변.

## Review

**Collaborator:** aidlc-product-lead-agent
**Verdict:** READY
**Reviewer:** aidlc-product-lead-agent
**Date:** 2026-09-07T04:34:09Z
**Iteration:** 1
**성격:** advisory 단일 패스 (사람 triage용 결정 지원). 아래 발견은 승인 게이트에서 검토할 항목이며, fix-and-re-review 루프를 전제하지 않는다.

두 산출물은 상류(intent-statement / scope-document / intent-backlog)와 확정 답변에 잘 근거해 있고, 제품 경계(단일 Vault·fail-closed·조용한 덮어쓰기 금지·compiler 미실행 advisory·삭제/이동/이름변경 부재·노트 untrusted)를 전반에서 지킨다. 이월 열린 질문(R-01 지표, R-02 충돌 감지 메커니즘, INT-07 host matrix, 세부 API)도 downstream으로 올바르게 유예됐다. Critical 없음. 아래는 advisory 개선 항목이다.

### 발견 항목

- **[Major] §4 advisory 점검 예시가 제품 권고를 "OKC 확인 제약"으로 오분류할 위험.** §4 예시에서 "대상 없는 wikilink [[oauth-flow]]"의 근거를 `OKC 확인 제약 (portable link)`로 표기했다. 그러나 scope-document가 명시한 OKC 확인 제약 목록은 portable path, Unicode/case collision, symlink·제외 경계이며(scope-document > 다루는 콘텐츠), 링크 유효성(dangling wikilink)은 포함되지 않는다. dangling wikilink는 §4의 세 번째 항목(MOC 링크 없음 → "제품 권고")과 같은 발견성/제품 권고 범주에 더 가깝다. 이 예시는 오히려 SCP-05·INT-05가 방지하려는 "제품 권고를 compiler/OKC 규칙으로 오인"을 시연하게 된다.
  - 제안: 링크 유효성 finding의 근거를 "제품 권고"로 재분류하거나, "portable link"가 실제로 OKC가 확인한 제약임을 근거(공개 소스·문서)와 함께 명시하라. 근거 분류의 정확성 자체가 이 제품의 차별점이자 경계이므로, 대표 예시가 정확해야 한다.

- **[Minor] INT-07 / SCP-06 공개 설치·저장소 UX 여정이 정보 구조·흐름으로 스케치되지 않음.** happy path(§user-flow §1)는 `connect`부터 시작하고, install→connect→first note→diagnose→update/uninstall 여정(SCP-06, INT-07, VS-1/VS-5)은 두 산출물 어디에도 상호작용 흐름이나 정보 구조로 나타나지 않는다. INT-07은 첫 릴리스 Must capability이며 VS-1의 quick start와 VS-5 전체를 포괄한다.
  - 제안: host·platform matrix(INT-07) 확정은 올바르게 유예하되, 설치·연결 확인·업데이트/제거의 최소 사용자 흐름 골격은 이 단계에서 표현하는 것을 권장. 유예 대상(어떤 host를 지원할지)과 이 단계에서 다룰 수 있는 흐름(설치 후 어떤 순서로 사용자에게 무엇을 보여줄지)을 구분하면 좋다.

- **[Minor] §5 추적성 표에 INT-06(사용자 선택 수정 적용) 행이 없음.** 본문(§4 "수정안 적용은 사용자가 개별 선택", user-flow §1 8단계)에는 INT-06이 근거로 등장하나, §5 추적성 표는 Q1~Q6만 나열하고 INT-06을 명시적 행으로 담지 않는다.
  - 제안: 완전성을 위해 "advisory finding의 사용자 선택 적용 → INT-06, SCP-05, INT-03" 행을 추가하라. 현재도 본문 추적은 존재하나 표가 traceability 요약이므로 표에서도 닫히는 편이 좋다.

- **[Nit] starter 구조의 `attachments/ (선택)` 폴더가 경계 오인을 유발할 수 있음.** binary attachment의 완전한 authoring은 Out of Scope(scope-document > 첫 릴리스 Out of Scope)다. §3의 `attachments/`는 구조 placeholder일 뿐 제품이 첨부를 author하지 않음을 라벨로 분명히 하면 경계 오인을 예방할 수 있다.
  - 제안: `attachments/`에 "제품은 첨부를 생성·편집하지 않으며 사용자 파일 배치용 폴더 권고일 뿐"이라는 한 줄 주석을 추가.

### Summary

산출물은 상류 정합성·추적성·경계 준수·유예 표시가 모두 견고하여 advisory 관점에서 READY다. 승인 전에 가장 먼저 검토할 것은 Major 1건(§4 예시의 "OKC 확인 제약" 오분류)으로, 이는 제품의 핵심 경계(제품 권고와 OKC 규칙 구분)를 대표 예시에서 정확히 지키기 위한 것이다. 나머지는 완전성·명료성 개선이다.

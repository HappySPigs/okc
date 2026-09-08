# Obsidian MCP의 세션 기록 자동 배치: 소스 조사

조사일: 2026-09-09, Asia/Seoul.

사용자 질문: “지금 obsidian mcp들은 obsidian vault들을 어떻게 적절한곳에 자동 반영하는데? 직접 소스코드까지 확인해서 조사해”

범위: OKC 전체 흐름 중 세션 기록이 원본 Vault에 들어가는 단계의 책임과 외부 구현 사례 비교. 모듈의 기존 요구사항·설계를 대체하지 않는다.

## 조사 방법과 한계

공개 저장소 8개를 직접 shallow clone하고 당시 기본 브랜치의 커밋을 고정했다. 검색 결과와 README로 후보를 찾은 뒤, 도구 입력 스키마, 실제 저장 함수, 검색 함수, 에이전트 스킬·프롬프트, 세션 훅을 추적했다. 아래 링크는 확인한 커밋과 소스 행을 가리킨다.

제3자 스킬과 AGENTS 문서는 조사 대상 자료로 읽었으며 이 작업의 지침으로 채택하지 않았다. 의존성 설치, 제3자 서버·자동화 스크립트 실행, 사용자의 실제 Vault 변경, 외부 LLM 호출은 수행하지 않았다. 결과는 정적 소스 분석이며 모델의 실제 배치 정확도를 측정한 것은 아니다. 기본 브랜치 코드는 배포 패키지 버전과 다를 수 있다.

| 저장소 | 확인한 커밋 | 마지막 커밋 시각 |
|---|---|---|
| `MarkusPfundstein/mcp-obsidian` | [`5ee0b84fa831`](https://github.com/MarkusPfundstein/mcp-obsidian/commit/5ee0b84fa8319fd2fdf0db0ee1febb065e712a15) | 2026-08-31T13:49:01+02:00 |
| `cyanheads/obsidian-mcp-server` | [`2d4e8d1114d6`](https://github.com/cyanheads/obsidian-mcp-server/commit/2d4e8d1114d650b45568caa9a163e6e853423266) | 2026-09-04T02:46:06-07:00 |
| `bettyguo/obsidian_mcp` | [`46905f221711`](https://github.com/bettyguo/obsidian_mcp/commit/46905f2217112a6b1713a7d0978789b21262704e) | 2026-06-05T19:41:34-04:00 |
| `maxkuminov/obsidian-mcp` | [`a2c1a2d1c101`](https://github.com/maxkuminov/obsidian-mcp/commit/a2c1a2d1c101cb1b98de1926fdc5e184bb480554) | 2026-09-06T16:40:45-04:00 |
| `honam867/obsidian-memory-layer-mcp` | [`d1dccb742f7e`](https://github.com/honam867/obsidian-memory-layer-mcp/commit/d1dccb742f7eda1fb08fce0e45a85918d15ce672) | 2026-04-03T09:35:06+07:00 |
| `eugeniughelbur/obsidian-second-brain` | [`d631fd67aea0`](https://github.com/eugeniughelbur/obsidian-second-brain/commit/d631fd67aea0afe2b1e44fa5496bc04629348456) | 2026-09-06T22:37:41+02:00 |
| `aaronsb/obsidian-semantic-mcp` | [`c7c1003d392f`](https://github.com/aaronsb/obsidian-semantic-mcp/commit/c7c1003d392f84f908489d0400647919d062c4a4) | 2025-07-11T21:18:43-05:00 |
| `basicmachines-co/basic-memory` | [`3452c821d76c`](https://github.com/basicmachines-co/basic-memory/commit/3452c821d76c083823d020984d71e06904a1ff1e) | 2026-09-08T11:10:43-05:00 |

## 핵심 관찰

도구가 경로를 요구한다는 사실만으로 사용자가 경로를 결정해야 하는 것은 아니다. 조사한 구현들에서는 호출 에이전트가 검색과 읽기를 수행하고, Vault 관례에 따라 저장 경로와 수정 방법을 정하는 방식이 확인됐다. MCP는 후보 검색과 실제 파일 변경을 제공하고, 스킬·프롬프트가 그 사이의 판단 절차를 정의한다.

자동화는 다음처럼 구분해야 한다.

| 구분 | 실제 자동화되는 일 | 여전히 별도인 일 |
|---|---|---|
| 파일 생성·추가 | 지정된 경로에 쓰고 파일 유무에 따라 생성/추가 | 세션 내용과 관련된 경로 선택 |
| 검색·그래프 | 관련 후보와 링크 관계 반환 | 같은 주제인지, 연결만 할지, 내용을 통합할지 결정 |
| 고정 규칙 배치 | 프로젝트·기록 종류를 정해진 폴더로 매핑 | 임의의 기존 Vault 구조를 이해하는 판단 |
| 에이전트 워크플로 | 대화 해석, 후보 비교, 위치 선택, 내용 반영 | 지침을 실제로 수행하는 호스트/에이전트 실행 |
| 세션 훅 | 특정 시점에 기록 절차 시작 | 저장할 내용과 목적지의 의미 판단 |

## 1. MarkusPfundstein/mcp-obsidian

[AppendContentToolHandler](https://github.com/MarkusPfundstein/mcp-obsidian/blob/5ee0b84fa8319fd2fdf0db0ee1febb065e712a15/src/mcp_obsidian/tools.py#L192)는 filepath와 content를 필수로 받는다. 실행 함수는 그 값을 [Obsidian.append_content](https://github.com/MarkusPfundstein/mcp-obsidian/blob/5ee0b84fa8319fd2fdf0db0ee1febb065e712a15/src/mcp_obsidian/obsidian.py#L124)에 넘기고, 어댑터는 해당 Vault 경로로 REST 요청을 보낸다. 별도 검색·읽기·heading/block patch 도구도 등록돼 있다.

확인한 경로에서 본문을 분석해 다른 노트의 목적지를 선택하는 호출은 없다. 새 파일/기존 파일을 처리하는 append 동작과, 어느 파일이 적절한지 찾는 판단은 다르다. 자연어 요청에서 목적지를 선택하는 역할은 호출 에이전트에 있다.

## 2. cyanheads/obsidian-mcp-server

[write 도구](https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/src/mcp-server/tools/definitions/obsidian-write-note.tool.ts#L12)와 [append 도구](https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/src/mcp-server/tools/definitions/obsidian-append-to-note.tool.ts#L15)가 [TargetSchema](https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/src/mcp-server/tools/definitions/_shared/schemas.ts#L11)를 받는다. 대상은 명시적 경로, Obsidian의 현재 활성 노트, 또는 날짜 기반 주기 노트다. section으로 특정 heading/block/frontmatter를 지정할 수 있다.

section 없는 append는 지정한 파일이 없으면 생성한다. 그러나 이는 정해진 대상에서의 파일 유무 처리다. 세션 본문과 관련된 다른 노트를 검색해 목적지를 바꾸는 과정은 이 함수에 없다. document-map 읽기로 구역 후보를 제공하고 에이전트가 선택한다. 이름의 대소문자나 확장자 유사 경로를 제안하는 보조 코드는 내용의 의미를 판별하는 분류기가 아니다.

## 3. maxkuminov/obsidian-mcp

[get_vault_guide_impl](https://github.com/maxkuminov/obsidian-mcp/blob/a2c1a2d1c101cb1b98de1926fdc5e184bb480554/src/mcp_server/tools.py#L2324)는 일반 Obsidian 작성 안내와 Vault 루트의 CLAUDE.md를 함께 반환한다. 여기서 폴더·이름·태그 관례를 에이전트에 전달한다.

[find_related_impl](https://github.com/maxkuminov/obsidian-mcp/blob/a2c1a2d1c101cb1b98de1926fdc5e184bb480554/src/mcp_server/tools.py#L3558)는 원본 노트의 chunk embedding 평균을 사용해 pgvector cosine-distance 후보를 반환한다. keyword 검색과 그래프 주변 탐색도 별도 도구다. [create_note_impl](https://github.com/maxkuminov/obsidian-mcp/blob/a2c1a2d1c101cb1b98de1926fdc5e184bb480554/src/mcp_server/tools.py#L3075)는 호출자가 정한 path에 저장하며 검색 결과를 자체적으로 선택하지 않는다.

이 조합은 “Vault 규칙 읽기 → 관련 후보 찾기 → 본문 확인 → 에이전트 판단 → 지정 경로/구역 편집”을 지원한다. 의미 검색이 있다는 사실 자체가 자동 병합을 뜻하지는 않는다.

## 4. aaronsb/obsidian-semantic-mcp

[SemanticRouter.enrichResponse](https://github.com/aaronsb/obsidian-semantic-mcp/blob/c7c1003d392f84f908489d0400647919d062c4a4/src/semantic/router.ts#L678)는 실행 결과에 다음 도구 호출 후보를 붙인다. 설정의 조건과 현재 결과를 평가해 suggested_next를 만든다. create/update 실행 분기는 전달받은 path와 content를 API에 넘긴다.

[SemanticResponse와 안내 계약](https://github.com/aaronsb/obsidian-semantic-mcp/blob/c7c1003d392f84f908489d0400647919d062c4a4/src/types/semantic.ts#L15)은 이 힌트가 선택적인 안내임을 명시한다. 힌트를 반환한 뒤 다음 검색·쓰기까지 서버가 계속 실행하는 구조는 아니다. “semantic workflow”라는 이름과 자율 배치 기능은 구분해야 한다. 확인한 기본 브랜치의 마지막 커밋은 2025-07-11이다.

## 5. bettyguo/obsidian_mcp

[find_related](https://github.com/bettyguo/obsidian_mcp/blob/46905f2217112a6b1713a7d0978789b21262704e/src/obsidian_mcp/session.py#L270)는 기존 노트 경로를 받으면 해당 노트 기반 dense 검색을, 자유 텍스트를 받으면 hybrid 검색을 수행한다. 실제 세션은 기본적으로 sentence-transformer embedder를 사용한다. [create_note](https://github.com/bettyguo/obsidian_mcp/blob/46905f2217112a6b1713a7d0978789b21262704e/src/obsidian_mcp/session.py#L342)는 전달받은 경로에 파일을 생성한다.

[review-and-link 스킬](https://github.com/bettyguo/obsidian_mcp/blob/46905f2217112a6b1713a7d0978789b21262704e/skills/review-and-link/SKILL.md#L14)은 Inbox 또는 최근 노트에서 후보를 고르고, 관련 노트·backlink를 찾고, 링크를 제안하도록 한다. 이 스킬은 사용자에게 후보를 보여주고 선택을 받는 절차다. 따라서 이 사례의 자동화는 검색과 제안까지이며, 사용자가 위치·관계 선택을 하지 않아야 한다는 OKC의 현재 의도와 동일하지 않다.

## 6. honam867/obsidian-memory-layer-mcp

[Vault.save](https://github.com/honam867/obsidian-memory-layer-mcp/blob/d1dccb742f7eda1fb08fce0e45a85918d15ce672/src/vault.ts#L161)는 에이전트가 넘긴 project와 type을 사용한다. [getTypeDir](https://github.com/honam867/obsidian-memory-layer-mcp/blob/d1dccb742f7eda1fb08fce0e45a85918d15ce672/src/vault.ts#L637)가 decision, learning, todo, reference 등을 정해진 디렉터리로 매핑한다. 저장 경로는 AI-Memory/projects/<project>/<type-folder>/<slug>-<timestamp>.md 형태다.

[generateAutoLinks](https://github.com/honam867/obsidian-memory-layer-mcp/blob/d1dccb742f7eda1fb08fce0e45a85918d15ce672/src/vault.ts#L605)가 자동 생성하는 링크는 해당 프로젝트의 context/progress 문서다. 주제의 의미를 비교해 발견한 관계가 아니다. 저장 함수는 새 시각 기반 ID를 만들어 파일을 쓰며, 관련 주제 노트를 검색해 자동으로 내용을 합치지 않는다.

[save-brain 절차](https://github.com/honam867/obsidian-memory-layer-mcp/blob/d1dccb742f7eda1fb08fce0e45a85918d15ce672/commands/save-brain.md#L5)가 대화에서 결정·학습·진행 상황을 추출하고 이미 저장한 항목을 제외하도록 에이전트에 지시한다. 사용자가 파일 경로를 지정할 필요는 줄어들지만, 임의의 기존 Vault에서 가장 알맞은 노트를 찾는 방식은 아니다.

## 7. Basic Memory

Obsidian에 한정된 서버는 아니지만 Markdown Vault와 호환되는 대화 기억 사례로 포함했다.

[memory-capture 스킬](https://github.com/basicmachines-co/basic-memory/blob/3452c821d76c083823d020984d71e06904a1ff1e/skills/memory-capture/SKILL.md#L38)은 thread_id가 있으면 metadata 검색, 없으면 주제 검색으로 같은 대화의 기존 기록을 찾게 한다. 발견하면 읽고 최신 내용을 종합해 같은 노트를 갱신하고, 없으면 생성한다. 이 판단 절차를 실행하는 주체는 스킬을 읽은 에이전트다. 한 대화의 기록을 계속 갱신하는 흐름이며 여러 기존 주제 문서에 자동 분배하는 기능과는 범위가 다르다.

[write_note 구현](https://github.com/basicmachines-co/basic-memory/blob/3452c821d76c083823d020984d71e06904a1ff1e/src/basic_memory/mcp/tools/write_note.py#L215)은 directory를 필수로 받는다. 신규 작성 완료 후에는 vector 검색으로 최대 3개의 유사 기존 노트를 찾아 응답에 붙이는 코드가 있다. [유사 노트 탐색 보조 함수](https://github.com/basicmachines-co/basic-memory/blob/3452c821d76c083823d020984d71e06904a1ff1e/src/basic_memory/mcp/tools/write_note.py#L47)와 같은 파일의 작성 후 분기에서 확인된다. 검색이 설정되지 않았다면 안내를 생략할 수 있다.

이 안내는 작성 이후의 제안이다. 구현 주석과 응답은 유사도만으로 중복과 별개인 관련 주제를 구분하기 어렵다는 점을 설명하고, 본문 확인 후 보강/연결/유지를 판단하도록 한다. 자동으로 기존 노트에 덮어쓰지는 않는다.

## 8. Obsidian Second Brain

이 사례는 스킬·명령·훅과 선택적인 MCP 서버를 함께 제공한다. 다음 세 경로를 구분해야 한다.

[obsidian-log 명령](https://github.com/eugeniughelbur/obsidian-second-brain/blob/d631fd67aea0afe2b1e44fa5496bc04629348456/commands/obsidian-log.md#L13)은 대화에서 프로젝트를 추론하고 필요하면 검색한다. [folder-map 규칙](https://github.com/eugeniughelbur/obsidian-second-brain/blob/d631fd67aea0afe2b1e44fa5496bc04629348456/references/folder-map.md#L5)으로 저장 위치를 정하며, 작업 기록을 작성하고 프로젝트의 Recent Activity와 일일 노트에 링크를 반영하도록 지시한다. Vault의 _CLAUDE.md 규칙이 우선이며 기존 레이아웃과 기본 폴더 매핑이 보조 기준이다.

[obsidian-capture 명령](https://github.com/eugeniughelbur/obsidian-second-brain/blob/d631fd67aea0afe2b1e44fa5496bc04629348456/commands/obsidian-capture.md#L15)은 아이디어 폴더의 관련 기존 노트를 먼저 찾아 보강하고, 없을 때 새 노트를 만들도록 한다. 이 명령 파일은 에이전트의 실행 절차다.

선택적으로 활성화하는 [PostCompact 백그라운드 훅](https://github.com/eugeniughelbur/obsidian-second-brain/blob/d631fd67aea0afe2b1e44fa5496bc04629348456/hooks/obsidian-bg-agent.sh#L161)은 대화 transcript에서 최근 압축 요약을 읽고 별도 headless 에이전트를 실행한다. 프롬프트에는 기존 노트 검색, 프로젝트/사람/작업 기록/아이디어/결정별 반영, 일일 노트 연결이 정의돼 있다. 기본 설치에서는 별도 enable 설정 없이는 실행되지 않는다. 이 subprocess는 MCP를 로드하지 않고 제한된 파일 도구를 사용한다.

반면 MCP의 [save_note](https://github.com/eugeniughelbur/obsidian-second-brain/blob/d631fd67aea0afe2b1e44fa5496bc04629348456/integrations/obsidian-mcp-server/vault_ops.py#L887)는 path가 없으면 Inbox의 날짜·제목 기반 경로를 만들고, [capture_idea](https://github.com/eugeniughelbur/obsidian-second-brain/blob/d631fd67aea0afe2b1e44fa5496bc04629348456/integrations/obsidian-mcp-server/vault_ops.py#L956)도 그 함수를 호출한다. 같은 이름이 있으면 수정 도구 사용을 안내하며, 자체적으로 관련 노트로 목적지를 바꾸지는 않는다. 따라서 패키지의 자율 기록 경험을 MCP capture 함수 단독의 기능으로 설명하면 잘못이다.

## OKC에 대한 해석

아래는 조사에서 도출한 설계 시사점이며 새 구현이나 승인된 변경 설계는 아니다.

현재 [okc-mcp 도구](../../../okc-mcp/src/server.ts)와 [작성 지침](../../../okc-mcp/src/guide.ts)에 검색·읽기·생성·수정 기반은 있다. create_note와 update_note라는 도구 이름 또는 path 입력 자체가 사용자에게 생성/수정 판단을 맡겨야 한다는 뜻은 아니다. 이번 조사에서 그 판단을 에이전트가 수행하는 실제 절차가 확인됐다.

OKC의 보완 범위는 세션에서 지식을 추출하고, 원본 Vault의 관례와 후보 노트를 읽고, 기존 문서/구역 보강·새 노트 생성·관련 링크 추가를 결정해 실행하는 절차를 제품의 기본 동작으로 정의하고 검증하는 것이다. 이 절차는 스킬/프롬프트와 기존 도구의 조합으로 구현할 수도 있으므로, 별도 MCP 내부 LLM 또는 새 저장 도구가 필수라는 결론은 나오지 않는다.

구체적으로 검증할 상황은 다음과 같다.

- 사용자가 경로와 생성/수정 여부를 지정하지 않아도 기록이 완료되는가.
- 기존 노트와 같은 주제이면 적절한 구역이 보강되는가.
- 관련되지만 독립적인 주제이면 기존 본문에 억지로 합치지 않고 연결되는가.
- 한 세션의 서로 다른 주제가 각각 알맞은 기록에 반영되는가.
- 같은 세션을 재기록할 때 중복 기록을 만들지 않는가.
- 결과에 반영된 경로와 변경 내역을 설명할 수 있는가.

목적지 판단의 검색 대상은 수정 가능한 로컬 원본이어야 한다. OKC에서 웹 게시본 기본 조회와 로컬 작성 대상이 분리돼 있으므로, 웹 검색 결과를 곧바로 로컬 수정 경로로 해석해서는 안 된다. 이 절차는 hooks 업로드 전의 원본 작성 책임이고, 뒤의 web/core 통합·검토·게시 책임과 연결된다. [현재 source 선택](../../../okc-mcp/src/server.ts), [로컬 저장 처리](../../../okc-mcp/src/authoring.ts).

## 완료한 조사 항목

- [x] 검색으로 일반 Obsidian MCP, semantic 검색, 세션 memory, 스킬/훅 사례를 선정했다.
- [x] 저장소 8개의 커밋과 실제 소스 파일을 확인했다.
- [x] 저장 도구에서 경로를 누가 결정하는지 추적했다.
- [x] 검색 후보 반환, 고정 폴더 매핑, 에이전트 판단, 후속 안내를 구분했다.
- [x] 세션 훅이 실제로 어떤 코드를 실행하는지 확인했다.
- [x] OKC의 기존 저장 기반과 필요한 세션 반영 절차를 구분했다.


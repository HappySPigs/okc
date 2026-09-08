# OKC Local MCP 범위 정의

**상태:** Scope Definition 승인 대기  
**기준:** [`intent-statement`](../intent-capture/intent-statement.md)와 확정된 [`scope-definition-questions`](scope-definition-questions.md)  
**첫 사용자:** 새 Obsidian source Vault를 시작하거나 연결한 Vault의 노트를 개선하려는 개인 OKC 사용자

## 범위 목표

첫 릴리스는 사용자가 로컬 MCP를 설치하고 하나의 Obsidian source Vault를 명시적으로 연결한 뒤, 같은 세션에서 노트를 계속 만들고 읽고 찾고 안전하게 개선하며, 그 결과를 Obsidian과 이후 세션에서도 그대로 사용하는 완결된 authoring 경험을 제공한다.

제품 가치는 단순 파일 CRUD가 아니라 다음 세 요소의 결합에서 나온다.

- 사용자가 확인하고 통제하는 단일 source Vault 경계
- 외부 편집을 조용히 덮어쓰지 않는 지속적 authoring
- OKC에 유리한 구조 안내와 advisory 입력 점검 및 사용자 선택형 개선

이 제품은 editable source Vault를 작성하는 도구다. OKC managed project, sealed output, compiler 실행, 검토 승인 권한은 제품 경계 밖에 둔다.

## 첫 릴리스 In Scope

| ID | Capability | 사용자에게 제공되는 결과 | 범위 경계 |
|---|---|---|---|
| SCP-01 | 명시적 단일 Vault 연결과 통제 | 사용자가 어느 source Vault가 연결됐고 읽기·쓰기가 어디까지 허용되는지 확인한다. | 현재 디렉터리를 암묵적 Vault로 사용하지 않으며, Vault 밖·제외 영역·허용되지 않은 경로에는 접근하지 않는다. |
| SCP-02 | 지속적 핵심 authoring | 기존 노트와 MCP가 만든 노트를 읽고 검색하며, 새 노트를 만들고 append하고 기존 노트의 지정 부분을 수정한다. | 결과는 정상 Vault 파일로 즉시 지속되어 Obsidian과 이후 MCP 세션에서 보인다. |
| SCP-03 | 안전한 기존 노트 개선 | 사용자가 의도한 노트와 부분만 바꾸고, 읽은 뒤 외부에서 내용이 달라졌다면 쓰지 않고 충돌을 설명한다. | 조용한 최신-wins 덮어쓰기와 불명확한 heading/block 추측을 허용하지 않는다. |
| SCP-04 | OKC 친화적 구조 안내 | 폴더 역할, metadata, 근거·상태·결정·open item, 링크와 MOC 같은 starter 패턴을 제안한다. | 실제 OKC 제약과 제품 권고를 구분하며, 권고 구조는 사용자가 승인하거나 재정의할 수 있다. |
| SCP-05 | Advisory 입력 점검과 개선 | frontmatter, portable path, 링크, 구조 등 확인 가능한 문제를 보여 주고 수정안을 제안하며, 사용자가 선택한 수정을 안전하게 반영한다. | 실제 compiler를 호출하지 않았다면 “OKC 통과”나 승인을 주장하지 않는다. |
| SCP-06 | 공개 설치·저장소 UX | quick start부터 연결 확인, synthetic 예제, 첫 노트 walkthrough, 호환성·보안·문제 해결·업데이트·제거까지 따라갈 수 있다. | 개인 Vault·machine-local 설정·키·파생 state는 저장소나 예제에 포함하지 않는다. |

### 다루는 콘텐츠

- 첫 릴리스의 핵심 authored material은 Obsidian Markdown 노트다.
- YAML frontmatter, heading, block, wikilink, Markdown link처럼 현재 Obsidian/OKC 흐름에 직접 필요한 구조를 다룬다.
- 이미 존재하는 노트와 이 MCP가 만든 노트를 같은 안전 정책 아래 다룬다.
- malformed metadata나 모호한 대상은 정상으로 가장하지 않고 사용자가 이해할 수 있는 문제로 노출한다.
- OKC가 실제 확인한 portable path, Unicode/case collision, symlink와 제외 경계는 제품 권고와 구분되는 호환성 guardrail 후보로 다룬다.

## 첫 릴리스 Out of Scope

| 제외 항목 | 이유 | 이후 재검토 조건 |
|---|---|---|
| 노트 삭제, 이동, 이름 변경 | 복구, 링크 파급, 경로 충돌의 위험이 핵심 작성 흐름보다 크다. | 핵심 수정 안전성이 검증되고 실제 사용자 수요가 확인될 때 |
| 여러 Vault 동시 연결·비교 | 승인된 제품 경계는 사용자가 명시한 단일 source Vault다. | 명확한 multi-Vault 사용자 여정과 권한 모델이 생길 때 |
| Obsidian 앱·CLI·플러그인 필수 연동 | 기본 제품은 앱 실행과 무관한 로컬 authoring을 지향한다. | history, rename, UI 같은 app-aware 가치가 핵심 수요로 확인될 때 |
| 원격/호스팅 MCP 또는 동시 협업 서비스 | local-first 개인 사용자의 첫 문제를 넓히고 별도 보안 모델을 요구한다. | 실제 공유·다중 기기 요구가 정의될 때 |
| 실제 OKC compiler 실행, 통과 판정, 승인 | 이 MCP는 source authoring 제품이며 compiler/approval authority가 아니다. | 별도 승인된 OKC integration scope가 생길 때 |
| Canvas, Bases, binary attachment의 완전한 authoring | 현재 첫 가치와 OKC materialization 근거는 Markdown 중심이다. | OKC 지원 의미와 사용자 요구가 함께 확인될 때 |
| 영속 full-text/semantic index와 embeddings | 새 Vault 중심 첫 사용자에게 필요한 규모가 아직 측정되지 않았다. | 대표 Vault에서 bounded search가 목표를 충족하지 못할 때 |
| shell 실행, 임의 provider 호출, 파괴적 파일 도구 | authoring 목적과 신뢰 경계를 불필요하게 넓힌다. | 이번 제품 범위에서는 재검토하지 않는다. |
| 기존 draft 코드의 기능 목록을 제품 baseline으로 채택 | 기존 구현은 승인되지 않은 초안이며 현재 scope의 근거가 아니다. | 후속 reverse engineering에서 요구사항 대비 증거로만 평가한다. |

## 사용자 가치 흐름

```mermaid
flowchart LR
    A[설치 및 host 설정] --> B[명시적 Vault 연결 확인]
    B --> C[기존 노트 탐색 또는 새 노트 생성]
    C --> D[Obsidian에서 지속 결과 확인]
    D --> E[OKC 구조 안내에 따라 작성·개선]
    E --> F[Advisory 입력 점검]
    F --> G{수정안을 선택했는가?}
    G -- 예 --> H[충돌 안전 조건으로 source Vault 반영]
    G -- 아니오 --> I[원문 유지]
    H --> J[Obsidian·후속 세션에서 계속 작성]
    I --> J
```

이 흐름에서 사용자는 매번 제품 내부 저장소로 가져오거나 export하지 않는다. 정상 source Vault 파일이 지속성의 기준이며, MCP 상태와 OKC 산출물은 그 밖에 남는다.

## 의존성과 전달 순서

선택된 전략은 walking skeleton 우선이다. 첫 세로 흐름은 설치·연결 확인·읽기·새 노트 쓰기·Obsidian 확인을 최소 폭으로 관통한다. 이후 기능 수를 늘리기 전에 쓰기 경계와 충돌 거부를 굳히고, 그 위에 OKC 안내와 점검·수정 경험을 얹는다.

| 순서 | 제품 slice | 입증할 것 | 선행 조건 |
|---|---|---|---|
| 1 | 연결→읽기/생성→지속 확인 | 사용자가 실제 source Vault에서 가장 짧은 end-to-end 가치를 얻는다. | 없음 |
| 2 | 안전한 append·부분 수정·충돌 거부 | 기존 노트와 외부 Obsidian 편집을 잃지 않고 계속 작성할 수 있다. | Slice 1 |
| 3 | 수정 가능한 starter 안내 | 사용자가 compiler 규칙으로 오인하지 않고 OKC에 유리한 구조를 선택한다. | Slice 1~2 |
| 4 | Advisory 점검→수정 제안→선택 적용 | 문제 발견이 실제 source 개선으로 연결되며 기존 변경 안전성을 유지한다. | Slice 2~3 |
| 5 | 공개 릴리스 여정 완결 | 새 사용자가 설치부터 update/uninstall까지 독립적으로 운영한다. | Slice 1~4의 검증 근거 |

이 순서는 후속 설계나 Bolt를 미리 확정하지 않는다. `intent-backlog.md`의 proto-Unit 수준 우선순위이며, 실제 기술 의존성과 작업 단위는 Inception에서 결정한다.

## 릴리스 판단 기준

고정된 외부 날짜는 없다. 첫 릴리스는 다음 결과를 검증할 수 있을 때 준비된 것으로 본다.

- synthetic Vault에서 설치부터 첫 지속 노트 확인까지 완결된다.
- 같은 세션에서 여러 번 작성하고 이후 세션에서 다시 읽을 수 있다.
- Vault 밖 쓰기, 허용되지 않은 경로, 모호한 대상, 읽은 뒤 변경된 노트는 fail-closed로 처리된다.
- OKC에서 확인된 제약, 제품의 추천 패턴, 아직 compiler로 확인하지 않은 advisory가 사용자에게 구분된다.
- 사용자가 선택한 점검 수정만 적용되고, 충돌 시 외부 편집을 덮어쓰지 않는다.
- 업데이트·문제 해결·제거 경로가 문서화되며 제거가 사용자의 authored note를 삭제하지 않는다.

정량 success metric의 baseline, target, 관찰 기간과 측정 방법은 Requirements Analysis 전에 확정해야 한다. 현재 범위에서는 “OKC 입력 전 구조·링크·metadata 문제 감소”라는 방향만 승인돼 있다.

## Assumptions & Open Questions

- Assumption: 첫 사용자는 Markdown 중심의 작거나 새로 시작한 Vault에서 핵심 여정을 검증한다.
- Assumption: 사용자가 선택한 readiness 수정의 직접 적용도 SCP-03과 같은 외부 편집 충돌 보호를 따른다.
- Open question: “입력 문제 감소”의 baseline, target, 관찰 기간, 측정 방법을 어떤 synthetic/사용자 workflow로 정의할 것인가?
- Open question: 첫 호환성 표에 포함할 MCP host, protocol version, OS, runtime, Obsidian, OKC 조합은 무엇인가?
- Open question: starter 구조의 구체적인 폴더 역할·metadata·link/MOC 안내는 다음 Rough Mockups와 Requirements에서 검증해야 한다.
- Open question: 충돌 거부 후 사용자가 이해하기 쉬운 재읽기·비교·재시도 흐름은 어떤 형태인가?

## Sources

- [`intent-statement`](../intent-capture/intent-statement.md) — 승인된 문제, 고객, 성공 방향, 단일 Vault 및 지속 작성 경계.
- [`scope-definition-questions`](scope-definition-questions.md) — 첫 릴리스 흐름, 변경 절단선, 충돌 행동, 구조 안내, 입력 점검, 저장소 UX, 순서와 일정에 대한 사용자 결정.
- [`competitive-analysis`](../market-research/competitive-analysis.md) — 지속 쓰기의 table-stakes 성격과 안전성·OKC 안내의 차별화 근거.
- [`market-trends`](../market-research/market-trends.md) — local-first, Obsidian 자동화, conflict-aware write, 검색 범위에 대한 시장 근거.
- [`build-vs-buy`](../market-research/build-vs-buy.md) — 집중형 build/compose와 app integration 선택 확장 권고.


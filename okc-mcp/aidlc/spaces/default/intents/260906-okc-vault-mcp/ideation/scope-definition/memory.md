<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
- 2026-09-06T17:12:45Z — “Vault 변경”은 repository code 변경이 아니라 연결된 source Vault의 기존 노트와 MCP가 만든 노트 모두에 대한 authoring operation을 뜻한다; 사용자가 질문의 의미를 확인한 뒤 이 경계를 명시적으로 다시 제시했다.
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
- 2026-09-06T17:12:45Z — 첫 릴리스는 create/read/search/append/safe partial update를 Must로 두고 delete/move/rename을 제외한다; 지속 작성 가치는 보존하면서 복구와 link 파급 위험을 뒤로 미룬다.
- 2026-09-06T17:12:45Z — advisory finding의 사용자 선택 수정은 직접 source Vault에 반영하되, 외부 편집이 감지되면 조용히 병합하거나 덮어쓰지 않고 거부한다; 편의보다 사용자 원문 보존을 우선한다.
- 2026-09-06T17:12:45Z — walking skeleton으로 install/connect/read-write/Obsidian 확인의 최소 세로 흐름을 먼저 입증한 뒤 safety와 OKC guidance를 확장한다; 고정 날짜보다 승인된 안전성·사용성 결과를 릴리스 조건으로 둔다.
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
- 2026-09-06T17:12:45Z — OKC 입력 문제 감소의 baseline·target·관찰 기간·측정 방법을 Requirements Analysis 전에 확정해야 한다.
- 2026-09-06T17:12:45Z — 첫 지원 MCP host·protocol·OS·runtime·Obsidian·OKC 호환 조합과 충돌 후 최소 recovery UX는 후속 단계에서 검증해야 한다.
- 2026-09-06T17:12:45Z — starter Vault의 구체 폴더 역할·metadata·link/MOC 안내는 Rough Mockups에서 사용자 흐름으로 검증해야 한다.
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->

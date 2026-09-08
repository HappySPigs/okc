<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations

- 2026-09-06T16:20:02Z — 기존 Obsidian MCP 조사는 경쟁 제품의 결함을 입증하는 작업이 아니라 재사용 가능한 작성 로직과 OKC용 구조 안내의 간극을 찾는 작업으로 해석한다; 공개 소스와 공식 문서를 근거로 현재 동작을 검증한다.
- 2026-09-06T16:48:00Z — 같은 세션과 이후 세션에 남는 Vault 쓰기는 경쟁군이 이미 제공하는 table stakes이며, OKC 제품의 차별점은 이를 OKC 품질 안내·명시적 경계·외부 편집 충돌 방지와 하나의 경험으로 묶는 데 있다고 해석한다.
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->

## Tradeoffs
- 2026-09-06T16:48:00Z — 앱 비의존성을 위해 직접 파일 접근을 기본 제품 방향으로 두고 Obsidian CLI·플러그인 연동은 선택적 확장으로 남긴다; 대신 앱 수준 rename·history·UI 기능을 첫 범위에서 기대하지 않는다.
- 2026-09-06T16:48:00Z — 편리한 현재 디렉터리 fallback이나 MCP Roots 의존보다 사용자가 확인할 수 있는 명시적 단일 Vault 경계를 우선한다; Roots는 현행 MCP에서 폐기 방향이고 애초에 서버측 권한 경계를 대신하지 않는다.
- 2026-09-06T16:48:00Z — GitHub 활동은 범주 수요의 정성 신호로만 사용하고 OKC 시장 규모나 활성 사용자 수로 환산하지 않는다; 과장된 TAM 대신 후속 사용자 검증을 요구한다.
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->

## Open questions
- 2026-09-06T16:48:00Z — 대표 Vault 규모가 어느 정도일 때 bounded scan 대신 Vault 밖의 영속 인덱스가 필요한지 측정해야 한다.
- 2026-09-06T16:48:00Z — 첫 지원 MCP host들의 현행 protocol 호환 범위와, 사용자가 이해할 수 있는 expected-version 충돌 복구 UX를 요구사항 단계에서 확정해야 한다.
- 2026-09-06T16:48:00Z — OKC 입력 문제 감소의 baseline·target·측정 기간·관찰 방법과 앱 연동의 실제 필요성은 사용자 검증이 필요하다.
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->

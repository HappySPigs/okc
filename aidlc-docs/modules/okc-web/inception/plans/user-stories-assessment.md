# User Stories Assessment — okc-web

**Stage**: INCEPTION / User Stories (Part 1, Step 1 — mandatory need assessment)
**Date**: 2026-09-08

## Request Analysis
- **Original Request**: okc-core 위에 부서간 개인 Obsidian Vault를 권한 관리로 통합하고(관리자만), conflict를 리뷰·해소하며, 개인별 업로드(토큰), 병합 Vault를 okc-mcp가 RAG로 쓰도록 서빙하는 웹 플랫폼.
- **User Impact**: **Direct** — 여러 유형의 인간 사용자(관리자/기여자/뷰어)가 직접 상호작용하고, 비인간 소비자(okc-mcp)가 API를 소비.
- **Complexity Level**: **Complex** — 인증/RBAC, 업로드+토큰, 통합 오케스트레이션(다단계 human-in-the-loop), conflict/critic 리뷰, 서빙 등 4개 신규 계층.
- **Stakeholders**: 관리자(curator), 개인 기여자, (선택) 뷰어, 미래 okc-mcp 소비자, okc-core 유지보수 경계.

## Assessment Criteria Met
- [x] **High Priority — New User Features**: 업로드·리뷰·통합·서빙 전부 신규 사용자 기능.
- [x] **High Priority — Multi-Persona System**: admin/curator, contributor, viewer, okc-mcp 소비자.
- [x] **High Priority — Customer-Facing API**: 업로드 API(토큰), read-only 서빙 API(okc-mcp 계약).
- [x] **High Priority — Complex Business Logic**: freeze-then-run, hash-bound 승인, critic 차단/waive, 모순 보존 등 다중 시나리오.
- [x] **High Priority — Cross-Team / Cross-System**: okc-web ↔ okc-core ↔ 미래 okc-mcp.
- [x] **Medium — Security/Permissions**: 관리자 전용 게이팅(원 요구사항 ②)이 스토리로 명확화 가치 큼.

## Decision
**Execute User Stories**: **Yes**
**Reasoning**: 다수 페르소나·사용자 대면 기능·복잡한 통합 워크플로우·API 소비자 계약이 모두 존재. 스토리는 (1) 관리자 전용 권한 경계, (2) conflict 리뷰의 재해석된 결정 표면(승인/waive/omission/regenerate), (3) 업로드/토큰 흐름, (4) okc-mcp 서빙 계약을 테스트 가능한 명세로 고정해 데모 수용 기준을 제공. High Priority 지표 5개 충족 → skip 대상 아님.

## Expected Outcomes
- 페르소나별 명확한 권한/행동 경계 → 원 요구사항 ①②의 RBAC 오해 방지.
- conflict 리뷰 스토리로 okc-core 제약(승자 선택 없음)을 UX 관점에서 고정.
- 업로드·서빙 API의 수용 기준 확정 → 데모 end-to-end 검증 가능.
- 팀/이해관계자 정렬 및 이후 Application Design·Code Generation의 입력.

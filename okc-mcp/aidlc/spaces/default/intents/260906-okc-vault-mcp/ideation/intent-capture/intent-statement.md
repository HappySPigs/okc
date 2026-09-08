# Intent Statement

## Problem Statement

처음부터 OKC에 맞는 Obsidian Vault를 만들려는 개인 사용자는 근거, 메타데이터, 링크가 일관된 형식의 노트를 지속적으로 작성하기 어렵다. 이 제품은 사용자가 로컬 MCP 세션에서 그 형식을 유지하며 지식을 축적하도록 돕는 것을 핵심 문제로 삼는다. [desc][Q1][Q2][Q4][Q9]

## Target Customer

첫 번째 대표 고객은 새 Vault를 시작하며 처음부터 OKC에 적합한 구조를 만들고 싶은 개인 OKC 사용자다. [Q2]

제품은 사용 편의뿐 아니라 Vault 소유자가 자신의 데이터, 구조, 편집 권한을 통제해야 한다는 관심을 함께 반영한다. [Q5][Q6]

## Success Metrics

- 우선 성공 지표는 OKC 입력 전에 발견되는 구조, 링크, 메타데이터 문제의 감소다. [Q3]

## Initiative Trigger

지금 이 작업을 시작한 직접적인 계기는 OKC가 활용하기 좋은 지식이 쌓이도록 일관된 형식의 Vault를 계속 구성할 필요가 있기 때문이다. [Q4]

기존 Obsidian MCP의 실제 로직과 OKC 저장소의 실제 동작 및 검증 결과를 조사하여 제품 판단의 근거로 사용한다. [desc][Q10]

## Initial Scope Signal

- Workflow-selected scope는 `okc-local-mcp`다. [scope]
- 사용자가 확인한 첫 제품 경계는 설치형 로컬 MCP가 명시적으로 연결한 단일 Obsidian source Vault의 작성, 읽기, 개선, 입력 점검을 지원하고, 기존 Obsidian MCP 조사와 OKC용 Vault 및 저장소 UX 설계를 포함하는 것이다. [desc][Q8]
- 같은 MCP 세션에서 연결한 Vault에 새 노트를 반복 추가하고 기존 노트를 안전하게 수정하며, 저장 결과를 Obsidian과 이후 세션에서도 그대로 사용해야 한다. [Q9]
- 현재 요청자가 제품 범위와 안전한 기본값을 결정하고, 각 MCP 사용자는 자신의 Vault 연결, 구조 재정의, 변경 허용 범위를 통제한다. [Q6]

## Assumptions & Open Questions

None.

## Review

**Verdict:** READY
**Reviewer:** aidlc-product-lead-agent
**Date:** 2026-09-06T16:08:18Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/260906-okc-vault-mcp/ideation/intent-capture/intent-statement.md > Success Metrics | The stated outcome is a reduction in pre-input structure, link, and metadata issues, but it has no baseline, target, observation window, or measurement method. | Before requirements are finalized, define the baseline, target reduction or pass rate, time window, and the validation workflow that produces the measurement. | New |
| R-02 | Major | aidlc/spaces/default/intents/260906-okc-vault-mcp/ideation/intent-capture/intent-statement.md > Initial Scope Signal | Repeated in-session creation and modification is confirmed, but the user-controlled change boundary does not yet specify collision, overwrite, or concurrent-edit behavior for an existing note. | In requirements and design, define the write-authorization model and observable behavior for existing-path collisions, external changes, and recovery/undo. | New |

### Summary

The intent is source-grounded, scope-bounded to one explicitly connected source Vault, and confirms the essential persistent in-session authoring behavior. The two major items are decision-support gaps to resolve before implementation detail is committed; they do not invalidate the current intent framing.

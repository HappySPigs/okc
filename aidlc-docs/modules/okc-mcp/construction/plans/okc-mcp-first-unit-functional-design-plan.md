# Functional Design — Plan (autopilot) · unit `okc-mcp-first-unit`

Role: designer. Detailed, technology-agnostic business logic for the single unit. Builds on [`../../inception/application-design/application-design.md`](../../inception/application-design/application-design.md). Sources: requirements REQ-001..013, stories US-*.

> **AUTOPILOT DECISION (2026-09-08)**: Design questions answered with recommended MVP defaults (below). No ambiguity. Plan self-approved; artifacts authored directly (no Workflow tool).

## Artifacts (checklist)
- [x] `functional-design/domain-entities.md`
- [x] `functional-design/business-rules.md`
- [x] `functional-design/business-logic-model.md`
- [~] frontend-components.md — **N/A** (headless stdio MCP; no UI)

## Auto-answered design questions
- **Domain model**: entities = Vault, Note, Frontmatter, ContentHash, BackupRecord, AuditIssue, Rejection, ToolCall/ToolResult. [Answer]: recommended
- **Core algorithms**: deterministic content hash; structure-preserving partial frontmatter merge; safe path resolution; literal Unicode search. [Answer]: recommended
- **Business rules focus**: path safety, bounds, hash-conflict, refuse-overwrite, partial-merge preservation, single external backup, atomic in-place write, heuristic audit, trust boundary, typed rejections. [Answer]: recommended
- **Error handling**: typed structured rejections; fail-safe (no partial writes/backups on rejection). [Answer]: recommended
- **Data persistence**: user Markdown files in the one Vault (mutated in place); tool state (config/backups) outside the Vault. [Answer]: recommended
- **Integration**: none external at runtime (no network); downstream OKC consumes the Vault out-of-band. [Answer]: recommended

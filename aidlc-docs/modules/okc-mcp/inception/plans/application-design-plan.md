# Application Design — Plan (autopilot)

Role: architect. High-level component identification + interfaces + service orchestration for the single unit `okc-mcp-first-unit`. Detailed business rules come later in Functional Design. Sources: [`../requirements/requirements.md`](../requirements/requirements.md), [`../user-stories/stories.md`](../user-stories/stories.md).

> **AUTOPILOT DECISION (2026-09-08)**: Design questions answered with recommended, MVP-scoped defaults (below). No ambiguities. Plan self-approved; artifacts generated via workflow.

## Mandatory artifacts (execution checklist)
- [x] `application-design/components.md` — component definitions, responsibilities, interfaces
- [x] `application-design/component-methods.md` — method signatures + purpose + I/O (rules deferred to Functional Design)
- [x] `application-design/services.md` — service definitions + orchestration
- [x] `application-design/component-dependency.md` — dependency matrix, communication patterns, data flow
- [x] `application-design/application-design.md` — consolidated design
- [x] Validate completeness (REQ/story coverage) + MVP scope (no follow-up leakage) — all first-Unit REQ/stories mapped; REQ-012 non-runtime; no scope creep

## Design questions (auto-answered — recommended MVP defaults)

### Q1 — Component boundaries/grouping
Recommended: group by responsibility into a small core + thin surface — **VaultBoundary (path safety), NoteStore (read/create/write/list), FrontmatterEngine (YAML partial-merge), HashService, BackupManager, AuditEngine, DiscoveryService(list/search incl. Korean)**, plus a thin **McpServer/ToolRegistry** surface and **SetupService (config/diagnostics/registration)**.
[Answer]: A (recommended) — responsibility-grouped core + thin MCP surface

### Q2 — Architectural style
Recommended: **layered** — thin MCP tool handlers → orchestration services → core components. No DI framework, no plugin system (MVP). Core components have no upward dependencies.
[Answer]: A (recommended) — simple layered, no framework

### Q3 — Service orchestration
Recommended: a small set of services — **AuthoringService** (create/update/partial-frontmatter), **AuditService** (read-only audit), **DiscoveryService** (list/search/read-with-hash), **SetupService** (install/config/diagnostics/registration). The mutating write path is a fixed pipeline: **validate path → check expectedHash → write single external backup → write in place**.
[Answer]: A (recommended) — 4 thin services; fixed safe write pipeline

### Q4 — Dependencies & communication
Recommended: **in-process synchronous function calls**; unidirectional (surface → services → core); no events/queues/network.
[Answer]: A (recommended) — in-process, unidirectional

### Q5 — Error/rejection contract
Recommended: typed structured rejections surfaced as MCP tool errors — `path-denied`, `hash-mismatch`, `malformed-yaml`, `overwrite-refused`, `bounds-exceeded`, `not-found` — never silent partial writes.
[Answer]: A (recommended) — typed structured rejections, fail-safe

## Generation approach
Workflow (ultracode): 3 independent architecture proposals (angles: MVP-minimal, safety/trust-first, story-coverage-first) → per-proposal judge (MVP fit, REQ/story coverage, interface clarity, scope-creep) → MVP-biased synthesis → completeness critic. Main loop renders the 5 artifacts from the synthesized structured design.

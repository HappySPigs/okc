# Code Generation Plan — okc-schema3-product

## Single source of truth for this retrospective stage

The user's request is documentation-only. In this brownfield continuation,
Code Generation means reconcile and document the existing implementation; it
does not mean regenerate, duplicate, or refactor application source.

## Unit context

- **Requirement input**: FR-1 through FR-17 and all current REQ areas.
- **Modules**: seven Cargo packages plus tests, guide, and release configuration.
- **Dependencies**: inward package graph recorded in the unit dependency artifact.
- **Database state**: private project SQLite journal schema 4 and optional build workspace schema 2.
- **Public contracts**: Schema 3 artifacts, interop DTO schema 2, API v1, current CLI grammar.

## Numbered execution plan

- [x] **Step 1 — Confirm workspace and source baseline.** Record commit, package metadata, layout, and dirty state without modifying unrelated files.
- [x] **Step 2 — Trace core business logic.** Map snapshot/parser/corpus/integration/materialization modules to REQ/ALG contracts.
- [x] **Step 3 — Trace AI/application logic.** Map profiles, preflight, recordings, project journal, review, invalidation, and workers.
- [x] **Step 4 — Trace public APIs.** Map CLI/TUI, interop, Python, and Node methods and side-effect boundaries.
- [x] **Step 5 — Trace persistence and artifacts.** Record project layout, schema versions, compiled directory layout, publication and verification.
- [x] **Step 6 — Trace tests.** Map unit, integration, contract, security, E2E, performance, and package evidence.
- [x] **Step 7 — Generate as-built documentation.** Create `implementation-summary.md` and `source-traceability.md` under the unit code directory.
- [x] **Step 8 — Preserve code.** Verify no application source, schema, dependency, API, or golden byte changed.
- [x] **Step 9 — Hand off to Build and Test.** Run relevant commands and record exact current outcomes.

## Explicit non-steps

- No new code file or migration.
- No dependency upgrade.
- No acceptance of ADR-0028 through ADR-0031.
- No Pack, non-Markdown, MCP/plugin, command-provider, or stable-release implementation.

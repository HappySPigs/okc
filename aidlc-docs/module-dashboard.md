# OKC Module Dashboard

A single status-and-navigation view over the four independent `okc-*` modules.

> **Snapshot as of 2026-09-09 (Asia/Seoul).** This is a *reference view*: it links to
> each module's AI-DLC docs by repository-relative path and never copies, moves, or merges
> them. The **source of truth for any module's status is that module's own
> `aidlc-docs/aidlc-state.md`** — the cells below are a hand-written summary and can lag.
>
> **To refresh:** open a module's `aidlc-state.md`, read its current `Current Stage` /
> `Current Phase` line, and update that module's row and section here. Do not edit anything
> under a `<module>/aidlc-docs/` tree from this file. Ownership policy: see
> [`README.md`](README.md) and the repository `CLAUDE.md`.

## Umbrella status

Root integration workspace — **CONSTRUCTION COMPLETE** (cross-module gap implementation,
independent review, and aggregate verification passed on 2026-09-09 KST). Module states
remain independent. See [`aidlc-state.md`](aidlc-state.md).

## Modules at a glance

| Module | Type | Stack | Current AI-DLC stage | Docs |
|---|---|---|---|---|
| `okc-core` | Brownfield | Rust + Python/Node bindings | CONSTRUCTION — documented through Build & Test; repository integration authorized | [`aidlc-docs/`](../okc-core/aidlc-docs/) |
| `okc-hooks` | Greenfield | Rust (Cargo workspace, 10 crates) | CONSTRUCTION COMPLETE — all units U0–U8 coded + cargo-verified (237 tests) | [`aidlc-docs/`](../okc-hooks/aidlc-docs/) |
| `okc-mcp` | Brownfield | TypeScript (npm) | ⚠️ Two state files disagree — see note below | [`aidlc-docs/`](../okc-mcp/aidlc-docs/) |
| `okc-web` | Greenfield | FastAPI (Python) + React/Vite | CONSTRUCTION COMPLETE — W0–W5 done | [`aidlc-docs/`](../okc-web/aidlc-docs/) |

---

## `okc-core`

- **Type / stack:** Brownfield — native product/library workspace (Rust with Python/Node bindings).
- **Current stage:** CONSTRUCTION — documentation complete through Build and Test; repository integration authorized. ([state](../okc-core/aidlc-docs/aidlc-state.md))
- **Units:** [`okc-schema3-product`](../okc-core/aidlc-docs/construction/okc-schema3-product/)
- **Key artifacts:**
  [requirements](../okc-core/aidlc-docs/inception/requirements/requirements.md) ·
  [application design](../okc-core/aidlc-docs/inception/application-design/application-design.md) ·
  [build & test summary](../okc-core/aidlc-docs/construction/build-and-test/build-and-test-summary.md)
- **Agent instructions:** [`AGENTS.md`](../okc-core/AGENTS.md) · [`CLAUDE.md`](../okc-core/CLAUDE.md)

## `okc-hooks`

- **Type / stack:** Greenfield — Rust, Cargo workspace of 10 library crates plus a thin `watcher-bin` binary.
- **Current stage:** CONSTRUCTION COMPLETE — all 10 units U0–U8 designed, coded, and real-cargo verified (237 tests pass, clippy clean); Operations phase (placeholder) remaining. ([state](../okc-hooks/aidlc-docs/aidlc-state.md))
- **Units:**
  [u0-foundation](../okc-hooks/aidlc-docs/construction/u0-foundation/) ·
  [u1-content-core](../okc-hooks/aidlc-docs/construction/u1-content-core/) ·
  [u2-change-detect](../okc-hooks/aidlc-docs/construction/u2-change-detect/) ·
  [u3-upload-client](../okc-hooks/aidlc-docs/construction/u3-upload-client/) ·
  [u4-sync-state](../okc-hooks/aidlc-docs/construction/u4-sync-state/) ·
  [u5-auth-consent](../okc-hooks/aidlc-docs/construction/u5-auth-consent/) ·
  [u6-observability](../okc-hooks/aidlc-docs/construction/u6-observability/) ·
  [u7a-lifecycle-deploy](../okc-hooks/aidlc-docs/construction/u7a-lifecycle-deploy/) ·
  [u7b-ops-control](../okc-hooks/aidlc-docs/construction/u7b-ops-control/) ·
  [u8-orchestration](../okc-hooks/aidlc-docs/construction/u8-orchestration/)
- **Key artifacts:**
  [requirements](../okc-hooks/aidlc-docs/inception/requirements/requirements.md) ·
  [user stories](../okc-hooks/aidlc-docs/inception/user-stories/) ·
  [application design](../okc-hooks/aidlc-docs/inception/application-design/application-design.md) ·
  [build & test summary](../okc-hooks/aidlc-docs/construction/build-and-test/build-and-test-summary.md)
- **Agent instructions:** [`CLAUDE.md`](../okc-hooks/CLAUDE.md)

## `okc-mcp`

- **Type / stack:** Brownfield — TypeScript, npm.
- **Current stage:** Per [`aidlc-state.md`](../okc-mcp/aidlc-docs/aidlc-state.md) the AI-DLC state is **CONSTRUCTION COMPLETE** (user-selected session capture verified 2026-09-09 KST; 103 tests, build and package inclusion passed).
- ⚠️ **State discrepancy:** the module also keeps an older process-correction note, [`state.md`](../okc-mcp/aidlc-docs/state.md), which reads *"Inception — awaiting review of requirements and process"* and predates the switch back to AI-DLC v1. Treat `aidlc-state.md` as the authoritative AI-DLC state; `state.md` and the *unapproved, slated-for-replacement* [`methodology.md`](../okc-mcp/aidlc-docs/methodology.md) are retained for audit continuity only.
- **Units:**
  [okc-mcp-first-unit](../okc-mcp/aidlc-docs/construction/okc-mcp-first-unit/) ·
  [okc-mcp-retrieval-unit](../okc-mcp/aidlc-docs/construction/okc-mcp-retrieval-unit/) ·
  [okc-mcp-session-capture](../okc-mcp/aidlc-docs/construction/okc-mcp-session-capture/) ·
  [okc-mcp-web-knowledge](../okc-mcp/aidlc-docs/construction/okc-mcp-web-knowledge/)
- **Key artifacts:**
  [requirements](../okc-mcp/aidlc-docs/inception/requirements/requirements.md) ·
  [user stories](../okc-mcp/aidlc-docs/inception/user-stories/) ·
  [application design](../okc-mcp/aidlc-docs/inception/application-design/application-design.md) ·
  [build & test summary](../okc-mcp/aidlc-docs/construction/build-and-test/build-and-test-summary.md)
- **Agent instructions:** [`CLAUDE.md`](../okc-mcp/CLAUDE.md)

## `okc-web`

- **Type / stack:** Greenfield — FastAPI backend (Python 3.11+, consumes `okc-core` via its Python bindings) and a React + Vite SPA (TypeScript).
- **Current stage:** CONSTRUCTION COMPLETE — waves W0–W5 done; Operations = placeholder/SKIP. ([state](../okc-web/aidlc-docs/aidlc-state.md); note: older historical `Current Stage` prose is retained below the top summary in that file for audit continuity.)
- **Units:**
  [u1-auth](../okc-web/aidlc-docs/construction/u1-auth/) ·
  [u2-upload](../okc-web/aidlc-docs/construction/u2-upload/) ·
  [u3-orchestration](../okc-web/aidlc-docs/construction/u3-orchestration/) ·
  [u4-review](../okc-web/aidlc-docs/construction/u4-review/) ·
  [u5-serving](../okc-web/aidlc-docs/construction/u5-serving/) ·
  [u6-frontend](../okc-web/aidlc-docs/construction/u6-frontend/)
- **Key artifacts:**
  [requirements](../okc-web/aidlc-docs/inception/requirements/requirements.md) ·
  [user stories](../okc-web/aidlc-docs/inception/user-stories/) ·
  [application design](../okc-web/aidlc-docs/inception/application-design/application-design.md) ·
  [build & test summary](../okc-web/aidlc-docs/construction/build-and-test/build-and-test-summary.md) ·
  [module integration drafts](../okc-web/aidlc-docs/integration/)

---

## Cross-module integration pointers

Umbrella-owned coordination artifacts (root `aidlc-docs/`):

- [Integration gap review](inception/reverse-engineering/integration-gap-review.md) — cross-module gap analysis.
- [Execution plan](inception/plans/execution-plan.md) — cross-module ordering and acceptance criteria.
- [Aggregate build & test summary](construction/build-and-test/build-and-test-summary.md) — final implementation/verification report.

Module-authored integration material referenced (not owned) here:

- [`okc-web/aidlc-docs/integration/module-integration-guide.md`](../okc-web/aidlc-docs/integration/module-integration-guide.md) — module-side monorepo integration guide and drafts.

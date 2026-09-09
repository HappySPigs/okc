# Root AI-DLC Integration Workspace

This directory belongs to the OKC umbrella project. It documents how independently managed `okc-*` modules are assembled, verified, packaged, and released together.

## Ownership boundary

Root `aidlc-docs/` owns:

- the repository-wide module inventory and dependency map;
- cross-module interfaces, compatibility constraints, and integration decisions;
- unified build, integration-test, packaging, and release requirements;
- coordination plans and aggregate verification results for work spanning modules.

Each `<module>/aidlc-docs/` owns:

- that module's intent, requirements, stories, designs, state, and audit trail;
- module-local implementation and test decisions;
- evidence required to resume the module workflow independently.

Root artifacts link to module artifacts with repository-relative paths. They do not duplicate or merge module histories.

## Workspace routing

| Work scope | AI-DLC workspace root | Artifact location |
|---|---|---|
| One `okc-*` module | The module directory | `<module>/aidlc-docs/` |
| Multiple modules or repository-wide assembly | Git repository root | `aidlc-docs/` |

For a cross-module initiative, keep the coordination plan and shared acceptance criteria here. Run module-local implementation work in each affected module and link its artifacts back from the root initiative.

## Registered modules

For a consolidated status-and-navigation view across all modules, see the [module dashboard](module-dashboard.md).

| Module | Module AI-DLC docs | Agent instructions | Status |
|---|---|---|---|
| `okc-core` | [`okc-core/aidlc-docs/`](../okc-core/aidlc-docs/) | [`okc-core/AGENTS.md`](../okc-core/AGENTS.md), [`okc-core/CLAUDE.md`](../okc-core/CLAUDE.md) | Existing |
| `okc-hooks` | [`okc-hooks/aidlc-docs/`](../okc-hooks/aidlc-docs/) | [`okc-hooks/CLAUDE.md`](../okc-hooks/CLAUDE.md) | Existing daemon implementation |
| `okc-mcp` | [`okc-mcp/aidlc-docs/`](../okc-mcp/aidlc-docs/) | [`okc-mcp/CLAUDE.md`](../okc-mcp/CLAUDE.md) | Existing local authoring/retrieval implementation |
| `okc-web` | [`okc-web/aidlc-docs/`](../okc-web/aidlc-docs/) | [`okc-web/CLAUDE.md`](../okc-web/CLAUDE.md) | Existing backend/frontend implementation |

When another `okc-*` module is added, preserve its `aidlc-docs/` directory and add it to this registry. Do not rename generated AI-DLC paths during the merge.

### Consolidated mirror

For convenience, a **read-only** copy of every module's `aidlc-docs/` is gathered under [`modules/`](modules/) so the whole project's AI-DLC trail can be browsed from one tree. It is **not authoritative** — each `<module>/aidlc-docs/` above remains the single source of truth, and the originals are preserved in place. Edit modules in their own directory; never resume a module workflow from the mirror. See [`modules/README.md`](modules/README.md).

## Root workflow state

The root state and audit track the 2026-09-09 cross-module review and completed implementation. See [aidlc-state.md](aidlc-state.md), [audit.md](audit.md), the [original gap review](inception/reverse-engineering/integration-gap-review.md), and the [final implementation/verification report](construction/build-and-test/build-and-test-summary.md). Module states remain independent.

## Framework pin

- Upstream: `https://github.com/awslabs/aidlc-workflows.git`
- Tag: `v1.0.1`
- Commit: `e49341dbeb8af82758dd85e96ed7fe9bcf38a447`
- Installed rule surfaces: `/AGENTS.md`, `/CLAUDE.md`, and `/.aidlc-rule-details/`

Upgrade all three installed rule surfaces from the same upstream tag. Preserve this ownership policy and verify module rule versions separately.

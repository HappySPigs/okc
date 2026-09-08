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

| Module | Module AI-DLC docs | Agent instructions | Status |
|---|---|---|---|
| `okc-core` | [`okc-core/aidlc-docs/`](../okc-core/aidlc-docs/) | [`okc-core/AGENTS.md`](../okc-core/AGENTS.md), [`okc-core/CLAUDE.md`](../okc-core/CLAUDE.md) | Existing |

When another `okc-*` module is added, preserve its `aidlc-docs/` directory and add it to this registry. Do not rename generated AI-DLC paths during the merge.

## Root workflow state

The root `aidlc-state.md` and `audit.md` are intentionally not pre-created by installation. AI-DLC creates them when the first repository-wide integration workflow starts. Their absence does not affect any module's existing state.

## Framework pin

- Upstream: `https://github.com/awslabs/aidlc-workflows.git`
- Tag: `v1.0.1`
- Commit: `e49341dbeb8af82758dd85e96ed7fe9bcf38a447`
- Installed rule surfaces: `/AGENTS.md`, `/CLAUDE.md`, and `/.aidlc-rule-details/`

Upgrade all three installed rule surfaces from the same upstream tag. Preserve this ownership policy and verify module rule versions separately.

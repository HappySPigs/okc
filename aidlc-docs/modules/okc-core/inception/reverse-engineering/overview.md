# Reverse Engineering Overview

**Project**: OKC 0.3.0, current Schema 3

**Baseline**: `e448bf28ee3dcb43d18428eb1bdb9ac70163bf34`

**Analysis mode**: Brownfield, documentation-first, source-verified

This artifact set is a navigation and reconciliation layer. It does not replace
the normative specifications, stable algorithms, accepted ADRs, current-state
record, or requirement traceability matrix.

## System summary

OKC turns one to ten immutable Obsidian Vault directory/ZIP/`tar.zst` snapshots
into a new, deterministic, auditable Schema 3 Markdown directory. AI providers
produce recorded taxonomy, synthesis, and critic proposals. Local validation
and explicit curator approvals create the only compile authority,
`ApprovedIntegrationPlan`. Final compile, verify, and explain operations are
provider-free.

The current output contains `knowledge/`, `legacy/`, and `.okc/` audit files.
It does not currently carry through attachments, Canvas, or Base files, does
not complete every link rewrite, and does not expose an OKCPack writer.

## Artifact set

| Artifact | Purpose |
|---|---|
| [business-overview.md](business-overview.md) | Business actors, transactions, and vocabulary |
| [architecture.md](architecture.md) | Package boundaries, state ownership, and key flows |
| [code-structure.md](code-structure.md) | Existing source and test file inventory |
| [api-documentation.md](api-documentation.md) | Current Rust, CLI, Python, and Node contracts |
| [component-inventory.md](component-inventory.md) | Package and adjacent-asset inventory |
| [interaction-diagrams.md](interaction-diagrams.md) | End-to-end transaction sequences |
| [technology-stack.md](technology-stack.md) | Languages, tools, pinned libraries, and targets |
| [dependencies.md](dependencies.md) | Internal dependency direction and external roles |
| [code-quality-assessment.md](code-quality-assessment.md) | Evidence, risks, and technical debt |
| [decision-intent-timeline.md](decision-intent-timeline.md) | Session-grounded user intent mapped to decisions |
| [reverse-engineering-timestamp.md](reverse-engineering-timestamp.md) | Analysis metadata and completeness checklist |

## Authoritative sources

| Concern | Source |
|---|---|
| Product identity and boundary | [`PROJECT_CONTEXT.md`](../../../../../okc-core/PROJECT_CONTEXT.md) |
| Implemented, partial, and missing state | [`docs/CURRENT_STATE.md`](../../../../../okc-core/docs/CURRENT_STATE.md) |
| Requirement-to-code/test mapping | [`docs/TRACEABILITY.md`](../../../../../okc-core/docs/TRACEABILITY.md) |
| Current behavior | [`docs/specs/`](../../../../../okc-core/docs/specs/) |
| Algorithm status and formulas | [`docs/algorithms/README.md`](../../../../../okc-core/docs/algorithms/README.md) |
| Accepted and proposed decisions | [`docs/adr/README.md`](../../../../../okc-core/docs/adr/README.md) |
| Unresolved decisions | [`docs/history/OPEN_QUESTIONS.md`](../../../../../okc-core/docs/history/OPEN_QUESTIONS.md) |

## Reverse-engineering limits

- The analysis describes current source and public contracts; it does not prove
  correctness beyond the recorded checks.
- Historical ADR text can describe superseded V1/V2 behavior. ADR-0027 and the
  current specifications determine the active boundary.
- ADR-0028 through ADR-0031 are proposed. They are design input only.
- The Git remote/archive-tag mismatch is reported as an open documentation and
  release-metadata defect, not silently resolved here.

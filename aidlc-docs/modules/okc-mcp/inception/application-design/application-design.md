# Application Design (Consolidated) — okc-mcp first Unit

**Status**: Application Design output (autopilot) — awaiting review. Consolidates [`components.md`](components.md), [`component-methods.md`](component-methods.md), [`services.md`](services.md), [`component-dependency.md`](component-dependency.md). Detailed business rules are deferred to Functional Design (Construction). Product/Construction gate remains in force.

## Overview
The first-Unit okc-mcp is a local, single-process, single-Vault, stdio MCP with a **simple layered architecture**: a thin **McpServer/ToolRegistry** surface exposes only safe tools that delegate to four **services** (Authoring, Audit, Discovery, Setup), which orchestrate six **core components** (VaultBoundary, HashService, FrontmatterEngine, NoteStore, BackupManager, AuditEngine). Every mutation flows through one **fixed safe write pipeline** so the trust boundary (REQ-008/011) and data-safety invariants (REQ-004/005) hold uniformly. Communication is in-process, synchronous, and unidirectional (surface → services → core); the dependency graph is a DAG with no cycles.

## Components at a glance
| ID | Component | Layer | Serves |
|---|---|---|---|
| C1 | VaultBoundary | core | REQ-008 |
| C2 | HashService | core | REQ-004, REQ-006 |
| C3 | FrontmatterEngine | core | REQ-005, REQ-013 |
| C4 | NoteStore | core | REQ-003, REQ-006 |
| C5 | BackupManager | core | REQ-004, REQ-009 |
| C6 | AuditEngine | core | REQ-007 |
| S1 | AuthoringService | service | REQ-003/004/005/013 |
| S2 | AuditService | service | REQ-007/002/008 |
| S3 | DiscoveryService | service | REQ-006/008 |
| S4 | SetupService | service | REQ-001/002/009/010 |
| X1 | McpServer/ToolRegistry | surface | REQ-001/011 |

## Fixed safe write pipeline (design invariant)
`assertWithinVault → (create: assert-not-exists | update: check expectedHash) → build content (FrontmatterEngine) → single external backup (updates only) → atomic writeInPlace`. Rejections short-circuit before any write or backup. See [`services.md`](services.md).

## Requirements coverage
| REQ | Covered by |
|---|---|
| REQ-001 installable local stdio MCP | X1, S4 |
| REQ-002 respect existing Vaults | S4, S2 |
| REQ-003 knowledge authoring (refuse overwrite) | S1, C4 |
| REQ-004 conflict-aware + single external backup, manual recovery | S1, C2, C5 |
| REQ-005 preserve structure; reject malformed YAML | C3, S1 |
| REQ-006 discovery (list, literal Korean search, read-with-hash) | S3, C4, C2 |
| REQ-007 heuristic audit (no compiler-pass claim) | S2, C6 |
| REQ-008 bounded authority | C1 (+ enforced by S1/S2/S3/C4/C5) |
| REQ-009 separate knowledge & tool state | C5, S4 |
| REQ-010 reviewable installation | S4 |
| REQ-011 trust boundary (no dangerous tools) | X1 |
| REQ-012 lifecycle records | N/A (AI-DLC artifact trail, not a runtime component) |
| REQ-013 in-note organization (no move/rename/merge) | S1, C3 |

## Story coverage (first Unit)
- Installer US-IN-01/02/03/04/05/06/07/08/09 → X1, S4, C1, C4/C5 (backup posture US-IN-07)
- Author US-AU-01 → S2/C6; US-AU-02/03/04 → S1/C3; US-AU-05 → S1/C2/C5; US-AU-06 → S1/C4; US-AU-07 → S3
- Reviewer US-RV-01 → S3/C2; US-RV-02 → S1/C2; US-RV-03 → C5; US-RV-04 → C5 (+ S1 restore path); US-RV-05 → C3

## Gaps & scope
- **No capability gap** for first-Unit REQ-001..011 and REQ-013; all 21 stories map to a component/service/method. REQ-012 is intentionally non-runtime.
- **MVP scope held**: no component implements file move/rename/merge, multi-Vault, real OKC ingestion, semantic search, Obsidian UI/Dataview, remote HTTP, deletion, or auto-approval (requirements §7 follow-ups / §8 exclusions). Single-Vault registration and the no-dangerous-tools surface are boundary *enforcement*, not deferred features.

## Deferred to Functional Design (Construction)
Exact content-hash algorithm; YAML comment-preservation strategy and merge semantics; backup naming/traceability format and location convention; audit heuristics per category and thresholds; deterministic list ordering rule; concrete tool input/output schemas and error payloads; atomic-write mechanism.

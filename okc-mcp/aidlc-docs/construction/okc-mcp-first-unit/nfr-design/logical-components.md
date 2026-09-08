# Logical Components & Resiliency Resolution — okc-mcp-first-unit

**Status**: NFR Design output (autopilot, standard). For a local single-process tool, the "logical components" are the same application components (no infrastructure logical components like queues/caches/circuit breakers). This doc records that mapping and resolves the enabled resiliency-extension design-stage decisions.

## Logical component mapping
| NFR concern | Logical realization | Infra component? |
|---|---|---|
| Path safety | `VaultBoundary` (P1) | No |
| Data protection | `BackupManager` + fixed pipeline (P2) | No (local files outside Vault) |
| Reliability | typed rejections (P3) + atomic write (P4) | No |
| Correctness | `FrontmatterEngine` + `HashService` (P5) | No |
| Bounded resources | bounds config enforced by VaultBoundary/NoteStore (P6) | No |
| Trust boundary | `McpServer/ToolRegistry` safe surface (P7) | No |
| Testability | PBT + behavior suites (P8) | No |

No queues, caches, circuit breakers, load balancers, or external stores are introduced — none are warranted for a local, single-Vault, offline tool.

## Resiliency extension — design-stage resolution (RESILIENCY baseline enabled)
The requirements-stage compliance summary already marked infra rules N/A. The design-stage decisions deferred there are resolved here (autopilot / MVP):

| Rule | Decision | Rationale |
|---|---|---|
| RESILIENCY-04 (deploy/rollback/CI) | CI **deferred** (local `npm run build`/`test` now); **rollback = reinstall previous tarball version** (version-pinned per D13); **deployment = direct local install** | Local dev tool; no pipeline warranted at MVP. |
| RESILIENCY-08 (regional topology) | **N/A** | No cloud/region; single local machine. |
| RESILIENCY-13 (failover/recovery procedures) | Recovery runbook = documented **manual restore from the single external backup** (RTO=manual) | Matches D12; no automated failover exists by design. |
| RESILIENCY-14 (chaos/DR testing) | **PBT + behavior tests now**; chaos/DR drills **deferred to Operations** | Test scenarios captured (P8); execution is an Operations activity. |
| RESILIENCY-15 (incident response) | **Lightweight**: actionable structured error messages + docs; no formal on-call/COE | Local single-user tool; no production service to page on. |

All other RESILIENCY rules (05/06/07/09/10/12 cross-region parts) remain **N/A** for a local single-process offline tool, as recorded in requirements §9. **No blocking resiliency findings** at NFR Design.

## Handoff to Code Generation
The design is complete and MVP-scoped. Code Generation (GATED) will implement components C1–C6, services S1–S4, surface X1, the fixed write pipeline, and the PBT/behavior test suites, finalizing the deferred concretes listed in [`../../../inception/application-design/application-design.md`](../../../inception/application-design/application-design.md) and [`../nfr-requirements/tech-stack-decisions.md`](../nfr-requirements/tech-stack-decisions.md).

# Code Generation — Source Traceability

The normative mapping remains [`docs/TRACEABILITY.md`](../../../../docs/TRACEABILITY.md).
This view groups current source by the single AI-DLC unit.

## Requirement-to-source map

| Requirement area | Primary implementation | Direct evidence |
|---|---|---|
| REQ-SNP/SRC | `okc-core/source.rs`, `snapshot.rs`, `source_io.rs`, `identity.rs`, `corpus.rs` | core unit tests, `corpus_builder_contract`, adversarial order property |
| REQ-PAR | `okc-core/parse.rs`, `ir.rs`, `json.rs` | parser tests, strict JSON mutations, corpus workflow |
| REQ-DED/CNF | `okc-core/dedup.rs`, private `plan.rs`, current `integration.rs` | dedup/path tests and evidence/contradiction closure |
| REQ-MAT/CMP/PRV | `okc-core/integration.rs`, `okc-app/artifact_service.rs` | compile/verify/explain, publication barrier, tamper/symlink/unplanned-file tests |
| REQ-AI | `okc-ai/src/lib.rs`, `okc-app/integration_execution.rs` | provider shape/schema/vector/retry tests, pipeline schema/cache tests |
| REQ-SEC | core source/artifact guards; app scanner/provider service; interop validation; TUI sanitation | hostile path/archive/JSON/provider/credential/artifact/terminal tests |
| REQ-INT | `okc-core/integration.rs`, `okc-app/integration_service.rs`, `project_state.rs` | taxonomy/disposition/evidence/critic/approval/regeneration/invalidation tests |
| REQ-SDK-001 | `okc-core/lib.rs`, `okc/main.rs`, `okc/commands.rs` | corpus public contract and `cli_contract.rs` |
| REQ-SDK-002 | `okc-interop`, `bindings/python`, `bindings/node` | interop tests; Python/Node API, typing, workflow, package tests |
| REQ-APP | `okc-app/workspace_bootstrap.rs`, `worker.rs`, `integration_service.rs`; `okc/tui.rs` | discovery/worker/reducer tests and POSIX PTY smoke |
| REQ-REL | `dist-workspace.toml`, parent `.github/workflows`, `docs/RELEASE.md` | local package checks and pending remote/signing gates |
| REQ-MCP/OBS | no production package | future specifications only |
| REQ-PERF | bounds, optional workspace, `corpus_probe.rs` | diagnostic 100k-note ingestion probe only |
| REQ-MEM | algorithm registry/experimental documents | isolation/promotion documentation only |

## Algorithm-to-source map

| Algorithm | Status | Implementation mapping |
|---|---|---|
| ALG-SNP-001 | normative | source/snapshot/source-I/O/identity/corpus modules |
| ALG-NRM-001 | normative | parser/IR/private planning path; current non-Markdown output excluded |
| ALG-DED-001 | normative | `dedup.rs` exact grouping |
| ALG-DED-002 | normative proposal-only | bounded MinHash/LSH candidates; scalable union path partial |
| ALG-CNF-001 | normative | private path/conflict analysis plus current safe taxonomy/materialization validation |
| ALG-PRV-001 | normative | plan-derived `ProvenanceRecord` construction/verification/explanation |
| ALG-SEM-001 | normative | app integration execution and core taxonomy validation; complete scale path partial |
| ALG-INT-001 | normative | current integration DTO/closure/materialization and app review service |
| ALG-CLM-001 | normative-future | no current compiler use |
| ALG-MEM-001–006 | experimental | no default compiler use |
| ALG-MEM-007 | research-only | no product use |

## Surface parity map

| Operation | Rust/app | CLI/TUI | Python | Node.js |
|---|---|---|---|---|
| Project create/open | `ProjectStore` | yes | yes | yes |
| Source/config mutation | app services | yes | yes | yes |
| Preflight/integrate | `IntegrationService` | yes | yes | yes |
| Taxonomy/cluster review | `IntegrationService` | yes | yes | yes |
| Compile current directory | core/app | yes | yes | yes |
| Verify current directory | core/ArtifactService | yes | yes | yes |
| Explain one output | core/ArtifactService | yes | yes | yes |
| Cwd discovery | `WorkspaceBootstrap` | CLI/TUI only | no | no |
| Native keychain/update | optional app features | CLI/TUI only | no | no |
| Pack/old-schema execution | absent | absent | absent | absent |

## Negative traceability

Absence is tested where it is part of the contract: retired CLI shapes, Pack
options, old-schema decoding, command providers, credential-bearing binding
profiles, relative binding paths, silent remote consent, and generic artifact
result shapes must continue to fail or remain unavailable.

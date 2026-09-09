# Functional Design — Business Rules

## Source and corpus rules

| Rule | Requirement/algorithm | Enforcement |
|---|---|---|
| BR-001 Source bytes, permissions, and timestamps are never intentionally modified | REQ-SNP-001; ADR-0003 | read-only source pipeline and immutability tests |
| BR-002 Each accepted source/file/document/block/output has stable domain-separated identity | REQ-SNP-002; ALG-SNP-001 | typed hashes and sealed records |
| BR-003 MCP/tool names cannot influence source IDs, corpus, approval, or output | REQ-SRC-001; ADR-0016 | no origin field in source/domain DTOs |
| BR-004 Source order cannot affect corpus/output; duplicate IDs or whole-Vault bytes fail | REQ-SRC-002 | sorted source set and duplicate validation |
| BR-005 Source paths are strict relative portable UTF-8 with retained original spelling | REQ-PAR-001, REQ-SEC-001; ALG-NRM-001 | NFC/logical path and separate original path |
| BR-006 Symlinks, special files, traversal, unsafe archives, and resource overrun fail closed | REQ-SEC-001 | snapshot/source I/O guards and bounds |
| BR-007 Exact groups retain all provenance; near similarity is proposal-only | REQ-DED-001/002; ALG-DED-001/002 | private analysis and current approval closure |

## Provider and disclosure rules

| Rule | Requirement/algorithm | Enforcement |
|---|---|---|
| BR-008 Only supported capability-driven provider kinds may be configured | REQ-AI-001 | strict enum and profile validation |
| BR-009 Provider requests/responses are bounded, hashed, recorded, and locally validated | REQ-AI-002/003 | portable schema, strict JSON, recording objects |
| BR-010 Providers never approve, mutate project/source state, or publish | REQ-AI-002; ADR-0004 | package/trait boundary |
| BR-011 Sensitive scan precedes disclosure and never stores matched text | REQ-SEC-002 | versioned block/metadata findings |
| BR-012 Remote cache misses need explicit consent for that call; consent is not persisted | REQ-SEC-002, REQ-SDK-002 | service/interop booleans and CLI flags |
| BR-013 Affected sensitive semantic work must use a local profile | REQ-SEC-002 | per-role route authorization |
| BR-014 Secrets are referenced, resolved just in time, zeroized/redacted, and never serialized | REQ-SEC-003 | profile constraints and secret wrappers |
| BR-015 HTTP redirects, credential URLs, unbounded bodies, or elapsed deadlines fail | REQ-SEC-001/003 | transport validation and total retry budget |

## Integration and review rules

| Rule | Requirement/algorithm | Enforcement |
|---|---|---|
| BR-016 Every Markdown document belongs to exactly one taxonomy cluster | REQ-INT-001; ALG-SEM-001 | taxonomy coverage equality |
| BR-017 Every block and metadata value has exactly one disposition | REQ-INT-002; ALG-INT-001 | exact target-set equality |
| BR-018 Every non-empty section cites current cluster-owned source evidence | REQ-INT-003 | ID/hash/cluster validation |
| BR-019 Contradictions retain at least two independently evidenced contextual claims | REQ-CNF-001, REQ-INT-003 | contradiction validation |
| BR-020 Every cluster, including a singleton, requires synthesis, critic, and approval | REQ-AI-004, REQ-INT-004/005 | complete cluster revision set |
| BR-021 Critical/major findings block; minor findings need exact current waivers | REQ-INT-004 | critic/waiver validation |
| BR-022 Omission takes effect only with matching target/hash, curator, and rationale | REQ-INT-002/005 | omission approval validation |
| BR-023 All approvals are immutable/hash-bound and become stale on dependency change | REQ-INT-005 | append-only objects/journal and revalidation |
| BR-024 `ApprovedIntegrationPlan` is the only current compile authority | REQ-MAT-001 | public compile signature and plan validation |

## Artifact and publication rules

| Rule | Requirement/algorithm | Enforcement |
|---|---|---|
| BR-025 Compile performs no provider, keychain, environment-secret, or arbitrary process access | REQ-AI-003/004, REQ-MAT-001 | core-only offline materializer |
| BR-026 Destination is absent; stage is sibling; final commit is atomic no-replace | REQ-CMP-001 | platform-specific publisher |
| BR-027 A race winner is preserved and never deleted/replaced | REQ-CMP-001 | final namespace primitive and guard lifetime |
| BR-028 Verification derives inventory from the approved plan, not supplied manifest authority | REQ-PRV-001, REQ-SEC-001 | regenerated expected tree |
| BR-029 Every note/redirect has exactly one closed `ProvenanceRecord` | REQ-PRV-001; ALG-PRV-001 | deterministic ledger construction |
| BR-030 Current output is Markdown notes/redirects plus four audit files only | REQ-INT-006 | current materializer and verifier |
| BR-031 Schema 1/2 recognized markers return unsupported; no reader/migration is invoked | REQ-CMP-003; ADR-0027 | bounded artifact detector |
| BR-032 Current OKCPack and non-Markdown materialization remain absent | REQ-CMP-002, REQ-PAR-002/003 | negative CLI/API/source tests and documentation boundary |

## Application and public API rules

| Rule | Requirement | Enforcement |
|---|---|---|
| BR-033 CLI and TUI call shared `okc-app` services | REQ-APP-001 | dependency/module structure |
| BR-034 Cwd discovery is bounded/deterministic; bindings never use it | REQ-APP-002, REQ-SDK-002 | WorkspaceBootstrap vs explicit interop path boundary |
| BR-035 Project mutations are single-writer and stale current authority before configuration replacement | REQ-INT-005, REQ-APP-002 | project reservation, lock, journal order |
| BR-036 Job/work/progress queues are bounded and cannot hide terminal completion | REQ-APP-002, REQ-SDK-002 | sync channels and separate result slot |
| BR-037 Python/Node paths are explicit absolute lexical paths | REQ-SDK-002 | interop validation |
| BR-038 Python uses snake_case; Node uses camelCase without rewriting opaque user maps | REQ-SDK-002 | wrapper conversion boundary and tests |
| BR-039 Public callers branch on structured error code/category, not messages | REQ-SDK-001/002 | interop and CLI error families |
| BR-040 Stable release is forbidden until QG-001 through QG-008 pass | REQ-REL-001 | release procedure/current state |

## No-fallback rules

- No heuristic taxonomy or merge fallback when provider/validation fails.
- No plaintext credential fallback.
- No replacing rename or last-writer-wins publication.
- No silent old-schema migration.
- No automatic truth selection by source count or model confidence.
- No experimental algorithm promotion without an accepted ADR and evidence.

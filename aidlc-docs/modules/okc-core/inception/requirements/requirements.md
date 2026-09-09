# Requirements — Current-Project Reconciliation

> This document does not create a competing normative requirement set. The
> stable REQ identifiers and exact MUST/MUST NOT language live in
> [`docs/specs/product-and-scope.md`](../../../../../okc-core/docs/specs/product-and-scope.md)
> and [`docs/TRACEABILITY.md`](../../../../../okc-core/docs/TRACEABILITY.md). This AI-DLC
> artifact connects session-grounded intent to that current contract and to the
> as-built Construction documentation.

## Intent analysis

- **Initial request, 2026-09-07**: reconstruct AI-DLC documentation using this
  project's Codex session history.
- **Continuation request, 2026-09-08**: complete the documentation through the
  Construction phase based on the currently configured project.
- **Request type**: system-wide brownfield documentation/reconciliation.
- **Behavior change**: none authorized or required.
- **Depth**: comprehensive because the documented system spans hostile-input
  handling, AI disclosure, immutable approval, native publication, three public
  language surfaces, and release gates.
- **Execution mode**: one-pass batch draft followed by review, continuing the
  user's previously recorded process choice.

## Current product statement

OKC compiles immutable Obsidian Vault snapshots into a new deterministic,
auditable Schema 3 Markdown directory. AI is mandatory for creating a new
integration, but provider output is proposal-only. Local validation, an
independent critic, and explicit curator approval produce the sealed
`ApprovedIntegrationPlan`; compile, verify, and explain are offline and
provider-free.

## Functional requirements mapped from intent

Session citation keys S1–S15 are defined in
[`decision-intent-timeline.md`](../reverse-engineering/decision-intent-timeline.md).
Status is copied conceptually from current specifications and traceability; the
authoritative status remains there.

| ID | Session-grounded requirement | Normative mapping | Current classification |
|---|---|---|---|
| FR-1 | Ingest multiple Obsidian Vault snapshots and integrate them into one new output | REQ-SNP-001/002, REQ-SRC-001/002 | Implemented for 1–10 directory/ZIP/tar.zst sources |
| FR-2 | Source meaning and identity must be MCP-origin-neutral | REQ-SRC-001; ADR-0016 | Implemented; no MCP identity enters corpus/output |
| FR-3 | Never mutate source Vaults or embed raw source copies in output | REQ-SNP-001, REQ-PRV-001; ADR-0003/0006 | Implemented current slice; broader platform race proof open |
| FR-4 | Detect exact/near duplicates and preserve conflicts with provenance | REQ-DED-001/002, REQ-CNF-001, REQ-PRV-001 | Exact/current closure implemented; scalable semantic path partial |
| FR-5 | AI is required to create a Schema 3 semantic integration | REQ-AI-004; ADR-0022 | Implemented vertical slice; scale/hierarchy work remains |
| FR-6 | Any supported LLM fits a provider-neutral capability interface with record/replay | REQ-AI-001/003; ADR-0004/0023 | Five provider shapes implemented; real-provider matrix open |
| FR-7 | Independent critic and explicit human approval gate every compiled result | REQ-AI-002, REQ-INT-004/005; ADR-0024 | Implemented for current records |
| FR-8 | One executable named `okc` exposes CLI and TUI | REQ-APP-001; ADR-0019 | Implemented |
| FR-9 | TUI covers provider connection, Vault selection, preflight, taxonomy/cluster review, compile, and verify | REQ-APP-001/002; ADR-0025 | Implemented local synthetic workflow; all-host/real-provider gaps remain |
| FR-10 | `cd <workspace> && okc` uses bounded cwd discovery and shared application services | REQ-APP-002; ADR-0025 | Implemented |
| FR-11 | Secrets use named environment variables or opaque native keychain accounts | REQ-SEC-003; ADR-0025 | Implemented; native CI matrix open |
| FR-12 | Installation/update ergonomics should resemble a single command-line tool | REQ-REL-001; ADR-0020/0021 | Config/client present; stable signed publication absent |
| FR-13 | Python and Node libraries expose the same workflow through one Rust facade | REQ-SDK-002; ADR-0026 | Locally implemented; remote native/package matrix open |
| FR-14 | Main contains only the current Schema 3 implementation | REQ-CMP-003; ADR-0027 | Implemented in source; repository/tag metadata drift needs reconciliation |
| FR-15 | Output is deterministic and independently verifiable; the Pack intent remains future | REQ-CMP-001/002/003, REQ-MAT-001 | Directory implemented; current OKCPack correctly absent |
| FR-16 | README and quickstart-style guide describe current CLI/TUI/SDK behavior | QG-007; REQ-SDK-001/002 and REQ-APP-001 context | Implemented, with documentation gates maintained |
| FR-17 | Realistic interlinked demo Vaults exercise the flow | Development tooling | Implemented as `demo/VAULT_A/B/C`; not a normative product REQ |

## Non-functional requirements

| ID | Requirement | Normative mapping | Acceptance boundary |
|---|---|---|---|
| NFR-1 Determinism | Same approved inputs produce the same IDs, paths, provenance, and bytes | REQ-SNP-002, REQ-CMP-001/002; QG-002 | Cross-language inventory SHA-256 remains `452ca0671e806a93b4f36f218cf9e62da899f6404c74705c2cf0ca14e413c7e5` |
| NFR-2 Provenance | Every output note/redirect closes to exact source, proposal, critic, approval, and plan | REQ-PRV-001; QG-003 | Missing/foreign/stale evidence fails verification |
| NFR-3 Security | Treat sources, provider output, project state, paths, and artifacts as hostile | REQ-SEC-001/002/003; QG-004 | Bounds, no-follow/no-clobber, disclosure, redaction, strict decoding; full fuzz/platform proof open |
| NFR-4 Portability | Current behavior and packages target Linux/Windows/macOS declared hosts | REQ-SDK-002, REQ-REL-001 | Local/emulated evidence is not the complete native matrix |
| NFR-5 Performance | Qualify 10 Vaults, 100,000 notes, 20 GB, including semantic candidates | REQ-PERF-001; QG-006 | No accepted current numeric time/RSS budget; gate not passed |
| NFR-6 Reliability | Resume complete tasks, invalidate stale authority, bound work, publish atomically | REQ-INT-005, REQ-APP-002, REQ-CMP-001 | Caught failure coverage exists; crash/recovery/ancestor races remain |
| NFR-7 Maintainability | Keep one policy owner, docs-first traceability, safe Rust, strict types | ADR-0001/0002; QG-007 | Specs/algorithms/ADR/current state/traceability stay synchronized |
| NFR-8 Supply chain | Locked, auditable, reproducible, signed multi-platform distributions | REQ-REL-001; QG-008 | Partial local packaging only; stable publication prohibited |
| NFR-9 Usability | CLI/TUI and SDKs expose predictable progress, errors, review, and recovery | REQ-APP-001/002, REQ-SDK-001/002 | Stable CLI exits and structured SDK errors; accessibility is terminal-focused |

## Mechanical completeness invariants

1. Every current Markdown document occurs in exactly one approved taxonomy cluster.
2. Every source block and frontmatter value has exactly one disposition.
3. Every non-empty synthesis section has current cluster-owned evidence.
4. Contradiction sets preserve at least two independently evidenced claims.
5. Critical/major critic findings block; minor findings need exact waivers.
6. Omissions need exact curator-bound rationale and approval.
7. Every authority record is immutable and hash-bound; dependent change makes it stale.
8. `ApprovedIntegrationPlan` is the only materialization authority.
9. The destination is absent and the final namespace operation cannot replace a winner.
10. Verification derives allowed files and bytes from the plan, not from an untrusted manifest.

## Public surface constraints

- Rust exposes current corpus and integration operations only.
- CLI exposes project/provider/integrate/integration/review/TUI, compile,
  directory verify/explain, doctor, and update only.
- Python and Node use explicit absolute paths, bounded jobs, per-call remote
  consent, structured errors, and interop schema 2.
- Language bindings do not perform cwd discovery, prompt, print, install global
  signal/tracing handlers, access the native keychain, or invoke updates.
- Recognizable Schema 1/2 markers return a typed unsupported-schema error; no
  current reader/migration is restored.

## Out of current scope

- A shipped MCP server or Obsidian plugin.
- Web/registry/cloud runtime infrastructure.
- A command-provider adapter.
- Attachment, Canvas, or Base materialization and complete link rewriting.
- A current OKCPack writer/reader.
- Experimental memory/retrieval algorithms in the default compiler.
- A stable release or package publication claim.

## Algorithm status boundary

| Status | IDs | Construction treatment |
|---|---|---|
| Normative | ALG-SNP-001, ALG-NRM-001, ALG-DED-001/002, ALG-CNF-001, ALG-PRV-001, ALG-SEM-001, ALG-INT-001 | Mapped to current modules; partial portions remain explicit |
| Normative-future | ALG-CLM-001 | Not part of current compilation |
| Experimental | ALG-MEM-001 through ALG-MEM-006 | Isolated from default path |
| Research-only | ALG-MEM-007 | No product commitment |

## Extension disposition

The Security Baseline, Resiliency Baseline, and Property-Based Testing
extensions are installed but were not explicitly opted in. They remain
disabled for this batch. The repository's own normative security/reliability/
test rules are nevertheless fully represented above.

## Resolved continuation choices

- Complete AI-DLC artifacts through Construction: authorized by the current request.
- Use one product unit (`okc-schema3-product`) with seven internal packages:
  selected because OKC is a local modular product, not independently deployed services.
- Skip User Stories: this task changes documentation only; session FRs provide
  the traceability input.
- Skip Infrastructure Design: no runtime infrastructure change exists.
- Execute Code Generation retrospectively: inventory and trace existing code,
  without changing application behavior.

## Still-open product/release decisions

- Whether/when to accept ADR-0028 through ADR-0031.
- Exact QG-006 budget and reference machine.
- Current Pack/non-Markdown format, manual amendment, scanner exception, and
  complete cancellation/recovery contracts.
- Official repository/remote and rewritten archive-tag reconciliation.
- Protected native release/signing/publication authority.

## Construction trace

The detailed design begins at
[`application-design.md`](../application-design/application-design.md) and
continues at [`construction/README.md`](../../construction/README.md).

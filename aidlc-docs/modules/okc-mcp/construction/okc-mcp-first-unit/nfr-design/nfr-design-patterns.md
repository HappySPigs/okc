# NFR Design Patterns — okc-mcp-first-unit

**Status**: NFR Design output (autopilot, standard). Patterns that realize the NFRs for a local, single-process, offline tool. Referenced rules from [`../functional-design/business-rules.md`](../functional-design/business-rules.md).

## P1 · Single path-safety choke point (Security — NFR-SEC-1)
All filesystem access routes through `VaultBoundary`; no component calls `fs` directly for note paths. Resolution uses realpath + descendant check + link/hidden rejection. Benefit: one auditable place enforces REQ-008; impossible to bypass by construction (core components receive only the boundary-checked API).

## P2 · Fixed safe write pipeline (Reliability/Data-protection — NFR-DR-*)
Every mutation goes through `applyMutation` (validate → exists/hash check → build content → single external backup → atomic write). No alternative write path exists. Benefit: REQ-004/005/008 hold uniformly for create/update/standardize/fix-yaml/reinforce; new tools cannot silently skip backup or hash checks.

## P3 · Typed fail-safe rejection (Reliability — NFR-DR-3)
All refusals are one of `{path-denied, hash-mismatch, malformed-yaml, overwrite-refused, bounds-exceeded, not-found}`, raised before any side effect. No partial writes, no backup on rejection. Surface serializes to structured MCP errors. Benefit: predictable, testable failure behavior; no corrupt intermediate state.

## P4 · Atomic write (Reliability — NFR-DR-3)
Write to a temp file in the same directory, then atomic rename over the target (internal mechanism, not a user-visible note move). Benefit: a crash mid-write leaves the original intact (BR-ATOMIC).

## P5 · Structure-preserving parse/serialize (Correctness — NFR-TEST-1)
CST/comment-preserving YAML handling; partial merge touches only requested keys. Property-tested round-trip fidelity. Benefit: REQ-005/013 — no silent loss of unknown keys, comments, or body.

## P6 · Bounded iteration, no cache (Performance/Security — NFR-PERF-1, NFR-SEC-3)
List/search/audit iterate with early enforcement of `maxFileCount`/`maxResponseBytes` and per-file `maxFileBytes`; refuse rather than truncate silently. No caching layer (freshness + simplicity). Benefit: deterministic termination; resource-exhaustion resistance.

## P7 · Capability minimization / untrusted input (Security — NFR-SEC-2)
The tool registry exposes only the safe tool set; note content is data only (never executed, shell-interpolated, or used to build outbound requests); no network client is present. Benefit: REQ-011 trust boundary is structural, not just conventional.

## P8 · Property-based + behavior testing (Testability — NFR-TEST-*)
`fast-check` properties for pure/serialization logic; targeted behavior tests for each rejection and the backup/atomic invariants. Benefit: correctness of the safety-critical logic is verified against generated inputs, not just examples.

## Patterns intentionally NOT used (justified N/A for a local tool)
- Retries / circuit breakers / bulkheads — no remote dependencies (only the local filesystem).
- Caching / connection pooling — no repeated remote/db calls.
- Auto-scaling / load balancing / multi-zone — single local process.

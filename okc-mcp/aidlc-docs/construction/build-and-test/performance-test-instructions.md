# Performance Test Instructions — okc-mcp (first Unit)

Web follow-up (2026-09-09): HTTP is bounded by per-request timeout and cancellation, streamed per-response bytes, maxFiles and per-snapshot maxScanBytes. A transient note cache exists only within one operation. `tests/web.test.ts` checks these bounds; no production throughput/latency claim is made. First-unit local-only statements below are historical.

## Applicability
**Largely N/A for the first Unit.** This is a local, single-process, on-demand stdio tool with **no throughput/latency SLA**, no concurrent users, no network, and no deployed service (NFR-PERF-1, NFR-SCALE-1, NFR-AVAIL-1). Formal load/stress testing is deferred (RESILIENCY-14 → Operations).

## What replaces load testing here: bounded-resource guarantees
Performance for this tool means **deterministic termination within configured bounds**, not tail-latency targets. These are enforced and tested as behavior:
- **`maxFileBytes`** — a single note over the limit is refused (`bounds-exceeded`), not read/written. (`tests/vault.test.ts`)
- **`maxFileCount`** — directory/scan enumeration stops at the cap (`SCAN_LIMIT`). (`tests/vault.test.ts`)
- **`maxScanBytes`** — search/audit stop accumulating past the cap. (`tests/server.test.ts`)
- **`maxResponseBytes`** — an over-budget response is refused rather than truncated, and the stdio connection survives. (`tests/server.test.ts`)
- No caching, no unbounded waits, no network calls (P6).

## Optional local smoke check (not a gated SLA)
On a Node ≥ 22.13 host, to sanity-check interactive latency on a realistically sized Vault:
```bash
# Point a config at a copy of a large Vault, then time representative calls:
time node dist/cli.js doctor --config /absolute/okc-mcp.json     # list/scan within bounds
# Drive list_notes / search_notes / audit_vault via an MCP client and observe they return
# within the configured bounds (or refuse with bounds-exceeded) and terminate deterministically.
```

## Deferred (follow-up Units / Operations)
Real load/stress/scalability testing on large user Vaults and OS matrices (requirements §7 follow-up 4; RESILIENCY-14).

## Session capture bounds

Session scanning inherits maxFiles/maxScanBytes/maxNoteBytes. Topic, query, item and candidate counts are bounded. Captures reserve receipt response space before any write; preview snippets report truncation. Bounds and cancellation have deterministic regressions in capture.test.ts. There is no new latency SLA or hosted inference benchmark.

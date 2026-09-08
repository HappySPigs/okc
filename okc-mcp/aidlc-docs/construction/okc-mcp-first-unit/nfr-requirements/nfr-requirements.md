# NFR Requirements — okc-mcp-first-unit

**Status**: NFR Requirements output (autopilot, minimal). Consolidates requirements §5 per-unit. This is a local, single-process, single-user, offline tool — scaling/availability/DR infra NFRs are largely N/A; security and correctness are first-class.

## Security (product-level, first-class — REQ-008/011)
- **NFR-SEC-1**: All filesystem access confined to the one registered Vault; symlink/hardlink/hidden-control paths rejected. (BR-PATH-*)
- **NFR-SEC-2**: No dangerous capability exposed (no shell, arbitrary HTTP, delete, auto-approval, internal AI); note content treated as untrusted; no network egress. (BR-TRUST-*)
- **NFR-SEC-3**: File size, file count, and response size are bounded to prevent resource exhaustion. (BR-BOUND-*)

## Data protection & reliability (REQ-004/005/009)
- **NFR-DR-1**: Every mutation to an existing file is preceded by a single external pre-change backup (RPO = last save point).
- **NFR-DR-2**: Recovery is documented manual restore (RTO = manual); no generational retention/auto-cleanup.
- **NFR-DR-3**: Conflict-aware (`expectedHash`) writes; atomic writes; malformed-YAML rejection; structure preservation. Failures are fail-safe.

## Correctness & testing (REQ-005/006/007; D11 PBT-partial)
- **NFR-TEST-1**: Property-based tests for pure functions + serialization round-trips: `parse↔serialize` fidelity (unknown keys/comments/body preserved), `mergePartial` only-specified-keys, `hash` determinism, literal Korean search.
- **NFR-TEST-2**: Behavior tests for path-safety rejection, hash-mismatch rejection, refuse-overwrite, single-backup/none-on-reject, bounds-exceeded.
- **NFR-TEST-3**: Audit output must never assert OKC compiler validation passed.

## Performance & capacity (local)
- **NFR-PERF-1**: Interactive local latency; no hard SLA. Audit/search/list terminate deterministically within configured bounds. No unbounded waits; no network calls.

## Scalability / availability (mostly N/A)
- **NFR-SCALE-1**: Single local process, single Vault, on-demand — no horizontal scaling, no multi-tenant, no auto-scaling. Capacity governed by resource bounds (NFR-SEC-3).
- **NFR-AVAIL-1**: No uptime SLA; runs on demand over stdio. No cross-region/HA (N/A — local tool).

## Release governance (D13)
- **NFR-OPS-1**: Lightweight — version tagging + CHANGELOG + release notes. Local npm tarball install; rollback = reinstall previous version.

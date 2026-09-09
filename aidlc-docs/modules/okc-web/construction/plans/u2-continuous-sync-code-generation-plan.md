# U2 continuous Vault synchronization

Scope: module-local implementation under the root cross-module integration initiative.
Authorization: the user requested “AIDLC 모두 확인해서 검증하고, 없는 부분들 구현해”.
The existing authorized implementation workflow continues; existing histories remain intact.

## Requirements and design

This extends FR-UP-1..4 and E2-S1..S6 from one-time ingestion to repeated snapshots.
The existing curator review, immutable source bytes, ten distinct source limit, and
manual freeze/integrate/approve/compile/publish decisions remain authoritative.

- One upload capability identifies one stable source. Rotation retains that identity.
- Both multipart uploads and hooks synchronize immutable full source revisions.
  Replacing a source revision reflects deleted and renamed files without appending
  another source. Historical landed snapshots remain available for provenance.
- Hooks use `/api/sync/negotiate`, `/api/sync/blob/{hash}/{offset}`, and
  `/api/sync/commit`, authenticated with the existing upload token in a Bearer header.
- CBOR follows the hooks Rust serde contract: digests and file bytes are integer
  arrays. The canonical manifest hashes sorted path, SHA-256, and byte-count tuples.
- Negotiation persists a session, its base source revision, and expected manifest.
  Subsequent requests carry `X-OKC-Upload-Session`; resumable offsets are server-owned.
  Exact chunk and commit replay is idempotent. A changed base returns HTTP 409.
- Commit verifies all bytes and performs core add/rebind through the single writer
  before returning success. The source row, revision history, token usage, and
  durable receipt are updated transactionally. Failed DB finalization can be retried
  against the same immutable landing and idempotent core source binding.
- Limits cover CBOR body size, file count, snapshot bytes, chunk bytes, and safe
  relative paths. File path aliases and overlapping file/directory paths are rejected.
- Core stays behind the adapter. Existing `rebind_source` is reused. No semantic
  merge or provider call is added to ingestion.

## Plan

1. [x] Read common AI-DLC rules, existing module state, upload requirements/stories,
   component/service design, prior U2 plan, and build/test evidence; coordinate wire contract.
2. [x] Add persistent session/revision schema and typed source rebind adapter.
3. [x] Implement stable source commit semantics and retain multipart upload behavior.
4. [x] Implement bounded authenticated CBOR negotiation, chunk transfer, and commit.
5. [x] Preserve source identity through token rotation and expose hooks configuration.
6. [x] Add real-binding tests for repeated revisions, deletion/rename, retries,
   stale bases, restart/resume, content/path validation, token scope, and source limits.
7. [x] Run focused pytest, ruff, and mypy; fix failures and record exact results.
8. [x] Update module state, audit, API/design summary, and build/test instructions.
9. [x] Close the independently verified native error-boundary gap: map synchronous
   binding validation exceptions at every public engine adapter method, test real
   malformed calls and HTTP errors, and correct the invalid design reference link.
10. [x] Recover the final-chunk fsync-to-cache-publish crash window during negotiation:
    verify/promote complete partials or reset corrupt bytes, then run regression tests.
11. [x] Reject NFC/NFD-equivalent path and ancestor collisions before negotiation
    or multipart extraction, preserving original path bytes for manifest hashing;
    verify that a rejected update cannot mutate the existing source.

Extensions: Security Baseline, Resiliency Baseline, and Property-Based Testing remain
disabled by existing module configuration. Their rules are skipped; functional
input-validation and persistence requirements continue to apply.

Content validation: CommonMark prose and balanced code spans; no diagrams or
embedded machine-readable blocks require additional rendering validation.

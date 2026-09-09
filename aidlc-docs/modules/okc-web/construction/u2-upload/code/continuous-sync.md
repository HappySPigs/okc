# Continuous snapshot uploads — implementation and contract

Status: implemented and verified with real native bindings, 2026-09-09 Korea time.
Plan: [continuous-sync code generation](../../plans/u2-continuous-sync-code-generation-plan.md).
This extends FR-UP-1..4 and E2-S1..S6. Existing manual curator approval and semantic
compilation policies remain unchanged. Full snapshot replacement is supported;
incremental semantic recompilation is still outside the current contract.

## Stable identity and history

An upload token owns `source_identity`; rotation keeps that identity. The first
upload adds the source. Later uploads rebind its path and snapshot ID using the
existing core API. Source count is distinct Vaults, not upload count, and remains
bounded at ten. The `sources` row represents the current revision. Its content
hash changes the existing project source-set fingerprint, invalidating freeze.
`source_revisions` retains prior hash/path pairs; landed bytes are immutable.

Multipart uploads retain the existing URL, asynchronous job response, validation
codes, and duplicate-content rejection. Their hash now covers canonical path,
file SHA-256, and file size, so archive timestamps do not define Vault identity.
Repeated requests sharing a capability share one in-flight source reservation.

Schema migration 2 adds/backfills token source identity and creates revision and
transfer-session tables. Old projects and upload tokens remain usable. Existing
historical sources are not silently collapsed or deleted.

## Hooks HTTP API

Base endpoint: `https://<host>/api/sync`. Token issuance includes the relative
`sync_endpoint` value `/api/sync`. Use the existing upload token in
`Authorization: Bearer <token>`; do not put it into the machine endpoint URL.
All request and success bodies use `application/cbor`.

| Method and suffix | Request | Success response |
|---|---|---|
| POST `/negotiate` | `manifest_digest`, `entries` triples of path/hash/size | `server_has`, `session_id`, `resume_offsets` |
| PUT `/blob/{sha256_hex}/{offset}` | `blob`, `offset`, `len`, `chunk_sha256`, `bytes` | `received` absolute byte offset |
| POST `/commit` | `manifest_digest`, `path_hash_map` containing `entries` map of path to hash | `committed: true`, `server_vault_content_id` |

After negotiation, send `X-OKC-Upload-Session` on blob and commit requests. Digests
are CBOR arrays of 32 unsigned byte integers; chunk bytes are also integer arrays,
matching Rust serde/ciborium. Sizes and offsets are unsigned integers. The server
rejects body/URL mismatches, unsafe paths, duplicate NFC-normalized and case-folded
paths, file/directory overlap, undeclared blobs, wrong lengths, and checksum
mismatches before binding. Normalization applies only to collision keys; exact
original paths remain part of the manifest digest. Multipart ZIP ingestion applies
the same portable file/ancestor collision checks before materialization.

Manifest digest is SHA-256 of the concatenation of sorted entry records. Each record
contains: UTF-8 path length as big-endian u64, UTF-8 path bytes, raw 32-byte SHA-256,
and size as big-endian u64. There is no domain prefix. An empty manifest hashes
the empty byte string.

## Commit, retries, and conflicts

Negotiation persists the current source hash as the session's base revision.
Identical negotiation resumes its session and returns server-owned offsets. Complete
cached blobs are reused only after verifying their expected size and SHA-256.
Repeated chunks must match stored bytes. Full blob checksums are verified before
cache publication and again before source materialization.
Negotiation recovers a crash between final chunk fsync and cache publication by
verifying and promoting the complete partial file. A corrupt complete partial
is reset to offset zero, so the client cannot skip a missing committed blob.

Commit first verifies the exact negotiated manifest and every blob. It stages a
new directory, verifies the canonical file manifest, then serially calls the real
core add/rebind. Only after the binding succeeds does one SQLite transaction update
the current source, revision history, token usage, and committed session receipt.
The response is a completed commit, not a queued job acknowledgment. Replaying a
committed session returns its stored content identity and cannot revert a newer
revision. A stale uncommitted base returns HTTP 409 `PROJECT_BUSY`; re-negotiate.
If core succeeds but SQLite finalization fails, the same session recovers through
the native source binding and finalizes the database without adding another source.

Deleting or renaming files replaces the full current snapshot. An empty Vault
gets an empty `.obsidian/` directory marker so the existing core source-selection
API recognizes it; no file or invented knowledge enters the manifest. A project
with no knowledge documents may have no semantic output to compile.

## Bounds and remaining scope

- Decoded chunk bytes: 8 MiB; encoded CBOR request: 17 MiB.
- Files: 100,000 maximum; full snapshot bytes: 128 MiB by default, configurable
  through `OKC_WEB_MAX_SYNC_BYTES`.
- Pending sessions: at most 16 per token and current base revision.
- Input is single-writer at core commit. Old snapshots and blobs are retained;
  automated retention/garbage collection is not introduced by this change.
- Archive multipart ingestion still accepts ZIP and Markdown; tar.zst support
  remains excluded from this web build. Core compiled output remains Markdown.
- HTTPS termination is supplied by deployment; the hooks client requires HTTPS.

## Evidence

`tests/test_hooks_sync.py` covers real add/rebind across 13 revisions, rename/delete
and empty Vaults, authoritative resume offsets, chunk replay, stale bases, durable
commit replay, token isolation/rotation, malformed content and limits, ten distinct
sources, and recovery after injected SQLite failure following native success.
Its wire-contract test consumes the actual Rust `protocol-fixture` executable.

The adapter also maps native validation exceptions raised before a Job exists;
malformed taxonomy edits and relative paths produce structured HTTP 400 errors.
`tests/test_adapter_validation.py` verifies nine real-binding/HTTP cases.

Focused aggregate: 63 pytest tests passed (hooks receiver, legacy uploads,
adapter validation, foundation, and production W1 spine). Ruff passed; mypy passed for the 16 owned
source/test files. Existing Starlette/httpx deprecation warnings remain.
See [verification instructions](../../build-and-test/continuous-sync-verification.md).

Five additional portable-path regressions were subsequently added: composed versus
decomposed Unicode filenames and file/ancestor aliases, plus case-insensitive ZIP
collisions. Updated upload suites: 35 tests passed; Ruff clean; mypy clean for nine
files. Rejected updates preserve the existing native source and create no revision.

Extension compliance: configured Security, Resiliency, and PBT extensions are
disabled and N/A. Existing functional authentication, validation, immutability,
single-writer, and provenance constraints apply and are covered by the tests.

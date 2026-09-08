# Versioned Serving Implementation

Continuation requirements: root INT-05/06 and module E5-S1..S5. Current user direction authorizes designing and filling the original four-module functional gaps.

## Functional behavior

Successful compile records the actual output path after native core verification, plus CompiledVaultManifest, its byte SHA-256 revision, file hashes, input fingerprint, project configuration fingerprint, review-audit fingerprint and owner labels. No directory mtime inference remains. Recompiling an already verified approved plan to a new absent path is permitted for crash/corrupt-storage recovery.

Snapshot records retain publication history. Recompilation of identical manifest bytes refreshes the verified physical copy/context without changing revision or overwriting original files. Publish/restore reverify the selected artifact, then atomically update publication and revision pointer. Restore may intentionally expose an older stale revision. Unpublish denies all revisions.

Read endpoints /contract, /files, /file, /verify and /explain accept revision. Unknown revisions are rejected rather than silently selecting latest. Registered historical revisions stay readable while the project is published. File reads reject unregistered paths, traversal, symlinks, byte-limit excess and hash changes. Provenance uses core's relative output-path argument and snapshot owner labels. Existing legacy publication rows remain readable without revision; recompile/publish upgrades them.

## Access API

Admin session is required for:

- PUT /api/projects/{id}/serving/access with mode public or private.
- POST /api/projects/{id}/serving/tokens to issue a read-only token; plaintext returned once.
- DELETE /api/projects/{id}/serving/tokens/{token_id} to revoke.
- GET /api/projects/{id}/serving/history.
- POST /api/projects/{id}/serving/restore with revision.

Private machine reads require Authorization Bearer. Only token hashes are stored, scope is one project, upload tokens cannot read private artifacts. Existing projects default to explicit legacy public mode. Access administration is available through the API; no new frontend settings UI is claimed.

## Design rationale and limitations

Keep original immutable-output/evidence-preserving core policies. Keep serialized native mutations and explicit curator steps. New tables are idempotently created by SnapshotStore in the existing SQLite state DB; no cloud infrastructure introduced. Snapshot content is not copied to a second mutable cache. Metadata fingerprints provide conservative freshness: even a rejected recorded review can label a publication stale until a fresh verified compile confirms unchanged content.

No automatic LLM approval or vector index was introduced. Legacy outputs require a verified recompile to obtain versioned receipts. Native core remains the verification authority.

## Verification

Four new tests in tests/test_serving_revisions.py exercise real core with the core's deterministic loopback provider: successful upload/review/compile/publish, custom output path, revision consistency across repeat source changes, provenance, private token enforcement/revocation, restore/unpublish, tamper rejection, identical-revision storage repair, and recovery after rejected review.

Existing serving/orchestration tests also pass. Aggregate current counts and actual MCP/Rust interoperability belong to the [root verification report](../../../../../aidlc-docs/construction/build-and-test/build-and-test-summary.md). Provider responses are synthetic, not live-model quality evidence.

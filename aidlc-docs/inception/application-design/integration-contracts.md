# Integration Contracts and Unit Design

## Upload boundary (INT-01/02/08)

Hooks retains canonical CBOR and SHA-256 framing. New web receiver uses a Bearer upload token under /api/sync with negotiate, blob and commit operations. Negotiation returns a session ID plus authoritative resume offsets; later steps carry X-OKC-Upload-Session. Session captures the source revision, ensuring concurrent/outdated commits cannot replace a later success. Commit acknowledgement follows successful core add/rebind and durable receipt, never merely queued acceptance. Existing multipart endpoints remain available.

A token's project/slot names one stable source. Revisions change content/path, not source identity. Full snapshot omission represents deletion; historical input directories remain immutable. Core's existing rebind_source is used by the web single-writer queue.

Detailed encoding belongs to hooks protocol and web sync implementation; both agents coordinate concrete DTOs and shared regression fixtures.

## Serving boundary (INT-03/05/06)

Existing GET /api/serving/{project_id}/contract, /files, /file, /verify, /explain remain. Responses add revision (manifest SHA-256); requests accept optional revision to address a registered immutable published snapshot. A missing revision never silently resolves to current. Unpublishing denies every revision. Re-publish/restore atomically changes the current pointer while retaining previous snapshot records.

Compile result path is recorded explicitly. Publish reads the actual verified CompiledVaultManifest, not ProjectManifest. Source fingerprint and project-manifest digest are kept separately from artifact corpus hash for freshness. Metadata includes plan/taxonomy IDs and source provenance.

Existing projects retain public mode. Admin can set private mode and issue/revoke scoped read tokens; only hashes are stored. MCP sends Bearer credentials; upload tokens do not grant reading.

## MCP boundary (INT-03/04)

Optional web config contains baseUrl (server root), projectId, token. Web configured selects remote by default; HTTP failures do not silently return local knowledge. Each read sequence first obtains contract revision then pins file requests. Reads/searches are bounded and return source/revision/stale/provenance metadata. Local authoring remains a separate source path. Web-only read-only configuration is supported. Initialization creates a conventional source Vault without replacing existing notes.

## Shared acceptance

One local source can be changed over eleven times without consuming eleven source slots. Delete/rename updates the current source, retries acknowledge once, stale sessions cannot roll back later revisions. A reader pinned to revision A continues reading A after B is published; private and unpublished projects enforce read policy. Core compiler output remains unchanged and independently verifiable.

Module implementation plans are linked from the root [execution plan](../plans/execution-plan.md). No root artifact replaces module requirements or histories.

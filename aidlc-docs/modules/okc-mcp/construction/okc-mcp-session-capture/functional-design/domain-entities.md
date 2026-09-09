# Session capture domain entities

- CaptureSession: stable non-empty sessionId (host identifier when available; otherwise generated and reused).
- CaptureTopic: stable item id, summary and bounded query variants; semantic interpretation belongs to the host.
- CandidateNote: local path/hash, title, aliases/tags, matching evidence and ATX section paths.
- CaptureItem: id, chosen path, expectedHash (null for a new note), optional title and section path, content, rationale.
- CaptureBlock: balanced Markdown comment markers binding SHA-256(sessionId) and itemId; contained prose is normal source knowledge.
- CaptureReceipt: session identity, preview/applied/partial status, per-file hashes, item IDs, backup IDs, verification and runtime failure details.

Markers are identity metadata, not approval or executable instructions. Existing records discovered across the bounded corpus are authoritative for repeat placement; a different destination for the same identity is refused rather than duplicating it.


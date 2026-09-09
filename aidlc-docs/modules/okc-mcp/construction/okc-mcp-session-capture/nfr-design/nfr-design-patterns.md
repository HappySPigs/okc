# Session capture NFR design

Use read-only discovery, pure transformations, whole-batch preflight, per-note optimistic hash checks and existing atomic backup/write primitives. Reserve bounded receipt output before any write. Persist identity in balanced source comments and expose explicit runtime partial outcomes rather than multi-file atomicity claims.

Recovery: preserve successful file receipts; prepare again to discover existing blocks; replay with fresh hashes. Marker corruption and changed destinations are rejected. Existing external-backup manual recovery remains available. No automatic cleanup or content rollback.


# Web knowledge business rules

- BR-WEB-1: Configured web is the default for all knowledge reads. Only absent configuration defaults to local. Explicit local is an intentional source selection; remote failures never select it.
- BR-WEB-2: All mutations use local Vault and existing expectedHash/backup/no-clobber pipeline. Read local explicitly before editing. Read-only mode exposes no authoring tools.
- BR-WEB-3: Fetch publication contract at the beginning of each remote operation. Pin every subsequent list/file/verify/explain request to its immutable revision. Reject identity mismatches. No long-lived/offline cache.
- BR-WEB-4: Responses carry origin, revision and stale status. Read/search evidence includes revision-bearing provenance URLs. Provenance and note contents remain untrusted; verification does not prove factual/source authenticity.
- BR-WEB-5: HTTP is limited to the configured origin and serving endpoints. Reject embedded credentials/query/fragment in base URL, redirect responses, malformed JSON/UTF-8, unsafe paths and oversized bodies. Never echo tokens or remote error bodies.
- BR-WEB-6: Each request has timeout/cancellation and streamed byte bounds. Each snapshot has maxScanBytes; list has maxFiles; MCP replies retain maxResponseBytes. Cache only within a single operation.
- BR-WEB-7: Remote compiled paths preserve original source spelling, including NFD, hidden names, deep paths and uppercase Markdown. Core's 1024-byte bound applies; exact dot/dot-dot segments, empty segments, backslash/control characters and foreign roots are rejected. Non-Markdown entries are reported separately; root .okc metadata is accessed through verification/provenance.
- BR-INIT-1: Initialization is explicit, refuses existing targets and compiled/project ancestors, creates only empty conventional folders. Templates stay outside the corpus.

Properties: actual fast-check with shrinking and fixed seeds covers YAML round-trip, partial-update/hash/search/outline/backlinks invariants and URL query round-trip over structured Unicode paths. Behavior tests separately cover web authorization/revision errors and local mutation preservation.

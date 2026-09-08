# Web knowledge domain entities

`KnowledgeSource`: local source Vault or configured published web project. Automatic selection chooses web whenever configured. `WriteTarget`: one local editable source Vault, optional only in web-only read-only mode.

`PublishedRevision`: project ID, manifest SHA-256 revision, live/stale state and bound corpus/taxonomy/plan identifiers. `WebSnapshot`: one operation's pinned revision and bounded transient note cache. `Evidence`: path, full-file SHA-256, content range, publication metadata and revision-bearing file/provenance URLs.

`SourceInitialization`: explicitly requested absent directory, existing validated parent, optional conventional empty folders. No sample knowledge is materialized. Template is an MCP resource outside the source corpus.

No entity authorizes writes to a compiled artifact. Existing local `Vault`, `BackupRecord`, hash and atomic update entities remain unchanged.

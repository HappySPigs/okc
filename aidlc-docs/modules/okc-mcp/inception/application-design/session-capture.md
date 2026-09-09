# Session capture application design

Single cohesive unit: okc-mcp-session-capture. No additional service, database or infrastructure.

- Host agent: identifies session topics, compares candidates and picks local notes/sections; follows the shipped prompt/guide.
- Capture discovery: reads bounded local corpus, presents lexical evidence and existing capture locations, and reports scan coverage.
- Capture renderer: pure marker and section transformations, stable item identity, duplicate/malformed marker detection.
- Capture application service: groups items by note, preflights all changes, previews or applies through the existing authoring pipeline, returns verified file receipts.
- MCP adapter: two local-authoring tools plus prompt/resource. prepare is read-only; apply defaults to preview.
- Vault: exposes a read-only prospective-write inspection so invalid new paths are rejected before a batch starts.

Dependency order: existing Vault/notes/authoring -> capture domain/service -> MCP adapter -> client guide/tests. Hooks/web/core contracts remain unchanged.


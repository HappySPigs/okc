# Umbrella Architecture

The desired dependency flow is local source Vault → hooks → web → core → compiled artifact → web serving → MCP → agent. MCP also authors local source Vaults. Curator decisions enter through web into core.

The current implementation has two disconnected boundaries: hooks/web upload and web/MCP retrieval. Web/core is a real Python-native integration, with publication identity and recurring source-update gaps.

| State | Owner |
|---|---|
| Source files and authoring recovery | Local Vault / MCP |
| Last committed upload manifest, dirty state, resume offsets | Hooks |
| Accounts, tokens, source registration, upload jobs, publication pointer | Web |
| Project journal, approvals, corpus and compiled artifact | Core |
| Remote reader cache/index, once introduced | MCP; not currently implemented |

Web contains the integration worker queue. It must coordinate source revisions, review checkpoints, and publication policy; core remains authoritative for compilation and evidence semantics.

Module architecture authority: [core](../../../okc-core/aidlc-docs/inception/reverse-engineering/architecture.md), [hooks](../../../okc-hooks/aidlc-docs/inception/application-design/application-design.md), [MCP](../../../okc-mcp/aidlc-docs/inception/application-design/application-design.md), [web](../../../okc-web/aidlc-docs/inception/application-design/application-design.md).

Detailed cross-module findings: [integration-gap-review.md](integration-gap-review.md).

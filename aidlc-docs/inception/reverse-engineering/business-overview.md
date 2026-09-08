# Umbrella Business Overview

OKC coordinates local Obsidian knowledge contributions, reviewed compilation, publication, and agent access.

| Transaction | Participating modules | Observed completeness |
|---|---|---|
| Author source knowledge | MCP → local Vault | Existing local note authoring; initialization/profile scope needs definition. |
| Submit changed source | Local Vault → hooks → web | Daemon exists; upload protocols disagree. |
| Review and compile | Web → core → curator → core | Existing explicit review flow; winner-selection meaning remains a decision. |
| Publish agent knowledge | Web → MCP → agent | Web serving exists; MCP remote reader is absent. |
| Update existing contribution | Hooks → web → core | Stable source revision replacement is not wired in web. |

Text context diagram: authors and agents use local source Vaults; hooks submits changes to web; web coordinates core and curator decisions; published artifacts should be consumed by MCP.

Vocabulary: a source Vault is an input contribution; a source revision is a successive snapshot of that contribution; a compiled Vault is an immutable reviewed artifact; publication selects which artifact readers see. These concepts must not share one identity accidentally.

Authoritative module descriptions: [core](../../../okc-core/aidlc-docs/inception/reverse-engineering/business-overview.md), [hooks](../../../okc-hooks/aidlc-docs/inception/requirements/requirements.md), [MCP](../../../okc-mcp/aidlc-docs/inception/requirements/requirements.md), [web](../../../okc-web/aidlc-docs/inception/requirements/requirements.md).

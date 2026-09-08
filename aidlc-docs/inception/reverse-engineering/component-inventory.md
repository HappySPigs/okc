# Umbrella Component Inventory

Four top-level AI-DLC modules exist.

| Module | Integration role | Module authority |
|---|---|---|
| okc-core | Compiler/application libraries, CLI and Python/Node bindings | [state](../../../okc-core/aidlc-docs/aidlc-state.md) |
| okc-hooks | Local daemon and ten Cargo workspace crates | [state](../../../okc-hooks/aidlc-docs/aidlc-state.md) |
| okc-mcp | TypeScript stdio MCP source authoring and local retrieval | [state](../../../okc-mcp/aidlc-docs/aidlc-state.md) |
| okc-web | FastAPI control/serving backend and React frontend | [state](../../../okc-web/aidlc-docs/aidlc-state.md) |

Shared integration infrastructure consists of root GitHub Actions workflows and the core Python wheel consumed by web. Module build manifests and lockfiles remain separate.

The [older web integration guide](../../../okc-web/aidlc-docs/integration/module-integration-guide.md) still describes hooks as an Obsidian plugin and MCP/web as unimplemented or in design. Treat these status statements as stale; do not copy them into current contracts. Its original module history remains preserved.

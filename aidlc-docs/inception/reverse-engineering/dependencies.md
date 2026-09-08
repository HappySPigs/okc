# Cross-Module Dependencies

| Consumer | Dependency | Binding |
|---|---|---|
| Web backend | Core | Built Python okc-compiler wheel / import okc |
| Hooks | Web | Intended HTTP upload dependency; contracts incompatible |
| MCP | Web | Intended default knowledge source; unimplemented remote reader |
| MCP and hooks | Local source Vault | Shared content and filesystem boundary |

Hooks has no required direct core library dependency; its approved design removed that coupling. MCP does not need to adopt core as a runtime dependency merely to read published files. Shared acceptance fixtures can validate authoring/core compatibility without duplicating compilation policy.

Root verification ordering should eventually cover core wheel → web → hooks/web upload → publication → MCP remote consumption. This is a future coordination recommendation, not an executed build sequence.

Version boundaries and evidence: [API inventory](api-documentation.md), [core dependencies](../../../okc-core/aidlc-docs/inception/reverse-engineering/dependencies.md), [hooks requirements](../../../okc-hooks/aidlc-docs/inception/requirements/requirements.md), [web build manifest](../../../okc-web/backend/pyproject.toml).

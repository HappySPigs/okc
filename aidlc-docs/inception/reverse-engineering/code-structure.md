# Integration Code Structure

Only files owning cross-module seams are inventoried here; module-local inventories and history remain in their own workspaces.

| Boundary | Current source |
|---|---|
| Hooks HTTP protocol | [protocol.rs](../../../okc-hooks/crates/upload-client/src/protocol.rs) |
| Hooks daemon orchestration | [daemon.rs](../../../okc-hooks/crates/watcher-bin/src/daemon.rs), [coordinator.rs](../../../okc-hooks/crates/watcher-bin/src/coordinator.rs) |
| Web upload and source registration | [router.py](../../../okc-web/backend/app/upload/router.py), [ingest.py](../../../okc-web/backend/app/upload/ingest.py) |
| Web/core adapter | [engine.py](../../../okc-web/backend/app/adapter/engine.py) |
| Core facade | [interop lib.rs](../../../okc-core/crates/okc-interop/src/lib.rs) |
| Web publication/read API | [serving router](../../../okc-web/backend/app/serving/router.py), [serving service](../../../okc-web/backend/app/serving/service.py) |
| MCP source configuration and guard | [config.ts](../../../okc-mcp/src/config.ts), [vault.ts](../../../okc-mcp/src/vault.ts) |
| MCP tool surface | [server.ts](../../../okc-mcp/src/server.ts) |

Build entry points: separate Cargo workspaces for core/hooks; npm/TypeScript for MCP; Python/uv and npm/Vite for web. Root workflows live in [.github/workflows](../../../.github/workflows). No umbrella runtime launcher or full four-module integration harness was found in the inspected inventory.

For core-local structure use [the module artifact](../../../okc-core/aidlc-docs/inception/reverse-engineering/code-structure.md).

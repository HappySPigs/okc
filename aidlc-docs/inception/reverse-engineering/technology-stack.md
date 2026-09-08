# Integration Technology Stack

| Surface | Declared technology | Manifest authority |
|---|---|---|
| Core | Rust workspace, Python PyO3/Maturin and Node bindings | [Cargo.toml](../../../okc-core/Cargo.toml), [Python pyproject](../../../okc-core/bindings/python/pyproject.toml) |
| Hooks | Rust edition 2024, declared rust-version 1.89, notify/ureq, CBOR | [Cargo.toml](../../../okc-hooks/Cargo.toml) |
| MCP | Node >=22.13, TypeScript, MCP SDK, Zod, YAML | [package.json](../../../okc-mcp/package.json) |
| Web backend | Python >=3.11, FastAPI, Pydantic, SQLAlchemy, local core wheel | [pyproject.toml](../../../okc-web/backend/pyproject.toml) |
| Web frontend | React/Vite/npm | [package.json](../../../okc-web/frontend/package.json) |

These are local manifest observations, not recommendations of current external versions. Lockfiles own resolved versions. No external product documentation or package installation was needed for this code review.

Runtime storage is local files and module-specific state, with SQLite in core/web. Module-local infrastructure designs remain authoritative; this review does not propose a cloud deployment.

# Tech Stack Decisions — okc-mcp-first-unit

**Status**: NFR Requirements output (autopilot). Recommended MVP choices; concrete library/versions are finalized at Code Generation (gated). Bounded by requirements: local Node/TypeScript stdio MCP, npm tarball install.

| Concern | Decision | Rationale (MVP) |
|---|---|---|
| Runtime | Node.js LTS | Required by REQ-001/010; ubiquitous, offline-capable. |
| Language | TypeScript (strict) | Type safety for the safety-critical write path and typed rejections. |
| MCP transport | stdio via the standard MCP server SDK | REQ-001 (stdio, no REST/API key); minimal surface. |
| YAML | A **comment/structure-preserving** YAML library (CST-based, e.g. `eemeli/yaml`) | REQ-005 requires preserving unknown keys, comments, and body; a plain load/dump loses comments. |
| Hashing | Node built-in crypto (e.g. SHA-256) | Deterministic content hash (REQ-004/006); no dependency. |
| Property testing | `fast-check` | D11 PBT-partial: pure functions + serialization round-trips. |
| Test runner | Node's built-in `node:test` (or a light runner) | MVP: avoid heavy frameworks; runnable offline. |
| Packaging | `npm pack` local tarball | REQ-010; lightweight release (D13). |
| CI/CD | **None required for first Unit** — local `npm run build`/`test` scripts; CI deferred | RESILIENCY-04: no pipeline needed for a local dev tool at MVP; can add GitHub Actions later. |
| Filesystem | Node `fs` with realpath-based boundary checks; atomic write via temp-file + same-dir rename | BR-PATH-*/BR-ATOMIC-*. |

**No** database, network client, message queue, cache, container, or cloud SDK — none are needed for a local single-Vault file tool.

**Deferred to Code Generation**: exact library versions, hash algorithm constant, atomic-write helper, project layout, tsconfig, and test file organization.

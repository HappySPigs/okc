# Unit of Work — okc-schema3-product

## Unit identity

- **ID**: UOW-OKC-001
- **Name**: `okc-schema3-product`
- **Type**: Brownfield modular native product/library unit
- **Deployability**: One product with several build/package artifacts; not microservices
- **Version boundary**: Product 0.3.0, artifact schema 3, interop schema 2, journal schema 4

## Objective

Maintain the complete current Schema 3 workflow from immutable source
selection through provider-assisted reviewed integration to offline directory
compilation, verification, and provenance explanation across Rust, CLI/TUI,
Python, and Node.js.

## Included modules

| Module | Package | Responsibility |
|---|---|---|
| Compiler | `okc-core` | corpus, domain closure, bytes, publication, verification |
| AI | `okc-ai` | provider capabilities, transport, proposal validation |
| Application | `okc-app` | project state, disclosure, review, orchestration, worker |
| Interop | `okc-interop` | runtime-neutral jobs, DTOs, errors, reservations |
| Operator | `okc` | CLI and TUI |
| Python | `okc-python` | ABI3 Python adapter/package |
| Node | `okc-node` | Node-API adapter/package |

## Responsibilities

- Preserve current public/schema/byte compatibility.
- Enforce immutable hostile source and no-replace output boundaries.
- Require recorded AI proposals, critic closure, and explicit curator approval.
- Offer equivalent behavior across all current public surfaces.
- Keep release blockers and future components explicit.

## Exclusions

MCP/Obsidian adapters, cloud services, Pack/non-Markdown materializers,
experimental memory algorithms, and stable package publication are outside the
current unit implementation.

## Completion definition for this documentation run

- As-built functional and NFR design exists.
- Existing source is traced to requirements and tests.
- Build/test instructions and current outcomes are recorded.
- No runtime source, schema, dependency, API, or golden is changed.

# Reverse Engineering — Component Inventory

**Baseline**: `e448bf28ee3dcb43d18428eb1bdb9ac70163bf34`

**Workspace version**: 0.3.0

**Cargo packages**: 7

## Cargo workspace packages

| Package | Path | Kind | Current responsibility |
|---|---|---|---|
| `okc-core` | `crates/okc-core` | Rust library | Safe corpus construction; current integration records; provider-free directory compile, verify, and explain |
| `okc-ai` | `crates/okc-ai` | Rust library | Provider-neutral Schema 3 requests, profiles, transports, response normalization, and validation |
| `okc-app` | `crates/okc-app` | Rust library | Projects, journal schema 4, immutable objects, routes, credentials, disclosure, review, worker, artifacts, updater |
| `okc-interop` | `crates/okc-interop` | Rust library | Runtime-neutral API-v1 facade, interop-schema-2 DTOs, jobs, errors, scheduling, reservations |
| `okc` | `crates/okc` | Native executable | One CLI/TUI surface backed by `okc-app` |
| `okc-python` | `bindings/python` | PyO3 `cdylib` + Python package | `okc-compiler` distribution, `okc` import, CPython 3.11 ABI3 adapter |
| `okc-node` | `bindings/node` | napi-rs `cdylib` + npm package | `okc-compiler`, ESM/CommonJS, TypeScript, Node-API 9 adapter |

## Adjacent assets

| Asset | Path | Responsibility |
|---|---|---|
| Normative knowledge base | `docs/` | Requirements, architecture, algorithms, ADRs, traceability, release state, history |
| Operator guide | `guide/` | VitePress quickstart, CLI/TUI, provider, SDK, conflict, and troubleshooting guidance |
| Demo Vaults | `demo/VAULT_A`, `demo/VAULT_B`, `demo/VAULT_C` | Interlinked hostile-input and workflow examples |
| POSIX TUI harness | `tests/tui_pty_smoke.py` | Synthetic loopback-provider end-to-end terminal workflow |
| Distribution configuration | `dist-workspace.toml` | cargo-dist targets, installers, checksums, SBOM, attestation, signing intent |
| GitHub Actions | `../.github/workflows/` | Four-host Rust and native SDK candidate matrices |
| AI-DLC rules | `.aidlc-rule-details/` | Local process templates used for this artifact set |

## Deliberately absent components

- No MCP adapter crate is shipped; `REQ-MCP-001` is future-only.
- No Obsidian plugin is shipped; `REQ-OBS-001` is future-only.
- No Schema 1/2 compiler, reader, writer, migration, or deprecated facade is on main.
- No command-provider adapter is exposed.
- No current OKCPack writer or reader is exposed.
- No web service, registry, or runtime cloud infrastructure is part of the product.

## Packaging counts

- Five ordinary Rust packages: `okc-core`, `okc-ai`, `okc-app`,
  `okc-interop`, and `okc`.
- Two native language binding packages: `okc-python` and `okc-node`.
- One executable target: `okc`.
- One example target: `corpus_probe`.
- Five explicit Rust integration-test targets plus package-local unit tests.

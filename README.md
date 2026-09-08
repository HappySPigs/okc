# OKC — Obsidian Knowledge Compilation

OKC turns many independently maintained [Obsidian](https://obsidian.md) vaults
into a single, deterministic, evidence-traceable, and verifiable knowledge base.
Contributors keep authoring notes locally; a curator collects successive vault
revisions and runs a human-in-the-loop integration that proposes taxonomy and
synthesis, checks it with an independent critic, and only lets curator-approved
content into the final artifact. Compilation, verification, and provenance
explanation are provider-free and reproducible.

> **Status:** development-stage monorepo, not a stable release. The engine is a
> `0.3.0` tree and `okc-web` is a hackathon MVP (single-organization,
> single-process, local/trusted environment). Several gates remain open — see
> each module's README for its own boundary and caveats.

## Modules

This repository is an umbrella monorepo of four independently managed modules.

| Module | Stack | Role | Docs |
|---|---|---|---|
| [`okc-core`](okc-core/) | Rust | The compilation engine: ingest immutable vault snapshots → propose taxonomy/synthesis (via LLM providers) → validate + critic + curator approvals → provider-free deterministic compile, verify, and provenance explain. Ships a shared CLI/TUI and a public Rust boundary (`CorpusBuilder::build`). | [README](okc-core/README.md) |
| [`okc-hooks`](okc-hooks/) | Rust | Local daemon that watches an Obsidian vault and auto-uploads changes to `okc-web`. Handles file events, startup/periodic scans, retries, and resumable uploads. | [README](okc-hooks/README.md) |
| [`okc-web`](okc-web/) | Python (FastAPI) + React SPA | Curator platform for **permission-gated integration** of departmental vaults into one merged base. Collects revisions via upload tokens, freezes the source set, drives the human-in-the-loop pipeline on top of `okc-core`, and serves version-pinned publications. | [README](okc-web/README.md) |
| [`okc-mcp`](okc-mcp/) | Node.js / TypeScript | Installable stdio MCP server to author local Obsidian source notes and read published OKC knowledge. Reads the integrated vault from `okc-web` when configured, otherwise the local source vault. | [README](okc-mcp/README.md) · [한국어](okc-mcp/README_KOR.md) |

## How it fits together

```
   author notes                     watch + upload                integrate + serve
 ┌──────────────┐   local vault   ┌──────────────┐   /api/sync   ┌──────────────────┐
 │  Obsidian /  │ ───────────────►│  okc-hooks   │ ─────────────►│      okc-web     │
 │   okc-mcp    │                 │   (daemon)   │   (CBOR,       │  curator console │
 └──────────────┘                 └──────────────┘    resumable)  │  + integration   │
        ▲                                                          │    pipeline      │
        │                                                          └───────┬──────────┘
        │            read published, version-pinned knowledge              │ okc binding
        └──────────────────────── okc-mcp ◄───────────────────────────────┤ (okc-compiler)
                                (serving API)                              ▼
                                                                    ┌──────────────┐
                                                                    │   okc-core   │
                                                                    │    engine    │
                                                                    └──────────────┘
```

1. **Author.** A contributor writes notes in a local Obsidian source vault —
   directly, or through `okc-mcp`'s authoring tools.
2. **Upload.** The `okc-hooks` daemon watches that vault and pushes revisions to
   `okc-web` over the authenticated `/api/sync` endpoint; repeated uploads
   update the same source.
3. **Integrate.** In `okc-web`, the curator freezes the collected source set and
   runs the multi-step, human-in-the-loop pipeline (provider → disclosure →
   taxonomy → clusters → critic review), then compiles a deterministic merged
   vault. `okc-web` consumes the engine only through the `okc` Python binding
   (`okc-compiler`, built from `okc-core` with maturin/pyo3).
4. **Serve.** `okc-web` publishes fixed, verifiable revisions with history and
   restore.
5. **Read.** `okc-mcp` reads the published integrated knowledge back, so AI
   clients and agents consume the verified corpus. A web error never silently
   falls back to local knowledge.

## Repository layout

```
okc/
├── okc-core/          # Rust compilation engine (CLI/TUI + Rust API)
├── okc-hooks/         # Rust vault-watch upload daemon
├── okc-web/           # FastAPI backend + React SPA integration platform
├── okc-mcp/           # Node/TypeScript stdio MCP server
├── aidlc-docs/        # Umbrella AI-DLC workspace: cross-module coordination
├── scripts/           # Cross-module integration & AI-DLC verification helpers
│                      #   (mcp-integration-client.mjs, test_integration.py, verify-aidlc.mjs)
└── .github/workflows/ # Per-module CI (core, hooks, mcp, web backend/frontend)
```

Each module keeps its own `aidlc-docs/`, agent instructions
(`AGENTS.md` / `CLAUDE.md`), and history; the root owns only cross-module
contracts and integration.

## Getting started

There is no single build for the whole repo — start with the module you need and
follow its README, which carries the authoritative prerequisites and steps.

- **Build/run the engine** → [`okc-core/README.md`](okc-core/README.md) (Rust toolchain).
- **Run the integration platform** → [`okc-web/README.md`](okc-web/README.md)
  (Python 3.11+ with [`uv`](https://docs.astral.sh/uv/), a Rust toolchain to
  build the `okc` binding, and Node 20+; expects a sibling/local `okc-core`).
- **Watch and upload a vault** → [`okc-hooks/README.md`](okc-hooks/README.md).
- **Author/read via MCP** → [`okc-mcp/README.md`](okc-mcp/README.md) (Node.js 22.13+).

## Development & AI-DLC

This repository is managed with the [AWS AI-DLC workflow](https://github.com/awslabs/aidlc-workflows)
(pinned to `v1.0.1`). Every top-level `okc-*` directory is an independent AI-DLC
project that owns its requirements, designs, state, and audit trail; the root
[`aidlc-docs/`](aidlc-docs/README.md) coordinates only module discovery,
cross-module interfaces, unified build/test orchestration, and release
integration. See [`CLAUDE.md`](CLAUDE.md) / [`AGENTS.md`](AGENTS.md) for the
workspace routing rules and [`aidlc-docs/README.md`](aidlc-docs/README.md) for
the module registry and integration state.

Each module has its own CI under [`.github/workflows/`](.github/workflows/):
`core-ci`, `core-sdk-bindings`, `hooks-ci`, `mcp-ci`, `web-backend-ci`, and
`web-frontend-ci`.

## License

Licensing is per-module: `okc-core` is dual-licensed under
[MIT](okc-core/LICENSE-MIT) or [Apache-2.0](okc-core/LICENSE-APACHE), and
`okc-mcp` ships its own [LICENSE](okc-mcp/LICENSE). `okc-hooks` and `okc-web` do
not yet declare a license, and there is no repository-wide license file — treat
those modules as all-rights-reserved until one is added.

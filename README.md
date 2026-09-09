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

### CI

Per-module build + test runs on every push (definitions in
[`.github/workflows/`](.github/workflows/); live status on the
[Actions tab](https://github.com/HappySPigs/okc/actions)):

[![core-ci](https://github.com/HappySPigs/okc/actions/workflows/core-ci.yml/badge.svg)](https://github.com/HappySPigs/okc/actions/workflows/core-ci.yml)
[![core-sdk-bindings](https://github.com/HappySPigs/okc/actions/workflows/core-sdk-bindings.yml/badge.svg)](https://github.com/HappySPigs/okc/actions/workflows/core-sdk-bindings.yml)
[![hooks-ci](https://github.com/HappySPigs/okc/actions/workflows/hooks-ci.yml/badge.svg)](https://github.com/HappySPigs/okc/actions/workflows/hooks-ci.yml)
[![mcp-ci](https://github.com/HappySPigs/okc/actions/workflows/mcp-ci.yml/badge.svg)](https://github.com/HappySPigs/okc/actions/workflows/mcp-ci.yml)
[![web-backend-ci](https://github.com/HappySPigs/okc/actions/workflows/web-backend-ci.yml/badge.svg)](https://github.com/HappySPigs/okc/actions/workflows/web-backend-ci.yml)
[![web-frontend-ci](https://github.com/HappySPigs/okc/actions/workflows/web-frontend-ci.yml/badge.svg)](https://github.com/HappySPigs/okc/actions/workflows/web-frontend-ci.yml)

`core-ci` runs `cargo fmt`/`clippy -D warnings`/workspace tests across
Linux·Windows·macOS (x64+arm64); `core-sdk-bindings` builds and tests the
Python (maturin) and Node (napi) bindings; `hooks-ci` runs clippy + the
proptest suite; `mcp-ci` runs the TypeScript type-check + test suite;
`web-backend-ci` builds the real `okc` binding and runs ruff/mypy/pytest plus
the four-module `scripts/test_integration.py`; `web-frontend-ci` runs the
Vite build + vitest. (`showcase-pages` deploys the showcase site and runs no
tests.)

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
├── install.sh         # One-command local install of okc-hooks + okc-mcp
├── install.ps1        #   (Windows PowerShell equivalent)
├── uninstall.sh       # Symmetric teardown
├── okc-install.config.example.json  # Combined installer config template
├── aidlc-docs/        # Umbrella AI-DLC workspace: cross-module coordination
├── scripts/           # Cross-module integration & AI-DLC verification helpers
│                      #   (okc-install-render.mjs, mcp-integration-client.mjs,
│                      #    test_integration.py, verify-aidlc.mjs)
├── demo/              # One-command local full-flow demo harness (run-demo.sh + video)
├── showcase/          # Static showcase / "why OKC" site (deployed to GitHub Pages)
└── .github/workflows/ # Per-module CI + showcase Pages deploy
```

Each module keeps its own `aidlc-docs/`, agent instructions
(`AGENTS.md` / `CLAUDE.md`), and history; the root owns only cross-module
contracts and integration.

## Quick install (okc-hooks + okc-mcp)

Fill **one** config file, run **one** command, and both the `okc-hooks` upload
daemon and the `okc-mcp` server (registered into your coding agents) are installed
locally. The installer just wraps each module's own `setup` command; `okc-core`
and `okc-web` are not covered by it (see [Getting started](#getting-started)).

**1. Create your config** from the template:

```sh
cp okc-install.config.example.json okc-install.config.json
```

**2. Fill in the values** — this is the only file you edit:

| Field | Meaning |
|---|---|
| `okc_web_base_url` | Your okc-web site root, e.g. `https://okc.example.com` (no path). |
| `hooks.vault_path` | Absolute path of the Obsidian vault to watch and upload. |
| `hooks.upload_token` | okc-web **upload token** (issued in the okc-web console). |
| `hooks.data_dir` | Absolute path for the daemon's local state. |
| `mcp.agents` | Coding agents to register into — any of `["claude","codex"]`. |
| `mcp.vault_path` / `mcp.state_path` | Local authoring vault + state (local mode). |
| `mcp.web` | Set `enabled: true` with `project_id` and a **serving read token** to read the published web vault instead (read-only). |

Set `"enabled": false` under `hooks` or `mcp` to install just one. Tokens come
from the okc-web console (hooks = upload token; mcp = serving read token — public
projects need none).

**3. Install both at once:**

```sh
./install.sh            # macOS / Linux
./install.ps1           # Windows (PowerShell)
```

The installer renders your combined config into per-module configs (written
`0600`; tokens are never printed), builds each module, then runs each module's
`setup` — registering the hooks auto-start daemon (launchd / systemd `--user` /
Windows SCM) and adding okc-mcp to each selected agent through its official CLI
(`claude mcp add --scope user`, `codex mcp add`; if a CLI isn't on `PATH`, the
snippet to paste is printed instead).

**Prerequisites:** Node ≥ 22 (always), a Rust toolchain (`cargo`, for the hooks
build), and `npm` (for the mcp build). Re-running is idempotent; use
`SKIP_BUILD=1 ./install.sh` to skip rebuilds when the modules are already built.

**Uninstall:** `./uninstall.sh` deregisters the daemon and unregisters okc-mcp
from your agents — your vaults are never touched.

Installer files: `install.sh`, `install.ps1`, `uninstall.sh`,
`okc-install.config.example.json`, and `scripts/okc-install-render.mjs`.

## Getting started

For a one-command local setup of `okc-hooks` + `okc-mcp`, use
[Quick install](#quick-install-okc-hooks--okc-mcp) above. Otherwise there is no
single build for the whole repo — start with the module you need and follow its
README, which carries the authoritative prerequisites and steps.

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
`web-frontend-ci`; `showcase-pages` deploys the showcase site to GitHub Pages.

## License

Licensing is per-module: `okc-core` is dual-licensed under
[MIT](okc-core/LICENSE-MIT) or [Apache-2.0](okc-core/LICENSE-APACHE), and
`okc-mcp` ships its own [LICENSE](okc-mcp/LICENSE). `okc-hooks` declares
`MIT OR Apache-2.0` in its `Cargo.toml` but ships no license text file yet.
`okc-web` does not declare a license, and there is no repository-wide license
file — treat `okc-web` as all-rights-reserved until one is added.

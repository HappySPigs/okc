# Reverse Engineering — Technology Stack

**Source**: current Cargo/package manifests and pinned lockfile at
`e448bf28ee3dcb43d18428eb1bdb9ac70163bf34`.

## Languages and targets

| Concern | Technology |
|---|---|
| Compiler/application | Rust 2024 Edition; `rust-version = 1.97.1` |
| Python binding | CPython 3.11+, PyO3 0.29.2, ABI3 (`abi3-py311`) |
| Node binding | Node.js 22.13+, napi-rs 3.12.2, Node-API 9 |
| Node module surface | ESM, CommonJS, TypeScript declarations |
| Guide | VitePress 1.6.4 on Node.js 22+ |
| License | `MIT OR Apache-2.0` |

## Core libraries

| Capability | Current dependency/approach |
|---|---|
| CLI and TUI | Clap 4, Ratatui 0.30.2, Crossterm 0.29.0 |
| Markdown/text | Comrak 0.48, Unicode normalization 0.1.25, Unicode segmentation, caseless 0.2.2 |
| Serialization | Serde, `serde_json`, `serde_yaml_ng` 0.10, TOML 0.9 |
| Storage | bundled `rusqlite` 0.37, filesystem content-addressed objects |
| Hashing/encoding | SHA-256, hex, data-encoding |
| Archives | ZIP 4, tar 0.4, zstd 0.13 |
| Parallel work | Rayon for deterministic collected work; bounded `std::thread` workers for application/jobs |
| Provider HTTP | ureq 3.4.0 with rustls/platform verifier and bounded synchronous transport |
| Secrets | zeroize; keyring 4.2.0 for optional native CLI/TUI storage |
| Publication | tempfile plus rustix on Linux/macOS and atomicwrites on Windows |
| Updates/releases | axoupdater 0.10.0, cargo-dist 0.32.0 |
| Native packaging | Maturin 1.x/PyO3; `@napi-rs/cli`/napi-rs; npm |

## Build and quality policy

- Cargo workspace resolver 2 with committed `Cargo.lock`.
- Workspace `unsafe_code = "forbid"`; the napi-rs adapter alone allows macro-
  generated unsafe under ADR-0026.
- Clippy `all` and `pedantic`; release gates promote warnings to errors.
- Release profile uses thin LTO, one codegen unit, and stripped symbols.
- CI targets Linux x86_64 GNU, Windows x86_64 MSVC, macOS x86_64, and macOS arm64.
- Python version tests cover 3.11 through 3.14; Node tests cover 22.13 and the
  current supported line in workflow configuration.

## Runtime topology

OKC is local-first and has no runtime service mesh, container requirement, or
cloud database. Runtime state is local files plus SQLite. Network access is
limited to explicitly configured provider calls and the receipt-aware updater;
offline compilation, verification, and explanation do not use the network.

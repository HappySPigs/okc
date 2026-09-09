# Build Instructions

## Working directories

- **Cargo/product workspace**: `/Users/sihun/workspace/projects/okc/okc-core`
- **Git checkout root**: `/Users/sihun/workspace/projects/okc`
- Run product commands from the Cargo/product workspace.
- Parent GitHub Actions workflows set `defaults.run.working-directory: okc-core`.

## Prerequisites

| Tool | Required boundary |
|---|---|
| Rust | pinned 1.97.1 from `rust-toolchain.toml` |
| Cargo | matching Rust toolchain and committed `Cargo.lock` |
| Python | 3.11+; Maturin 1.x, pytest, and mypy for binding validation |
| Node.js | 22.13+; npm and package lock for Node binding |
| Guide | Node.js 22+ and `guide/package-lock.json` |
| Native tools | platform C/C++ linker prerequisites used by Rust/PyO3/napi-rs |

On this macOS host, `/opt/homebrew/opt/rustup/bin` must be present in `PATH` so
Cargo can locate `rustc`. This is a host setup detail, not a project runtime requirement.

## Build steps

### 1. Verify the pinned toolchain

```bash
rustc -vV
cargo -V
rustup show active-toolchain
```

### 2. Build every Rust package and target

```bash
cargo check --locked --workspace --all-targets --all-features
cargo build --locked --workspace --all-features
```

Primary binary: `target/debug/okc` for a debug build.

### 3. Check optional core feature boundaries

```bash
cargo clippy --locked -p okc-core --no-default-features --lib -- -D warnings
cargo clippy --locked -p okc-core --no-default-features --features archives --lib -- -D warnings
cargo clippy --locked -p okc-core --no-default-features --features sqlite --lib -- -D warnings
```

### 4. Build the Python extension/package

```bash
python3 -m maturin build \
  --manifest-path bindings/python/Cargo.toml \
  --release --locked
```

For editable test use in a virtual environment:

```bash
python3 -m maturin develop \
  --manifest-path bindings/python/Cargo.toml \
  --release --locked
```

Release packaging sets `SOURCE_DATE_EPOCH` to the selected source commit's Unix
timestamp before Maturin and compares repeated wheel bytes.

### 5. Build the Node.js addon

```bash
npm ci --ignore-scripts --prefix bindings/node
npm run build --prefix bindings/node
```

### 6. Build the guide

```bash
npm ci --prefix guide
npm run docs:build --prefix guide
```

## Build artifacts

| Artifact | Typical location |
|---|---|
| `okc` debug binary | `target/debug/okc` |
| Rust release binary/library objects | `target/release/` |
| Python wheel | `target/wheels/` unless `--out` is supplied |
| Python sdist | caller-selected Maturin output directory |
| Node native addon | `bindings/node/okc-compiler.<platform>.node` |
| npm tarballs | caller-selected `npm pack --pack-destination` directory |
| Guide static site | `guide/.vitepress/dist/` |

Generated native/package/guide artifacts are ignored and are not canonical source.

## Troubleshooting

### Cargo cannot execute `rustc`

Ensure the pinned rustup bin directory is in `PATH`; on the analyzed host it is
`/opt/homebrew/opt/rustup/bin`.

### macOS Python extension cannot load Expat

The historical host had a Homebrew loader mismatch. A process-local
`DYLD_LIBRARY_PATH=/opt/homebrew/opt/expat/lib` resolved it. Do not change
system libraries or treat this workaround as product behavior.

### Native binding cannot load

Build the addon/extension for the exact host architecture and run tests against
the built or clean-installed distribution, not an incompatible committed/generated binary.

### Dependency download is unavailable

Use the committed lockfiles and existing caches where possible. Do not remove
`--locked` to conceal drift. Network installation is an environment concern;
the compiler's offline compile path does not require it.

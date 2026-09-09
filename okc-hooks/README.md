# okc-hooks

A local daemon that watches an Obsidian vault and automatically uploads its
changes to `okc-web`. It handles filesystem events, a startup scan, periodic
reconciliation scans, retries, and resumable uploads over the authenticated
`okc-web` `/api/sync` endpoint (CBOR wire protocol). Merging and serving the
integrated vault are `okc-web`'s job — `okc-hooks` only detects and transfers.

> **Status:** `0.1.0` development tree. Rust workspace, MSRV **1.89**,
> edition 2024. Config-file token storage is the approved default; optional OS
> secure-store integration is deferred.

## Workspace

A 10-crate Cargo workspace under [`crates/`](crates/), layered as a dependency
DAG rooted at `foundation`:

| Crate | Role |
|---|---|
| `foundation` | shared types, errors, config primitives (DAG root) |
| `content-core` | streaming SHA-256, manifest diffing (added/modified/deleted) |
| `change-detect` | cross-platform filesystem watch (`notify`) + debounce |
| `sync-state` | local sync-state store bound to vault path / endpoint / token selector |
| `auth-consent` | consent gating + blocking rustls HTTPS client (`ureq`) |
| `observability` | structured logging / diagnostics |
| `upload-client` | resumable CBOR upload protocol client |
| `lifecycle-deploy` | OS auto-start service registration (launchd / systemd `--user` / Windows SCM) |
| `ops-control` | operational control surface |
| `watcher-bin` | the `watcher-bin` daemon binary that ties it together |

## Prerequisites

- A Rust toolchain (`cargo`) — MSRV 1.89.
- An `okc-web` instance reachable over **trusted HTTPS**, and an **upload token**
  issued by its console for the destination project.

## Quick start

The simplest path is the repository's one-command installer, which builds this
module and registers the auto-start service for you — see
[root `README.md` → Quick install](../README.md#quick-install-okc-hooks--okc-mcp).

To build and set it up directly:

```bash
# 1. build the daemon
cargo build --locked --release -p watcher-bin

# 2. fill in a config (absolute paths; endpoint must be https; token = okc-web upload token)
cp examples/config.example.json my-config.json
#    edit: vault_path, server_endpoint, token, data_dir

# 3. validate the config, install it 0600, and register the auto-start service
./target/release/watcher-bin setup --config /absolute/path/to/my-config.json
```

`setup` is idempotent: it validates strictly (unknown keys, non-`https`
endpoint, or a blank token abort before anything is written), copies the config
to the user-level path (`~/.config/okc-watcher/config.json` on Linux,
`~/Library/Application Support/okc-watcher/config.json` on macOS,
`%APPDATA%\okc-watcher\config.json` on Windows) with permissions `0600`, and
registers an OS service that runs `watcher-bin run <config>`. The token is never
printed or logged.

Run it in the foreground (e.g. for debugging) with:

```bash
./target/release/watcher-bin run /absolute/path/to/config.json
```

Remove the service (and, with `--purge-token`, the installed config) — your
vault is never touched:

```bash
./target/release/watcher-bin uninstall --purge-token
```

### Config fields

| Field | Meaning |
|---|---|
| `vault_path` | absolute path of the Obsidian vault to watch |
| `server_endpoint` | okc-web `/api/sync` base — must be `https` |
| `token` | okc-web **upload token** (selects the project + stable source) |
| `data_dir` | absolute path for this watcher's local sync state (one per vault) |
| `debounce_ms` | file-event debounce window (e.g. `2000`) |
| `reconciliation_interval_s` | periodic full-scan interval (e.g. `900`) |
| `chunk_size_bytes` | upload chunk size (e.g. `1048576`) |

Uploads from a source replace its previous revision. Symlinks are skipped;
regular vault files follow the approved no-filter policy. When deliberately
changing the destination project, vault, or token selector, use a fresh
`data_dir` (the state store is fingerprint-bound to the old target and refuses
silent retargeting).

## Docs

- [Watcher setup and sync contract](aidlc-docs/construction/build-and-test/watcher-setup.md) — full configuration, service registration, and state-binding rules
- [Integration sync repair plan](aidlc-docs/construction/plans/integration-sync-repair-code-generation-plan.md)
- [Build and test summary](aidlc-docs/construction/build-and-test/build-and-test-summary.md)

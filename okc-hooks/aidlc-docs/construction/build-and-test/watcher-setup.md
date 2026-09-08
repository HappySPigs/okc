# Watcher Setup and Sync Contract

## Configuration

Create a JSON configuration using absolute local paths and the token issued by okc-web for the destination project. A separate `data_dir` is needed for each independent monitored vault.

```json
{
  "vault_path": "/absolute/path/to/vault",
  "server_endpoint": "https://okc.example/api/sync",
  "token": "token-issued-by-okc-web",
  "data_dir": "/absolute/path/to/watcher-state",
  "debounce_ms": 2000,
  "reconciliation_interval_s": 900,
  "chunk_size_bytes": 1048576
}
```

The token selects the project and stable source on the server. Uploads from this source replace its previous revision. Use the same target and local state directory when restarting. A versioned SHA-256 fingerprint stored beside the sync-state file binds it to the canonical vault path, normalized HTTPS base, and token selector; no token or selector is stored there. Startup and config reload reject target changes without deleting state or uploading to a different target. Verifier-only token changes with the same selector remain compatible.

When deliberately changing the destination project, vault, or token selector, select a fresh `data_dir` and do not reuse an explicit old `state_path`. Legacy committed state without a binding is also rejected with this instruction. Preserve the old directory for the old target; acknowledge and grant consent for the new configuration. A token rotation that changes its selector needs a fresh local state directory because the client cannot prove that two distinct selectors address the same source.

The configured endpoint must use trusted HTTPS. Config token storage is the approved default; optional OS secure-store integration is still deferred. Raw regular vault files are included, consistent with the approved no-filter policy. Symlinks are skipped. Wanted files are temporarily spooled one at a time, requiring temporary disk space up to one file's size; the spool is removed on close.

## One-command setup (config-driven)

Fill in a copy of [`../../../examples/config.example.json`](../../../examples/config.example.json) (the fields above; `vault_path`/`data_dir` must be absolute, `server_endpoint` must be `https`, `token` is the okc-web **upload token**), then run:

```bash
./target/release/watcher-bin setup --config /path/to/your/filled-config.json
```

`setup` validates the config with the standard strict rules (unknown keys, non-`https` endpoint, or a blank token abort before anything is written), copies it to the user-level path (`~/.config/okc-watcher/config.json` on Linux, `~/Library/Application Support/okc-watcher/config.json` on macOS, `%APPDATA%\okc-watcher\config.json` on Windows) with permissions `0600`, and registers the auto-start OS service bound to that config path (the generated launchd plist / systemd `--user` unit / SCM entry launches `watcher-bin run <config>`, so the `/etc/...` discovery default and `sudo` are not needed). A best-effort DNS reachability check only warns; it never blocks setup, and the token is never printed or logged. Re-running `setup` is idempotent (config re-written `0600`, service re-registered).

To undo it, use the existing uninstall (deregisters the service and, with `--purge-token`, removes the `setup`-written config; the vault is never touched):

```bash
./target/release/watcher-bin uninstall --purge-token
```

## Build and run

From `okc-hooks/`, build with Rust 1.89 or newer:

```bash
cargo build --release --workspace
export OKC_WATCHER_CONFIG=/absolute/path/to/config.json
./target/release/watcher-bin run
```

In another terminal with the same `OKC_WATCHER_CONFIG`, inspect status, acknowledge the disclosure, and grant standing upload consent:

```bash
./target/release/watcher-bin status --json
./target/release/watcher-bin consent view
./target/release/watcher-bin consent acknowledge
./target/release/watcher-bin consent grant
./target/release/watcher-bin sync-now
```

The foreground process stays running. `pause`, `resume`, `stop`, `history`, and `reload` use the same local IPC endpoint. CLI `install` delegates to the native OS service manager; a service must be able to discover its configuration independently of a terminal environment (default Unix config path: `/etc/okc-watcher/config.json`). Native service installation and cross-OS operation remain deployment checks, not claims made by unit tests.

## Receiver protocol

All requests carry `Authorization: Bearer ...` and `Content-Type: application/cbor`. `server_endpoint` is the common base URL.

1. `POST /negotiate` sends `manifest_digest` and entries `[relative_path, raw_sha256, size]`. The response contains `server_has`, `session_id`, and `resume_offsets` pairs `[raw_sha256, offset]`.
2. `PUT /blob/{hex_sha256}/{offset}` sends a `ChunkFrame` with `blob`, `offset`, `len`, `chunk_sha256`, and `bytes`. It includes `X-OKC-Upload-Session`. A frame contains at most 8 MiB of raw bytes; client configuration is capped at that ceiling.
3. `POST /commit` sends `path_hash_map.entries` and `manifest_digest`, with the same session header. The response contains `server_vault_content_id` and `committed`. A successful response acknowledges source revision application, not merged-vault publication.

Rust serde encodes digest arrays and `Vec<u8>` as CBOR arrays of unsigned integers; paths are strings and sizes are integers. The manifest digest is SHA-256 over sorted entries, each framed as: path UTF-8 length (8-byte big-endian), path UTF-8 bytes, raw SHA-256 (32 bytes), and size (8-byte big-endian). The Rust `protocol-fixture` example emits actual serialized requests for receiver tests.

Server-provided offsets override local acknowledgements for a negotiated session. A source-revision conflict causes a complete renegotiation after backoff. Transient failure retries no longer require another filesystem event; incoming edits coalesce until the retry deadline. Authentication failures require fixing the token; successful raw upload does not imply that human review, compilation, or publication has completed.

## Verification

```bash
PROPTEST_RNG_SEED=20260909 cargo test --workspace --features proptest-support
cargo clippy --workspace --all-targets --features proptest-support -- -D warnings
cargo run -p upload-client --example protocol-fixture -- notes/hello.md 'Hello from Rust'
```

The root integration suite owns receiver/core/merged-vault verification. Module tests cover retry without new events, backoff coalescing, shutdown during waiting, generated failure sequences, coherent file bytes, session header propagation, server offsets, and CBOR round-trips.

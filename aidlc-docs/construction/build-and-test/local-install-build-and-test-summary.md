# Build & Test Summary — Local Config-Driven Install (okc-hooks + okc-mcp)

Root aggregate verification for the 2026-09-09 initiative. Requirements: [../../inception/requirements/local-install-requirements.md](../../inception/requirements/local-install-requirements.md). Units: [../../inception/plans/local-install-unit-of-work.md](../../inception/plans/local-install-unit-of-work.md). Per-module functional designs live under each module's `aidlc-docs/construction/local-install/`.

## What was built

- **U1 · okc-hooks** — `watcher-bin setup --config <src>`: validate (strict schema, https-only, non-blank token; fail-closed **before** any write) → write user-level config at `0600` (`~/.config/okc-watcher/config.json` / macOS `~/Library/Application Support/...` / Windows `%APPDATA%`) → register the existing auto-start service **bound to that config** via additive `ServiceSpec.config_path` (launchd/systemd `--user`/SCM emit `run <config>`; no `/etc`, no sudo). Teardown: `watcher-bin uninstall --purge-token` deregisters + removes the setup-written config (vault untouched). Best-effort DNS reachability warns only; token never logged (redacted `TokenSecret`, verified by test). Core `loader.rs` discovery defaults untouched.
- **U2 · okc-mcp** — `okc-mcp setup [--config <path>]`: atomic `0600` config write (temp→rename→chmod, `0700` parent) → register into selected `agents` via official CLIs (`claude mcp add --scope user okc-mcp -- <node> <cli.js> serve --config <path>`, `codex mcp add okc-mcp -- ...`); idempotent upsert (remove-then-add); print-only fallback when a CLI is absent (no hand-editing agent config). `unregister [--purge]` removes registration (+optional config). New optional `agents` config field (`claude`|`codex`, deduped, strict). Best-effort web validation via read-only `GET .../contract`; token never in argv/logs/snippet.

## Aggregate verification (real command output)

### okc-mcp — `npm run check`
- typecheck: **PASS**; build (`tsc`): **PASS (exit 0)**.
- tests (`node --import tsx --test`): **101 pass / 0 fail / 0 cancelled**, incl. `PBT: config round-trips the agents field`, `config dedupes repeated agents`, and the new `setup`/`unregister` behavior + redaction + fallback tests.

### okc-hooks — `cargo test --workspace --features proptest-support`
- build: **PASS** (debug + `--release` per U1 report); clippy `-D warnings`: **0 warnings** (U1 report).
- tests: **all crates `test result: ok`, 0 failed.** Highlights:
  - `watcher-bin` `tests/setup.rs`: **8/8** (happy-path 0600 + service bound to dest; unknown-key/non-https/blank-token rejected pre-write; token-never-leaks in Display+Debug; idempotent re-run; registration-failure surfacing; config serde round-trip PBT).
  - `lifecycle-deploy`: **22 lib + 7 prop** (incl. new bind-`run <config>` / omit-when-absent tests).
  - `ops-control`: **23** still green after the `NativeServiceOps` config-path signature change.
  - Existing foundation/change-detect/upload-client/sync-state/observability/watcher-bin integration + property suites: green.

## Security compliance (applicable Security Baseline rules)

| Rule | Result |
|---|---|
| SECURITY-03/12 (no secrets in logs; no hardcoded creds) | Compliant — tokens only in `0600` files; redaction asserted by tests in both modules. |
| SECURITY-05 (input validation) | Compliant — serde strict + `known_config_keys()` (hooks); zod `.strict()` (mcp). |
| SECURITY-06 (least privilege / perms) | Compliant — `0600` configs (0700 parent for mcp); user-scope service. |
| SECURITY-09 (safe errors / no default token) | Compliant — errors carry paths/reasons/codes only; no default token. |
| SECURITY-10 (supply chain) | Compliant — no new deps in hooks (Cargo.lock unchanged by U1); mcp prefers `node:child_process`; lockfiles pinned. |
| SECURITY-11 (credential separation) | Compliant — isolated in `watcher-bin/src/setup.rs` and `okc-mcp/src/install.ts`. |
| SECURITY-13 (safe deserialization) | Compliant — schema-validated parsing; mcp uses official agent CLIs instead of parsing their config. |
| SECURITY-15 (fail-closed / cleanup) | Compliant — validate before write; temp cleanup on failure; runtime auth already fails closed. |
| SECURITY-01/02/04/07/08/14 | N/A — local client-side installer; no data store/network intermediary/HTML/topology/server authz/alerting introduced. |

## Not done live (honest scope boundary)

- Real OS-service registration and real `claude`/`codex mcp add` were **not executed** during verification — tests use injectable fakes (recording `ServiceRegistrar` / `CommandRunner`), and only safe CLI smoke tests (missing/unreadable/bad-scheme args) were run for real. End-to-end registration against a live agent + a live okc-web token was not performed.
- Deferred per requirements: unified config, file-watcher auto-install, OS secure-store token backend, okc-web token auto-issue, Claude Desktop target, npm publish, root `install.sh`.
- U1 note: `setup` reuses foundation validation, which rejects a *blank* token but permits a *wholly absent* token (env/secure-store fallback). Stricter "token required at setup" was intentionally not added.

## Working-tree note

Only files under `okc-hooks/` and `okc-mcp/` (+ these root coordination docs) belong to this initiative. Other modified paths in the tree (okc-web backend, okc-core, CI, etc.) are unrelated concurrent work and were not touched or verified here. No commits were made.

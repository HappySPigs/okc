# Functional Design — U1 okc-hooks config-driven `setup` + teardown

**Unit**: U1 (root initiative "Local Config-Driven Install"). Module workspace: `okc-hooks/`.
**Requirements**: LIR-H1..H4, LIR-X2, NFR-1..4, Security Baseline (applicable rules).
**Scope**: add a config-driven `setup` operator subcommand (and a symmetric teardown path) that turns a filled-in config into a working local install, reusing the existing daemon, config validation, and OS-service registration. No refactor of existing behavior (NFR-1).

## Command surface

- `watcher-bin setup --config <source-path>` — read the user-authored config at `<source-path>`, validate it, place it at the user-level config path with `0600`, and register the auto-start OS service bound to that config path. Exit `0` on success, `1` on failure, `2` on argument error. (LIR-H1)
- Teardown reuses the existing `watcher-bin uninstall [--purge-token] [--purge-logs] [--keep-service]` — service deregistration plus, with `--purge-token`, removal of the `setup`-written config. The vault is never touched. (LIR-H4)

`setup` is dispatched in `main.rs` **before** the operator-CLI path, because it must work when no config yet exists at the discovery default (it is what creates the config). The other operator commands still flow through the existing `dispatch_args` IPC/service routing.

## Config path resolution (per OS)

`watcher_bin::setup::user_config_path()` returns the user-level destination, independent of foundation's `loader::resolve_config_path` (the core discovery default `/etc/okc-watcher/config.json` is deliberately left unchanged):

| OS | Path |
|---|---|
| Linux / other Unix | `$XDG_CONFIG_HOME/okc-watcher/config.json`, else `~/.config/okc-watcher/config.json` |
| macOS | `~/Library/Application Support/okc-watcher/config.json` |
| Windows | `%APPDATA%\okc-watcher\config.json` |

The mechanism mirrors the crate's existing env/HOME-based resolution (no new `dirs`-style dependency). `HOME`/`APPDATA` absence falls back to the temp dir. The **original file bytes are copied verbatim** (not re-serialized): the typed core model holds only 6 fields, so re-serializing would drop federated fields (`data_dir`, `debounce_ms`, …). Copying preserves the full document that the daemon later reads.

## Service binding mechanism

The generated OS service must launch the daemon bound to the user-level config, so the `/etc/...` default and `sudo` are avoided. `lifecycle_deploy::ServiceSpec` gains one optional field `config_path: Option<PathBuf>` (additive; `ServiceSpec::new` defaults it to `None`, preserving the existing `install` behavior) plus a `with_config_path(..)` builder. When set, each per-OS adapter emits `run <config>` as an explicit launch argument:

- **launchd** (macOS): `ProgramArguments = [<exec>, "run", <config>]` — each as its own `<string>`, so a config path containing spaces (Application Support) is safe.
- **systemd `--user`** (Linux): `ExecStart=<exec> run <config>`.
- **Windows SCM**: `binPath= <exec> run <config>`.

This reuses the already-supported daemon entry `watcher-bin run <config-path>` (`main.rs` maps arg[1] to `WatcherDaemon::run(Some(path))` → `load_runtime_config(Some(path))`), so no new daemon argument parsing was introduced.

`setup` reuses the existing `ServiceManager::install` path via a small injectable seam `ServiceRegistrar` (native impl `NativeServiceRegistrar` wraps `native_controller()` + `ServiceManager` + `ServiceSpec::with_config_path`). Tests inject a recording fake, so no privileged/OS interaction occurs in unit tests.

## Validation and redaction

- **Hard gate** (reject, fail-closed): the config is validated with foundation's existing `validate(raw, &known_config_keys())` — strict unknown-key rejection over the full federated key union, `vault_path` absolute, `server_endpoint` `https`-only, `token` present-but-non-blank. Failure aborts before any write or service registration. (LIR-H3 "schema + scheme", SECURITY-05/13/15)
- **Best-effort** (warn only, never abort): after a successful write+register, a DNS resolution of the endpoint host (`std::net::ToSocketAddrs`, no new dependency) emits a warning on failure. Runtime auth already fails closed, so reachability is advisory. (LIR-H3)
- **Redaction**: the token is never printed or logged. Validation errors surface only through `ConfigError::report()` (field/reason text, never values); `SetupError` carries paths and reasons but no config contents; the `TokenSecret` redacting type (`Debug`/`Display` → `***`) is never `expose()`d into any output. A test asserts a real token value never appears in the rendered error (Display or Debug). (SECURITY-03/09)

## Idempotency (NFR-3)

Re-running `setup` overwrites the destination config (create/truncate with mode `0600`, then an explicit `set_permissions(0o600)` to normalize permissions on an overwrite) and re-registers the service (the controllers treat re-register as an upsert and deregister as idempotent). No corruption or duplicate state results.

## Teardown config removal

`ops_control::NativeServiceOps` now carries the actual loaded `config_path` and uses it as the `ConfigToken` uninstall artifact (previously the speculative `data_dir/config.json`). Combined with a user-config fallback in `main.rs::run_cli` (if discovery-default load fails, retry `load_runtime_config(user_config_path())`), `watcher-bin uninstall --purge-token` finds and removes the `setup`-written config without requiring `OKC_WATCHER_CONFIG`. The vault-safety double-check in the `Uninstaller` still prevents deleting anything under the vault root.

## Security-rule mapping (applicable Security Baseline)

| Rule | How satisfied |
|---|---|
| SECURITY-03 (no secrets in logs) | Token never printed/logged; errors use `ConfigError::report()` and `SetupError` (no values); `TokenSecret` redacts; test asserts no leak. |
| SECURITY-05 (input validation) | Reuses strict serde 2-pass validation (unknown keys, `https`-only, non-blank token). |
| SECURITY-06 / 11 / 12 (least privilege / cred isolation / no hardcoded creds) | Config written `0600`; service runs as the current user (`--user`/LaunchAgent, no `sudo`); token only from the user-authored config; credential handling stays in foundation's `TokenSecret`. |
| SECURITY-09 (safe errors, no default token) | Generic reason-only errors, no internal path/stack leakage of secrets, no default/fallback token invented. |
| SECURITY-10 (supply chain) | No new dependencies (`std::net`, foundation, lifecycle-deploy, serde only); `Cargo.lock` unchanged. |
| SECURITY-13 (safe deserialization) | serde strict validation; original bytes copied, not reconstructed. |
| SECURITY-15 (fail-closed, cleanup) | Hard-gate aborts before any side effect; all I/O errors handled and surfaced; runtime auth already fails closed with no token. |

## Files

- `crates/watcher-bin/src/setup.rs` (new) — `user_config_path`, `ServiceRegistrar` seam + `NativeServiceRegistrar`, `perform_setup` core, `run_setup` native entry, `SetupError`/`SetupReport`, private `write_private` (0600).
- `crates/watcher-bin/src/main.rs` — `setup` dispatch branch + arg parser; user-config fallback in `run_cli`; pass real config path to `NativeServiceOps`.
- `crates/watcher-bin/src/lib.rs` — expose `setup` module.
- `crates/lifecycle-deploy/src/service.rs` — additive `ServiceSpec.config_path` + `with_config_path`; adapters emit `run <config>` when bound; content-generation unit tests.
- `crates/ops-control/src/cli.rs` — `NativeServiceOps` carries the real `config_path`; `ConfigToken` artifact targets it.
- `crates/watcher-bin/tests/setup.rs` (new) — happy path (0600 + bound register), invalid/unknown-key, non-`https`, blank token, token-redaction, idempotent re-run, registration-failure surfacing, and a `proptest-support`-gated config serde round-trip (NFR-4).
- `examples/config.example.json` (new) — fill-in-the-blanks sample matching the real schema, referenced from `watcher-setup.md`.

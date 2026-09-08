# Requirements — Local Config-Driven Install for okc-hooks + okc-mcp

**Initiative**: Root umbrella, started 2026-09-09 KST. Scope = cross-module coordination of okc-hooks + okc-mcp with a shared okc-web credential dependency.
**Answers source**: [local-install-requirement-verification-questions.md](local-install-requirement-verification-questions.md) — answered on autopilot per user directive (recommended options, minimal scope).

## Intent Analysis

- **User request (verbatim intent)**: Each of okc-hooks and okc-mcp should have a "just fill in the values" config file; once filled, each module installs itself locally with no further manual wiring. Both likely need an okc-web API endpoint + token. okc-hooks installs as an auto-running daemon; okc-mcp registers itself globally into whichever coding agent the user uses (Claude / Codex), and the config also records which coding agent that is.
- **Request type**: New Feature / Enhancement (add config-authoring + self-install/registration commands over existing implementations).
- **Scope estimate**: Multiple components — two modules coordinated at the repository root; okc-web is a consumed dependency (no okc-web code change required).
- **Complexity estimate**: Moderate. Most primitives already exist (hooks OS-service registration; both okc-web HTTP clients). The new work is ergonomics: config authoring, credential placement, and agent registration.

## Current-State Baseline (what already exists)

- **okc-hooks**: strict JSON config (serde), lock-free `ConfigProvider`; `install`/`uninstall` already register/deregister an auto-start OS service (`com.okc.watcher`) via launchd (macOS) / systemd --user (Linux) / SCM (Windows). Uploads to okc-web `/api/sync/*` with `Authorization: Bearer <upload-token>` over enforced HTTPS. **Missing**: any command to create the config or place the token — config must pre-exist at `/etc/okc-watcher/config.json` or `OKC_WATCHER_CONFIG`.
- **okc-mcp**: strict JSON config (zod), okc-web serving reads (`/api/serving/{projectId}/*`) with optional `Bearer <read-token>`. `client-config` **prints** an `mcpServers` snippet only. **Missing**: any write-registration into an agent; any "which coding agent" concept; npm publish.
- **okc-web**: admin-issued, shown-once tokens — **upload token** (hooks) and **serving read token** (mcp). Public projects need no mcp token. No okc-web change is in scope.

## Functional Requirements

### okc-hooks
- **LIR-H1** — Add a user-invokable `setup` subcommand to `watcher-bin` that: (a) reads a user-authored config, (b) validates it, (c) writes/places it at a **user-level** path (`~/.config/okc-watcher/config.json`, platform equivalent for Windows), then (d) registers the auto-start OS service by **reusing the existing `install` service logic**, passing the config path explicitly (via `--config`/`OKC_WATCHER_CONFIG` in the generated service unit) so the core discovery default (`/etc/...`) is not relied upon and no `sudo` is needed.
- **LIR-H2** — The `setup`-managed config carries exactly the values the user fills: `vault_path`, `server_endpoint` (okc-web `/api/sync` base), `token` (upload token), plus optional existing tuning fields. `setup` enforces file permissions `0600` on the written config.
- **LIR-H3** — `setup` runs **best-effort, non-blocking validation**: config schema validity + endpoint scheme/reachability (HTTPS enforced, DNS/TLS check). It **warns** on problems but still completes writing/registration (runtime auth already fails closed). It **must never print or log the token**.
- **LIR-H4** — Provide symmetric teardown by **reusing the existing `uninstall`** (service deregistration + optional removal of the `setup`-generated config); it must never touch the vault.

### okc-mcp
- **LIR-M1** — Extend the config schema with a coding-agent concept: an `agents` field enumerating targets to register into, limited in v1 to `claude` (Claude Code) and `codex`.
- **LIR-M2** — Add a `setup` command that generates/updates the mcp config JSON (`vaultPath`/`statePath` or `web.baseUrl`+`projectId`+`token`+`readOnly`, plus `agents`) at a user-level path, enforcing `0600`.
- **LIR-M3** — `setup` registers the MCP server into each selected agent via that agent's **official CLI** — `claude mcp add --scope user okc-mcp -- <node> <cli.js> serve --config <path>` and `codex mcp add ...` (user/global scope) — pointing at the generated config. If an agent's CLI is not found on `PATH`, **fall back to printing the snippet** (existing `client-config` behavior) with a clear message; do not hand-edit the agent's JSON/TOML.
- **LIR-M4** — When `web` is configured, `setup` runs **best-effort, non-blocking validation** via a read-only `GET .../contract`; warn only; never print/log the token.
- **LIR-M5** — Provide a symmetric unregister command using each agent's CLI (`claude mcp remove`, `codex mcp remove`) and optional removal of the generated config; never touch the vault.

### Cross-cutting
- **LIR-X1** — **Two separate module configs** (no unified/shared config). Root `aidlc-docs/` only coordinates ordering, shared acceptance, and contracts.
- **LIR-X2** — Preserve the token distinction: hooks uses the **upload token**; mcp uses the **serving read token**; public projects need no mcp token. `setup` prompts/fields make the distinction explicit.

## Non-Functional Requirements

- **NFR-1 (Surgical)** — Reuse existing service registration and HTTP clients; add commands rather than refactoring existing behavior. Every change traces to a requirement above.
- **NFR-2 (Platform parity)** — Match the platforms the existing code already supports (macOS + Linux primary; Windows best-effort as already coded). Primary dev platform is macOS.
- **NFR-3 (Idempotency)** — `setup` is safe to re-run: re-writing config and re-registering an already-registered agent/service must not corrupt state (upsert semantics; back up before overwrite where a file is edited).
- **NFR-4 (PBT — partial)** — Property-based tests only for pure config parse/serialize round-trips: hooks serde config round-trip, and mcp zod config round-trip including the new `agents` field.

## Security Compliance (Security Baseline — enabled, applicable rules only)

| Rule | Status | Note |
|---|---|---|
| SECURITY-03 (no secrets in logs) | Applicable — enforce | `setup`/validation must never print/log tokens; redact in all output. |
| SECURITY-05 (input validation) | Applicable — enforce | Config validated (serde strict / zod strict); reject unknown keys, bad URLs, blank tokens. |
| SECURITY-06 / 12 (least privilege / no hardcoded creds) | Applicable — enforce | Config `0600`; service runs as user; tokens only from user-authored config/env, never source. |
| SECURITY-09 (hardening / safe errors) | Applicable — enforce | No default token; generic error messages; no internal path/stack leakage to user. |
| SECURITY-10 (supply chain) | Applicable — enforce | Keep `Cargo.lock` / `package-lock.json` committed and pinned; no new unpinned deps. |
| SECURITY-13 (integrity / safe deserialization) | Applicable — enforce | Config deserialization is schema-validated; prefer official agent CLIs over hand-parsing agent config to avoid corrupting it. |
| SECURITY-15 (exception handling / fail-closed / cleanup) | Applicable — enforce | Handle all I/O and subprocess errors; runtime auth already fails closed (no token → no request); clean up temp files/backups. |
| SECURITY-11 (credential separation of concerns) | Applicable — enforce | Keep credential/token handling isolated in a small dedicated module per side. |
| SECURITY-01 (encryption at rest) | N/A | No data store; token stored `0600` locally. Full OS secure-store backend deferred (out of scope). |
| SECURITY-02, 04, 07, 08, 14 | N/A | No network intermediary, HTML serving, network topology, server-side authz, or alerting introduced — this is a local client-side installer; okc-web owns those. |

## Out of Scope / Deprioritized (to keep scope minimal)

- Unified/shared cross-module config schema (LIR-X1 keeps them separate).
- File-watcher that auto-reinstalls on config save (chose one-shot `setup`).
- OS secure-store token backend for hooks (keep `0600` plaintext; the stub stays deferred).
- Auto-issuing okc-web tokens from the installer (requires admin auth flow).
- Claude Desktop registration target (only Claude Code + Codex in v1).
- Publishing okc-mcp to npm.
- A repository-root `install.sh` convenience wrapper — **deprioritized**; may be added only if trivial after both module `setup` commands exist.

## Key Requirements Summary

Two independent, minimal `setup` (and matching teardown) commands — one per module — that turn a filled-in, `0600` config into a working local install: okc-hooks self-registers its existing auto-start daemon at a user-level config path; okc-mcp writes its config and registers itself into the user's chosen coding agents (Claude Code, Codex) via those agents' official CLIs, with graceful print-only fallback. Credentials are validated best-effort and never logged; the token distinction (upload vs read) is preserved. Security baseline is enforced on applicable rules; resiliency is skipped; PBT is applied only to config round-trips.

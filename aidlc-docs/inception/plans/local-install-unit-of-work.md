# Units of Work — Local Config-Driven Install

Two independent units, one per module. Implemented in each module's own workspace; this root doc owns the coordination view only.

## U1 — okc-hooks: config-driven `setup` + teardown

- **Module workspace**: `okc-hooks/` (artifacts under `okc-hooks/aidlc-docs/`).
- **Requirements**: LIR-H1..H4, LIR-X2, NFR-1..4, SECURITY applicable rules.
- **Crates touched (expected)**: `watcher-bin` (subcommand wiring), `ops-control` (CLI dispatch), `lifecycle-deploy` (reuse existing service register/unregister); `foundation` config reused (avoid changing discovery defaults).
- **Deliverable**:
  - `watcher-bin setup [--config <path>]`: validate a user-authored config → write it to a user-level path (`~/.config/okc-watcher/config.json`, platform equivalent) with `0600` → register the auto-start OS service via the existing install path, binding the config path explicitly in the generated service unit.
  - Best-effort, non-blocking validation (schema + HTTPS scheme + reachability); token never logged.
  - Teardown: reuse/extend existing `uninstall` to also optionally remove the `setup`-generated config; never touch the vault.
  - A shipped, commented sample config (fill-in-the-blanks) under the module.
  - Tests: config round-trip PBT (serde), setup happy-path + invalid-config + missing-token (redaction) with a fakeable service-ops + command runner; idempotent re-run.
- **Dependencies**: none on U2.

## U2 — okc-mcp: `setup` + agent registration/unregister + `agents` config

- **Module workspace**: `okc-mcp/` (artifacts under `okc-mcp/aidlc-docs/`).
- **Requirements**: LIR-M1..M5, LIR-X2, NFR-1..4, SECURITY applicable rules.
- **Files touched (expected)**: `src/config.ts` (add `agents`), `src/cli.ts` (add `setup` + `unregister`/`uninstall`), a small registration module (agent CLI detection + invocation + print-only fallback), reuse `src/web.ts` for validation.
- **Deliverable**:
  - `okc-mcp setup`: write/update the mcp config JSON (local vault or web `baseUrl`+`projectId`+`token`+`readOnly`, plus `agents`) at a user-level path with `0600` → register into each selected agent via its official CLI (`claude mcp add --scope user ... -- <node> <cli.js> serve --config <path>`, `codex mcp add ...`); if a CLI is absent, print the snippet (existing behavior) with a clear message.
  - Best-effort, non-blocking web validation via read-only `GET .../contract`; token never logged.
  - `okc-mcp unregister` (or `uninstall`): remove registration via `claude mcp remove` / `codex mcp remove`; optional config removal; never touch the vault.
  - Update `examples/` sample configs to include `agents`; update README/README_KOR install sections.
  - Tests: config round-trip PBT (zod, incl. `agents`); setup/register happy-path + CLI-absent fallback + web-validation-warn with a fakeable command runner + fetch; idempotent re-run.
- **Dependencies**: none on U1.

## Coordination

- Shared okc-web token semantics: hooks = **upload token** (`/api/sync`), mcp = **serving read token** (`/api/serving`, optional for public projects). No shared code; verified together in root Build & Test.
- Both units enforce the same security posture: `0600` configs, no secret logging/echo, config validation, fail-closed runtime (already true), pinned deps.

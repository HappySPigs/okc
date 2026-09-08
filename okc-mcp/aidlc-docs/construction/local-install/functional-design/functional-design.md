# Functional Design — Local Config-Driven Install (Unit U2)

Scope: okc-mcp only. Root coordination view lives at `../../../../../aidlc-docs/inception/plans/local-install-unit-of-work.md` (U2). Requirements: `../../../../../aidlc-docs/inception/requirements/local-install-requirements.md` (LIR-M1..M5, LIR-X2, NFR-1..4, Security Baseline applicable rules).

## Summary

Before U2, `okc-mcp client-config` only **printed** an MCP registration snippet; there was no "which coding agent" concept and nothing wrote into an agent. U2 adds a config-driven `setup` command that (a) places the mcp config at a user-level path with `0600`, and (b) registers the server into the user's chosen coding agents via their **official CLIs**, plus a symmetric `unregister`. A user fills a config and runs one command.

Design principle: minimal and surgical (NFR-1). The web HTTP client (`src/web.ts`) and the config schema/loader (`src/config.ts`) are reused, not refactored. All new logic lives in one small credential-isolated module `src/install.ts` (SECURITY-11).

## `agents` config schema (LIR-M1)

`src/config.ts` gains one optional field on the strict object:

```ts
agents: z.array(z.enum(['claude', 'codex'])).transform(values => [...new Set(values)]).optional()
```

- v1 targets only `claude` (Claude Code) and `codex`. Unknown members are rejected by the enum; unknown keys are rejected by `.strict()` (SECURITY-05).
- Deduped, first-seen order preserved. Absent means `setup` registers nothing.
- Round-trips through serialize/parse (NFR-4 PBT: `parse(JSON.parse(JSON.stringify(x))).agents === x.agents`).

## `setup` flow (LIR-M2, M3, M4)

`okc-mcp setup [--config <path>]` → `runSetup(options)` in `src/install.ts`:

1. **Resolve paths.** Destination is the user-level config path `userConfigPath()` = `${XDG_CONFIG_HOME | ~/.config}/okc-mcp/config.json`. Source = `--config <path>` if given, else the destination itself (re-register an already-placed config). `configHome` is overridable for tests.
2. **Load + validate** the source via the existing `loadConfig` (zod strict; 64 KiB bound; keeps config outside the Vault). Invalid/unreadable → throw, fail closed (SECURITY-13, 15). A friendly non-secret message is raised when the default destination does not yet exist.
3. **Write the config** to the destination with `0600`: create the `0700` parent, write to a `*.tmp` sibling with mode `0600`, atomic `rename`, then `chmod 0600` (defeats umask). The temp file is cleaned up on failure (SECURITY-06, 15). The written JSON is the canonical parsed config (may include `web.token`) — the token lives only in this `0600` file, never on stdout/stderr.
4. **Best-effort web validation** (LIR-M4): when `web` is configured, run a read-only `GET .../contract` via `new WebVault(config).snapshot()` (reused, injectable as `validateWeb`). On failure, print a warning carrying only the `VaultError` code — never the token — and **continue** (non-blocking; runtime auth already fails closed).
5. **Register into each agent** in `config.agents` via its official CLI (see argv below). Idempotent upsert: `remove` (ignored) then `add` (NFR-3). A non-zero add prints a warning but does not abort.
6. **Print a token-free summary** to stderr (`configPath`, `mode`, `agents`, `registered`, `web`). Fallback snippets go to stdout.

### Agent CLI argv (single source of truth in `src/install.ts`)

The server invocation is centralized in `serverInvocation(node, cli, configPath)` = `[node, cli, 'serve', '--config', configPath]`, reused by both the agent registration and the printed snippet (`clientConfigSnippet`, also consumed by `client-config`).

| Action | argv |
|---|---|
| Claude Code register | `claude mcp add --scope user okc-mcp -- <node> <cli.js> serve --config <configPath>` |
| Codex register | `codex mcp add okc-mcp -- <node> <cli.js> serve --config <configPath>` |
| Claude Code upsert/remove | `claude mcp remove okc-mcp --scope user` |
| Codex remove | `codex mcp remove okc-mcp` |

`<node>` = `process.execPath`; `<cli.js>` = the installed `dist/cli.js` (`fileURLToPath(import.meta.url)` from the CLI). Both are injectable for tests.

### CLI detection + fallback (LIR-M3)

`detect(bin)` spawns `<bin> --version`; a spawn failure (ENOENT) means the CLI is absent (a non-zero exit still counts as installed). When absent, `setup` **prints the `client-config` snippet** for that agent to stdout plus a clear stderr message and does **not** hand-edit the agent's JSON/TOML (SECURITY-13). The subprocess boundary is an injectable `CommandRunner`; the default `spawnRunner` uses `node:child_process` `spawn` with `shell: false` and rejects only on spawn error.

## `unregister` flow (LIR-M5)

`okc-mcp unregister [--config <path>] [--purge]` → `runUnregister(options)`:

- Determine agents from the config (`config.agents`); if no config is readable, attempt removal from all known agents (best-effort).
- For each agent, if the CLI is present run its `mcp remove okc-mcp` (see table); warn on non-zero (may already be absent). If absent, print a manual-removal message.
- `--purge` removes the generated destination config only. The Vault is **never** touched.

## Idempotency (NFR-3)

`setup` re-runs safely: the config is re-written (perms re-enforced) and each agent is re-registered via remove-then-add (upsert). `unregister` tolerates already-absent registrations.

## Security-rule mapping (Security Baseline — applicable rules)

| Rule | How U2 satisfies it |
|---|---|
| SECURITY-03 (no secrets in logs) | Token never printed: absent from argv (config path only), summary, warnings, and the fallback snippet; validation errors carry only `VaultError` codes. Verified by test asserting the token appears in no stdout/stderr byte. |
| SECURITY-05 (input validation) | `loadConfig` + zod strict; `agents` enum; unknown keys/bad URLs/blank tokens rejected. |
| SECURITY-06/12 (least privilege / no hardcoded creds) | Config `0600`, parent `0700`; token only from user-authored config; no default token in source. |
| SECURITY-09 (safe errors) | Generic failure message in the CLI catch; no stack/internal-path leakage; no default token. |
| SECURITY-10 (supply chain) | No new dependency; only `node:*` builtins added; `package-lock.json` unchanged. |
| SECURITY-13 (safe deserialization/integrity) | zod-validated config; agents registered via official CLIs, never by parsing/editing their config files. |
| SECURITY-15 (fail-closed / cleanup) | Subprocess and I/O errors handled; temp file removed on write failure; runtime auth already fails closed; no unhandled rejections. |
| SECURITY-11 (credential separation) | All install/credential-placement logic isolated in `src/install.ts`. |

## Files (repo-relative)

- `okc-mcp/src/config.ts` — add optional `agents` field.
- `okc-mcp/src/install.ts` — new: `runSetup`, `runUnregister`, `clientConfigSnippet`, `spawnRunner`, argv builders, `userConfigPath`.
- `okc-mcp/src/cli.ts` — wire `setup`/`unregister`; `client-config` reuses `clientConfigSnippet`.
- `okc-mcp/examples/*.json` — add an `agents` example.
- `okc-mcp/tests/install.test.ts` — new: setup/unregister behavior with a fake runner + fake web validation.
- `okc-mcp/tests/properties.pbt.test.ts` — add the `agents` config round-trip property.
- `okc-mcp/README.md`, `okc-mcp/README_KOR.md` — document `setup`/`unregister`.

## Verification

`npm run typecheck` PASS · `npm run build` PASS. New/changed tests: 19/19 green (5 install + 2 agents-config/PBT + existing config). Full suite: 82/83 (one pre-existing, unrelated `server.test.ts` US-IN-06 failure from in-flight session-capture work that added `apply_session_capture`/`prepare_session_capture` to `server.ts` without updating its test — not touched by U2). Real-CLI smoke: `setup` wrote `0600` config, warned non-blocking on web validation, token present in the file and absent from stdout/stderr; `client-config` regression intact.

# Integration Test Instructions — okc-mcp (first Unit)

## Purpose
This first Unit is a single local process (one MCP server over stdio); there are no separate services to wire together. "Integration" here means the **end-to-end path from a real MCP client over stdio through the surface → services → core components → filesystem**, plus the CLI setup path. These are covered by `tests/server.test.ts`, which spawns the actual `serve` process and drives it with the MCP SDK `Client` over a `StdioClientTransport`.

## Test scenarios (in `tests/server.test.ts`)
1. **Startup + surface** — server reports version/instructions; exposes exactly the design tool set (`list_notes`, `read_note`, `search_notes`, `audit_vault`, `create_note`, `update_note`, `standardize_frontmatter`, `fix_yaml`, `reinforce_sources_links`); no shell/http/delete/approve/AI tool; guide resource + `capture_knowledge` prompt present. (US-IN-06, BR-TRUST-1)
2. **Create → preview/apply/refuse-overwrite; read range + whole-file hash.** (US-AU-06, US-IN-08, US-RV-01)
3. **standardize_frontmatter** preserves unknown keys/comments/BOM/CRLF body; one external backup with a source-traceable sidecar; stale hash → `hash-mismatch`; no replacement backup on reject. (US-AU-02/05, US-RV-02/03/05)
4. **update_note** applies a combined body + frontmatter change through the one pipeline. (US-AU-05)
5. **fix_yaml** repairs malformed frontmatter in place and rejects a still-invalid correction. (US-AU-03)
6. **reinforce_sources_links** adds literal source + body text only. (US-AU-04)
7. **Literal Korean search + categorized audit** paginate stable results without modifying notes; every finding has one of the six categories. (US-AU-01/07)
8. **Read-only session** omits every mutation tool and rejects direct invocation. (US-IN-09)
9. **Errors** carry a canonical `kind`, redact untrusted parser/source text, and a response-size refusal preserves the stdio connection. (US-IN-09, REQ-008/011)

## Setup / run
No external services. The test harness creates temp Vault/state/config directories per test.
```bash
npm test                 # includes tests/server.test.ts (Node >= 22.13)
```

## CLI setup path (manual integration check)
```bash
node dist/cli.js config --vault /absolute/Vault > okc-mcp.json   # save OUTSIDE the Vault
node dist/cli.js doctor --config /absolute/okc-mcp.json          # per-check diagnostics, no network
node dist/cli.js client-config --config /absolute/okc-mcp.json   # absolute-path client snippet, no secrets
node dist/cli.js serve --config /absolute/okc-mcp.json           # stdio server
```

## Cleanup
Temp directories are removed by each test's `t.after`. Manual runs: delete the config and the state directory you created.

## Actual result in this environment (2026-09-08)
**NOT EXECUTED** — requires Node ≥ 22.13 (host has 16.17.1). Run on a compliant host.

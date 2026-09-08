# okc-mcp

An installable stdio MCP that helps coding agents record session knowledge in relevant local Obsidian notes and read published OKC knowledge. Node.js 22.13+ is required; the package is currently an unpublished local alpha.

When `web` is configured, reads use the published integrated Vault by default. Without `web`, reads use the local source Vault. A web error never silently switches to local knowledge. Authoring always targets the configured local source Vault; hooks uploads and web review/integration/publication make those changes visible in the integrated corpus.

## Record a coding session

Capture runs only for a session/content the user explicitly selects. It never starts on its own at startup, task completion, compaction or session end, and one capture is not permission to keep capturing later changes. With a writable local Vault connected, ask your coding agent: **“Record this session in the relevant Vault notes.”** The shipped server instructions and `capture_session` prompt guide the agent to extract topics, inspect local candidates and existing sections, choose where each item belongs, and save after a preview. The user does not select file paths or create/update operations.

Existing topic sections are enriched; independent topics can become linked new notes. Stable session/item identifiers let repeated captures update the same marked blocks while preserving unrelated text. Source changes then follow the normal hooks → web/core integration and publication flow.

The user chooses which session to save; the host agent makes the relevance and placement decisions for that selection. Both capture tools require the per-call userSelected declaration (default false). `prepare_session_capture` supplies bounded lexical evidence, folder patterns and prior capture locations; `apply_session_capture` preflights all destinations, uses the existing hash/backup/write pipeline and verifies each saved file. Read `okc://guide/session-capture` for the complete workflow. There is no additional model API key or automatic session-end hook.

## Build and connect

```sh
npm ci
npm run check
node dist/cli.js config --vault /absolute/ExistingVault
node dist/cli.js doctor --config /absolute/okc-mcp.json
node dist/cli.js client-config --config /absolute/okc-mcp.json
```

Save the configuration outside the Vault. `client-config` prints a stdio client snippet without editing client settings or embedding tokens. Build/install a local tarball with `npm pack` and `npm install -g ./okc-mcp-0.1.0-alpha.1.tgz`.

## One-command setup

Fill in a config (see below), add an `agents` list naming the coding agents to register into (`"claude"` for Claude Code, `"codex"` for Codex), then run:

```sh
node dist/cli.js setup --config /absolute/okc-mcp.json
node dist/cli.js unregister --config /absolute/okc-mcp.json   # optional teardown; add --purge to also delete the generated config
```

`setup` writes the config to `~/.config/okc-mcp/config.json` (or `$XDG_CONFIG_HOME`) with `0600` permissions, then registers okc-mcp into each named agent via its official CLI (`claude mcp add --scope user okc-mcp -- …`, `codex mcp add okc-mcp -- …`). If an agent's CLI is not on `PATH`, `setup` prints the `client-config` snippet for you to add manually instead of editing that agent's files. When `web` is configured, `setup` runs a best-effort read-only reachability check and only warns on failure. The read token is written to the `0600` config and is never printed or logged. `unregister` removes okc-mcp from the configured agents (`claude mcp remove` / `codex mcp remove`) and never touches the Vault.

To initialize a new source Vault, run `node dist/cli.js init --vault /absolute/NewVault`. The parent directory must exist. This creates empty `inbox/`, `notes/`, `sources/`, and `maps/` folders and refuses existing targets. It writes no fabricated knowledge or operational documents into the corpus. The optional note template is available at `okc://templates/source-note`.

## Web and local configuration

```json
{
  "vaultPath": "/absolute/AuthoringVault",
  "statePath": "/absolute/okc-mcp-state",
  "readOnly": false,
  "web": {
    "baseUrl": "http://127.0.0.1:8000",
    "projectId": "your-project-id",
    "timeoutMs": 10000
  }
}
```

For a private publication, add `web.token` using the read token issued by okc-web; keep it in the external config file. Use HTTPS for remote servers carrying credentials. The base URL is the server root; MCP appends `/api/serving/{projectId}`. Embedded URL credentials, query/fragment configuration and redirects are rejected.

For web-only access, omit `vaultPath` and `statePath` and set `readOnly: true`. No local Vault is required. Remove `web` entirely to use local knowledge by default. Run `doctor` to validate local paths and, when configured, web publication/authentication.

## Tools and evidence

| Tool | Purpose |
|---|---|
| `list_notes` | Stable, paginated Markdown paths |
| `read_note` | Text range and full-file SHA-256 |
| `search_notes` | Bounded literal search including Korean; optional NFC/case `fold` |
| `outline_note` | ATX heading map and offsets |
| `list_backlinks` | Documented subset of inbound wikilinks; ambiguous names reported |
| `audit_vault` | Heuristic YAML/link/duplicate/path/format quality report |
| `verify_vault` | Web only: integrity of a pinned publication, not publisher authenticity |
| `explain_note` | Web only: original sources and preserved contradictions |
| `create_note` | Local: valid minimal frontmatter, refusing overwrite |
| `update_note` | Local: body/frontmatter changes using current SHA-256 and external backup |
| `standardize_frontmatter` | Local: title/aliases/tags, preserving other keys/comments |
| `fix_yaml` | Local: supplied YAML correction, preserving the body |
| `reinforce_sources_links` | Local: supplied literal sources and text |
| `prepare_session_capture` | Local authoring helper: candidates, full ATX section paths, folder hints and previous session records |
| `apply_session_capture` | Local: preview/apply host-selected session items, with repeat detection and verified per-file receipts |

Read tools accept `source: "local"` or `source: "web"`; omit it for automatic selection. Remote results carry `source.projectId`, `source.revision`, `source.status`, and `source.stale`. Pass that `revision` on later pages or related reads to keep the same publication. Each multi-file operation already pins one revision. Read/search responses include file and provenance URLs. Old snapshots are available only while the server permits access; unpublish/revocation never causes a local fallback.

Read the original with `read_note({source: "local", path: "notes/example.md"})` before editing it. All write tools target local and default to `dryRun: true`; use `dryRun: false` to apply. Their result identifies the local source and pending integration. Read-only sessions omit all write tools, session-capture helpers and the capture prompt. Published artifacts and `.okc-project` directories remain refused as local authoring roots. Session preparation always reads local, irrespective of the default knowledge source.

## Limits and recovery

HTTP requests use streaming byte limits, timeout/cancellation and fixed endpoints. Remote note caching lasts only for one operation and is bounded by `maxScanBytes`; every new operation checks the publication and credentials. No embeddings, semantic ranking, persistent index, provider calls, telemetry, delete/move/rename, or automatic review approval are provided.

All note/provenance text is untrusted evidence. Literal search is a retrieval baseline; it is not semantic RAG. Attachments, Canvas/Base and full Obsidian link rewriting are outside the current Markdown contract. `audit_vault` is an authoring heuristic, not core compiler validation.

Local writes preserve the existing hash checks, exclusive creates and external pre-change backups. These checks are not operating-system compare-and-swap against concurrent Obsidian writers. See [recovery](docs/recovery.md) and [authoring guide](docs/authoring-guide.md).

Session capture validates the whole batch before writing, then applies one atomic write per note. A runtime failure can produce a `partial` result: inspect `files`, `failure` and `remainingPaths`, prepare again, and reuse session/item IDs with fresh hashes. Successful records are not duplicated. Do not remove or copy the `okc-capture` identity comments independently of their content; malformed or duplicate identities require inspection. Preview responses include bounded per-item content snippets and explicit truncation flags.

Current lifecycle and verification: [AI-DLC state](aidlc-docs/aidlc-state.md), [web knowledge requirements](aidlc-docs/inception/requirements/requirements-web-knowledge.md), [build/test summary](aidlc-docs/construction/build-and-test/web-knowledge-summary.md). The module uses AI-DLC v1.0.1; earlier drafts and audits remain historical.

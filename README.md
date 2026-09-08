# okc-mcp

> **Status: Inception review pending.** The feature and installation documentation below describes an unapproved implementation draft.
> Construction began before the requirements review; that process error has been recorded and further implementation is paused.
> Confirm the product scope and success criteria in the [Inception review proposal](aidlc-docs/inception/review.md) first.

**A local Obsidian Vault authoring MCP for building high-quality knowledge inputs for OKC.**

Create Markdown notes without the Obsidian app, plugins, or API keys; read and search existing notes; and audit frontmatter, links, duplicates, and input quality. OKC remains responsible for actual integration, review, and compilation.

The current local implementation is `0.1.0-alpha.1` and has not been published to npm. It requires Node.js **22.13+**. Actual validation environments and limitations will be recorded in a Construction validation log after that phase is authorized.

## Install and connect

Build the package from this repository and install it locally.

```sh
npm ci
npm run check
npm pack
npm install -g ./okc-mcp-0.1.0-alpha.1.tgz
```

Generate a configuration using the absolute path of an existing Vault. **Store the configuration outside the Vault.**

```sh
okc-mcp config --vault /absolute/path/to/MyVault > /absolute/path/to/okc-mcp.json
okc-mcp doctor --config /absolute/path/to/okc-mcp.json
okc-mcp client-config --config /absolute/path/to/okc-mcp.json
```

Merge the `mcpServers` entry printed by `client-config` into your MCP client's configuration. Preserve any other server settings. The generation command never edits another application's configuration automatically. Configuration screens and wrapper objects vary by client, but the connection uses standard stdio. The output includes absolute paths to the installed Node executable and server file to reduce PATH differences in GUI applications.

To run from the development tree without installing, use `node dist/cli.js` instead of `okc-mcp` in the commands above. To run the server directly, use the following command; stdin and stdout are reserved for the MCP protocol after startup.

```sh
okc-mcp serve --config /absolute/path/to/okc-mcp.json
```

## First use

Ask the connected AI client something like:

> Read the authoring guidance and find notes about HTTP caching in my Vault.
> Draft a new note that distinguishes claims from the sources I provided,
> apply it, and then audit its quality as OKC input. Do not invent missing sources.

Authoring guidance is available through the `okc://guide/authoring` resource and the `capture_knowledge` prompt. The authoring tools default `dryRun` to `true`. A result with `applied: false` is only a preview; call the tool with `dryRun: false` to save the change. That argument does not prove separate human approval.

| Tool | Purpose |
|---|---|
| `vault_info` | Connection mode, file count, limits, and referenced OKC version |
| `list_notes` | Sorted relative paths with pagination |
| `read_note` | A content range and the SHA-256 of the **entire file** |
| `search_notes` | Literal search, including Korean text, with short excerpts |
| `create_note` | A new note with minimal frontmatter; never replaces an existing file |
| `replace_note` | Replaces the full body after a hash check and external backup |
| `patch_frontmatter` | Sets selected YAML keys while preserving the body, existing keys, and comments |
| `audit_vault` | Audits YAML, links, duplicates, operational noise, and unsupported formats |

The `offset` and `length` used for partial reads count JavaScript string characters, not bytes or line numbers. Lists, searches, and audits use `offset` and `limit`; continue with `nextOffset`. Read and review the entire range before editing a long note. Use the `expectedHash` returned by `read_note`.

## What makes a good Vault for OKC

You do not need to reorganize an existing Vault. For a new Vault, a shallow structure such as `inbox/`, `notes/`, `sources/`, and `maps/` is a reasonable starting point. What matters is **one clear claim per paragraph, nearby sources, unambiguous links, and metadata that is not needlessly repetitive**.

Fields such as `source`, `status`, and `type`, beyond `title`, `aliases`, and `tags`, are optional authoring conventions. Do not assume that OKC interprets them as approval, publication permission, or classification policy. Keep templates, MCP configuration, backups, and operational documents outside the Vault so they do not become input knowledge.

```text
MyKnowledge/
├── AuthoringVault/        ← edited by Obsidian and this MCP
│   ├── inbox/
│   ├── notes/
│   ├── sources/
│   └── maps/
├── tooling/
│   └── okc-mcp.json
├── Knowledge.okc-project/ ← OKC work and review state
└── artifacts/             ← preserved OKC outputs
```

Backups and locks live under the configured `statePath`. By default, this is `.local/state/okc-mcp/<vault-id>` under the user's home directory; neither a path inside the Vault nor a path containing the Vault is allowed. OKC registers the authoring Vault separately as a source and captures snapshots. Changes made after a snapshot become input to the next capture. Resulting artifacts and `.okc-project` directories are not editable targets for this MCP.

See the [authoring guide](docs/authoring-guide.md) and [OKC Vault design](aidlc-docs/inception/okc-vault-design.md) for detailed examples.

## Configuration and troubleshooting

Every path in the [example configuration](examples/okc-mcp.example.json) is absolute. With `readOnly: true`, the server does not register any of the three authoring tools. Diagnostics check paths and scan access; they do not guarantee write access or compiler compatibility.

| Error or situation | Next action |
|---|---|
| `CONFLICT` | Read the note again and review the change against its latest content. Do not blindly retry the previous request. |
| Create path already exists | Read and update the existing note, or choose a new path. |
| `NOTE_INVALID` | Check duplicate YAML keys, syntax, and the types of `title`, `aliases`, and `tags`. |
| Path or link rejected | Use a normal relative `.md` path within the registered Vault. Hidden paths, symlinks, and hardlinks are unsupported. |
| Response limit | Reduce `limit` or the requested read `length`. |
| Scan limit | Narrow the connected source or inspect its size before adjusting configured limits. Do not treat a partial result as a complete audit. |
| Lock conflict | Wait for the other MCP write to finish, then inspect the latest state. Follow the [operations guide](docs/operations.md) for crash recovery. |

## Current guarantees and limitations

- All note-query results are sent to the MCP host. The server itself does not call AI services, remote search, or telemetry. If the host uses a remote model, the host's data-handling policy applies.
- Input auditing uses selected OKC `0.3.0` sources as **authoring heuristics**. It does not replace compiler validation, complete Obsidian link interpretation, sensitive-data detection, or factual verification.
- OKC still has release requirements around preserving attachments, Canvas, and Base files and fully rewriting links.
- Hash checks and file replacement do not provide operating-system compare-and-swap with external Obsidian or Sync processes. Avoid editing the same note concurrently. Backups and observed stale-hash checks support recovery and conflict review.
- Node path checks do not claim complete isolation from malicious concurrent replacement of ancestor directories. Environments where uncontrolled processes replace the filesystem are unsupported.
- Delete, rename, automatic folder moves, snapshot export, OKC approval, compilation, and semantic search are not currently exposed as tools.

## Design and development

The official AI-DLC 2.7.1 Codex workflow is installed in this project. In a new Codex conversation, run `$aidlc --doctor` and follow the [setup and usage guide](docs/aidlc-setup.md). The product is still awaiting Inception review; installing the workflow does not record approval to enter Construction.

- [AWS AI-DLC application proposal](aidlc-docs/methodology.md)
- [Source review of four existing Obsidian MCPs](aidlc-docs/inception/existing-mcp-research.md)
- [Requirements](aidlc-docs/inception/requirements.md) · [Implementation design](aidlc-docs/construction/design.md)
- [Repository and installation UX](aidlc-docs/inception/repository-ux.md) · [Contributing](CONTRIBUTING.md)
- [Current status and follow-up units](aidlc-docs/state.md) · Requirements traceability (to be created during Construction)

This product implements the protocol with the [MCP SDK](https://github.com/modelcontextprotocol/typescript-sdk/tree/v1.30.0). User-facing semantics for Obsidian links and properties follow the [official Obsidian Help documentation](https://obsidian.md/help/Linking%2Bnotes%2Band%2Bfiles/Internal%2Blinks). Differences from actual OKC parser behavior are documented as audit limitations.

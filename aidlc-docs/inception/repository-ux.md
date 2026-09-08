# Repository, installation, and operations UX for OKC users

Design baseline: 2026-09-06. The product's purpose is to **author and organize an Obsidian Vault suitable for OKC input**. Distinguish the initial implementation target below from later distribution recommendations. The root README and Construction records are authoritative for actual supported options and validation results.

## Product promise

Users install a local MCP package, register a Vault to edit, and then create and update notes and audit input quality. The default dependency set does not include an Obsidian plugin, a REST API key, or a native OKC package. Markdown-file operations should work while Obsidian is closed.

With local stdio, the MCP client launches the server process. Protocol messages use stdout and diagnostics use stderr. See the [MCP stdio transport specification](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports).

The first-success experience is creating one note, running an input-quality audit, and opening the result in Obsidian. Users should not need to create a compiler project or configure a provider first.

## Manage the repository independently

Maintain `okc-mcp` as a separate repository and package. It may reference the consuming OKC input contract, but using the MCP must not require a Rust workspace or a specific sibling-repository path.

| Responsibility | Owner |
| --- | --- |
| Note creation and updates; metadata, link, and duplicate auditing; preparation-change preview/apply | `okc-mcp` |
| Local configuration, path restrictions, response limits, installation, diagnostics, and MCP connection | `okc-mcp` |
| Immutable snapshots, canonical IR, approval policy, compilation, and artifact validation | OKC |
| Personal notes, Obsidian settings, caches, and optional backups | User-local storage |

Document OKC input compatibility as a support target and validate it with fixtures. The MCP does not manage the OKC project journal or internal SQLite database directly. Because the current OKC core collects a source snapshot and performs integration and compilation from it, distinguish the live source Vault from the captured snapshot in time. See the [OKC pipeline](https://github.com/dolgogae/okc/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/specs/vault-compilation-pipeline.md).

## File boundaries users can understand

| Space | Meaning | Management model |
| --- | --- | --- |
| Editable source Vault | Notes authored through Obsidian and the MCP | Create and update only inside an explicitly registered path |
| MCP configuration | Vault access and operating boundaries | Contains private absolute paths and is not committed to Git |
| Preparation preview | Proposed note changes after a quality audit | Initially provided by each write tool's `dryRun`; updates verify a whole-content baseline hash |
| OKC snapshot/project | Source capture at a point in time and integration work | Owned by OKC and never changed through MCP authoring |
| Compiled output | Integrated result and provenance | Never used as an editable target for this source-authoring MCP |

Do not reorganize an existing folder structure wholesale at first contact. Offer a good default structure and minimal metadata, and begin with an audit for existing Vaults. Preserve meaning in Markdown and frontmatter rather than treating the execution result of a specific Obsidian plugin as knowledge content. The Vault-design document defines concrete note-type, field, and folder recommendations.

Because Obsidian and the MCP may edit the same note concurrently, an update request includes the content hash returned by the read. If the baseline hash changed, read the latest content and prepare the change again. The initial Node implementation rejects observed conflicts but does not share an operating-system compare-and-swap primitive with external editors. It cannot guarantee prevention of a concurrent write immediately after the final check. Tell users to avoid concurrent edits to the same note and provide an original-content backup location in external state.

## First-install flow

The initial package target is `okc-mcp@0.1.0-alpha.1` on Node.js `>=22.13`. Before public registry release, validate installation from a local tarball. The commands below define the initial CLI contract; the README and test results determine whether each one is actually complete.

1. Build a validated `okc-mcp-0.1.0-alpha.1.tgz`.
2. Install it with `npm install -g ./okc-mcp-0.1.0-alpha.1.tgz`.
3. Save the JSON printed by `okc-mcp config --vault /absolute/path/to/Vault` as a configuration file.
4. Check paths and readiness with `okc-mcp doctor --config /absolute/path/to/okc-mcp.json`.
5. Register the `mcpServers` settings printed by `okc-mcp client-config --config /absolute/path/to/okc-mcp.json` in the client.
6. In a synthetic or user-selected Vault, list notes, create one note, and run an input-quality audit.

Run the server with `okc-mcp serve --config /absolute/path/to/okc-mcp.json`. Generated client configuration uses absolute paths to the Node executable and entry file to reduce PATH differences in GUI applications. Configuration generation prints content and never edits another application's settings automatically.

Do not put native OKC SDK builds or a sibling-repository checkout in the first-use flow. Handoff to OKC should be documented as a separate step, and MCP preparation readiness must remain distinct from successful compilation.

## Authoring and organization UX

Separate tools so their data-mutation scope is visible to the user and MCP client.

| Operation | User experience | Data-mutation boundary |
| --- | --- | --- |
| Read, search, or audit | Inspect current notes and discovered problems | Read-only |
| Create note | Review a new path and content before creation | Reject if a file already exists |
| Update note | Request a change against current content | Update only when the baseline content hash matches |
| Preview note change | Inspect expected output, hash, and warnings from `dryRun: true` | Do not change the Vault before apply |
| Apply note change | Repeat the request with `dryRun: false` | Treat a changed baseline hash as a conflict |

Do not infer authoring intent and alter meaning merely because metadata is missing, and do not automatically delete duplicate candidates. Audits provide evidence for a problem and possible corrections. Separate large-scale moves and semantic merges into later capabilities after designing recovery and link preservation.

Preview and hash checks are consistency controls. The current preview covers a single-note operation per tool; it does not issue preview tokens or batch transactions. A large create preview may be truncated, while frontmatter and full-replacement previews return summaries and hashes. Full diff UI and atomic multi-file application remain future work. A natural-language confirmation or tool argument does not authenticate that a person approved the change. The MCP client's approval UI and the user's actual request scope govern execution.

## Diagnostics and error messages

Show both the problem and the next action. The table is a UX requirement; verify implementation of each entry with test evidence.

| Situation | Essential message | Next action |
| --- | --- | --- |
| Vault path missing | Registered name and nonexistent path | Update configuration with a real absolute path |
| Path escape or symlink | Only permitted files inside the registered path can be handled | Select a permitted relative path |
| Create path already exists | Existing note was not replaced | Read the existing note or select another name |
| Update hash conflict | File changed after it was read | Read the latest content and prepare the change again |
| Stale update request | Target hash changed after preview | Read the latest content and generate a new preview |
| Incomplete metadata | Which field and value affect input quality | Review the recommendation and update explicitly |
| Broken link or duplicate candidate | Bounded list of targets and supporting evidence | Confirm the target, then organize explicitly |
| Result too large | Truncated range and result limit | Narrow the search or request the next range |

`doctor` diagnoses file access and configuration readiness. It does not reorganize the user's Vault while troubleshooting. Logs and issue attachments exclude note bodies, tokens, and personal links by default.

## What users see first in the repository

Structure the README as **capabilities → install now → create the first note → audit input quality → limitations and troubleshooting**. Link research and AI-DLC records separately. State alpha status and genuinely supported platforms near the beginning.

| Repository material | Purpose |
| --- | --- |
| README | Installation, connection, first authoring task, and current limitations |
| Example configuration | Minimal settings without personal paths |
| Synthetic Vault | Reproduce create, update, and audit behavior without personal notes |
| Changelog | Changes to tool contracts, configuration, runtime, and the input profile |
| Contributing guide | How to validate small changes and file-write boundaries |
| Issue templates | Report installation, conflict, and audit problems with a minimal example |
| AI-DLC records | Research evidence, requirements, decisions, and implementation/validation status |

Never include personal Vaults, `.obsidian`, OKC projects, tokens, personal absolute paths, caches, or backups in the npm package. Validate installation scope with an npm `files` allowlist and tarball-content inspection. Example fixtures contain only deliberately authored synthetic data.

Keep the MCP package version distinct from the recommended Vault-profile version. A profile change must not migrate existing notes automatically; explain changed recommendations in the changelog and a fresh preview.

## Distribution and updates

The initial stage ends with local alpha-installation validation. Later public distribution requires release-candidate evidence:

- Install the `npm pack` artifact into an empty directory and verify CLI diagnostics, a stdio handshake, note creation, conflict rejection, and auditing.
- List macOS arm64/x64, Linux x64, and Windows x64 as supported only where actual execution evidence exists. Shared JavaScript does not substitute for platform validation.
- Record the referenced compiler version and source-fixture collection evidence for OKC input compatibility. If no compiler was run, identify the result as design based on documentation and code analysis.
- Release notes state changed behavior, required user actions, validated combinations, and known limitations.
- For public npm publication, use OIDC trusted publishing and provenance. Provenance connects an artifact to its build origin; it does not guarantee functional correctness. See the [npm provenance documentation](https://docs.npmjs.com/generating-provenance-statements/).

After publication, tell clients to run the exact package version selected by the user. Do not update automatically at server startup. Explain that reinstalling an earlier package version repairs the program but does not undo note changes that were already applied. File recovery follows the user's existing version history or the explicitly implemented backup scope; do not promise an undo capability that does not exist.

Do not state that a public package name, npm owner, public URL, or release status is confirmed before checking it directly. Maintenance documentation must match the selected license and current `package.json`.

## Maintenance success criteria

These are measurements to collect, not achieved performance claims.

| Metric | Measurement method |
| --- | --- |
| Friction until first note | Record steps and blockers when completing the task in a clean environment using only the README |
| Conflict prevention | Verify that an update with the old hash is rejected after an external edit |
| Preparation-change comprehension | Review whether a user can explain which file changes and why from the preview alone |
| Input-quality improvement | Compare broken links and metadata problems before and after an audit and their disposition |
| Update burden | Record required configuration changes and whether the previous package can be reinstalled |
| Accuracy of support claims | Link each platform and OKC input profile to actual execution evidence |

Resolve early problems through synthetic reproduction and minimal diagnostics without remote telemetry. Feed connection, conflict, and input-quality findings into requirements for the next Bolt.

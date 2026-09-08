# Research on existing Obsidian MCP implementations

Research date: 2026-09-06. Status: Inception research evidence. Purpose: gather design evidence for an **MCP that locally authors and manages an input Vault that OKC can process effectively**. Querying compiled results or manipulating OKC projects is not the primary goal.

## Scope and evaluation criteria

Two representative REST bridges and two direct-filesystem servers were cloned from GitHub. The actual entry points, tool registration, file handling, search, configuration, and licenses at the commits below were inspected. README installation claims were kept separate from behavior verified in source. This review did not run other projects' tests, measure performance, validate distributed binaries, or perform a complete security audit. It does not present performance numbers from documentation as measurements, and no source code was copied.

| Project | Commit reviewed | Version in source | License verified | Basic architecture |
|---|---|---|---|---|
| MarkusPfundstein/mcp-obsidian | `5ee0b84fa8319fd2fdf0db0ee1febb065e712a15` | 0.2.2 | MIT, LICENSE inspected | Python stdio → HTTP → Local REST API plugin |
| cyanheads/obsidian-mcp-server | `2d4e8d1114d650b45568caa9a163e6e853423266` | 3.5.1 | Apache-2.0, LICENSE inspected | TypeScript MCP → service → Local REST API plugin |
| bitbonsai/mcpvault | `c5abeda9bed11864079f70ae7f33d134e294aad2` | 0.16.0 | MIT, LICENSE inspected | Node.js stdio → file service → Vault |
| lstpsche/obsidian-mcp | `fea2e1f50a8a76d5232b40d07b654ae8037985c7` | 2.5.0 | MIT, LICENSE inspected | Rust MCP → Vault/index → filesystem |

Versions come from package manifests; this is not a claim that they match current npm or crates.io releases. Evidence: [Markus manifest][markus-package], [cyanheads manifest][cyan-package], [MCPVault manifest][bonsai-package], and [Rust manifest][rust-package]. Licenses: [Markus][markus-license], [cyanheads][cyan-license], [MCPVault][bonsai-license], and [Rust][rust-license].

## 1. MarkusPfundstein: small REST wrapper

**Verified in code.** `server.py` registers 15 tools and sends stdio requests to each handler's synchronous `run_tool()`. `obsidian.py` uses `requests` to call `/vault/`, `/search/simple/`, JSONLogic `/search/`, and periodic endpoints. There is no separate local search index; Obsidian's API owns search and frontmatter interpretation. The API key comes from an environment variable and is sent in a Bearer header. The default host is loopback, HTTPS certificate validation is disabled by default, and connect/read timeouts are 3/6 seconds. [Server][markus-server], [HTTP service][markus-service]

**Editing logic.** Append uses POST, full write uses PUT, and partial editing uses PATCH. PATCH sends a heading, block, or frontmatter locator in a header. If a bare nested-heading name fails, it reads the source heading hierarchy and retries only when exactly one candidate exists. Duplicate headings produce an ambiguity error. The delete tool requires `confirm=true`, but that is a model-supplied tool argument and is not evidence of separate human approval. [HTTP service][markus-service], [tool definitions][markus-tools]

**Paths and concurrency.** The wrapper joins string paths into URLs. Vault containment and filesystem atomicity depend on the upstream implementation. No `expectedHash` check against the revision read by the user appears in the reviewed tools or service. This review does not infer guarantees from the REST plugin, whose code was outside scope. [HTTP service][markus-service]

**Installation UX — README evidence.** Installation requires Python 3.11+, uvx, the Obsidian Local REST API plugin enabled, and an API key. Docker configuration is also provided. The Python MCP SDK is constrained to 1.x. Building the OKC input-authoring tool on this stack would add Obsidian runtime, plugin, and transport configuration as additional failure points. [README][markus-readme], [manifest][markus-package]

**Lesson to adopt.** A small tool-to-service structure and refusal to guess among ambiguous headings are useful. REST dependence, unconditional synchronous I/O, and a model-fillable confirmation Boolean should not be defaults for the new MCP.

## 2. cyanheads: structured editing and capability-aware exposure

**Verified in code.** MCP tool definitions are separated from `ObsidianService`. Actual data access is delegated through an undici HTTP client to the Local REST API. Startup checks the Omnisearch connection and includes that mode in the search schema only after success. Text and JSONLogic search results apply read-path policy before cursor pagination. This is not an architecture in which a locally persistent index owns search. [Entry point][cyan-index], [service][cyan-service], [search tool][cyan-search]

**Editing UX.** Full write defaults to `overwrite=false` and returns an error when a file already exists. A `document-map` locates headings, blocks, and frontmatter before section editing. A single heading name is permitted only when unique. Frontmatter updates use `yaml.parseDocument` to preserve comments and quoting styles of untouched YAML nodes where possible, while body-only edits have a separate path that reattaches raw frontmatter unchanged. Re-serializing YAML must not be presented as complete byte preservation. [Write tool][cyan-write], [frontmatter handling][cyan-frontmatter]

**Safety and concurrency.** Read/write path allowlists and read-only mode are enforced again in the service. The command-palette tool is opt-in. Deletion uses framework input requests and responses as a confirmation flow. HTTP retries are limited to GET, PUT, and DELETE and exclude POST and PATCH to avoid duplicate appends. No revision hash or `If-Match` user-edit conflict check appears in the reviewed service or tools. Checking existence before PUT is not atomic create-if-absent. [Path policy][cyan-policy], [delete tool][cyan-delete], [service][cyan-service], [write tool][cyan-write]

**Installation UX — documentation and manifest evidence.** In addition to npx configuration, the project provides an `.mcpb` bundle manifest and client installation links. It requires Node.js 24+ or Bun plus the Obsidian plugin and API key. Required environment variables are declared as user settings in the bundle. This review did not execute the bundle in every claimed client. [README][cyan-readme], [bundle manifest][cyan-bundle]

**Lesson to adopt.** Separate create from replacement by default, use outline-first partial editing, bound response sizes, and expose only capabilities actually available. Installation forms and recoverable error messages are useful references, but HTTP and plugin dependencies should not be defaults.

## 3. MCPVault: simple direct-filesystem installation

**Verified in code.** A Node stdio entry point creates `createServer()` and defines 18 tools. Beyond read, write, patch, frontmatter, and tags, it exposes outline, line-range reads, and wikilink resolution. `--read-only` both hides mutating tools from the list and blocks direct invocation. stdin closure and signals are handled to reduce orphaned processes after a client exits. [Entry point][bonsai-server], [server and tools][bonsai-create]

**Search logic.** Every search recursively enumerates Markdown files, applies path policy, then reads five at a time in parallel. It builds body and filename substring candidates and reranks them with a BM25-style score using document frequencies calculated during that call. The default result count is five and the maximum is 20. A persistent index or watcher is not a prerequisite. Because traversal and admissibility filtering are separate, limiting result count does not limit total I/O. [Search service][bonsai-search]

**Editing logic.** Append/prepend and frontmatter patches edit the original YAML through `parseDocument`. However, full writes default to overwrite; if reading an existing note fails, an append path may continue as a new write. Frontmatter parse failures may also fall back to empty metadata plus the original body. An authoring MCP must distinguish parse and permission failures from a missing file. General note writes and patches use `writeFile` directly; no `expectedHash` check or per-file transaction was found. [File service][bonsai-fs], [frontmatter][bonsai-frontmatter]

**Path logic.** The service fixes the Vault root with `realpath`, checks lexical containment, and verifies the actual location of existing symlink targets. Symlinks that point inside the Vault are allowed. For a new target, it attempts `realpath` on the direct parent but ignores some errors when that parent also does not exist. That is not evidence that every existing ancestor is checked or that all symlinks are rejected. A separate `PathFilter` restricts paths such as `.obsidian`, `.git`, and `node_modules`. [File service][bonsai-fs], [path filter][bonsai-filter]

**Installation UX — README and manifest evidence.** Run through npx with a Vault path on Node.js 20+. The Obsidian app and a plugin are not required. Omitting the path uses the current working directory. Simple startup is worth emulating, but the OKC authoring MCP should require an explicit Vault path to avoid changing the wrong directory. [README][bonsai-readme], [entry point][bonsai-server]

**Lesson to adopt.** Direct filesystem access over stdio, partial reads, concise results, and server-level read-only enforcement fit this product. Permissive path correction, hidden parse errors, and overwrite-by-default do not. A small-Vault MVP can begin with a bounded scan and decide on indexing only after measuring real scale.

## 4. lstpsche: Rust index and live updates

**Verified in code.** `Vault::open` creates a metadata index and optionally a Tantivy index. Metadata tracks notes, tags, links, and backlinks. Tantivy uses an in-memory index with title, heading, body, and frontmatter fields, the English `en_stem` tokenizer, and per-field weights. A notify watcher updates changed files and removes deleted ones through a 500 ms debounce and a capacity-256 channel. It flushes after a batch of Tantivy changes. [Vault startup][rust-vault], [metadata index][rust-index], [Tantivy][rust-tantivy], [watcher][rust-watcher]

**Editing and paths.** The implementation canonicalizes path components, compares NFC keys, checks the nearest existing ancestor for new files, and errors on ambiguous Unicode-normalized names. It does not categorically reject all symlinks inside the Vault. File writes use `fs::write`, append uses `OpenOptions`, and delete removes directly. Frontmatter is parsed into JSON values and re-serialized, which is not a suitable default for preserving YAML comments. No optimistic hash condition was found for general writes. [Paths][rust-path], [file handling][rust-fs], [frontmatter][rust-frontmatter]

**Operational behavior.** Profiles such as `OBSIDIAN_TOOLS=read` restrict the tool router. Startup still creates `.obsidian-mcp` and an ignore file; creation inside the Vault remains even when an external data directory is configured. “Expose read tools only” and “write no bytes to the Vault” are different properties. [Tool configuration][rust-tools], [startup][rust-vault]

**Installation UX — README evidence.** The project documents cargo installation and operating-system release binaries. It also offers a shared HTTP server/daemon mode, and semantic embeddings add feature and runtime choices. Startup numbers in the README were not reproduced in this review. The existence of an English tokenizer alone cannot guarantee quality for Korean Vaults. [README][rust-readme], [Tantivy][rust-tantivy]

**Lesson to adopt.** At larger scale, separate metadata, index, watcher, and update-status concerns. Do not add a semantic daemon, model downloads, or HTTP operations to the MVP. Keep caches, locks, and backups in a local state directory separate from user notes.

## 5. Proposed decisions for the OKC input-authoring MCP

The following are **new-project design proposals derived from the research**, not claims about the existing servers.

| Topic | Proposal | Rationale |
|---|---|---|
| Default connection | Installable Node.js CLI over stdio with an explicit Vault root | Author and read without running Obsidian; no API key or separate listener |
| Managed target | Explicitly connect only a live authoring Vault | OKC's immutable snapshot, project internals, and compiled artifacts are separate spaces |
| Minimal authoring capabilities | Create note, read, search, outline, section patch, and frontmatter patch | Focus on improving input structure and evidence incrementally |
| Source preservation | Preserve unknown properties, YAML comments, CRLF, and body whitespace; stop on malformed YAML | Do not silently transform user or plugin metadata |
| Conflicts | Return a content hash on read, require the expected hash on update, and serialize MCP writes to the same file | Detect stale model output before it silently replaces a newer note |
| Publishing writes | Exclusive create for new files; same-filesystem staging, immediate state recheck, and atomic replacement for existing files | Reduce create conflicts and partial writes during interruption |
| File boundary | Reject traversal, special files, every symlink, Unicode/case collisions, and out-of-bounds paths; recheck root identity | Match the OKC input boundary and apply one policy across every tool |
| Search and linking | Bounded results plus outline-first partial reads; report ambiguous wikilinks | Find needed evidence in small contexts without incorrect automatic linking |
| Readiness audit | Report path, UTF-8, YAML, link, and attachment limitations with evidence | Make OKC input blockers actionable without moving folders automatically |
| Tool authority | Separate mutations from readiness auditing; enforce read-only mode in handlers | Tool annotations do not substitute for enforcement or human consent |
| User experience | Diagnostic command, version-pinned installation example, configuration snippet, and documented external-state location | Explain install → connection check → first note → readiness end to end |

**Actual concurrent-editing limitation.** A hash check plus atomic rename is not operating-system compare-and-swap. It cannot eliminate a race where an external Obsidian or Sync process edits immediately after the final check. The MVP must limit its guarantee to internal MCP serialization and detection of observable stale hashes, and must prohibit automatic overwrite or retry after a conflict. Without a backup and recovery policy plus cooperation from external writers, it cannot claim that every concurrent edit is preserved. Path checks likewise do not provide complete isolation against adversarial concurrent symlink replacement.

**Boundary with OKC.** The [ObsidianKnowlegeComplication repository](https://github.com/dolgogae/ObsidianKnowlegeComplication) README and security contract state that the compiler treats source snapshots as immutable and validates UTF-8/NFC, symlinks, and resource bounds. This does not mean users cannot edit an authoring Vault. The MCP improves an explicitly opened **authoring space** and never changes an **immutable snapshot** already captured as an OKC source. Its readiness result is not a certificate that replaces actual OKC compiler ingestion and verification. Because current OKC materialization is Markdown-only, complete compatibility for attachments, Canvas, and Base must not be promised. [OKC README](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/README.md), [security contract](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/specs/security-and-trust-boundaries.md)

## 6. Cases to connect to pre- and post-implementation validation

- After a note is read and then changed externally, a section or frontmatter patch using the old hash fails with a conflict and preserves the current bytes.
- If a create target already exists, the tool rejects the request rather than silently changing an overwrite option.
- Untouched frontmatter comments, quoted values, unknown properties, body content, and CRLF are preserved. Updating malformed YAML fails.
- Duplicate headings, namesake wikilinks, and normalization-colliding paths show candidates and are never chosen automatically.
- An outside-Vault symlink, inside-Vault symlink, symlink parent, and `.git` or plugin path follow the same policy across create, read, search, and patch.
- Readiness checks and startup do not modify the Vault. Validate permitted locations for caches, logs, locks, and backups.
- Result-size bounds, scan budgets, and cancellation work for Korean and mixed Korean/English titles and bodies, large notes, and many files.
- MCP mutation, file deletion or moves, and remote-model calls remain limited to documented capabilities. Instructions inside notes do not become tool authority.

[markus-package]: https://github.com/MarkusPfundstein/mcp-obsidian/blob/5ee0b84fa8319fd2fdf0db0ee1febb065e712a15/pyproject.toml
[markus-license]: https://github.com/MarkusPfundstein/mcp-obsidian/blob/5ee0b84fa8319fd2fdf0db0ee1febb065e712a15/LICENSE
[markus-readme]: https://github.com/MarkusPfundstein/mcp-obsidian/blob/5ee0b84fa8319fd2fdf0db0ee1febb065e712a15/README.md
[markus-server]: https://github.com/MarkusPfundstein/mcp-obsidian/blob/5ee0b84fa8319fd2fdf0db0ee1febb065e712a15/src/mcp_obsidian/server.py
[markus-service]: https://github.com/MarkusPfundstein/mcp-obsidian/blob/5ee0b84fa8319fd2fdf0db0ee1febb065e712a15/src/mcp_obsidian/obsidian.py
[markus-tools]: https://github.com/MarkusPfundstein/mcp-obsidian/blob/5ee0b84fa8319fd2fdf0db0ee1febb065e712a15/src/mcp_obsidian/tools.py
[cyan-package]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/package.json
[cyan-license]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/LICENSE
[cyan-readme]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/README.md
[cyan-bundle]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/manifest.json
[cyan-index]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/src/index.ts
[cyan-service]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/src/services/obsidian/obsidian-service.ts
[cyan-frontmatter]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/src/services/obsidian/frontmatter-ops.ts
[cyan-policy]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/src/services/obsidian/path-policy.ts
[cyan-write]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/src/mcp-server/tools/definitions/obsidian-write-note.tool.ts
[cyan-delete]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/src/mcp-server/tools/definitions/obsidian-delete-note.tool.ts
[cyan-search]: https://github.com/cyanheads/obsidian-mcp-server/blob/2d4e8d1114d650b45568caa9a163e6e853423266/src/mcp-server/tools/definitions/obsidian-search-notes.tool.ts
[bonsai-package]: https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/package.json
[bonsai-license]: https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/LICENSE
[bonsai-readme]: https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/README.md
[bonsai-server]: https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/server.ts
[bonsai-create]: https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/src/createServer.ts
[bonsai-search]: https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/src/search.ts
[bonsai-fs]: https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/src/filesystem.ts
[bonsai-frontmatter]: https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/src/frontmatter.ts
[bonsai-filter]: https://github.com/bitbonsai/mcpvault/blob/c5abeda9bed11864079f70ae7f33d134e294aad2/src/pathfilter.ts
[rust-package]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/Cargo.toml
[rust-license]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/LICENSE
[rust-readme]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/README.md
[rust-vault]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/src/vault/mod.rs
[rust-index]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/src/vault/index.rs
[rust-tantivy]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/src/vault/tantivy_index.rs
[rust-watcher]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/src/vault/watcher.rs
[rust-path]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/src/vault/path.rs
[rust-fs]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/src/vault/fs.rs
[rust-frontmatter]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/src/vault/frontmatter.rs
[rust-tools]: https://github.com/lstpsche/obsidian-mcp/blob/fea2e1f50a8a76d5232b40d07b654ae8037985c7/src/tools/mod.rs

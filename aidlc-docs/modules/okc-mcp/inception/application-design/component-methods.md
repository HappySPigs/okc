# Component Methods — okc-mcp first Unit

**Status**: Application Design output (autopilot). Method **signatures + purpose + I/O only**. Detailed business rules, validation order, and edge-case semantics are deferred to Functional Design (Construction). Signatures are TypeScript-flavored pseudocode; exact types are finalized in Functional Design.

Common rejection type (returned/raised, never a silent partial write):
`Rejection = { kind: 'path-denied' | 'hash-mismatch' | 'malformed-yaml' | 'overwrite-refused' | 'bounds-exceeded' | 'not-found', message, ...detail }`

---

## C1 · VaultBoundary
| Method | Purpose | Inputs → Outputs |
|---|---|---|
| `resolveInVault(path)` | Resolve a caller path to a real absolute path inside the Vault | `path: string` → `AbsPath` \| `path-denied` |
| `assertWithinVault(path)` | Reject out-of-Vault, symlink, hardlink, hidden control paths | `path: string` → `void` \| `path-denied` |
| `assertOutsideVault(path)` | Confirm a target (e.g. backup dir) is outside the Vault | `path: string` → `void` \| `path-denied` |
| `checkBounds(kind, size)` | Enforce file-size / file-count / response-size limits | `kind, size: number` → `void` \| `bounds-exceeded` |

## C2 · HashService
| Method | Purpose | Inputs → Outputs |
|---|---|---|
| `hash(content)` | Deterministic content hash | `content: string` → `hash: string` |
| `matches(content, expectedHash)` | Conflict check for updates | `content, expectedHash: string` → `boolean` |

## C3 · FrontmatterEngine
| Method | Purpose | Inputs → Outputs |
|---|---|---|
| `parse(raw)` | Split frontmatter + body | `raw: string` → `{ frontmatter, body, comments }` \| `malformed-yaml` |
| `serialize(doc)` | Render back to Markdown, preserving comments | `doc` → `raw: string` |
| `mergePartial(doc, patch)` | Apply partial frontmatter change, preserving unknown keys / body / comments | `doc, patch: object` → `doc'` \| `malformed-yaml` |
| `validate(raw)` | Report whether frontmatter parses | `raw: string` → `{ ok, issues[] }` |

## C4 · NoteStore
| Method | Purpose | Inputs → Outputs |
|---|---|---|
| `read(path)` | Read note content + content hash | `path` → `{ content, hash }` \| `path-denied` \| `not-found` \| `bounds-exceeded` |
| `create(path, content)` | Create a new note, refusing overwrite | `path, content` → `{ hash }` \| `overwrite-refused` \| `path-denied` |
| `writeInPlace(path, content)` | Atomic in-place write (no move/rename) | `path, content` → `{ hash }` \| `path-denied` |
| `list(prefix?)` | Deterministic path listing | `prefix?` → `path[]` \| `bounds-exceeded` |

## C5 · BackupManager
| Method | Purpose | Inputs → Outputs |
|---|---|---|
| `backup(notePath, priorContent)` | Write the single pre-change backup outside the Vault | `notePath, priorContent` → `{ backupRef }` \| `path-denied` |
| `locate(notePath?)` | Report backup location for documented manual recovery | `notePath?` → `backupRef[]` |

## C6 · AuditEngine
| Method | Purpose | Inputs → Outputs |
|---|---|---|
| `audit(notes)` | Heuristic, read-only issue detection, grouped by category | `notes: {path, parsed}[]` → `{ category, path, detail }[]` (explicitly heuristic; no compiler-pass claim) |

---

## S1 · AuthoringService
| Method | Purpose | Inputs → Outputs |
|---|---|---|
| `createNote(input)` | Create an evidence-backed note (title/body + optional aliases/tags/source) | `{ path, title, body, aliases?, tags?, source? }` → `{ hash }` \| rejection |
| `updateNote(input)` | Conflict-aware body/frontmatter update | `{ path, expectedHash, changes }` → `{ hash }` \| `hash-mismatch` \| rejection |
| `standardizeFrontmatter(input)` | In-place title/aliases/tags standardization | `{ path, expectedHash, frontmatterPatch }` → `{ hash }` \| rejection |
| `fixYaml(input)` | In-place correction of invalid YAML frontmatter | `{ path, expectedHash, corrected }` → `{ hash }` \| `malformed-yaml` \| rejection |
| `reinforceSourcesLinks(input)` | In-place source/link reinforcement (literal text only; no link-graph resolution) | `{ path, expectedHash, changes }` → `{ hash }` \| rejection |
| _(internal)_ `applyMutation(...)` | Shared fixed pipeline: assertWithinVault → exists/hash check → build content → backup → atomic write | — |

## S2 · AuditService
| Method | Purpose | Inputs → Outputs |
|---|---|---|
| `auditVault()` | Read-only categorized audit over in-bounds notes | `()` → `{ issues[], heuristic: true }` \| `bounds-exceeded` |

## S3 · DiscoveryService
| Method | Purpose | Inputs → Outputs |
|---|---|---|
| `listNotes(prefix?)` | Deterministic listing | `prefix?` → `path[]` |
| `search(query)` | Literal substring search incl. Korean | `query: string` → `{ path, matches }[]` \| `bounds-exceeded` |
| `readNote(path)` | Read with content hash (baseline for updates) | `path` → `{ content, hash }` \| rejection |

## S4 · SetupService
| Method | Purpose | Inputs → Outputs |
|---|---|---|
| `registerVault(absPath)` | Register exactly one Vault, no migration | `absPath` → `{ vaultRoot }` \| rejects a second Vault |
| `generateConfig()` | Write reviewable config outside the Vault | `()` → `{ configPath }` |
| `diagnostics()` | Shallow local self-check (runtime, FS, Vault reachability); no network | `()` → `{ checks: {name, pass, detail}[] }` |
| `emitClientConfig()` | Ready-to-paste MCP client config, absolute paths, no secrets | `()` → `{ clientConfig }` |

## X1 · McpServer / ToolRegistry
| Method | Purpose | Inputs → Outputs |
|---|---|---|
| `registerTools()` | Bind only safe tools to service methods | `()` → `ToolSurface` (no shell/HTTP/delete/auto-approval/AI) |
| `listTools()` | Expose the safe tool surface for review | `()` → `ToolDescriptor[]` |
| `handle(toolCall)` | Dispatch a tool call; serialize typed rejections to MCP errors | `toolCall` → `result` \| structured error |
| `version()` | Report version string | `()` → `string` |

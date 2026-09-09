# Domain Entities — okc-mcp-first-unit

**Status**: Functional Design output (autopilot). Technology-agnostic domain model. No infrastructure concerns.

## Entities

### Vault
The single registered knowledge root.
- `root: AbsPath` — absolute path to the Vault directory (registered once).
- `bounds: Bounds` — `{ maxFileBytes, maxFileCount, maxResponseBytes }`.
- Invariant: exactly one Vault per session; never relocated or migrated.

### Note
A Markdown file inside the Vault.
- `path: VaultRelPath` — path relative to `Vault.root`.
- `raw: string` — full file text.
- `frontmatter: Frontmatter` — parsed YAML block (may be empty).
- `body: string` — Markdown body after frontmatter.
- `hash: ContentHash` — deterministic hash of `raw` (see ContentHash).

### Frontmatter
Structured metadata with **fidelity guarantees**.
- `title?: string`, `aliases?: string[]`, `tags?: string[]`, `source?: string | string[]` — the *known* keys the tool standardizes.
- `unknownKeys: Map<string, unknown>` — every other key, preserved verbatim.
- `comments: CommentSet` — YAML comments and formatting to be preserved on round-trip.
- Invariant: serialize(parse(raw)) preserves unknownKeys, comments, and body byte-for-byte when no change is requested.

### ContentHash
- `algorithm: string` (fixed, e.g. SHA-256; finalized at Code Generation).
- `value: string`.
- Computed over the exact on-disk bytes of `Note.raw`. Deterministic: identical bytes → identical value.

### BackupRecord
A single pre-change snapshot outside the Vault.
- `sourcePath: VaultRelPath` — the note it protects.
- `priorContent: string` — exact content before the mutation.
- `backupRef: AbsPath` — location outside the Vault.
- `createdAt: timestamp`.
- Invariant: written only when a mutation is actually applied; **single latest** per note (no generational history, no auto-cleanup); never inside the Vault.

### AuditIssue
A heuristic finding from a read-only audit.
- `category: 'yaml' | 'path' | 'link' | 'duplicate' | 'operational-noise' | 'unsupported-format'`.
- `path: VaultRelPath`.
- `detail: string`.
- `heuristic: true` (constant) — never asserts OKC compiler validation passed.

### Rejection
A typed, fail-safe refusal (returned or raised; never a silent partial effect).
- `kind: 'path-denied' | 'hash-mismatch' | 'malformed-yaml' | 'overwrite-refused' | 'bounds-exceeded' | 'not-found'`.
- `message: string`, `detail?: object` (e.g. current hash on `hash-mismatch`).

### ToolCall / ToolResult (surface)
- `ToolCall: { tool: string, args: object }` — note content in args is **untrusted data**, never executed or interpolated.
- `ToolResult: Success<T> | Rejection` — rejections serialized to structured MCP tool errors.

## Relationships
```
Vault 1 ── * Note
Note 1 ── 1 Frontmatter ── * unknownKeys / comments
Note 1 ── 1 ContentHash
Note 1 ── 0..1 BackupRecord (latest pre-change)
Vault 1 ── * AuditIssue (transient, per audit run)
```

## Value/immutability notes
- `ContentHash` and `AuditIssue` are computed values (not persisted state).
- `BackupRecord.priorContent` is immutable once written.
- Known frontmatter keys are a *projection*; the authoritative store is the file text + `unknownKeys` + `comments`.

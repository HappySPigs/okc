# Business Logic Model — okc-mcp-first-unit

**Status**: Functional Design output (autopilot). Technology-agnostic workflows and algorithms. Rules referenced by ID from [`business-rules.md`](business-rules.md).

## Core algorithms

### A1 · Safe path resolution (BR-PATH-*)
```
resolve(input):
  abs = realpath(join(Vault.root, input))     # resolves symlinks
  if any segment is a symlink/hardlink        -> path-denied  (BR-PATH-2)
  if abs is not a descendant of Vault.root     -> path-denied  (BR-PATH-1)
  if any segment is a hidden/control path       -> path-denied  (BR-PATH-3)
  return abs
```

### A2 · Content hash (BR-HASH-1)
```
hash(raw): return HASH_ALGO(bytes(raw))        # deterministic; algo fixed at Code Gen
matches(raw, expected): return hash(raw) == expected
```

### A3 · Structure-preserving partial merge (BR-STRUCT-1/2)
```
mergePartial(raw, patch):
  doc = parse(raw)                             # keeps unknownKeys + comments + body; malformed -> malformed-yaml
  for (k, v) in patch: doc.frontmatter[k] = v  # only these keys change
  out = serialize(doc)                          # unknownKeys, comments, body verbatim
  if not parses(out): -> malformed-yaml         # never emit invalid YAML
  return out
```

### A4 · Literal Unicode search (BR-DISC-2)
```
search(query): for each in-bounds note, codepoint-faithful substring match (Korean-safe); deterministic order
```

## Workflows

### W1 · Read / baseline  (US-RV-01, US-AU-07 · REQ-006/008)
`readNote(path)` → resolve(path) [A1] → enforce bounds [BR-BOUND-1] → load raw → `{ content: raw, hash: hash(raw) }`. `not-found` if absent.

### W2 · The fixed safe mutation pipeline  (applyMutation — used by ALL writes)
```
applyMutation(path, expectedHash, buildContent, isCreate):
  1. abs = resolve(path)                                   # A1 / BR-PATH
  2. exists = fileExists(abs)
     if isCreate and exists            -> overwrite-refused # BR-CREATE-1
     if not isCreate and not exists    -> not-found
     if not isCreate:
        current = read(abs)
        if not matches(current.raw, expectedHash) -> hash-mismatch (detail: current hash)  # BR-HASH-3/4
  3. newRaw = buildContent(current?)                        # e.g. A3 for partial; may -> malformed-yaml
  4. if not isCreate: BackupManager.backup(path, current.raw)   # single, outside Vault  # BR-BACKUP-1
        (steps 1-3 rejections happen BEFORE this -> no misleading backup, BR-BACKUP-2)
  5. atomicWrite(abs, newRaw)                               # BR-ATOMIC-1
  6. return { hash: hash(newRaw) }
```
Every AuthoringService method is a thin wrapper choosing `buildContent` and `isCreate`:
- `createNote` → isCreate=true, build from title/body(+aliases/tags/source).
- `updateNote` → isCreate=false, build = apply body/frontmatter changes.
- `standardizeFrontmatter` → build = mergePartial(current, {title/aliases/tags}) [A3].
- `fixYaml` → build = corrected frontmatter via mergePartial/validate [A3]; invalid → malformed-yaml.
- `reinforceSourcesLinks` → build = literal source/link edits [BR-STRUCT-4].

### W3 · Audit  (US-AU-01 · REQ-007)
`auditVault()` → list in-bounds notes [BR-AUDIT-4] → parse each [A3.parse] → AuditEngine categorizes (yaml/path/link/duplicate/operational-noise/unsupported-format) → `{ issues[], heuristic:true }`. Read-only.

### W4 · Discovery  (US-AU-07, US-RV-01 · REQ-006)
`listNotes` [A1 + deterministic order, BR-DISC-1]; `search` [A4]; `readNote` [W1].

### W5 · Setup  (US-IN-01..05 · REQ-001/002/009/010)
`registerVault(abs)` (one only; reject second; no migration) → `generateConfig` (outside Vault) → `diagnostics` (shallow, no network) → `emitClientConfig` (absolute paths, no secrets).

### W6 · Manual recovery  (US-RV-04 · REQ-004)
Documented: locate the latest `BackupRecord` for a note → user re-authors via `updateNote` (with a matching expectedHash) → restore flows through W2 like any other write. No special rollback tool.

## Error handling (BR-REJECT-*)
All workflows short-circuit on the first failed check and return a typed `Rejection`. No partial writes, no side effects, no backup on rejection. The surface (X1) serializes `Rejection` to a structured MCP tool error preserving `kind`.

## Explicitly out of scope (deferred / excluded)
File move/rename/merge, multi-Vault, real OKC ingestion/compiler checks, semantic search, Obsidian UI/Dataview, remote HTTP, deletion, auto-approval, internal AI. (requirements §7/§8.)

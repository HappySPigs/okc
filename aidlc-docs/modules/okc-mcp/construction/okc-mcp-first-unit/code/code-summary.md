# Code Generation Summary — `okc-mcp-first-unit`

**Status**: Code Generation Part 2 complete (autopilot). Documentation of the generated code. Application code lives at the workspace root; this file is a markdown summary only.
**Approach**: refactor-in-place of the hardened brownfield draft to the approved design (see the code-generation plan and audit for the recorded reconciliation decision + accepted deviation).

## Files: kept / edited / added / removed

| File | Disposition | Design element |
|---|---|---|
| `src/vault.ts` | **Edited** (added `locate` + backup→source sidecar; all safety internals preserved) | C1 VaultBoundary, C2 HashService, C4 NoteStore, C5 BackupManager |
| `src/config.ts` | **Kept** as-is | C1 bounds, BR-SETUP-3/5 (config outside Vault) |
| `src/notes.ts` | **Edited** (audit `category` projection; `fixYamlContent`, `reinforceContent`, `replaceBody`; `source` accepts string\|string[]) | C3 FrontmatterEngine, C6 AuditEngine |
| `src/guide.ts` | **Edited** (recommended-tool order updated) | authoring-guide resource text |
| `src/rejection.ts` | **Added** | Canonical `Rejection.kind` set + `VaultError.code`→kind map (BR-REJECT-2) |
| `src/authoring.ts` | **Added** | S1 AuthoringService — centralized `applyMutation` + 5 thin wrappers (nfr P2, W2) |
| `src/server.ts` | **Rewritten (surface)** | X1 McpServer/ToolRegistry — design tool surface, kind mapping, category output |
| `src/cli.ts` | **Edited** (`doctor` → per-check diagnostics) | S4 SetupService (CLI-delivered) |
| `tests/vault.test.ts` | **Kept + extended** (locate/backup-traceability) | Vault safety invariants |
| `tests/config.test.ts` | **Kept** | Config bounds + outside-Vault |
| `tests/notes.test.ts` | **Kept + extended** (category, fixYaml, reinforce, replaceBody) | C3/C6 |
| `tests/server.test.ts` | **Rewritten** | X1 tool surface + kind/category, redaction, bounds |
| `tests/authoring.test.ts` | **Added** | applyMutation pipeline invariants |
| `tests/properties.pbt.test.ts` | **Added** | PBT (self-contained generator) |
| `docs/authoring-guide.md`, `docs/recovery.md` | **Edited / Added** | US-IN-07, US-RV-04, RESILIENCY-13 |
| `examples/okc-mcp.example.json` | **Verified** (absolute paths, no secrets) | US-IN-05 |
| `package.json`, `CHANGELOG.md` | **Edited** | packaging + D13 lightweight release governance |
| `src/domain/`, `src/core/` (from-scratch skeleton) | **Removed** | Superseded by the superior draft modules |

## Design tool surface (X1) — 9 tools
Read: `list_notes`, `read_note` (whole-file hash), `search_notes` (literal, codepoint-faithful, Korean-safe), `audit_vault` (categorized, heuristic).
Authoring (only when `!readOnly`, `dryRun` default true → REQ-011 no auto-approval): `create_note`, `update_note`, `standardize_frontmatter`, `fix_yaml`, `reinforce_sources_links` — all via `authoring.applyMutation`.
Absent by construction (BR-TRUST-1): no shell / arbitrary-HTTP / delete / auto-approval / AI-invocation tool. `okc://guide/authoring` resource + `capture_knowledge` prompt retained (read-only, in-scope).

## Canonical rejection map (BR-REJECT-2)
`path-denied` ← INVALID_PATH, PATH_ESCAPE, PATH_DEPTH, PATH_COLLISION, PATH_CHANGED, SYMLINK, HARDLINK, NOT_REGULAR_FILE, IMMUTABLE_TARGET, INVALID_VAULT/STATE, STATE_OVERLAP, ROOT_CHANGED · `hash-mismatch` ← CONFLICT, INVALID_HASH, FILE_CHANGED · `malformed-yaml` ← NOTE_INVALID, INVALID_CONTENT, INVALID_UTF8 · `overwrite-refused` ← NOTE_EXISTS · `bounds-exceeded` ← NOTE_TOO_LARGE, NOTE_LIMIT, SCAN_LIMIT, RESPONSE_LIMIT · `not-found` ← NOTE_NOT_FOUND. Operational refusals outside the design set (e.g. VAULT_BUSY) surface with their `code` and no forced `kind`.

## Audit category map (BR-AUDIT-2)
`yaml` ← OKC_FRONTMATTER_INVALID/TEXT_ENCODING/METADATA_INVALID, NOTE_INVALID · `link` ← OKC_LINK_UNSAFE/UNRESOLVED/AMBIGUOUS/FRAGMENT_UNRESOLVED · `duplicate` ← OKC_DUPLICATE_BODY · `unsupported-format` ← OKC_ATTACHMENT_OUTPUT/NONMARKDOWN_OUTPUT/NOTE_TOO_LARGE · `path` ← OKC_AUDIT_SKIPPED · `operational-noise` ← default (OKC_INGEST_NOISE/EMPTY_NOTE/NAME_AMBIGUOUS/SENSITIVE_CANDIDATE, …).

## REQ / story → implementation
REQ-001/010 → cli.ts + server.ts (US-IN-01/04/05) · REQ-002 → vault.ts/setup (US-IN-02) · REQ-003 → authoring.createNote + vault.create (US-AU-06, US-IN-08) · REQ-004 → applyMutation + vault backup/hash + locate (US-AU-05, US-RV-02/03/04) · REQ-005/013 → notes frontmatter engine + fixYaml/reinforce/replaceBody (US-AU-02/03/04, US-RV-05) · REQ-006 → vault.read/list + search (US-AU-07, US-RV-01) · REQ-007 → notes.auditNotes + category (US-AU-01) · REQ-008 → vault path-safety + bounds (US-IN-09) · REQ-009 → config/backups outside Vault (US-IN-03/07) · REQ-011 → server safe surface (US-IN-06). REQ-012 = the AI-DLC artifact trail (non-runtime).

## Accepted design deviations (recorded)
1. **Structure**: C1–C6/S1–S4 realized as cohesive modules (`vault.ts`, `notes.ts`, `authoring.ts`, `server.ts`), not one class per component — to preserve the draft's battle-tested, interdependent safety internals. Conformance is met at the contract (canonical kinds), invariant (single `applyMutation`), and tool-surface levels.
2. **S4 setup** is CLI-delivered (`config`/`doctor`/`client-config`/`serve`) rather than MCP tools — defensible for MVP + REQ-010 reviewable install (review-endorsed).
3. **PBT** uses a self-contained seeded generator instead of D11's `fast-check` — the environment is offline (`npm` cannot fetch `fast-check`). Swap in `fast-check` when network install is available; the property intent (many generated, reproducible inputs) is preserved.
4. `read_note` returns bounded ranges (whole-file hash always returned); `search_notes` is literal/case-sensitive per BR-DISC-2.

## MVP scope hold
No file move/rename/merge, multi-Vault, real OKC ingestion/compiler checks, semantic search, Obsidian UI/Dataview, remote HTTP/network egress, delete, or auto-approval. Internal `rename`/`link`/`unlink` are atomic-write/lock/cleanup mechanisms, not exposed capabilities.

## Verification status
- `npm run typecheck` (tsc, src + tests) — **PASS**.
- `npm run build` (tsc emit → `dist/`) — **PASS**.
- Pure-logic runtime check (rejection map, hash, frontmatter preservation, fixYaml/reinforce/replaceBody, audit category) — **PASS** on the available runtime, against compiled `dist/`.
- Full `npm test` suite (`node --test` + real stdio MCP client) — **NOT RUN HERE**: this environment's Node is v16.17.1, below the project's required `>=22.13` (`node --test`/`--import` need Node ≥ 18/20). Run `npm run check` on a Node ≥ 22.13 host to execute the suite. This is an environment limitation, not a known code defect.

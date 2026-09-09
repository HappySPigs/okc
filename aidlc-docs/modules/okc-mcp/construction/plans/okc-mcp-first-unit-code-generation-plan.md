# Code Generation Plan — `okc-mcp-first-unit` (REVISED: refactor-in-place)

**Status**: Code Generation Part 1 (Planning). **Single source of truth** for Part 2 generation.
**Gate**: The product/Construction gate was lifted by explicit user instruction on 2026-09-08 (see `../../audit.md`). Autopilot did not auto-lift it.
**Mode**: Autopilot — plan self-approved; generation proceeds without per-step blocking. Recommended, MVP-scoped choices only.

## Reconciliation approach (REVISED after reading the draft + reconciliation review)
The existing draft is **production-grade and security-hardened** and **exceeds** the REQ-008/011 baselines (TOCTOU re-stat guards, `O_NOFOLLOW`, advisory write-lock, NFC/case-collision detection, `ROOT_CHANGED` identity checks, immutable-OKC-target rejection, BOM/closing-fence-preserving frontmatter, JSON-value metadata validation, sophisticated audit link resolution). The gaps vs the approved design are **contract/structural**, not safety. Therefore: **refactor-in-place**, preserving the hardened internals, and close the conformance gaps. A clean-slate layered rewrite is explicitly rejected (it would risk regressing battle-tested safety and exceeds MVP churn).

**Accepted design deviation (recorded):** the layered C1–C6 / S1–S4 decomposition is realized as cohesive modules, not one class per component. Design conformance is achieved at:
- **Contract level** — canonical `Rejection.kind ∈ {path-denied,hash-mismatch,malformed-yaml,overwrite-refused,bounds-exceeded,not-found}` (BR-REJECT), and audit findings projected to the fixed 6 `AuditCategory` (BR-AUDIT-2).
- **Invariant level** — one centralized `applyMutation` pipeline all writes funnel through (design's #1 invariant, nfr P2).
- **Tool-surface level** — the design's authoring tool set (`create_note`, `update_note`, `standardize_frontmatter`, `fix_yaml`, `reinforce_sources_links`) + discovery/audit tools.
- **S4 setup** stays **CLI-delivered** (`config`/`doctor`/`client-config`/`serve` ≈ registerVault/generateConfig/diagnostics/emitClientConfig), defensible for MVP + REQ-010 reviewable install (review-endorsed).

## Unit context
- **Unit**: `okc-mcp-first-unit` — the local, single-Vault, stdio Obsidian authoring MCP.
- **Design sources (authoritative)**: `aidlc-docs/inception/application-design/`, `aidlc-docs/construction/okc-mcp-first-unit/{functional-design,nfr-design,nfr-requirements}/`.
- **Tech stack**: Node ≥22.13, TS strict (NodeNext, noUncheckedIndexedAccess, exactOptionalPropertyTypes), `@modelcontextprotocol/sdk@1.30.0` stdio, `zod@4.5.4` schemas, `yaml@2.9.0` CST, Node `crypto` SHA-256 (bare 64-hex — preserves the existing `expectedHash` contract), `fast-check` PBT (add devDep), `node:test` runner.

## Files (flat layout, matching the draft)
```
KEEP (hardened core; light edits only)
  src/vault.ts     C1+C2+C4+C5 — path safety, bounds, hash, atomic write, backup, locks.
                   EDIT: add backup→source traceability (sidecar meta) + `locate(notePath?)`.
  src/config.ts    Bounds + config-outside-Vault (C1/BR-SETUP-3/5). KEEP as-is.
  src/guide.ts     Authoring-guide resource text. EDIT: reflect the final tool names.
ADD
  src/rejection.ts       Canonical Rejection.kind set + VaultError.code→kind mapper (BR-REJECT-2).
  src/authoring.ts       S1 — centralized applyMutation + createNote/updateNote/standardizeFrontmatter/fixYaml/reinforceSourcesLinks builders (W2, nfr P2).
EDIT
  src/notes.ts     C3+C6. ADD: category projection to the fixed 6 (BR-AUDIT-2); a `fixYaml` builder
                   (replace malformed frontmatter with validated corrected YAML, body preserved);
                   a literal `reinforceSourcesLinks` builder (source key + literal body append).
  src/server.ts    X1. Rewire tools to the design surface via authoring.ts + applyMutation; map errors
                   to canonical `kind` in reply(); add `category` to audit output. Preserve dryRun-default-true
                   (REQ-011 no auto-approval) and untrusted-content handling.
  src/cli.ts       Enhance `doctor` to emit per-check {name,pass,detail}[] (diagnostics, BR-SETUP-4).
TESTS
  tests/vault.test.ts   KEEP + add cases for `locate` and backup traceability.
  tests/config.test.ts  KEEP.
  tests/notes.test.ts   UPDATE for category projection + fixYaml/reinforce builders.
  tests/server.test.ts  UPDATE for the new tool surface + kind mapping.
  tests/authoring.test.ts   NEW — applyMutation pipeline: single-backup-on-accept/none-on-reject, hash-mismatch, overwrite-refused, malformed-yaml, path-denied, atomic.
  tests/properties.pbt.test.ts  NEW — fast-check: parse∘serialize fidelity, mergePartial only-named-keys, hash determinism, literal Korean search.
DOCS/PACKAGING
  docs/authoring-guide.md  UPDATE to the final tool surface.
  docs/recovery.md         NEW — manual recovery runbook (W6/US-RV-04/RESILIENCY-13).
  examples/okc-mcp.example.json  VERIFY absolute-path client-config shape (no secrets).
  package.json             ADD fast-check devDep; add docs/recovery.md to files[].
  CHANGELOG.md             APPEND first-Unit entry (D13).
```

## Generation steps
- [x] **Step 1 — `src/rejection.ts`.** Canonical `RejectionKind` union + `Rejection` shape + `toKind(code)` mapper (`INVALID_PATH|PATH_ESCAPE|SYMLINK|HARDLINK|PATH_COLLISION|PATH_DEPTH|IMMUTABLE_TARGET|…→path-denied`, `CONFLICT|INVALID_HASH→hash-mismatch`, `NOTE_INVALID|OKC_*→malformed-yaml` where YAML-related, `NOTE_EXISTS→overwrite-refused`, `NOTE_TOO_LARGE|SCAN_LIMIT|RESPONSE_LIMIT|NOTE_LIMIT→bounds-exceeded`, `NOTE_NOT_FOUND→not-found`). BR-REJECT-1/2. _Underpins all tools._
- [x] **Step 2 — `src/vault.ts` edits.** Persist backup traceability: alongside each backup file write a sidecar `{ sourcePath, sha256, createdAt }`; add `locate(notePath?)` returning `{ backupRef, sourcePath, createdAt }[]` from `statePath/backups`. Preserve all existing safety. BR-BACKUP-1/4, C5.locate. _Stories: US-RV-03/04, US-IN-07._
- [x] **Step 3 — `src/notes.ts` edits — audit category projection.** Add `category: AuditCategory` to every `Finding` via a code→category map (`OKC_FRONTMATTER_INVALID|OKC_TEXT_ENCODING|OKC_METADATA_INVALID→yaml`; `OKC_LINK_*→link`; `OKC_DUPLICATE_BODY→duplicate`; `OKC_INGEST_NOISE|OKC_EMPTY_NOTE|OKC_NAME_AMBIGUOUS|OKC_SENSITIVE_CANDIDATE|OKC_AUDIT_SKIPPED→operational-noise`; `OKC_ATTACHMENT_OUTPUT|OKC_NONMARKDOWN_OUTPUT|OKC_NOTE_TOO_LARGE→unsupported-format`; path-safety skips→path). Keep the heuristic `limitations` (BR-AUDIT-3). BR-AUDIT-2. _Stories: US-AU-01._
- [x] **Step 4 — `src/notes.ts` edits — new builders.** `fixYamlContent(current, correctedFrontmatter)`: textually split current into (fm, body) even when fm is malformed, validate `correctedFrontmatter` parses (else `NOTE_INVALID`/malformed-yaml), return `---\n<corrected>\n---\n<body>` preserving BOM/newline; then `validateNote`. `reinforceContent(current, {source?, appendBody?})`: literal only — merge `source` into frontmatter via existing `patchFrontmatter` and/or append literal text to body; no link-graph resolution (BR-STRUCT-4). _Stories: US-AU-03, US-AU-04._
- [x] **Step 5 — `src/authoring.ts` (S1, centralized `applyMutation`).** One pipeline `applyMutation({path, expectedHash, isCreate, build, dryRun})`: assert path (delegated to Vault) → create: build from empty, `vault.create` refuses overwrite; update: `vault.read` current, require+match `expectedHash` (missing/stale → hash-mismatch), `build(current)`, `vault.update` (single external backup + atomic). dryRun returns preview (proposed hash, issues) without writing. Thin wrappers: `createNote`, `updateNote({changes:{body?,frontmatter?}})`, `standardizeFrontmatter({frontmatterPatch:title/aliases/tags})`, `fixYaml({correctedFrontmatter})`, `reinforceSourcesLinks({source?,appendBody?})`. No mutation bypasses this path (nfr P2). W2, BR-HASH-3/4, BR-BACKUP-1/2. _Stories: US-AU-02/03/04/05/06, US-IN-08, US-RV-02/03/05._
- [x] **Step 6 — `src/server.ts` rewrite of the tool surface (X1).** Read tools: `list_notes`, `read_note`, `search_notes`, `audit_vault` (now with `category`). Authoring tools (only when `!readOnly`, dryRun default true): `create_note`, `update_note`, `standardize_frontmatter`, `fix_yaml`, `reinforce_sources_links` — all via `authoring.applyMutation`. Map every thrown error to canonical `kind` (keep `code` in detail) in `reply()`. Keep `okc://guide/authoring` resource + `capture_knowledge` prompt (non-executable, in-scope). Retire `vault_info`/`replace_note`/`patch_frontmatter` names (superseded). BR-TRUST-1/2, BR-REJECT-2, P3/P7. _Stories: US-IN-06, US-AU-*, US-RV-*._
- [x] **Step 7 — `src/cli.ts` diagnostics.** `doctor` emits `checks: {name,pass,detail}[]` (node runtime, config load, vault reachability, state dir, scan) with an overall pass; actionable failures; no network. Keep `config`/`client-config`/`serve`. BR-SETUP-4/5. _Stories: US-IN-04/05._
- [x] **Step 8 — Tests.** Keep `vault.test.ts`(+locate)/`config.test.ts`; update `notes.test.ts`(category+builders)/`server.test.ts`(surface+kind); add `authoring.test.ts` (pipeline invariants + every rejection kind) and `properties.pbt.test.ts` (fast-check). NFR-TEST-1/2/3, P8. _Covers all 21 stories' ACs._
- [x] **Step 9 — Docs/packaging.** Update `docs/authoring-guide.md`; add `docs/recovery.md`; verify `examples/okc-mcp.example.json`; add `fast-check` devDep + `docs/recovery.md` to `package.json files[]`; append `CHANGELOG.md`. _Stories: US-IN-01/03/05/07, US-RV-04._
- [x] **Step 10 — Code summary.** `aidlc-docs/construction/okc-mcp-first-unit/code/code-summary.md`: files kept/edited/added/removed, REQ/BR/story→file map, canonical-kind + category maps, the accepted design deviation, MVP-scope-hold statement. Documentation only.

## MVP scope guard (must hold)
No file move/rename/merge; no multi-Vault; no real OKC ingestion/compiler checks; no semantic search; no Obsidian UI/Dataview; no remote HTTP/network egress; no delete/auto-approval/AI-invocation tool. (requirements §7/§8.) The draft is already clean here — preserve it.

## Completion criteria
Steps 1–10 `[x]`; `npm run typecheck` clean; `npm test` green; `npm run build` emits `dist/`; tool surface matches the design; canonical kinds + audit categories present; no orphaned/duplicate files; MVP scope held. Then present the 2-option completion message and proceed to Build and Test.

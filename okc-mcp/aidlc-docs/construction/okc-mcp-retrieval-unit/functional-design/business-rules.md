# Business Rules — `okc-mcp-retrieval-unit`

**Status**: Functional Design (Construction) output for the new read-only retrieval Unit — **awaiting explicit user approval**. Baseline: [`requirements-retrieval-unit.md`](../../../inception/requirements/requirements-retrieval-unit.md). Extends the first-Unit rule families (BR-PATH/BOUND/HASH/CREATE/STRUCT/BACKUP/ATOMIC/AUDIT/DISC/TRUST/SETUP/REJECT). All rules here govern **read-only** behavior; none touch the write/backup/lock pipeline.

## BR-RANK — BM25 lexical ordering (REQ-014) — ❌ DROPPED (D21, 2026-09-09)

> REQ-014/BR-RANK-* were **dropped** from active scope (untraced to success criteria + muted IDF). The rules below are retained for historical context only; nothing here is implemented.

| ID | Rule |
|---|---|
| BR-RANK-1 | `search_notes` gains a `rank` enum (default `'path'`). `rank='path'` is **byte-identical** to today (deterministic path-ascending; NO added field). `ranking` and `score` are emitted **only** when `rank='bm25'`. |
| BR-RANK-2 | `rank='bm25'` reorders the **same** whole-query literal-substring match set; it never adds or removes a match. Output paths are a **permutation** of the `rank='path'` paths for the same query. |
| BR-RANK-3 | BM25 `df`/`avgdl`/`N` are derived entirely within the single bounded per-call scan; nothing is persisted. The scan still accumulates original `Buffer.byteLength` against `config.maxScanBytes` (throws `SCAN_LIMIT`, never a silent partial); `reply()` still refuses over-budget (`RESPONSE_LIMIT`). |
| BR-RANK-4 | Constants fixed (`k1=1.2`, `b=0.75`). Query terms are **deduplicated** (Set) before scoring. A whitespace-only query yields zero terms → falls back to `rank='path'`, label reflects `'path'`. |
| BR-RANK-5 | Sort key = rounded score (`Math.round(score*1e6)/1e6`) DESC, then path ASC using the **same UTF-16 code-unit comparator as `vault.list()`** (`a<b?-1:a>b?1:0`, no locale). Deterministic per platform (`Math.log` cross-engine variance disclosed). No `Date.now()`/`Math.random()`. |

## BR-FOLD — case/Unicode folding (REQ-015)

| ID | Rule |
|---|---|
| BR-FOLD-1 | `search_notes` gains a `fold` boolean (default `false`). `fold=false` byte-identical to today. `foldText(s)=s.normalize('NFC').toLowerCase()` — locale-independent `toLowerCase` (never `toLocaleLowerCase`), never NFKC. `fold` echo appears only when `true`. |
| BR-FOLD-2 | `fold=true` broadens matching (NFC + case-insensitive) but is **NOT a strict superset** of the codepoint-faithful default: an NFC composition across a base+combining-mark boundary (e.g. a leading-jamo substring inside decomposed Hangul) can drop a match the default finds. Disclosed; default `fold=false` loses nothing. |
| BR-FOLD-3 | Excerpts are always verbatim slices of ORIGINAL note bytes. Under fold, the folded match index centers the excerpt **only when the folded prefix preserves length** (`foldText(content.slice(0,index)).length===index`); otherwise fall back to the head window `content.slice(0,180)`. Never fabricated or mis-centered. |
| BR-FOLD-4 | Korean-safe: Hangul is caseless (`toLowerCase` no-op); NFC is lossless canonical unification. No query term is dropped or transformed; handling deterministic across environments. |

## BR-LINK — backlink query (REQ-016)

| ID | Rule |
|---|---|
| BR-LINK-1 | `list_backlinks` is read-only, registered outside the `if (!config.readOnly)` block with `readOnlyHint:true`. It computes `byPath`/`lookup`/edges **transiently** within one call from a bounded scan (no persistent index/watcher/graph); paginates `{offset,limit,nextOffset}`; marks `untrusted:true`. |
| BR-LINK-2 | The target `path` is resolved through the **same path policy as `read_note`** via `vault.read` (`portableRelative` + `Vault.target`: rejects traversal/symlinks/hardlinks/hidden/control paths; `PATH_COLLISION` on case/NFC variance; `NOTE_TOO_LARGE`). `targetSha256` comes from that read. |
| BR-LINK-3 | Link TEXT resolves only against already-listed note paths (`key()=NFC+lowercase`); it is **never** used for filesystem access and a link is never followed to read outside the Vault. |
| BR-LINK-4 | Only a documented wikilink subset is interpreted, using `audit_vault`'s resolver **refactored into one shared exported function** so the two tools cannot diverge; un-handled Obsidian constructs disclosed in `limitations[]`. The §8 "complete Obsidian link interpretation" exclusion is unchanged. |
| BR-LINK-5 | Ambiguous namesakes (`candidates.length>1`) are **reported** (`ambiguous:true` + sorted `candidates[]`), never auto-chosen; ambiguous edges dropped only when `includeAmbiguous=false`. |
| BR-LINK-6 | Deterministic ordering: edges sorted by `[sourcePath, rawLinkText]` with candidate arrays sorted (code-unit comparator); per-source dedup keeps the **first** occurrence (lowest index) so the retained excerpt is deterministic. No `Date.now()`/`Math.random()`. |
| BR-LINK-7 | Every backlink source and the target carry the full-file SHA-256 so downstream edits are conflict-checkable (matching `read_note`/`search_notes`). |

## BR-OUTLINE — outline-first reads (REQ-017)

| ID | Rule |
|---|---|
| BR-OUTLINE-1 | `outline_note` is read-only (`readOnlyHint:true`, outside the readOnly block). It reads ONE note via `vault.read` bounded by `config.maxNoteBytes` (`NOTE_TOO_LARGE`); **no full-vault scan** (`SCAN_LIMIT` N/A). Heading list paginates; `reply()` refuses over-budget; `untrusted:true`. |
| BR-OUTLINE-2 | Code-fence awareness comes from the shared `visibleMarkdown` blanking. The ATX+setext **matching** regex is new (CommonMark-closer, matches empty-title ATX) and is intentionally NOT identical to `audit_vault`'s heading detector — the two tools' heading sets need not agree. Only the fence/comment/inline-code blanking is shared. |
| BR-OUTLINE-3 | Offsets (`start`, `contentStart`, `end`) are UTF-16 code-unit indices into the FULL current file, identical to `read_note`'s slice basis. `end` = offset of the next heading with `level <= this level`, else `content.length`; sections nest their subsections. |
| BR-OUTLINE-4 | Heading `title` is a codepoint-faithful raw body slice (no folding; trailing ATX `#`s and `\r` trimmed), capped at a documented max length with a `titleTruncated` boolean; full faithful text always readable via `read_note` over `[start, contentStart)`. Deterministic (document order, distinct offsets). |

## BR-VISIBLE — shared foundational rule (REQ-016 + REQ-017)

| ID | Rule |
|---|---|
| BR-VISIBLE-1 | `visibleMarkdown` masking MUST be **length-preserving in UTF-16 code units** (each masked non-BMP/astral char maps to an equal-length run), so any visible-derived index maps exactly to `body`/`content` offsets. Fixes a latent defect at `src/notes.ts:164/167/170/184` (`/[^\r\n]/gu, ' '` collapses astral chars). **Behavior-neutral for `audit_vault`** (it consumes strings, not offsets) — pinned by a byte-identical `auditNotes()` regression test on astral-containing notes; prerequisite for correct `list_backlinks` excerpts and `outline_note` offsets. |

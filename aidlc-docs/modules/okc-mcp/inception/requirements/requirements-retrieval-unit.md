# Requirements delta — `okc-mcp-retrieval-unit` (read-only discovery/retrieval aids)

**Status**: Requirements Analysis output for a **new follow-up Unit** — **awaiting explicit user approval**. This document EXTENDS the approved [`requirements.md`](requirements.md) (it does not supersede it): it adds REQ-014..017, decision log D14..D20, and clarifying notes to §7/§8. The **product / Construction gate remains in force**: approving this document approves the requirements delta only, not implementation, testing, or packaging.

**Origin**: User request (2026-09-09) — "다른 mcp를 바탕으로 okc-mcp에 로직 추가해줘", scoping the four capabilities compared in [`existing-mcp-research.md`](../existing-mcp-research.md) (MarkusPfundstein / cyanheads / MCPVault / lstpsche). User chose all four and directed "요구사항/설계 먼저" (requirements/design before code). Produced via an ultracode design + 2-lens adversarial-verification + synthesis + completeness-critic workflow (14 agents; see [audit](../../audit.md)).

---

## 1. Intent

Add four **read-only** discovery/retrieval aids adapted from other Obsidian MCPs, without changing okc-mcp's identity (search/discovery is a *minimal aid*, not the value proposition) and without adopting a persistent index, watcher, or embeddings:

1. **BM25 lexical ordering** (from MCPVault) — opt-in relevance reordering of the *same* literal matches.
2. **Case/Unicode folding** (from all REST/index MCPs) — opt-in NFC + case-insensitive matching.
3. **Backlink query** (from lstpsche's link/backlink metadata) — a new `list_backlinks` tool.
4. **Outline-first reads** (from cyanheads/MCPVault `document-map`) — a new `outline_note` tool. *Note: this was already proposed for okc-mcp in existing-mcp-research.md §5 but never built — it is the most in-scope of the four.*

All four are computed **per-call by a bounded scan**, register **outside** the `if (!config.readOnly)` block (available under `readOnly` mode), mark results `untrusted:true`, reuse the existing path policy (`portableRelative` + `Vault.target`), honor `maxScanBytes`/`RESPONSE_LIMIT`, stay Korean/mixed-text safe and deterministic (no `Date.now`/`Math.random`), and return full-file SHA-256 for conflict-checking.

---

## 2. Decision Log (continues after D13)

| Ref | Decision | Rationale |
|---|---|---|
| D14 | Treat the four capabilities as a **new read-only follow-up Unit** `okc-mcp-retrieval-unit` (6th entry in §7), distinct from the approved first Unit; each read-only, no persistent index/watcher/embeddings, available under `readOnly`. | Three of four are new scope vs REQ-001..013 and touch documented promises, so they need their own Unit and REQ IDs rather than being retrofitted into REQ-006/REQ-013. |
| D15 | **BM25 is opt-in** via a `rank` enum on `search_notes` (`'path'` default, `'bm25'`), **not** a new tool and **not** a default change. `ranking`/`score` fields emitted **only** when `rank='bm25'` (default response byte-identical). Terms deduped; tiebreak uses `vault.list()`'s UTF-16 code-unit comparator; whitespace-only query falls back to `'path'`. | Both adversarial lenses flagged the original design silently adding a `ranking` field to the default response. Gating keeps "search is a minimal aid" literally true and preserves the regression baseline. |
| D16 | **Folding is opt-in** via a `fold` boolean on `search_notes` (default `false`), `foldText = NFC + locale-independent toLowerCase`. NFKC, accent-strip, and separable knobs excluded. `fold` echo appears only when `true`. Excerpt centering guards on **prefix-length preservation**, not total-length equality. | NFKC would alter apparent Korean; `toLocaleLowerCase` risks Turkish-İ nondeterminism. The invariant lens proved total-length guarding is unsound (compensating grow/shrink). |
| D17 | **Backlinks → new REQ-016**, NOT REQ-013. `list_backlinks` reuses `audit_vault`'s resolver verbatim over a **documented wikilink subset**; the §8 "complete Obsidian link interpretation" exclusion stays intact; namesake ambiguity reported, never auto-chosen; target resolved via `vault.read` (full path policy, `PATH_COLLISION`, `NOTE_TOO_LARGE`, `targetSha256`). | REQ-013 + BR-STRUCT-4 deliberately keep the **write** path link-graph-free; a read-only inbound **query** is genuinely new and must not be smuggled under REQ-013. |
| D18 | **Outline includes ATX + setext** with an honest heuristic disclaimer; returns char **offsets** (`start`/`contentStart`/`end`) rather than heading-name selection, so duplicate headings need no resolution. `end` = offset of next same-or-higher-level heading (else EOF). Section reads compose with `read_note`. Titles capped with `titleTruncated`. | Offsets avoid the auto-choose problem and reuse `read_note`'s offset+SHA contract. Setext is the highest-risk heuristic (see open questions). |
| D19 | **Shared foundational fix (BR-VISIBLE-1)**: make `visibleMarkdown` masking **length-preserving in UTF-16 code units** (`src/notes.ts:164/167/170/184`). Prerequisite for BOTH `list_backlinks` excerpts and `outline_note` offsets. | Adversarial verification **empirically found a latent defect**: the current `/[^\r\n]/gu, ' '` collapses each non-BMP char (emoji, CJK Ext-B) to one space, so `visible` is shorter than `body` and visible-derived indices mis-map. Behavior-neutral for `audit_vault` (it consumes strings, not offsets). |
| D20 | **Honesty gate**: all documented-promise edits (the `search_notes` description, `construction/design.md`, §8 clarifying notes, new tool descriptions, REQ-014..017/BR-*) are ratified at the Requirements approval gate and MUST land **atomically (same commit)** with code. A behavior test asserts the shipped `search_notes` description no longer contains the stale "No regex, folding" clause (assertion conditional on which of REQ-014/015 ships). | The honesty stance is the product differentiator; promise-narrowing must be an explicit approved decision, not an embedded assertion. |
| D21 | **Scope resolution (2026-09-09 user go/no-go)**: implemented **REQ-015 `fold` + REQ-017 `outline_note` (ATX-only) + BR-VISIBLE-1**, then **REQ-016 `list_backlinks` IMPLEMENTED**; **REQ-014 BM25 DROPPED** (removed from active scope). | BM25 is untraced to any §6 success criterion / §7 follow-up and its IDF is muted (whole-query-contiguous candidate rule) → degrades to occurrence-density sort. Backlinks is a genuinely useful read-only inbound query staying within the §8 subset. Outline+fold were the recommended safe first cut. Setext + combined rank+fold remain out (the latter moot once BM25 is dropped). |

---

## 3. Functional Requirements (continue after REQ-013)

| ID | Requirement | Acceptance criterion (summary) |
|---|---|---|
| REQ-014 | **[DROPPED 2026-09-09 — see D21]** ~~Optional lexical ORDERING for `search_notes`~~ — `rank` enum (`'path'` default \| `'bm25'`). `bm25` reorders the *same* literal matches by a per-call BM25 TF/DF score; opt-in, no index, reorder-only (never adds/removes a match); lexical/statistical, **not** semantic. | `rank='path'` byte-identical to today. `rank='bm25'` returns the same path **set** (a permutation), scores finite ≥0, order = rounded-score(1e-6) DESC then path ASC (same comparator as `vault.list()`), terms deduped, whitespace-only → `path` fallback with label. df/avgdl/N derived within the single bounded scan; SCAN_LIMIT/RESPONSE_LIMIT/pagination unchanged; `ranking`/`score` only when `bm25`; available under `readOnly`. |
| REQ-015 | **[IMPLEMENTED 2026-09-09]** Optional case+Unicode FOLDING for `search_notes` — `fold` boolean (default `false`). `true` applies NFC + locale-independent case-insensitive matching. NFKC/accent-strip/regex/stemming/ranking excluded; matching stays literal. | `fold=false` byte-identical to today. `fold=true` matches ASCII case-insensitively and unifies composed/decomposed Hangul; never corrupts/drops Hangul; `foldText = normalize('NFC').toLowerCase()` (never `toLocaleLowerCase`/NFKC). Excerpts stay verbatim slices of ORIGINAL bytes (prefix-preservation-guarded centering, else head window). Documented as **NOT a strict superset** of the literal default (combining-mark/sub-syllable substrings). Echoes `fold` only when `true`; available under `readOnly`. |
| REQ-016 | **[IMPLEMENTED 2026-09-09]** New read-only tool `list_backlinks` — given a target note path, returns every note whose `[[wikilink]]` (documented subset) resolves to it, computed per-call by reusing `audit_vault`'s forward-link resolver in reverse. Reports namesake ambiguity, never auto-chooses; retains the §8 exclusion. | Target validated via `vault.read` (same policy as `read_note`: traversal/symlink/hardlink/hidden rejected; `PATH_COLLISION`; `NOTE_TOO_LARGE`; `targetSha256`). Link TEXT resolved only against listed paths (`key()=NFC+lowercase`), never used for FS access, never followed outside the Vault. Ambiguous → `ambiguous:true` + sorted `candidates[]` (dropped only when `includeAmbiguous=false`). Deterministic ordering; paginated; `untrusted:true`; source+target SHA-256; `limitations[]` discloses un-handled forms; SCAN_LIMIT/RESPONSE_LIMIT enforced; available under `readOnly`. |
| REQ-017 | **[IMPLEMENTED 2026-09-09 — ATX-only; setext deferred]** New read-only tool `outline_note` — bounded, code-fence-aware heuristic heading map (ATX + setext) for one note: each heading's level, kind, untrusted title, and UTF-16 offsets (`start`/`contentStart`/`end`) into the current file, plus full-file SHA-256, so a section reads by composing with `read_note`. Realizes the §5-proposed outline-first reads. | `slice(start,…)` begins the heading line; `slice(contentStart,end)` is the section body (subsections nest); offsets in `[0, content.length]` on the same UTF-16 basis as `read_note`. `#` inside fenced code / inline code / HTML comment NOT reported; astral chars don't shift offsets (BR-VISIBLE-1); Korean titles codepoint-faithful; `end` = next same-or-higher-level heading else EOF; title capped with `titleTruncated` (full text via `read_note` over `[start, contentStart)`). Reads ONE note bounded by `maxNoteBytes` (no full-vault scan). Heuristic, not a full CommonMark/Obsidian parser; available under `readOnly`. |

**Traceability note (critic finding):** REQ-014/REQ-015 extend the `search_notes` tool that **REQ-006** defines (literal, codepoint-faithful, path-ordered, no folding). This delta keeps REQ-006 intact and layers opt-in modifiers on top; REQ-006's acceptance text should carry a cross-reference "→ optionally extended by REQ-014/REQ-015" so it does not read as a contradiction.

---

## 4. Exclusions & §7/§8 updates (nothing weakened)

- **§8 "Semantic search" — RETAINED.** Clarify: BM25 ordering (REQ-014) and NFC+case fold (REQ-015) are statistical/normalization, explicitly distinct from the still-excluded semantic/embedding search.
- **§8 "complete Obsidian link interpretation" — RETAINED.** Clarify: `list_backlinks` interprets only a documented subset (`[[note]]`, `[[note|alias]]`, `[[note#heading]]`/`[[note#^block]]`, `[[folder/note]]`, ±`.md`, `![[embed]]`) and discloses un-handled forms (Markdown `[]()` links, Canvas/Base, shortest-path auto-resolution, plugin/transclusion) in `limitations[]`.
- **§7 Follow-up Units — ADD a 6th entry**: read-only discovery/retrieval aids (this Unit).

---

## 5. Honesty stance updates (exact wording ratified at D20 gate, land atomically with code)

- **`search_notes` description** (`src/server.ts:78`) — replace the "No regex, folding, or semantic ranking" string with wording that (a) states the codepoint-faithful case-sensitive default, (b) describes `fold=true` as NFC + locale-independent case-insensitive and **not a strict superset**, (c) describes `rank='bm25'` as per-call lexical TF/DF reorder that never adds/removes a match, **not semantic**. If only one of REQ-014/015 is approved, ship only that clause.
- **`construction/design.md:31`** — extend the "bounded linear literal search" note with the opt-in `bm25`/`fold` modes and their honest limits.
- **`construction/design.md` "Explicit limitations"** — append the `list_backlinks` subset + `outline_note` heuristic limits.
- **New tool descriptions** for `list_backlinks` and `outline_note` (self-disclosing subset/heuristic; "not a search engine / not a complete Obsidian link interpreter").

---

## 6. Cross-Unit modification disclosure (critic finding — needs explicit user acceptance)

This Unit is **not purely additive**. It edits **already-shipped first-Unit code**:
1. `visibleMarkdown` masking length-preservation (`src/notes.ts:164/167/170/184`) — the BR-VISIBLE-1 latent-bug fix.
2. Extraction of `audit_vault`'s wikilink resolver out of `auditNotes` into a shared exported function (so the two tools cannot diverge).

Both are guarded by a **byte-identical `audit_vault`-output regression test** on astral-containing notes, but approving this Unit means accepting edits to shipped/approved audit internals with regression risk.

---

## 7. Open questions for the user (must resolve before code)

> **RESOLVED 2026-09-09 (user decisions):** #1 BM25 → **DROPPED** (D21). #2 rank+fold composition → **moot** (BM25 dropped). #3 folding → **opt-in `fold=false`, NFC-only, single boolean** (implemented). #4 backlinks → **IMPLEMENTED** with the documented subset (D21). #5 outline setext → **ATX-only shipped, setext deferred**. #6 cross-Unit edits → **accepted** (audit resolver extracted + visibleMarkdown fixed, guarded by tests). #7 `outline_note` title cap → **300 codepoints**; `list_backlinks` `includeAmbiguous` → **kept (default true)**; BM25 `score` exposure → **N/A (dropped)**. The list below is retained for historical context.

1. **BM25 (REQ-014) product go/no-go** — it is **untraced** to any of the five §6 success criteria or the enumerated §7 follow-ups, and because a candidate requires the whole query as a contiguous substring, its IDF is muted (degrades toward occurrence-density sort). Critic recommends **defer-by-default**. Ship opt-in / defer / drop?
2. **rank + fold composition** — both params live on `search_notes` (a 2×2 matrix). The combined `rank='bm25'` + `fold=true` semantics are **unspecified** (candidate set folded vs literal? term counting on folded vs original text?). Decide the spec, or **ship only one modifier** this Unit.
3. **Folding default & normalization** — confirm opt-in (`fold=false` default) + NFC-only + single boolean (NFKC/accent-strip excluded).
4. **Backlinks (REQ-016) go/no-go + subset** — confirm the read-only inbound query is wanted (subset stays within the §8 exclusion) and ratify the exact wikilink subset.
5. **Outline setext (REQ-017)** — keep setext with disclaimer, or **ATX-only first** (safer; a hand-authored `---` thematic break can misread as H2) and defer setext?
6. **Cross-Unit edits (§6)** — accept modifying shipped first-Unit code (guarded by a regression test)?
7. **Concrete params** — `outline_note` title cap value (e.g. align with the 300-char note-title max); expose BM25 numeric `score` or ordering-only; `list_backlinks` `includeAmbiguous` in MVP or defer.

---

## 8. MVP scope verdict

**Right-sized and strictly read-only**, proceed to functional-design finalization AFTER the Requirements gate + the BM25/backlinks go/no-go. All four reuse existing machinery (bounded scan / `vault.read` / `reply()` / `page` schema / `readAnnotations`) and honor every hard invariant.

- **Defer** (not this Unit): BM25F fielded weights, configurable `k1`/`b`/threshold, flipping the default to `bm25`, MCPVault's 5/20 pagination, NFKC/accent-strip/separable fold knobs, a `read_section` tool, heading-name/anchor selection, `direction:'in'|'out'|'both'`, cross-note graph/orphan/most-linked analytics, table-of-contents.
- **Reject outright** (never): per-term OR / fuzzy / folded candidate expansion in BM25 (violates "never invent matches"); any persistent index/watcher/embeddings.
- **Safest minimal cut** if scope pressure: **outline-first (ATX-only) → folding → (BM25, backlinks contingent on go/no-go).**

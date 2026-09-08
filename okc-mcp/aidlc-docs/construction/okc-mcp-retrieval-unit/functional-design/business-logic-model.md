# Business Logic Model — `okc-mcp-retrieval-unit`

**Status**: Functional Design (Construction) output — **awaiting explicit user approval**. Baseline: [`requirements-retrieval-unit.md`](../../../inception/requirements/requirements-retrieval-unit.md) + [`business-rules.md`](business-rules.md). Describes tool surface, algorithm, edge cases, and test strategy per capability. **No `src/` code written** — this is the design to implement after approval + the §7 open-question decisions.

---

## 0. Shared foundational fix (BR-VISIBLE-1) — implement first

`visibleMarkdown` (`src/notes.ts:164/167/170/184`) currently masks fenced code / inline code / HTML comments with `.replace(/[^\r\n]/gu, ' ')`. The `u` flag makes each **astral** char (emoji, CJK Ext-B) one match → replaced by **one** space → `visible.length < body.length`. Any visible-derived index then mis-maps into the raw body (wrong excerpts; can leak masked code bytes; wrong outline offsets).

**Fix**: length-preserving mask — e.g. `m => ' '.repeat(m.length)` (or drop the `u` flag). Behavior-neutral for `audit_vault` (its regexes only care that masked regions contain no heading/link/escape characters, not the space count). **Guard**: a regression test asserting `auditNotes()` findings are byte-identical before/after on notes with emoji / CJK Ext-B inside fences, inline-code spans, and HTML comments; plus a direct property test `visibleMarkdown(body).length === body.length` for all bodies.

---

## 1. BM25 lexical ordering (REQ-014, BR-RANK-*)

**Tool surface** — param on the EXISTING `search_notes` (no new tool). Appended to `{query, ...page}`: `rank: z.enum(['path','bm25']).default('path')`. Result when `rank='bm25'`: `{ matches:[{path, sha256, excerpt, score}], total, nextOffset, ranking:'bm25', untrusted:true }`. When `rank='path'` (default): **unchanged** — `{ matches:[{path, sha256, excerpt}], total, nextOffset, untrusted:true }` (no `ranking`/`score`). Suggested layout: a **pure** exported `bm25Rank(candidates, terms, {N, avgdl, df})` (new `src/search.ts`) for PBT.

**Algorithm** — single bounded scan, same cost model as today:
1. `terms = Array.from(new Set(query.split(/\s+/u).filter(t => t.length>0)))`. If `terms.length===0` → fall back to `rank='path'` and set the response label to `'path'`.
2. Per scanned note (path-ascending from `vault.list()`): `signal.throwIfAborted()`; `read`; accumulate original `Buffer.byteLength` → `SCAN_LIMIT` on overflow; `docText = path + '\n' + content`; `docLen` = codepoint count; count **non-overlapping** codepoint-faithful `indexOf` occurrences per term to build `df`/`tf`.
3. Candidate iff today's rule (`content.indexOf(query)>=0 || path.includes(query)`) — **match set unchanged**; non-candidates feed `df`/`avgdl` only.
4. `avgdl = N>0 ? corpusLenSum/N : 0`; `score = Σ_t idf(t)·(tf·(k1+1))/(tf + k1·(1 - b + b·docLen/avgdl))`, `idf(t)=log(1 + (N-df+0.5)/(df+0.5))` (≥0, no div-by-zero). `roundedScore=round(score*1e6)/1e6`.
5. Sort by `roundedScore` DESC then path ASC (code-unit comparator). Paginate. No `Date.now`/`Math.random`. **Cost is O(scanned-bytes × term-count)**, not O(1).

**Edge cases**: empty candidate set; whitespace-only query → path fallback; single-term/space-less Korean → IDF constant, order by TF saturation + length norm; path-only match → `excerpt=content.slice(0,180)`; overlapping occurrences counted non-overlapping; all-tie → path ASC; cross-platform `Math.log` near-ties → 1e-6 rounding + path tiebreak; duplicate terms deduped; corpus over budget → `SCAN_LIMIT` before ranking.

**Tests**: PBT (pure `bm25Rank`, seeded rng): permutation invariance; reorder-only equivalence (bm25 path-set == path-mode path-set); determinism; tie-break path-ascending; monotonic TF; length normalization; Korean-safety (finite ≥0). Behavior (real stdio): existing literal-Korean regression byte-compatible under default; `bm25` total == `path` total; two identical `bm25` calls identical; `readOnly` session accepts `rank='bm25'`; `SCAN_LIMIT`/`RESPONSE_LIMIT` still fire.

> **⚠️ Scope note (critic):** BM25 is **untraced** to any §6 success criterion or §7 follow-up; because a candidate requires the whole query as a **contiguous** substring, cross-candidate IDF is muted → degrades toward an occurrence-density sort. Recommended **defer-by-default** pending product go/no-go (§7 open question 1).

---

## 2. Case/Unicode folding (REQ-015, BR-FOLD-*)

**Tool surface** — param on `search_notes`: `fold: z.boolean().default(false)`. Top-level `fold:true` echoed only when `true` (default response byte-identical). Pure exported `foldText(s)=s.normalize('NFC').toLowerCase()`.

**Algorithm**: `needle = fold ? foldText(query) : query`. Per note (path-sorted): `throwIfAborted`; read; accumulate **original** `Buffer.byteLength` → `SCAN_LIMIT`; `hay = fold?foldText(content):content`; `hp = fold?foldText(path):path` (transient, discarded each iteration). `index=hay.indexOf(needle)`; match iff `index>=0 || hp.includes(needle)`. Excerpt = verbatim slice of ORIGINAL content — center on `index` **only** when `foldText(content.slice(0,index)).length===index`; else `content.slice(0,180)`. Locale-independent `toLowerCase`, no NFKC, no `Date.now`/`Math.random`.

**Edge cases**: Turkish-İ → MUST use `toLowerCase` (`'I'.toLowerCase()==='i'` deterministic); length-changing fold (`İ→i̇`) → prefix-preservation guard falls back; compensating grow/shrink → guard falls back; composed vs decomposed Korean unified under fold; leading-jamo substring inside decomposed Hangul can DROP a match (not a strict superset — disclosed); path-only match under fold → head window; large note → transient folded copy bounded by `maxNoteBytes`, released each iteration; `fold=false` → byte-identical to today.

**Tests**: PBT (pure `foldText`): idempotence; Hangul preservation; NFC unification; ASCII case-insensitivity; match-broadening **scoped to NFC-normalized corpora** (not unqualified monotonicity). Behavior: `fold=false` 'HTTP'≠'http'; `fold=true` both ways; composed==decomposed Korean; echoes `fold` only when true; notes byte-unchanged after search; `SCAN_LIMIT` under fold; excerpt is a verbatim substring for the prefix-preserving case; assert shipped description no longer says "No regex, folding".

---

## 3. `list_backlinks` (REQ-016, BR-LINK-*)

**Tool surface** — NEW read-only tool, alongside `audit_vault`, outside the readOnly block with `readAnnotations`. `inputSchema: { path: notePath, ...page, includeAmbiguous: z.boolean().default(true) }`. Result: `{ target, targetSha256, backlinks:[{path, sha256, linkText, fragment, embed, ambiguous, candidates:[], excerpt}], total, nextOffset, untrusted:true, limitations:[] }`. New pure exported fns `wikiLinkOccurrences(visible)` and `resolveWikiLink(sourcePath, rawTarget, {byPath, lookup, allFiles})` refactored **verbatim** out of `auditNotes`.

**Algorithm**: (1) resolve target via `vault.read(path)` (enforces path policy: `PATH_COLLISION`/`NOTE_TOO_LARGE`/symlink/traversal) → `targetSha256`, `targetKey=key(path)`. (2) bounded scan (`audit_vault` pattern) accumulating `Buffer.byteLength` → `SCAN_LIMIT`; collect `{path, content, sha256}`. (3) build `byPath`/`lookup`/`allFiles` (transient). (4) per source, extract occurrences via `wikiLinkOccurrences(visibleMarkdown(body))`; dedup per source by raw link text keeping the **first** occurrence. (5) `resolveWikiLink` each; keep an edge iff `resolved|ambiguous` AND candidates contain a path with `key===targetKey`; `ambiguous=candidates.length>1`; drop ambiguous when `includeAmbiguous=false`. (6) sort by `[sourcePath, rawLinkText]`, candidates sorted. (7) excerpt = `body.slice(max(0,index-60), +180)` — **relies on BR-VISIBLE-1**. Paginate; no `Date.now`/`Math.random`.

**Edge cases**: zero backlinks → `[]` (not an error); target not a listed `.md` → `NOTE_NOT_FOUND`; oversize → `NOTE_TOO_LARGE`; wrong case/NFC target → `PATH_COLLISION` (via `vault.read`, not silently folded); namesake ambiguity reported; aliased/heading/embed/path-qualified forms counted by target part (embed flag + raw text preserved); links in fence/inline/HTML comment/escaped `\[[..]]` masked; self-reference reported; unsafe/URI/absolute/`..` targets → resolver `unsafe`; emoji/astral before a link → correct only after BR-VISIBLE-1; vault over budget → `SCAN_LIMIT`.

**Tests**: PBT (pure): `wikiLinkOccurrences` split/exclusion/embed-alias-fragment + idempotence; `resolveWikiLink` determinism, sorted+deduped candidates, unsafe never resolves, permutation-invariant; inverse (`backlinksOf`) property; Korean/NFC/case no-op for Hangul. Behavior: unique excerpt/sha256/embed/ambiguous; aliased/heading/embed/path-qualified counted; ambiguity + `includeAmbiguous=false`; zero-backlink; `NOTE_NOT_FOUND`/`NOTE_TOO_LARGE`/`PATH_COLLISION` targets; fence/inline/comment/escaped not counted; **emoji-in-fence-before-link excerpt CONTAINS the link** (astral regression); `SCAN_LIMIT`; pagination; `readOnly` session; refactor guard that `audit_vault` link findings are byte-identical before/after extracting the shared resolver.

> **Disclosure (critic):** `list_backlinks` inherits `audit_vault`'s caveat (`notes.ts:460`) that lookup uses JavaScript NFC + lowercasing, **not** the compiler's pinned full Unicode case folding — this must appear in `limitations[]`/description, not be silently narrowed.

---

## 4. `outline_note` (REQ-017, BR-OUTLINE-*)

**Tool surface** — NEW read-only tool (section reads reuse `read_note`), outside the readOnly block with `readAnnotations`. `Params: { path: notePath, ...page }`. Result: `{ path, sha256, totalCharacters, headings:[{level, kind:'atx'|'setext', title, titleTruncated, start, contentStart, end}], total, nextOffset, untrusted:true }`. New pure exported `outlineHeadings(content)` in `src/notes.ts` reusing `visibleMarkdown()` + `splitRaw()`.

**Algorithm**: `body=splitRaw(content).body`; `bodyStart=content.length-body.length`; `visible=visibleMarkdown(body)` (**must be length-preserving**, BR-VISIBLE-1). Iterate lines tracking offset. **ATX**: `/^ {0,3}(#{1,6})(?:[\t ]+(.*?))?[\t ]*$/` on the blanked line; title sliced from BODY at the same offsets, trimmed, trailing `#`s stripped. **Setext**: underline `/^ {0,3}(=+|-+)[\t ]*$/` whose preceding line is non-blank, non-ATX, non-underline → level 1(`=`)/2(`-`); title from BODY of that line. Document order (distinct offsets, no sort). `end` = start of next heading with `level<=this level` else `content.length`. Cap title on a codepoint-safe boundary, set `titleTruncated`. Paginate; no `Date.now`/`Math.random`.

**Edge cases**: no headings → `[]`; `# heading` inside fence/inline-code/HTML-comment excluded; inline code in a title still detected, backticks/Korean preserved; ATX closing `## Title ##` → trailing `#`s stripped; setext H1/H2, `---` after a non-blank paragraph is H2, after blank is NOT (thematic break) — **highest-risk heuristic, false positives disclosed + fixture-tested**; malformed/unterminated frontmatter is documented heuristic behavior; CRLF `\r` trimmed; BOM/frontmatter offset via `bodyStart`; astral chars before a heading → correct only after BR-VISIBLE-1; single heading line > `maxResponseBytes` → title capped + `titleTruncated`, full text via `read_note`; note over `maxNoteBytes` → `NOTE_TOO_LARGE`; 4-space indented code before column-0 `===` excluded only by the ≤3-space indent limit (disclosed).

**Tests**: PBT (pure `outlineHeadings`): offset/composition round-trip (`slice(start,..)` begins the line, `slice(contentStart,end)` is the body; offsets monotonic, in `[0,len]`); code-fence property; astral-char property (emoji fence before a heading doesn't shift offsets); determinism; Korean fidelity + trailing `#`/`\r` stripping; ATX+setext incl. CRLF; setext thematic-break false-positive fixture. Behavior: in the read-only surface + under `readOnly:true`; `sha256 == read_note` full-file digest; composing `read_note(offset=start,length=end-start)` returns the section; fenced-`#` excluded; Korean headings; pagination; over-budget many-headings → `RESPONSE_LIMIT` with healthy connection; path policy reused (symlink/out-of-vault → denied); note bytes unchanged; title cap + `titleTruncated`.

---

## 5. Unresolved design gaps (must be decided before code — critic findings)

1. **Combined `rank='bm25'` + `fold=true` is UNSPECIFIED.** Both params sit on `search_notes` → a 2×2 matrix. Decide: (a) is the candidate set the FOLDED match set or the literal set? (b) is BM25 term counting done on folded or original text? (c) are query terms folded before df/tf? BR-RANK-2's "same matches" is ambiguous once fold broadens the base set. **Either specify this + add 2×2 tests, or ship only one modifier this Unit.**
2. **BR-VISIBLE-1 audit-output regression test** must exist independently of the resolver-extraction guard (behavior-neutrality is asserted, not yet tested).
3. **`outline_note` title cap** has no concrete value — pick one (e.g. align with the 300-char note-title max) so `titleTruncated` and the "always outlinable" guarantee are testable.
4. **Expose BM25 numeric `score` vs ordering-only** — a raw score invites meaningless cross-query comparison.
5. **`list_backlinks` `includeAmbiguous`** — keep in MVP or defer (always report ambiguous with `ambiguous:true`).
6. **Partial-approval wording/tests** — if only a subset of REQ-014/015 ships, the `search_notes` description AND the "No regex, folding" honesty-gate test assertion must be trimmed to the shipped modes.

---

## 6. Implementation status (2026-09-09)

Per the user's chosen scope ("권장: outline + folding + 버그수정"), a **reduced subset was implemented via TDD and verified green** (`npm run check` exit 0 — typecheck PASS · tests 58/58 · build PASS):

| Item | Status | Notes |
|---|---|---|
| BR-VISIBLE-1 (`visibleMarkdown` fix) | ✅ Shipped | `src/notes.ts` — 4 masks `/[^\r\n]/gu` → `/[^\r\n]/g` (length-preserving); RED-tested by the astral-offset property. |
| REQ-015 `fold` | ✅ Shipped | `src/search.ts` `foldText`; `src/server.ts` `search_notes` `fold` param (default false, byte-identical default response, prefix-guarded excerpt); D20 description edit applied. |
| REQ-017 `outline_note` | ✅ Shipped (ATX-only) | `src/notes.ts` `outlineHeadings`; `src/server.ts` read-only `outline_note` tool; 300-codepoint title cap. |
| **Setext detection (part of D18)** | ⏸ **Deferred** | ATX-only shipped — the doc's own "safest minimal cut"; setext is the highest-risk heuristic (thematic-break false positives). Revisit on request. |
| REQ-014 BM25 (`rank`) | ❌ **Dropped (D21, 2026-09-09)** | Untraced to success criteria + muted IDF (whole-query-contiguous candidate rule) → occurrence-density sort. Removed from active scope; no `rank` param / `bm25Rank` shipped. BR-RANK-* dropped. |
| REQ-016 `list_backlinks` | ✅ **Shipped (2026-09-09)** | `wikiLinkOccurrences` + `resolveWikiLink` extracted from `auditNotes` (audit findings byte-preserved, guarded by existing tests); added `backlinksOf` + read-only `list_backlinks` tool (target via `vault.read`, bounded scan, ambiguity reported, `limitations[]`). |

The unresolved combined `rank`+`fold` semantics (§5.1) do not arise in this subset because BM25 was not shipped; they must be resolved before REQ-014 lands.

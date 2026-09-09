# Personas — Obsidian MCP for authoring an OKC input Vault (First Unit)

**Status**: User Stories output — awaiting explicit user approval. Derived from the approved [`../requirements/requirements.md`](../requirements/requirements.md) (REQ-001..013, SC1..SC5). Generated and adversarially verified via workflow (generate → per-persona INVEST/REQ/MVP-scope audit → synthesis → completeness critic → repair).

Three personas carry the first Unit. **OKC is an implicit downstream consumer**, not an interactive persona — it appears only as the "so that" benefit (clean input) in the author/reviewer stories.

---

## P1 — 신규 설치자 (New Installer)

A technically comfortable knowledge worker or developer who has just obtained the local npm tarball and its documentation and wants to stand up the Obsidian MCP against one of their existing Vaults. Works entirely offline on a single machine (Node/npm + local filesystem) — no Obsidian app plugin, no REST bridge, no API key, no OKC binary. Their whole interaction is a one-time, review-heavy setup: install, bind exactly one Vault, generate reviewable config outside the Vault, self-check, emit absolute-path client config, and confirm the tool surface is safe before trusting it with real notes.

**Context**: Local, single-process, single-Vault, stdio-only setup driven purely from documentation. The installer never sees OKC directly, but the Vault they bind is the future input OKC will ingest, so they care that setup keeps tool state out of the Vault and imposes no forced migration. Success is measured by SC1 (install from docs alone), trust reinforced by SC2.

**Goals**
- Install the MCP from the local tarball following documentation alone — no undocumented step, no network/app/API-key dependency.
- Bind exactly one existing Vault by absolute path without relocating folders or migrating frontmatter.
- Generate reviewable configuration and ready-to-paste absolute-path client config stored outside the Vault.
- Run a shallow local self-check and confirm the exposed tool surface has no dangerous capabilities before authoring.

**Frustrations**
- Setup tools that demand the Obsidian app, a REST plugin, an API key, or a cloud/network round-trip.
- Installers that silently migrate or relocate a user's existing notes.
- Config, backups, or templates leaking into the knowledge Vault and polluting it.
- Client configs with relative paths or unresolved variables that fail to connect on the first try.
- Not being able to tell whether the tool can run shell, delete, or reach files outside the Vault.

**Mapped stories**: US-IN-01, US-IN-02, US-IN-03, US-IN-04, US-IN-05, US-IN-06, US-IN-09, US-IN-07, US-IN-08 · (follow-up: US-IN-FU-01, US-IN-FU-02)

---

## P2 — 기존 Vault 저작자 (Existing-Vault Author) — PRIMARY

The **primary persona of the first Unit**: someone who already maintains a substantial, messy Obsidian Vault (often mixing Korean and English) and wants to make it the best possible **input** for OKC without reorganizing files. Their core journey is **in-note tidying** — run a read-only heuristic audit, then standardize frontmatter (title/aliases/tags), fix invalid YAML, and reinforce sources and links strictly within existing files, plus author the occasional new evidence-backed note. Every mutation is conflict-aware and backed up first. They never move, rename, or merge files in this Unit.

**Context**: Works against a single, real, pre-existing Vault they cannot afford to corrupt or restructure. They value that no operation moves/renames/merges files, that unknown frontmatter keys, body, and YAML comments survive edits, and that the audit is honestly heuristic (never claiming OKC compiler validation). Serves SC2, SC3, SC4 primarily. OKC is the implicit downstream consumer — link validity, duplicate detection, and structure noise are surfaced heuristically for the author to act on in place.

**Goals**
- Understand what makes their Vault a weak OKC input via a categorized, read-only heuristic audit.
- Standardize frontmatter, fix invalid YAML, and reinforce sources/links entirely within existing files.
- Add or update knowledge without losing sources, conflicting claims, or links — always via conflict-aware, backed-up writes.
- Locate notes to tidy through deterministic listing and literal search (including Korean), with reads that return a content hash.
- Author new evidence-backed notes in an OKC-ready shape when needed.

**Frustrations**
- Tools that force migration or reorganize the folder layout before they can be useful.
- Edits that silently drop unknown frontmatter keys, body content, or YAML comments.
- Quality reports that overclaim (e.g. asserting the notes will pass OKC compilation).
- Search that fails on Korean substrings, and writes that overwrite a concurrently changed note.
- Being pushed into file move/rename/merge when they only want to tidy content in place.

**Mapped stories**: US-AU-01, US-AU-02, US-AU-03, US-AU-04, US-AU-05, US-AU-06, US-AU-07 · (follow-up: US-AU-09, US-AU-10)

> **Onboarding note**: The author's one-time setup (install, self-check, client-config, safe tool-surface review) is **served by the shared atomic installer stories** US-IN-01, US-IN-04, US-IN-05, US-IN-06 — cross-mapped here rather than duplicated as a bundled author onboarding story. This keeps first-Unit stories INVEST-Small and Independent. (The former bundled `US-AU-08` was retired to a stub for this reason.)

---

## P3 — 변경 검토·복구자 (Change Reviewer / Recoverer)

A safety-and-integrity-focused persona who governs how changes land and how earlier content is recovered. Before editing they read a note to capture a deterministic content-hash baseline; they rely on `expectedHash` to block writes when a file changed underneath them; they depend on a single automatic pre-change backup outside the Vault before any mutation; and when an edit goes wrong they perform a documented manual restore. They also enforce the structure-preserving, malformed-YAML-rejecting guardrails on partial updates. This may be the same human as the author wearing a review/recovery hat, or a distinct steward.

**Context**: Operates on a single local Vault with manual, generation-free recovery (RPO = last save point, RTO = manual). No automatic rollback or generational retention exists by design; restore is re-authored through the same conflict-aware, backup-generating write path as any other edit. Serves SC3 and SC5. OKC is the implicit downstream consumer — the reviewer safeguards the integrity and recoverability of the content that will be handed to OKC, and holds no OKC review-approval authority in this Unit.

**Goals**
- Capture a known content-hash baseline on every read to review against and reuse as `expectedHash`.
- Block updates whenever the supplied `expectedHash` no longer matches the on-disk file, surfacing a clear conflict.
- Guarantee a single pre-change backup outside the Vault before any mutation, and none on a rejected write.
- Manually recover exact prior content from that external backup when an edit went wrong.
- Ensure partial updates reject malformed YAML and preserve unknown keys, body, and comments verbatim.

**Frustrations**
- Silent overwrites of a note that changed since it was last read.
- Writes that proceed with no baseline or with a stale hash and no conflict signal.
- Backups that are misleadingly left behind on rejected mutations, or that can't be traced to a source note.
- Structure-destroying edits that corrupt YAML or drop content they never intended to touch.
- Expecting an automatic rollback/generational restore that the tool intentionally does not provide.

**Mapped stories**: US-RV-01, US-RV-02, US-RV-03, US-RV-04, US-RV-05 · (follow-up: US-RV-07)

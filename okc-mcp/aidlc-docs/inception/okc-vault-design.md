# Obsidian source Vault design for effective OKC input

Status: Inception design proposal. Research date: 2026-09-06. The source reviewed was [dolgogae/ObsidianKnowlegeComplication](https://github.com/dolgogae/ObsidianKnowlegeComplication/tree/7f87f7c81a72701710d8668b2569e51478dccc75) at commit `7f87f7c81a72701710d8668b2569e51478dccc75`. Authoring conventions and MCP capabilities proposed in this document are not claims that those features already exist in OKC.

## 1. Product role

The MCP serves people who collect and refine knowledge in Obsidian and later integrate multiple Vaults through OKC. The core journey is **author good source notes → organize links and evidence → audit before integration**. Project status queries, compilation of an approved plan, and validation of an output Vault are later responsibilities owned by OKC, not the center of this MCP's MVP.

The recommended implementation is therefore a TypeScript Obsidian-authoring MCP over local stdio. Direct filesystem access makes it independent of whether Obsidian is running. It edits a live Vault within boundaries explicitly granted by the user. A snapshot handed to OKC is treated as a separate immutable input. Distinguish routine editing through an authoring tool from `REQ-SNP-001`, which prohibits the compiler from changing a source Vault.

OKC's Node bindings expose project, integration, approval, and compilation APIs, not note-authoring APIs such as `list`, `read`, `write`, `search`, and `lint`. Requiring the native addon would add installation work unrelated to the requested authoring experience. If actual corpus validation is needed later, add an optional compatibility checker using the public Rust `CorpusBuilder::build`. Passing a JavaScript lint must not be presented as passing the core check.

Evidence: [product scope and requirements](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/specs/product-and-scope.md), [public SDK](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/specs/public-sdk-and-cli.md), [Node API declarations](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/bindings/node/index.d.ts), and [CorpusBuilder](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/crates/okc-core/src/corpus.rs).

## 2. What the current OKC implementation actually reads

| Observation | Implication for the authoring MCP | Local evidence |
|---|---|---|
| Markdown must be strict UTF-8 and frontmatter must be a YAML mapping representable as JSON. Duplicate YAML keys fail. | Write new files as UTF-8/LF; verify mapping structure, duplicate keys, and types. Do not needlessly reformat an existing document. | [parse.rs](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/crates/okc-core/src/parse.rs) `parse_markdown`, `parse_frontmatter` |
| Some malformed or non-mapping frontmatter becomes an opaque warning and is omitted from typed metadata. | Treat it as an authoring-lint error. Replacing it automatically with empty metadata could lose provenance. | [parse.rs](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/crates/okc-core/src/parse.rs) `opaque_frontmatter`; [corpus.rs](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/crates/okc-core/src/corpus.rs) `prepare_from_plan` |
| A string `title` and string or string-array `aliases` and `tags` are extracted specially. Non-string array members are excluded from lookup. | Recommend string arrays for aliases and tags. Title precedence is frontmatter title → first heading → filename. | [parse.rs](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/crates/okc-core/src/parse.rs) `string_list_field`, `parse_markdown` |
| Other metadata is preserved as typed values in the corpus. Each element of a top-level array receives a separate metadata disposition. | Keep authoring-convention fields optional. Every added field creates more knowledge that requires review. | [corpus.rs](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/crates/okc-core/src/corpus.rs), `REQ-INT-002` |
| Every Markdown document belongs to exactly one taxonomy cluster. Even a singleton requires synthesis, critic, and approval. | Do not generate large numbers of empty templates, operational READMEs, or generated indexes in source-knowledge folders. A folder name or `status: draft` does not imply exclusion. | [Product scope](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/specs/product-and-scope.md), `REQ-INT-001/004` |
| `.obsidian/**`, `.git/**`, and secret-file types are excluded, but Vault, parent, and global ignore files do not shape input. | Never claim that `.gitignore` excludes a template or draft from OKC. Keep templates and MCP work data outside the Vault. | [snapshot.rs](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/crates/okc-core/src/snapshot.rs) `collect_directory`, `is_excluded`; [security specification](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/specs/security-and-trust-boundaries.md) |
| The current block scanner splits ATX headings, blank-line-delimited paragraphs, and fenced code into evidence blocks. | Use clear paragraph boundaries and headings such as `## Claim` and `## Evidence`. Do not pack many independent claims into one block. | [parse.rs](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/crates/okc-core/src/parse.rs) `scan_blocks` |
| Markdown links, wikilinks, embeds, headings, and block suffixes are parsed. `[[...]]` inside a code span or fence is not a link. | Link and rename operations must understand syntax and scope. Never use a document-wide regex replacement. | [ALG-NRM-001](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/algorithms/stable/markdown-canvas-and-link-normalization.md) |
| Lookup uses paths within the source, normalized paths, and filename, title, and alias candidates. It does not guess among multiple candidates. | Disambiguate identical titles with different meanings and use document-relative paths when needed. Never merge colliding aliases automatically. | [plan.rs](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/crates/okc-core/src/plan.rs) `resolve_document_candidates`, `fragment_matches` |
| A `^block-id` recognizes ASCII alphanumerics, `-`, and `_` in the last token. | Add concise explicit IDs only to evidence that needs anchors. Do not confuse them with the core's hash-based BlockId. | [parse.rs](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/crates/okc-core/src/parse.rs) `explicit_block_id` |
| Attachments, Canvas, and Base participate in input-safety handling, but the current output materializer is Markdown-only. | Describe attachments and key evidence in Markdown as well. Knowledge represented only in graphs or Canvas is not guaranteed in current output. | [Current state](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/CURRENT_STATE.md), `REQ-PAR-002/003`, `REQ-INT-006` |

## 3. Recommended Vault structure

The folder structure is a starting default, not a migration rule imposed on existing users. The MCP should read and lint existing structures such as PARA. OKC depends on safe source files, clear documents and blocks, evidence, and preservable metadata—not particular folder names.

```text
KnowledgeWorkspace/
├── PersonalVault/               # Source Vault normally edited in Obsidian
│   ├── .obsidian/               # User settings; not an MCP editing target
│   ├── inbox/                   # New notes and not-yet-organized real knowledge
│   ├── notes/                   # Concept, experience, and decision notes
│   │   └── architecture/
│   │       └── Immutable snapshots.md
│   ├── sources/                 # Source notes for publications, meetings, or observations
│   ├── maps/                    # A small number of human-authored topic guides
│   └── assets/                  # PDFs, images, and other originals; audit output limitations
├── authoring-config/            # MCP settings, authoring profiles, and templates outside the Vault
├── authoring-state/             # Rebuildable indexes, change backups, and audit reports
├── snapshots/                   # Immutable input produced by a later export capability
└── compilation/                 # Later OKC projects and output
```

There is no reason for authoring `notes/` to mirror OKC output `knowledge/`. Authoring categories can change, while OKC taxonomy is proposed and approved separately. If `inbox/` or `maps/` is inside the selected source, it is also subject to integration. Do not generate hundreds of MOC Markdown files from a plain file listing. Keep only guides that explain real context.

Treat snapshot export as a separate future capability. Prevent the authoring Vault and snapshot destination from sharing a root, and ensure live edits after export cannot change an existing snapshot. A recursive copy alone cannot guarantee a consistent instant when several files change concurrently. A future implementation needs hash validation or change detection with retry or failure and should publish only to a new destination. Do not traverse Vault symlinks or execute `.obsidian/plugins` or other binaries. `sourceId` identifies a Vault in a snapshot or project; it is not a field repeated in every note. Its actual syntax is `[A-Za-z0-9_.-]`, 1–128 bytes, excluding `.` and `..`.

Evidence: [snapshot algorithm](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/algorithms/stable/snapshot-identity-and-hashing.md), [source.rs](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/crates/okc-core/src/source.rs), `REQ-SNP-001/002`, `REQ-SRC-001/002`, and `REQ-APP-002`.

## 4. Note-authoring profile

Default to small metadata and a self-contained body. The fields `type`, `created`, `source`, and `license` below are **optional conventions of this MCP**; OKC neither requires them nor interprets them as approval. Never manually insert OKC fields such as `document_id`, `snapshot_id`, `content_hash`, or `approved`.

Example general note:

```markdown
---
title: Immutable snapshots
aliases:
  - Point-in-time source capture
tags:
  - knowledge/compilation
type: concept
---
# Immutable snapshots

Freezing source content at a point in time without changing the original makes it possible to identify the input behind an integration result.

## Evidence

The OKC source contract prevents the compiler from changing source bytes, permissions, or timestamps. Evidence: [[sources/OKC source contract#Source immutability]]. ^source-immutability

## Scope

Editing a note that is still being authored and preserving a snapshot already used for integration are separate operations.

## Open question

Passing a consistently captured Vault while edits are concurrent requires a separate export design.
```

A source note contains only facts received from the user or actually verified. Do not guess URLs, authors, publication dates, access dates, or licenses. Never copy personal data or tokens into a `source` field, and audit credential or sensitive query values in URLs.

```markdown
---
title: OKC source contract
type: source
source: https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/specs/product-and-scope.md
---
# OKC source contract

## Source immutability

REQ-SNP-001 prevents the compiler from changing bytes, permissions, or timestamps in a source Vault.

## Identification

Repository commit reviewed: 7f87f7c81a72701710d8668b2569e51478dccc75.
This authoring note summarizes requirements from the document; consult the original specification in that repository.
```

Operating principles:

- Distinguish facts, interpretations, hypotheses, and counterexamples in the body. Preserve the source and date for conflicting evidence. Do not use a number such as `confidence: 0.9` as objective truth or proof of completed review.
- Record dates as ISO strings when useful. Do not overwrite `updated` on every MCP read or search. Needless byte changes create new snapshots and review obligations.
- Check that an alias does not overlap the actual meaning of another document. Prefer a revealing title such as `HTTP response caching` over a generic `Caching`.
- Keep folder nesting shallow and filenames human-readable. Non-English names, including Korean, are allowed. Audit NFC and case-fold collisions, Windows reserved names, trailing spaces or dots, and unsafe separators.
- When a key claim exists only in a long PDF or image without Markdown, report an `attachment-only-evidence` warning. Do not automatically promote OCR or a summary to verified fact.
- Do not repeat every tag as a body `#tag` or fill every note with empty sections. Use only the conventions that add value.

Evidence: [canonical IR](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/specs/canonical-knowledge-ir.md), [evidence-complete integration](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/algorithms/stable/evidence-complete-integration.md), `REQ-CNF-001`, and `REQ-INT-002/003/005`.

There is a real link-path compatibility difference. The [official Obsidian internal-links documentation](https://obsidian.md/help/Linking%2Bnotes%2Band%2Bfiles/Internal%2Blinks) describes folder-qualified wikilinks as Vault-root-relative. The reviewed OKC resolver first checks a path relative to the current document and then considers filename, title, and alias candidates. A nested note's `[[sources/B]]` can therefore refer to different files under the two interpretations. The authoring MCP should check both and warn when they resolve differently. It must not report every valid Obsidian-root path as broken or rewrite user links wholesale. The examples above use Obsidian authoring syntax. For explicit block IDs, default to the shared subset of letters, digits, and hyphens.

## 5. Proposed authoring-MCP tool contract

The initial installation receives one fixed live-Vault root. Tools accept only relative paths and cannot expand source, output, or project locations arbitrarily through a call. Inputs and responses use strict schemas, size bounds, and structured errors. These are pre-implementation contract candidates; Construction records determine what was actually implemented.

| Tool | Key input | Behavior and result |
|---|---|---|
| `vault_status` | None | Logical name of registered root, profile/version, supported and limited capabilities, and bounded file count; hides sensitive absolute paths by default |
| `list_notes` | `folder?`, `cursor?`, `limit?` | Sorted note paths and titles plus the next cursor; no file content |
| `read_note` | `path`, `section?` | Bounded content and SHA-256 revision; identifies note content as untrusted material |
| `search_notes` | `query`, `limit?`, `cursor?` | Local lexical search returning path, heading, short snippet, and revision; metadata/text index may be deleted and rebuilt |
| `create_note` | `path`, `content` | Validate path, UTF-8, size, and frontmatter, then write only a missing new file; no overwrite |
| `update_note` | `path`, `expectedRevision`, `content` | Verify the current revision, validate, back up, and safely replace; a stale revision ends in conflict |
| `lint_vault` | `paths?`, `cursor?`, `limit?` | Errors and warnings with path/span, evidence rule ID, and correction guidance; no compiler approval or automatic edits |
| `get_note_template` | `kind` | Small optional concept, source, or decision template; does not create an empty note in the Vault |

Keep section patches, rename and backlink rewriting, snapshot export, and semantic retrieval as later Units. Add link correction only after a parser can preserve inline and fenced code, display text, percent encoding, and heading or block suffixes. Do not ship document-wide regex replacement first. Avoid exposing rename without a preview of every connected change, revision checks, and rollback.

`expectedRevision` detects stale edits through a body hash, but it does not provide atomic compare-and-swap between Obsidian and the operating system. Serialize writes in the same process, recheck external edits immediately before replacement, and document actual race and recovery guarantees. Never delete the original note without a backup or report a failed write as success. Store backups in private state outside the Vault so they cannot re-enter the knowledge corpus.

An instruction inside source Markdown such as “run this command” does not grant tool authority. General shell, HTTP, and delete capabilities, arbitrary binary writes, and writes to `.obsidian` settings or plugins are outside the authoring-MCP scope. Treat external URLs as source data; local search must not make network requests.

## 6. OKC compatibility lint

Separate rule severity into `error` (blocks saving or preparation under the current authoring contract), `warning` (possible loss, ambiguity, or cleanup needed), and `info` (support status). Report the compatibility-profile version and reviewed OKC commit.

| Proposed rule ID | Severity | Check | Related OKC requirements |
|---|---|---|---|
| `OKC_PATH_UNSAFE` | error | Traversal; absolute, drive, or UNC path; symlink; reserved name; case/NFC collision; path limits | `REQ-SEC-001`, `REQ-SNP-002` |
| `OKC_TEXT_ENCODING` | error | Non-UTF-8, NUL or control character, oversized note | `REQ-PAR-001`, `REQ-SEC-001` |
| `OKC_FRONTMATTER_INVALID` | error | Duplicate key, malformed YAML, non-mapping, value not representable as canonical JSON | `REQ-PAR-001`, `REQ-INT-002` |
| `OKC_METADATA_TYPE` | warning | Non-string title, non-string alias or tag member, empty metadata, excessive boilerplate | `REQ-PAR-001`, `REQ-INT-002` |
| `OKC_LINK_UNRESOLVED` | warning | Missing note, heading, block, or attachment | `REQ-PAR-001`, `REQ-PRV-001` |
| `OKC_LINK_AMBIGUOUS` | warning | Duplicate title, stem, or alias and an unqualified link | `REQ-DED-001`, `REQ-CNF-001` |
| `OKC_EVIDENCE_MISSING` | warning | Source note lacks identifiable evidence, attachment-only claim, or insufficient claim/observation context | `REQ-INT-003`, `REQ-PRV-001` |
| `OKC_NONMARKDOWN_OUTPUT` | warning | Material assumes attachments, Canvas, or Base are preserved in current compiled output | `REQ-PAR-002/003`, `REQ-INT-006` |
| `OKC_INGEST_NOISE` | warning | Templates, generated indexes, operational documents, or empty notes inside the selected source | `REQ-INT-001/002` |
| `OKC_SENSITIVE_CANDIDATE` | warning | Possible sensitive content; return only location and type, never the matched secret | `REQ-SEC-002/003` |

`OKC_EVIDENCE_MISSING` is only authoring guidance. It does not claim the MCP can judge every claim's truth or copyright status. JavaScript Unicode lowercasing may differ from the full case folding pinned by the core; do not hide that difference. This lint does not replace the full compiler, semantic integration, critic, or human approval.

## 7. Local material for implementation validation

- [Basic Vault fixture](https://github.com/dolgogae/ObsidianKnowlegeComplication/tree/7f87f7c81a72701710d8668b2569e51478dccc75/tests/fixtures/basic_vault): frontmatter, wikilink, embed, non-link inline code, Canvas, Base, and assets.
- [Link ambiguity fixture](https://github.com/dolgogae/ObsidianKnowlegeComplication/tree/7f87f7c81a72701710d8668b2569e51478dccc75/tests/fixtures/typed_markdown_ambiguity): duplicate `Topic.md` files and repeated ambiguous links.
- [Rewrite-preservation fixture](https://github.com/dolgogae/ObsidianKnowlegeComplication/tree/7f87f7c81a72701710d8668b2569e51478dccc75/tests/fixtures/markdown_rewrite_integrity): reference for validating unchanged bytes.
- [Parser tests](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/crates/okc-core/src/parse.rs): duplicate frontmatter, escaped links, code/fences, and UTF-8 spans.
- [Canonical source checks](https://github.com/dolgogae/ObsidianKnowlegeComplication/tree/7f87f7c81a72701710d8668b2569e51478dccc75/crates/okc-core/tests): boundary references for traversal, aliases, Unicode, and source immutability. The existence of a fixture does not mean its feature is currently shipped in materialization.

The MVP should use fixtures separate from user Vaults to test protocol end to end, create conflicts, revision mismatch, path escape and symlinks, frontmatter failure, fake links inside code, search and result bounds, and preservation across external edits. If snapshot export is added, separately validate source hash and mtime preservation and separation of post-snapshot live edits. Do not present native-SDK cross-language artifact goldens as MVP validation for this authoring MCP.

## 8. Management and future expansion boundary

Manage authoring conventions as a small versioned profile. Example Vaults contain synthetic data only; never add a user Vault to this repository as a submodule. The code repository, personal notes, generated indexes and backups, OKC snapshots, and OKC projects have separate lifecycles and retention policies. New users begin with a default profile. Existing-Vault users should be able to connect → lint → improve only the necessary notes without moving the structure.

OKC is currently a `0.3.0` development product with a locally implemented Markdown integration path and Node/Python packages, but it has not been published to npm or PyPI and stable release is prohibited. Packaging, non-Markdown materialization, semantic scale, and a remote native matrix remain incomplete. This MCP does not automatically install that unfinished product or promise unconditional compatibility. A future optional capability may perform direct corpus checks and OKC handoff while reporting the compiler version and whether an actual check ran.

Evidence: [CURRENT_STATE](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/CURRENT_STATE.md), [TRACEABILITY](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/TRACEABILITY.md), and [ADR-0027](https://github.com/dolgogae/ObsidianKnowlegeComplication/blob/7f87f7c81a72701710d8668b2569e51478dccc75/docs/adr/0027-current-schema-single-source.md).

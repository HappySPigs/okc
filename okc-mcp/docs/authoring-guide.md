# Authoring source notes for OKC

A Vault is the workspace for knowledge being authored. Keep configuration, templates, generated operational documents, and backups outside it. Preserve snapshots captured by OKC; later edits become input to a future snapshot.

## Start with a small structure

For an existing Vault, connect it without reorganizing folders, then run `audit_vault`. A new Vault may use the folders below. There is currently no CLI command for creating the structure; `create_note` creates the requested note's parent folders when necessary.

| Folder | Purpose |
|---|---|
| `inbox/` | Collected material and questions that have not yet been reviewed |
| `sources/` | Identifiable source observations, quotations, and summaries |
| `notes/` | Topics, claims, and decisions linked to evidence |
| `maps/` | Small, human-readable topic guides and connections |

Folder names are not compiler classification policy. A `draft` tag does not exclude a note from OKC collection. Keep anything that must not be collected outside the selected source Vault so the boundary is explicit.

## Paragraphs and sources

Do not pack several independent facts into one paragraph. Distinguish verified facts, interpretations, hypotheses, and counterexamples, and keep sources close to the claims they support. Record only source details you actually know, such as the document name, URL, page, or date checked. Never invent a date or quotation merely to complete a template.

The following is an **example template stored outside the Vault** and intended to be filled with user-provided material.

```md
---
title: A specific topic
---
# A specific topic

## Observation

State one verified claim and its scope.

Evidence: [Document actually reviewed](https://example.test/replace-with-real-source), relevant section or page.

## Interpretation and open questions

Separate conclusions inferred from the observation from points that remain unverified.
```

Do not save empty titles or placeholder sources. Remove sections that the material does not need. When claims conflict, retain the original sources and their dates. Do not automatically delete or merge notes merely because they appear to be duplicates.

## Metadata and links

`title` must be a string. `aliases` and `tags` may be strings or lists of strings. Authoring tools reject empty values, complex YAML types, duplicate keys, and invalid numbers. Unknown user-defined keys are preserved. Unmodified body text and YAML comments are retained, although the serialization style of a changed YAML node may differ.

Fields such as `source`, `status`, `type`, and `created` are optional authoring conventions. OKC does not treat them as special approval, privacy, or classification signals. `sourceId` belongs to OKC's Vault binding and does not need to be repeated in every note.

Confirm that a link target exists and resolve the meaning when several candidates match. Obsidian treats folder-qualified links as Vault-root-relative, while the OKC parser uses document-relative paths and candidate lookup. Audit cases where those interpretations point to different documents. The current heuristics check basic wikilinks, ATX headings, and block anchors; they do not fully parse Markdown links or every plugin syntax. See the [Obsidian documentation on internal links](https://obsidian.md/help/Linking%2Bnotes%2Band%2Bfiles/Internal%2Blinks).

## Changes and review

Start by obtaining the current whole-file `sha256` from `read_note`. Because `replace_note` replaces all content, follow `nextOffset` and read the complete note before changing a long file. Use `patch_frontmatter` when only frontmatter should change. Preview with `dryRun: true`; applying with `false` returns a backup ID and the new hash. If a hash conflict occurs, review the latest content again. Do not edit the same note concurrently from an external editor.

After authoring, inspect `audit_vault` errors, warnings, and omitted items. Sensitive-data candidates are only pattern-based hints. Continue with OKC's actual sensitive-data preflight, evidence checks, critic, and human review. Preserve important material stored in attachments, Canvas, or Base files, and add a Markdown explanation when the claim would otherwise exist only in those formats.

This guidance is also exposed through the server's `okc://guide/authoring` resource and `capture_knowledge` prompt.

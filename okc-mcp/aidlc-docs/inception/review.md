# Inception review proposal — unapproved

This document is not an approved implementation specification. It collects product intent and design assumptions for review with the user. Existing code must not be used to infer or fix the product scope retroactively.

## Problem confirmed by the user

**An installable local Obsidian MCP is needed to help organize knowledge as effectively as possible for OKC input.**

The requested scope includes investigating the actual logic of existing MCPs, designing a Vault that fits OKC, repository management from the user's perspective, and proceeding from the beginning through implementation under AWS AI-DLC. Querying compiled OKC results is not the server's primary purpose.

## Constraints confirmed by research

- The current OKC implementation structures Markdown and frontmatter and manages evidence and review state for documents, blocks, and metadata. An authoring tool does not replace compiler approval policy.
- A user-edited Vault and a snapshot collected by OKC have different lifecycles.
- `title`, `aliases`, and `tags` must be distinguished from general user metadata. Folder names and arbitrary fields such as `status` must not be assumed to represent compiler policy.
- Templates and operational Markdown may be collected as knowledge. The current OKC implementation also has limitations around attachments, Canvas, Base, and link rewriting.

Evidence: [existing MCP research](existing-mcp-research.md) and [OKC analysis and Vault proposal](okc-vault-design.md).

## Assumptions not yet approved by the user

| Assumption | Decision needed |
|---|---|
| Start with one Vault | Does the first version need to author and compare multiple Vaults at once? |
| Direct filesystem access over stdio | Are the Obsidian UI, Dataview, or plugin features while Obsidian is running essential? |
| Node/TypeScript and npm installation | Is this installation path simple enough for the target users and operating systems? |
| Limit the first Unit to authoring, updates, and quality auditing | Which journey has priority: capture, organization, duplicate merging, or source enrichment? |
| Optional shallow folder structure | What balance is needed between improving existing Vaults and creating new ones, and what should migration feel like? |
| Heuristic quality auditing | At what point must actual OKC ingestion testing be included? |
| Hash checks plus external backups | What cleanup authority is acceptable, and at what granularity should users review changes? |

The presence of these assumptions in existing code is not evidence of a user decision.

## Proposed success criteria

The following draft combines scenarios and measures for review.

1. A new user can install from documentation alone and author a first note in a chosen Vault.
2. An existing-Vault user can understand problems and proposed improvements without forced migration.
3. Knowledge can be added or updated without losing sources, conflicting claims, or links.
4. Input-structure problems and unnecessary collection noise are reduced when the Vault is handed to OKC.
5. Users can review changes, identify conflicts, and recover earlier content.

“Best Vault” is not evaluated by folder preference or tool count alone. It should be assessed through OKC input quality and authoring UX in representative synthetic or consented user scenarios.

## Next stage and gate

The official workflow's Inception phase addresses requirements analysis, user stories, domain and contract design, implementation Units, and delivery planning in sequence. Apply it only after confirming stage selection and approval handling with the user. At this point, do not present Inception as complete or the Construction gate as passed.

Reference: [phases and stages in the pinned AWS version](https://github.com/awslabs/aidlc-workflows/blob/22ed2d101f4f01196b76d5725cf8d9aabe5fef9e/docs/guide/04-phases-and-stages.md).

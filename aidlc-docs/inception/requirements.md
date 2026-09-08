# Requirements — Obsidian MCP for authoring an OKC input Vault

Status: requirements draft dated 2026-09-06. This is the assistant's proposed initial scope and has not been reviewed or approved by the user. Construction is paused while actual requirements and assumptions are reconfirmed through the [Inception review proposal](review.md).

## Intent

The user requested an investigation of existing Obsidian MCP logic, a Vault structure suited to OKC, installation and repository-management UX for OKC users, and progress from initial design through implementation using AWS AI-DLC. The user then clarified that this should be **an Obsidian MCP that authors the best possible OKC input Vault**, not an adapter for querying a resulting Vault. That correction is the current product direction; the earlier read-oriented option is abandoned.

## Initial user journey

Install locally → explicitly connect one Vault to edit → create evidence-backed notes → read and search → update bodies and frontmatter against a known hash → audit input quality → give the source path to OKC, which owns snapshotting, integration, review, and compilation.

| ID | Requirement | Acceptance criterion |
|---|---|---|
| REQ-001 | Installable local stdio MCP | Works without the Obsidian app, a REST plugin, an API key, or an OKC binary. |
| REQ-002 | Respect existing Vaults | Connects without relocating folders or requiring migration to custom frontmatter. |
| REQ-003 | Knowledge authoring | Creates Markdown with a title and body plus optional aliases, tags, and source; refuses to replace an existing file. |
| REQ-004 | Conflict-aware updates | Rejects a mismatched `expectedHash` and preserves a pre-change backup outside the Vault. |
| REQ-005 | Preserve structure | Partial frontmatter updates preserve unknown keys, body content, and YAML comments. Malformed YAML is rejected. |
| REQ-006 | Discovery | Provides deterministic path listing, literal search including Korean text, and reads that include a content hash. |
| REQ-007 | Audit OKC input | Reports YAML, path, link, duplicate, operational-noise, and unsupported-format issues without claiming compiler validation passed. |
| REQ-008 | Bounded authority | Rejects access outside the registered Vault, symlinks, hardlinks, and hidden control paths; bounds file size, file count, and response size. |
| REQ-009 | Separate knowledge and tool state | Keeps configuration, backups, and templates outside the input Vault. Compiled artifacts and managed projects are not connected as editable Vaults. |
| REQ-010 | Reviewable installation | Supports configuration generation, diagnostics, absolute-path client configuration output, and local tarball installation. |
| REQ-011 | Trust boundary | Treats notes as untrusted data. Exposes no internal AI invocation, automatic approval, shell, arbitrary HTTP, or delete tool. |
| REQ-012 | Lifecycle records | Records research, design, implementation, testing, operations/release status, and follow-up work in files. |

## Exclusions and follow-up Units

The first Unit covers Markdown authoring in a single Vault. It excludes semantic search, Obsidian UI or Dataview execution, general-purpose deletion or rename, automatic taxonomy moves, remote HTTP, provider execution, OKC review approval, attachment/Canvas/Base conversion, and complete Obsidian link interpretation. Authoring authority over live sources and immutability of snapshots sealed by OKC belong to different lifecycle stages.

Follow-up Units are: (1) rename and recovery UX that preserves links, (2) integration with compiler corpus checks, (3) performance and operating-system validation on real user Vaults, and (4) publication and update paths. Priorities may change after user review.

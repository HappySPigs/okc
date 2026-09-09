# Components — okc-mcp first Unit

**Status**: Application Design output (autopilot). High-level component identification and responsibilities. Detailed business rules are deferred to Functional Design (Construction). Sources: [`../requirements/requirements.md`](../requirements/requirements.md), [`../user-stories/stories.md`](../user-stories/stories.md), [`../plans/application-design-plan.md`](../plans/application-design-plan.md).

**Architecture**: simple layered, in-process, unidirectional — **surface → services → core**. No DI framework, no plugin system, no network. Typed structured rejections everywhere (`path-denied`, `hash-mismatch`, `malformed-yaml`, `overwrite-refused`, `bounds-exceeded`, `not-found`). No dangerous capability (no shell, arbitrary HTTP, delete, auto-approval, or internal AI).

---

## Core components (no upward dependencies)

### C1 · VaultBoundary
- **Purpose**: The single choke point for path safety. Every filesystem access is resolved and authorized here.
- **Responsibilities**: resolve a caller path against the one registered Vault root; reject anything outside it, and reject symlinks, hardlinks, and hidden control paths; enforce configured bounds (file size, file count, response size); confirm that a backup target is *outside* the Vault.
- **Interface**: `resolveInVault`, `assertWithinVault`, `assertOutsideVault`, `checkBounds`.
- **Serves**: REQ-008. Stories: US-IN-09, and underpins US-AU-01/06/07, US-RV-01.

### C2 · HashService
- **Purpose**: Deterministic content identity for conflict-aware writes.
- **Responsibilities**: compute a deterministic content hash; compare a supplied `expectedHash` against current content.
- **Interface**: `hash`, `matches`.
- **Serves**: REQ-004, REQ-006. Stories: US-RV-01/02, US-AU-05.

### C3 · FrontmatterEngine
- **Purpose**: Structure-preserving YAML frontmatter handling.
- **Responsibilities**: parse frontmatter + body; serialize back; apply a **partial** frontmatter merge that preserves unknown keys, body content, and YAML comments; reject malformed YAML rather than guessing.
- **Interface**: `parse`, `serialize`, `mergePartial`, `validate`.
- **Serves**: REQ-005, REQ-013. Stories: US-AU-02/03/04, US-RV-05.

### C4 · NoteStore
- **Purpose**: Low-level note file IO, always through VaultBoundary.
- **Responsibilities**: read a note returning content + content hash; create a note refusing to overwrite an existing file; write in place atomically; list paths deterministically.
- **Interface**: `read`, `create`, `writeInPlace`, `list`.
- **Serves**: REQ-003, REQ-006. Stories: US-IN-08, US-AU-06, US-AU-07, US-RV-01.

### C5 · BackupManager
- **Purpose**: Pre-change data protection outside the Vault.
- **Responsibilities**: write exactly one pre-change backup of prior content to a location outside the Vault, traceable to its source note; never leave a backup on a rejected mutation; no generational retention or auto-cleanup; expose where backups live for documented manual recovery.
- **Interface**: `backup`, `locate`.
- **Serves**: REQ-004, REQ-009. Stories: US-RV-03/04, US-IN-07, US-AU-05.

### C6 · AuditEngine
- **Purpose**: Read-only heuristic quality assessment of the Vault as OKC input.
- **Responsibilities**: detect and categorize issues (YAML, path, link, duplicate, operational-noise, unsupported-format); attach the offending file path; state findings are heuristic and never claim OKC compiler validation passed; mutate nothing.
- **Interface**: `audit`.
- **Serves**: REQ-007. Stories: US-AU-01.

---

## Services (orchestration; depend only on core)

### S1 · AuthoringService
- **Purpose**: All note mutations, each via one fixed safe write pipeline.
- **Responsibilities**: `createNote`, `updateNote`, `standardizeFrontmatter`, `fixYaml`, `reinforceSourcesLinks`. Every mutating call runs the shared pipeline: **assertWithinVault → (create: assert-not-exists / update: check expectedHash) → build new content (FrontmatterEngine) → write single external backup → atomic write in place**.
- **Serves**: REQ-003/004/005/013. Stories: US-AU-02/03/04/05/06, US-IN-08.

### S2 · AuditService
- **Purpose**: Read-only Vault audit orchestration.
- **Responsibilities**: enumerate in-bounds notes via NoteStore/VaultBoundary, parse with FrontmatterEngine, run AuditEngine, return categorized issues. No mutation.
- **Serves**: REQ-007, REQ-002, REQ-008. Stories: US-AU-01.

### S3 · DiscoveryService
- **Purpose**: Locate and read notes safely.
- **Responsibilities**: deterministic `listNotes`; literal `search` including Korean substrings; `readNote` returning content + hash. All bounded and in-Vault.
- **Serves**: REQ-006, REQ-008. Stories: US-AU-07, US-RV-01, US-IN-08.

### S4 · SetupService
- **Purpose**: Installation-time setup, kept outside the Vault.
- **Responsibilities**: `registerVault` (exactly one, by absolute path, no migration); `generateConfig` (outside the Vault); `diagnostics` (shallow local self-check, no network); `emitClientConfig` (absolute paths, no secrets).
- **Serves**: REQ-001/002/009/010. Stories: US-IN-01/02/03/04/05.

---

## Surface

### X1 · McpServer / ToolRegistry
- **Purpose**: The thin stdio MCP surface.
- **Responsibilities**: register only the safe tools that map to service methods; expose **no** shell / arbitrary-HTTP / delete / auto-approval / internal-AI tool; treat all note content as untrusted; serialize typed rejections into structured MCP tool errors; report a version string.
- **Serves**: REQ-001, REQ-011. Stories: US-IN-01, US-IN-06.

---

## MVP scope guard
No component implements file move/rename/merge, multi-Vault, real OKC ingestion, semantic search, Obsidian UI/Dataview, remote HTTP, deletion, or auto-approval. Those are follow-up Units (requirements §7) / exclusions (§8). Single-Vault registration (SetupService) and the no-dangerous-tools surface (McpServer) are *enforcement* of first-Unit boundaries, not deferred features.

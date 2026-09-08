# Services — okc-mcp first Unit

**Status**: Application Design output (autopilot). Service definitions, responsibilities, and orchestration patterns. All services are in-process, synchronous, and depend only on core components (never on each other, never upward).

---

## The fixed safe write pipeline (shared by all mutations)

Every mutating operation in **AuthoringService** runs the same ordered pipeline (`applyMutation`). This is the single most important design invariant — it guarantees REQ-004/005/008 for every write path:

```
1. VaultBoundary.assertWithinVault(path)          -> else path-denied
2a. create:  NoteStore.read(path) must be not-found -> else overwrite-refused
2b. update:  NoteStore.read(path) -> current;
             HashService.matches(current, expectedHash) -> else hash-mismatch
             (expectedHash required for existing files; missing -> hash-mismatch/rejected)
3. build new content via FrontmatterEngine.mergePartial / validate -> else malformed-yaml
4. BackupManager.backup(path, priorContent)       (updates only; skipped for create; none on any prior rejection)
5. NoteStore.writeInPlace(path, newContent)        (atomic; no move/rename)
6. return { hash = HashService.hash(newContent) }
```

Rejections short-circuit the pipeline before any write or backup — so a rejected mutation never leaves a file change or a misleading backup.

---

## S1 · AuthoringService
- **Responsibility**: all note mutations — create, update, standardize frontmatter, fix YAML, reinforce sources/links.
- **Orchestrates**: VaultBoundary, NoteStore, FrontmatterEngine, HashService, BackupManager.
- **Pattern**: thin public methods, each delegating to `applyMutation` with an operation-specific content builder. No mutation bypasses the pipeline.
- **Requirements**: REQ-003, REQ-004, REQ-005, REQ-013. **Stories**: US-AU-02/03/04/05/06, US-IN-08.

## S2 · AuditService
- **Responsibility**: read-only, categorized heuristic audit of the Vault as OKC input.
- **Orchestrates**: VaultBoundary + NoteStore (enumerate in-bounds notes), FrontmatterEngine (parse), AuditEngine (categorize).
- **Pattern**: strictly read-only; skips out-of-bounds/symlink/hardlink/hidden paths; bounded; deterministic; annotates output as heuristic (no compiler-pass claim).
- **Requirements**: REQ-007, REQ-002, REQ-008. **Stories**: US-AU-01.

## S3 · DiscoveryService
- **Responsibility**: locate and read notes safely — deterministic listing, literal Korean-capable search, read-with-hash baseline.
- **Orchestrates**: VaultBoundary, NoteStore, HashService.
- **Pattern**: read-only; all results bounded (response size / file count); `readNote` returns the content hash that feeds AuthoringService updates.
- **Requirements**: REQ-006, REQ-008. **Stories**: US-AU-07, US-RV-01, US-IN-08.

## S4 · SetupService
- **Responsibility**: installation-time setup kept outside the Vault — register one Vault, generate config, run diagnostics, emit client config.
- **Orchestrates**: VaultBoundary (registration + outside-Vault placement); filesystem for config/diagnostics.
- **Pattern**: no migration/relocation of user files; single-Vault registration (rejects a second); shallow self-check with no network; absolute-path, secret-free client config.
- **Requirements**: REQ-001, REQ-002, REQ-009, REQ-010. **Stories**: US-IN-01/02/03/04/05.

---

## Surface interaction (X1 · McpServer / ToolRegistry)
The MCP server is the only entry point. It binds a **fixed, safe** set of tools to service methods, exposes no dangerous capability, treats note content as untrusted, and converts any service `Rejection` into a structured MCP tool error. Approximate tool → service map:

| MCP tool | Service method |
|---|---|
| `create_note` | AuthoringService.createNote |
| `update_note` | AuthoringService.updateNote |
| `standardize_frontmatter` | AuthoringService.standardizeFrontmatter |
| `fix_yaml` | AuthoringService.fixYaml |
| `reinforce_sources_links` | AuthoringService.reinforceSourcesLinks |
| `audit_vault` | AuditService.auditVault |
| `list_notes` | DiscoveryService.listNotes |
| `search_notes` | DiscoveryService.search |
| `read_note` | DiscoveryService.readNote |
| `register_vault` / `generate_config` / `diagnostics` / `emit_client_config` | SetupService.* |

No `delete_*`, `run_shell`, `http_*`, `approve_*`, or AI-invocation tool exists (REQ-011).

# Component Dependencies — okc-mcp first Unit

**Status**: Application Design output (autopilot). Dependency matrix, communication patterns, and data flows. All communication is **in-process synchronous function calls**, strictly **unidirectional**: surface → services → core. Core components never call services or the surface; services never call each other.

## Dependency matrix

| From \ To | VaultBoundary | HashService | FrontmatterEngine | NoteStore | BackupManager | AuditEngine |
|---|---|---|---|---|---|---|
| **McpServer/ToolRegistry** | — | — | — | — | — | — (depends on services S1–S4 only) |
| **AuthoringService (S1)** | ✓ | ✓ | ✓ | ✓ | ✓ | — |
| **AuditService (S2)** | ✓ | — | ✓ | ✓ | — | ✓ |
| **DiscoveryService (S3)** | ✓ | ✓ | — | ✓ | — | — |
| **SetupService (S4)** | ✓ | — | — | — | — | — |
| **NoteStore (C4)** | ✓ | ✓ | — | — | — | — |
| **BackupManager (C5)** | ✓ | — | — | — | — | — |
| **AuditEngine (C6)** | — | — | ✓ | — | — | — |
| **FrontmatterEngine (C3)** | — | — | — | — | — | — |
| **HashService (C2)** | — | — | — | — | — | — |
| **VaultBoundary (C1)** | — | — | — | — | — | — |

- **McpServer** depends only on the four services (S1–S4).
- **VaultBoundary, HashService, FrontmatterEngine** are leaf dependencies (no outgoing deps) — the stable foundation.
- **NoteStore** routes every IO through **VaultBoundary** and stamps reads with **HashService**.
- **BackupManager** uses **VaultBoundary** to guarantee the backup target is *outside* the Vault.

## Communication patterns
- Synchronous function calls only; no events, queues, timers, or network.
- Errors propagate as typed `Rejection` values (`path-denied`, `hash-mismatch`, `malformed-yaml`, `overwrite-refused`, `bounds-exceeded`, `not-found`); the surface serializes them into structured MCP tool errors.
- No shared mutable global state; the registered Vault root and bounds config are injected read-only into components at construction.

## Key data flows (text)

```
READ / baseline (US-RV-01, US-AU-07)
  McpServer(read_note) -> DiscoveryService.readNote
    -> VaultBoundary.assertWithinVault -> NoteStore.read -> HashService.hash
    -> { content, hash }

CREATE (US-AU-06, US-IN-08)
  McpServer(create_note) -> AuthoringService.createNote -> applyMutation
    -> VaultBoundary.assertWithinVault
    -> NoteStore.read == not-found (else overwrite-refused)
    -> FrontmatterEngine.serialize(new doc)
    -> NoteStore.create -> { hash }        (no backup: nothing to back up)

UPDATE / STANDARDIZE / FIX-YAML / REINFORCE (US-AU-02/03/04/05)
  McpServer(update_note|standardize_frontmatter|fix_yaml|reinforce_sources_links)
    -> AuthoringService.* -> applyMutation
    -> VaultBoundary.assertWithinVault
    -> NoteStore.read -> HashService.matches(expectedHash) (else hash-mismatch)
    -> FrontmatterEngine.mergePartial/validate (else malformed-yaml)
    -> BackupManager.backup(path, priorContent)   (single, outside Vault)
    -> NoteStore.writeInPlace (atomic) -> { hash }

AUDIT (US-AU-01)
  McpServer(audit_vault) -> AuditService.auditVault
    -> VaultBoundary(list/bounds) -> NoteStore.list/read (in-bounds only)
    -> FrontmatterEngine.parse -> AuditEngine.audit
    -> { issues[] by category, heuristic:true }   (read-only)

MANUAL RECOVERY (US-RV-04)
  documented: read BackupManager.locate() -> user re-authors via update_note
    (restore goes through the SAME conflict-aware, backup-generating pipeline)

SETUP (US-IN-01..05)
  McpServer(register_vault|generate_config|diagnostics|emit_client_config)
    -> SetupService.* -> VaultBoundary(register/outside-Vault)
    -> { vaultRoot | configPath | checks[] | clientConfig }
```

## Cycle check
The dependency graph is a DAG: surface → {S1,S2,S3,S4} → {C1..C6} → {C1,C2,C3}. No cycles; no service-to-service or core-to-service edges.

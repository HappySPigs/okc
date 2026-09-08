# Changelog

## 0.1.0-alpha.1 — Local candidate, not publicly released

- A stdio MCP for editing a single source Vault without the Obsidian app or plugins.
- Authoring tool surface: `create_note`, `update_note`, `standardize_frontmatter`, `fix_yaml`, `reinforce_sources_links` — every mutation flows through one fixed safe write pipeline (conflict check → single external pre-change backup → atomic write). No move/rename/merge/delete.
- Discovery/read tools: `list_notes`, `read_note` (with whole-file hash), literal Korean-safe `search_notes`.
- Bounded, read-only OKC input-quality `audit_vault`, findings grouped into six categories (yaml, path, link, duplicate, operational-noise, unsupported-format); never claims compiler validation passed.
- Typed rejections use a canonical `kind` set (`path-denied`, `hash-mismatch`, `malformed-yaml`, `overwrite-refused`, `bounds-exceeded`, `not-found`).
- Pre-change backups are traceable to their source note (sidecar metadata) and enumerable for documented manual recovery (see `docs/recovery.md`); single latest generation, manual restore.
- Bounded authority: access confined to the registered Vault (symlink/hardlink/hidden-control-path rejection), size/count/response bounds; no shell, HTTP, delete, auto-approval, or AI-invocation tool.
- CLI setup: `config` generation, per-check `doctor` diagnostics, absolute-path `client-config` output, and local npm packaging.
- Built under AWS AI-DLC v1: requirements, stories, application/functional/NFR design, and this construction pass are recorded in `aidlc-docs/`.

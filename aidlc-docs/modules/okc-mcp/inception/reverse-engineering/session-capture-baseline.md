# Session capture baseline

2026-09-09 KST, scoped brownfield review of the existing module.

- Node/TypeScript stdio server in src/server.ts; CLI initializes a local Vault and optional remote reader.
- search_notes is bounded substring search; outline_note and list_backlinks provide structure. Existing generic capture_knowledge prompt does not define session routing.
- src/authoring.ts owns the shared preview/hash/backup/write pipeline. src/vault.ts owns path validation, immutable-root checks, locks and filesystem writes.
- Source selection defaults reads to configured web; all mutations are local. Session routing must explicitly use local.
- Existing source revisions and hooks/web contracts accept ordinary Markdown; no downstream contract change is needed.
- Existing tests use Node test, actual stdio clients and fast-check. TypeScript strict checking is enabled.
- New files: src/capture.ts and src/capture-guide.ts. Existing server, Vault path inspection, authoring helper and documentation will be extended in place.

The earlier first-unit reverse-engineering deferral remains historical. This is a current, narrowly scoped baseline, not a reconstruction of that earlier stage.


# Consolidated module AI-DLC docs (read-only mirror)

This directory is a **convenience mirror** that gathers every `okc-*` module's
`aidlc-docs/` under one tree so the whole project's AI-DLC trail can be browsed
from a single place.

> **Not authoritative.** Each module's own `<module>/aidlc-docs/` remains the
> single source of truth for that module's requirements, designs, state, and
> audit history. The copies here are read-only snapshots — do **not** edit them
> and do **not** resume a module's AI-DLC workflow from this tree. Make changes
> in the module directory and re-sync.

## Layout

| Mirror | Authoritative source | Files |
|---|---|---|
| [`okc-core/`](okc-core/) | [`../../okc-core/aidlc-docs/`](../../okc-core/aidlc-docs/) | 53 |
| [`okc-hooks/`](okc-hooks/) | [`../../okc-hooks/aidlc-docs/`](../../okc-hooks/aidlc-docs/) | 109 |
| [`okc-mcp/`](okc-mcp/) | [`../../okc-mcp/aidlc-docs/`](../../okc-mcp/aidlc-docs/) | 72 |
| [`okc-web/`](okc-web/) | [`../../okc-web/aidlc-docs/`](../../okc-web/aidlc-docs/) | 58 |

The umbrella (cross-module) AI-DLC workspace lives one level up in
[`../`](../) — see the [root workspace README](../README.md) and the
[module dashboard](../module-dashboard.md).

## How this mirror was produced

- Copied non-destructively (`cp -r`); every module's original folder is preserved in place.
- Outbound relative links were rewritten to stay valid from the mirror's new depth
  (module code/`AGENTS.md` references now resolve to `../../../okc-<module>/…`,
  root umbrella references resolve within `../`, intra-module links are unchanged).
- A small number of pre-existing broken/draft links in the sources were left exactly
  as authored (they are broken in the originals too, e.g. draft repo-root-style paths
  in `okc-web/integration/monorepo-README.draft.md`).

## Re-syncing

The mirror is a point-in-time copy. To refresh it after module docs change,
re-copy the module's `aidlc-docs/` into the matching subdirectory here and
re-apply the relative-link fixups. Because it is non-authoritative, a stale
mirror never blocks module work.

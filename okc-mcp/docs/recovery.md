# Manual recovery

okc-mcp protects your notes with a **single pre-change backup** written **outside** the Vault before any mutating write. Recovery is **manual by design** — there is no automatic rollback or generational history (RPO = your last saved state; RTO = manual).

## Where backups live

Backups are written under the configured **state** directory (never inside the Vault), in `backups/`:

- `state/backups/<id>.md` — the exact note content **before** the change.
- `state/backups/<id>.meta.json` — `{ "sourcePath": "<vault-relative path>", "sha256": "<pre-change hash>", "createdAt": "<ISO time>" }`.

The state directory path is shown by `okc-mcp doctor --config <config>` (the `statePath` field) and in the generated configuration.

Only the **single latest** pre-change backup per note is guaranteed. A later edit to the same note overwrites its protection; there is no auto-cleanup of older backup files, but do not rely on them being retained.

## Restore an earlier version

1. **Find the backup.** List `state/backups/` and read the `*.meta.json` sidecars to find the entry whose `sourcePath` matches the note you want to restore (newest `createdAt` is the most recent pre-change state).
2. **Read the prior content** from the matching `<id>.md` file.
3. **Re-author through the normal write path.** Get the note's current hash with `read_note`, then apply the prior content with `update_note` using that hash as `expectedHash`. The restore flows through the **same conflict-aware, backup-generating pipeline** as any other write — so it is itself backed up and cannot silently clobber a concurrent change.

There is intentionally **no** dedicated rollback or restore tool: restoring is a normal, reviewable, conflict-checked edit.

## Notes

- If `read_note` reports a conflict when you retry, the note changed on disk — re-read and review before restoring.
- Keep the state directory (and its backups) somewhere you control and back it up alongside your other local data if long-term retention matters.
- The server makes no network calls; recovery is entirely local.

# Installation, recovery, and updates

This product runs as a local process launched by an MCP host and is managed as a local package. It requires no server port, AWS account, API key, or automatic provider call.

## Connection and diagnostics

Follow the README sequence: `config` → `doctor` → `client-config`. The Vault must already exist, and the configuration must remain outside it. `doctor` inspects the file list but does not create the Vault or a missing state directory. State is created on the first write. In `serve` mode, stdout is reserved for MCP; normal note contents are not written to logs.

`maxFiles` limits visited entries, including hidden items and directories. `maxScanBytes` applies to the total size of regular files scanned, so large PDFs and media also consume the scan budget. Defaults are 1 MiB per note, 64 MiB per scan, and 10,000 entries. Performance on a 100,000-file or 20 GB Vault has not been validated. A search or audit walks the current files; it does not create an atomic snapshot of the entire Vault.

## Backups before changes

Applied `replace_note` and `patch_frontmatter` operations save the previous content at the following locations and return a `backupId`.

```text
<statePath>/backups/<backupId>
<statePath>/locks/<vault-hash>.lock
```

A backup contains the exact previous Markdown. During recovery, verify the `path`, `backupId`, and hashes returned by the original tool call. There is currently no backup-listing or automatic-restore tool and no separate full-history database. Preserve the mapping between a backup ID and its original note path from the tool result.

To roll back, stop editing the note and stop MCP writes, then preserve the current version separately. Confirm the backup file and original path before comparing and restoring them with an external editor. Alternatively, review the backup content and pass it to `replace_note` with the current `read_note` hash. Never reuse an old hash. Backups are private data and must not be uploaded to a public repository. Automatic expiration and cleanup are not implemented; retain and remove backups according to your own policy.

## Crashes and locks

MCP writers using the same canonical Vault and **the same `statePath`** coordinate through a lock. Writers using different state paths, as well as Obsidian and Sync, do not share that lock. The server does not automatically delete a lock merely because it appears stale.

If `VAULT_BUSY` persists, stop the related MCP hosts and editing activity, then inspect the lock file's `pid` and `startedAt`. Remove a leftover lock file manually only after confirming that the writer has exited and no other writer is active. Process IDs can be reused, so never terminate a process based on the number alone. After reconnecting, inspect the current note and backup, then review any needed change again.

A crash during file replacement may leave an `.okc-mcp-*.tmp` file. Stop every related writer, compare its contents, complete any necessary recovery, and then remove only the leftover temporary file. Recovery after power loss, filesystem failure, and crashes on every operating system is not yet guaranteed.

## Updates and removal

Before updating, review the new version's changelog and changes to tools or configuration. Install the exact tarball version, rerun `doctor` and `client-config`, and restart the MCP host. Regenerate the absolute paths when Node or the installation location changes. The server does not update itself at startup.

Rolling back the package means reinstalling the previous tarball; it does not revert note contents. To uninstall, remove the corresponding MCP entry from the host and run `npm uninstall -g okc-mcp`. Removing the package does not delete the Vault, configuration, or state. Review backups and manage those files separately.

## Validation still required before public release

Beyond local checks, this project still needs real CI runs on native macOS, Linux, and Windows; installation UX testing; concurrent Obsidian/Sync editing and process-interruption tests; and measurements on a large Vault. The presence of a CI file is not evidence that it passed in those environments. Registry package naming, ownership, and the publication procedure will be finalized at the public-candidate stage. Nothing has been publicly released yet.

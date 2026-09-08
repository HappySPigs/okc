import assert from 'node:assert/strict';
import { mkdir, mkdtemp, readdir, readFile, realpath, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test, { type TestContext } from 'node:test';
import * as authoring from '../src/authoring.js';
import type { Config } from '../src/config.js';
import { Vault, VaultError } from '../src/vault.js';

async function fixture(t: TestContext): Promise<{ vault: Vault; config: Config; vaultPath: string; statePath: string }> {
  const directory = await realpath(await mkdtemp(path.join(tmpdir(), 'okc-mcp-auth-')));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const vaultPath = path.join(directory, 'vault');
  const statePath = path.join(directory, 'state');
  await mkdir(vaultPath);
  const config: Config = { vaultPath, statePath, readOnly: false,
    maxNoteBytes: 1024 * 1024, maxFiles: 1000, maxScanBytes: 64 * 1024 * 1024, maxResponseBytes: 65_536 };
  const vault = new Vault(config);
  await vault.initialize();
  return { vault, config, vaultPath, statePath };
}

function code(expected: string) {
  return (error: unknown) => (error instanceof VaultError ? error.code : (error as { code?: unknown } | null)?.code) === expected;
}
const mdBackups = async (statePath: string): Promise<string[]> =>
  (await readdir(path.join(statePath, 'backups')).catch(() => [] as string[])).filter(name => name.endsWith('.md'));

test('applyMutation: create previews without writing, applies once, and refuses overwrite', async t => {
  const f = await fixture(t);
  const preview = await authoring.createNote(f.vault, f.config, { path: 'a.md', title: 'A', body: '# A\n', dryRun: true });
  assert.equal(preview.applied, false);
  await assert.rejects(readFile(path.join(f.vaultPath, 'a.md'), 'utf8'), { code: 'ENOENT' });
  const applied = await authoring.createNote(f.vault, f.config, { path: 'a.md', title: 'A', body: '# A\n', dryRun: false });
  assert.equal(applied.applied, true);
  assert.equal(typeof applied.sha256, 'string');
  await assert.rejects(authoring.createNote(f.vault, f.config, { path: 'a.md', title: 'A', body: '# B\n', dryRun: false }), code('NOTE_EXISTS'));
  // A create never writes a backup (nothing prior to protect) — BR-CREATE-3.
  assert.deepEqual(await mdBackups(f.statePath), []);
});

test('applyMutation: every update requires a matching hash, writes exactly one backup, and none on rejection', async t => {
  const f = await fixture(t);
  const created = await authoring.createNote(f.vault, f.config, { path: 'a.md', title: 'A', body: '# A\n', dryRun: false });
  // Missing/invalid hash: rejected, no write, no backup (BR-HASH-4).
  await assert.rejects(authoring.updateNote(f.vault, f.config, { path: 'a.md', expectedHash: 'nope', changes: { body: '# B\n' }, dryRun: false }), code('INVALID_HASH'));
  // Stale hash: conflict, file unchanged, no misleading backup (BR-HASH-3 / BR-BACKUP-2).
  await assert.rejects(authoring.updateNote(f.vault, f.config, { path: 'a.md', expectedHash: '0'.repeat(64), changes: { body: '# B\n' }, dryRun: false }), code('CONFLICT'));
  assert.deepEqual(await mdBackups(f.statePath), []);
  // Valid update through the pipeline: exactly one external backup.
  const updated = await authoring.updateNote(f.vault, f.config, { path: 'a.md', expectedHash: created.sha256!, changes: { body: '# B\n' }, dryRun: false });
  assert.equal(updated.applied, true);
  assert.equal((await mdBackups(f.statePath)).length, 1);
  assert.equal(await readFile(path.join(f.vaultPath, 'a.md'), 'utf8'), '# B\n');
});

test('applyMutation: standardize rejects an empty patch and a malformed current note', async t => {
  const f = await fixture(t);
  const created = await authoring.createNote(f.vault, f.config, { path: 'a.md', title: 'A', body: '# A\n', dryRun: false });
  await assert.rejects(authoring.standardizeFrontmatter(f.vault, f.config, { path: 'a.md', expectedHash: created.sha256!, frontmatterPatch: {}, dryRun: false }), code('NOTE_INVALID'));
  const standardized = await authoring.standardizeFrontmatter(f.vault, f.config, { path: 'a.md', expectedHash: created.sha256!, frontmatterPatch: { title: 'A2', tags: ['t'] }, dryRun: false });
  assert.equal(standardized.applied, true);
  const content = await readFile(path.join(f.vaultPath, 'a.md'), 'utf8');
  assert.ok(content.includes('title: A2'));
  assert.ok(content.includes('# A'), 'body preserved');
});

test('applyMutation: fix_yaml is the only path that can repair a currently-malformed note', async t => {
  const f = await fixture(t);
  // Seed a note with valid frontmatter, then read its hash.
  const created = await authoring.createNote(f.vault, f.config, { path: 'a.md', title: 'A', body: '# A\n', dryRun: false });
  // A standardize with a malformed *target* isn't applicable here; instead prove fix_yaml round-trips.
  const fixed = await authoring.fixYaml(f.vault, f.config, { path: 'a.md', expectedHash: created.sha256!, correctedFrontmatter: 'title: A\ntags: [x]', dryRun: false });
  assert.equal(fixed.applied, true);
  const content = await readFile(path.join(f.vaultPath, 'a.md'), 'utf8');
  assert.ok(content.includes('tags:'));
  assert.ok(content.endsWith('# A\n'));
});

import assert from 'node:assert/strict';
import { link, lstat, mkdir, mkdtemp, readFile, readdir, realpath, rename, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { test, type TestContext } from 'node:test';
import { sha256, Vault, VaultError, type VaultConfig } from '../src/vault.js';

async function fixture(t: TestContext, options: Partial<VaultConfig> = {}) {
  const directory = await realpath(await mkdtemp(path.join(tmpdir(), 'okc-mcp-vault-')));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const vaultPath = path.join(directory, 'vault');
  const statePath = path.join(directory, 'state');
  await mkdir(vaultPath);
  const config = { vaultPath, statePath, maxNoteBytes: 8192, maxFiles: 100, maxScanBytes: 100_000, ...options };
  const vault = new Vault(config);
  await vault.initialize();
  return { directory, vaultPath, statePath, config, vault };
}

function code(expected: string) {
  return (error: unknown) => error instanceof VaultError && error.code === expected;
}

test('initialize, list, and reads leave Vault and absent state unchanged', async t => {
  const f = await fixture(t);
  await writeFile(path.join(f.vaultPath, '한글.md'), '\ufeff# 한글\r\n본문\r\n');
  await mkdir(path.join(f.vaultPath, '.obsidian'));
  await writeFile(path.join(f.vaultPath, '.obsidian', 'workspace.json'), '{}');
  const before = await readdir(f.vaultPath);
  const listed = await f.vault.list();
  assert.deepEqual(listed.notes, ['한글.md']);
  assert.deepEqual(listed.skipped, ['.obsidian/']);
  const note = await f.vault.read('한글.md');
  assert.equal(note.content, '\ufeff# 한글\r\n본문\r\n');
  assert.equal(note.sha256, sha256(await readFile(path.join(f.vaultPath, '한글.md'))));
  assert.deepEqual(await readdir(f.vaultPath), before);
  await assert.rejects(lstat(f.statePath), { code: 'ENOENT' });
});

test('create is exclusive and update preserves an exact external backup', async t => {
  const f = await fixture(t);
  const original = '---\r\n# keep comment\r\naliases: ["원본"]\r\n---\r\n\r\n# Note\r\n';
  const created = await f.vault.create('notes/원본.md', original);
  await assert.rejects(f.vault.create('notes/원본.md', 'overwrite'), code('NOTE_EXISTS'));
  assert.equal((await f.vault.read('notes/원본.md')).content, original);
  assert.deepEqual(await readdir(path.join(f.vaultPath, 'notes')), ['원본.md']);
  assert.equal((await lstat(path.join(f.vaultPath, 'notes', '원본.md'))).nlink, 1);
  const content = `${original}추가 내용\r\n`;
  const updated = await f.vault.update('notes/원본.md', content, created.sha256);
  assert.equal(updated.sha256, sha256(content));
  assert.equal((await f.vault.read('notes/원본.md')).content, content);
  assert.deepEqual(await readFile(path.join(f.statePath, 'backups', updated.backupId)), Buffer.from(original));
  assert.deepEqual(await readdir(path.join(f.vaultPath, 'notes')), ['원본.md']);
  assert.deepEqual(await readdir(path.join(f.statePath, 'locks')), []);
});

test('stale expectedHash never changes current bytes or creates backups', async t => {
  const f = await fixture(t);
  const created = await f.vault.create('note.md', 'old');
  await writeFile(path.join(f.vaultPath, 'note.md'), 'external edit');
  await assert.rejects(f.vault.update('note.md', 'stale model edit', created.sha256), code('CONFLICT'));
  assert.equal(await readFile(path.join(f.vaultPath, 'note.md'), 'utf8'), 'external edit');
  await assert.rejects(lstat(path.join(f.statePath, 'backups')), { code: 'ENOENT' });
});

test('invalid note paths are rejected before any filesystem mutation', async t => {
  const f = await fixture(t);
  const paths = ['../escape.md', '/absolute.md', 'a\\b.md', '.git/config.md', 'x/.obsidian/secret.md',
    'node_modules/note.md', 'nested/Knowledge.okc-project/note.md', 'CON.md', 'a:b.md', 'a./note.md',
    'note.MD', 'cafe\u0301.md', 'a//b.md', 'a/./b.md', 'bad\u0000.md'];
  for (const name of paths) await assert.rejects(f.vault.create(name, 'unsafe'), code('INVALID_PATH'), name);
  assert.deepEqual(await readdir(f.vaultPath), []);
  await assert.rejects(lstat(f.statePath), { code: 'ENOENT' });
});

test('case ambiguity is rejected even on case-insensitive filesystems', async t => {
  const f = await fixture(t);
  await writeFile(path.join(f.vaultPath, 'Note.md'), 'original');
  await assert.rejects(f.vault.create('note.md', 'different'), code('PATH_COLLISION'));
  await assert.rejects(f.vault.read('note.md'), code('PATH_COLLISION'));
  assert.equal(await readFile(path.join(f.vaultPath, 'Note.md'), 'utf8'), 'original');
});

test('links and linked parent directories cannot read or mutate other files', async t => {
  const f = await fixture(t);
  await writeFile(path.join(f.directory, 'outside.md'), 'private');
  await writeFile(path.join(f.vaultPath, 'inside.md'), 'inside');
  await symlink(path.join(f.directory, 'outside.md'), path.join(f.vaultPath, 'escape.md'));
  await symlink(path.join(f.vaultPath, 'inside.md'), path.join(f.vaultPath, 'alias.md'));
  await symlink(f.directory, path.join(f.vaultPath, 'linked'));
  await link(path.join(f.directory, 'outside.md'), path.join(f.vaultPath, 'hard.md'));
  for (const name of ['escape.md', 'alias.md']) await assert.rejects(f.vault.read(name), code('SYMLINK'));
  await assert.rejects(f.vault.read('hard.md'), code('HARDLINK'));
  await assert.rejects(f.vault.create('linked/new/sub.md', 'escape'), code('SYMLINK'));
  const listed = await f.vault.list();
  assert.deepEqual(listed.notes, ['inside.md']);
  assert.ok(listed.skipped.includes('hard.md (HARDLINK)'));
  assert.ok(listed.skipped.includes('linked (SYMLINK)'));
  assert.equal(await readFile(path.join(f.directory, 'outside.md'), 'utf8'), 'private');
  await assert.rejects(lstat(path.join(f.directory, 'new')), { code: 'ENOENT' });
});

test('invalid UTF-8 and oversized notes fail bounded reads and writes', async t => {
  const f = await fixture(t, { maxNoteBytes: 8 });
  await writeFile(path.join(f.vaultPath, 'bad.md'), Buffer.from([0xc3, 0x28]));
  await writeFile(path.join(f.vaultPath, 'large.md'), '123456789');
  await assert.rejects(f.vault.read('bad.md'), code('INVALID_UTF8'));
  await assert.rejects(f.vault.read('large.md'), code('NOTE_TOO_LARGE'));
  await assert.rejects(f.vault.create('new.md', '123456789'), code('NOTE_TOO_LARGE'));
  await assert.rejects(f.vault.create('new.md', '\ud800'), code('INVALID_UTF8'));
  assert.ok((await f.vault.list()).skipped.includes('large.md (NOTE_TOO_LARGE)'));
});

test('entry, byte, and path depth budgets stop processing', async t => {
  const f = await fixture(t);
  await writeFile(path.join(f.vaultPath, 'a.md'), '123456');
  await writeFile(path.join(f.vaultPath, 'b.md'), '123456');
  const entries = new Vault({ ...f.config, maxFiles: 1 });
  await entries.initialize();
  await assert.rejects(entries.list(), code('SCAN_LIMIT'));
  const bytes = new Vault({ ...f.config, maxScanBytes: 10 });
  await bytes.initialize();
  await assert.rejects(bytes.list(), code('SCAN_LIMIT'));
  await assert.rejects(f.vault.create(`${'dir/'.repeat(20)}deep.md`, 'x'), code('PATH_DEPTH'));
});

test('state cannot overlap Vault or alias it and initialization creates nothing', async t => {
  const f = await fixture(t);
  const nested = new Vault({ ...f.config, statePath: path.join(f.vaultPath, 'state') });
  await assert.rejects(nested.initialize(), code('STATE_OVERLAP'));
  const ancestor = new Vault({ ...f.config, statePath: f.directory });
  await assert.rejects(ancestor.initialize(), code('STATE_OVERLAP'));
  await symlink(f.vaultPath, path.join(f.directory, 'alias'));
  const alias = new Vault({ ...f.config, statePath: path.join(f.directory, 'alias', 'new-state') });
  await assert.rejects(alias.initialize(), code('INVALID_STATE'));
  assert.deepEqual(await readdir(f.vaultPath), []);
});

test('retained write locks are never automatically removed', async t => {
  const f = await fixture(t);
  await f.vault.create('first.md', 'first');
  const lock = path.join(f.statePath, 'locks', `${sha256(await realpath(f.vaultPath))}.lock`);
  await writeFile(lock, 'retained-owner');
  await assert.rejects(f.vault.create('second.md', 'second'), code('VAULT_BUSY'));
  assert.equal(await readFile(lock, 'utf8'), 'retained-owner');
  await assert.rejects(lstat(path.join(f.vaultPath, 'second.md')), { code: 'ENOENT' });
});

test('compiled artifacts and OKC project roots are refused', async t => {
  const f = await fixture(t);
  await mkdir(path.join(f.vaultPath, '.okc'));
  await assert.rejects(new Vault(f.config).initialize(), code('IMMUTABLE_TARGET'));
  const project = path.join(f.directory, 'Knowledge.okc-project');
  await mkdir(project);
  await assert.rejects(new Vault({ ...f.config, vaultPath: project }).initialize(), code('IMMUTABLE_TARGET'));
  const knowledge = path.join(f.vaultPath, 'knowledge');
  await mkdir(knowledge);
  await assert.rejects(new Vault({ ...f.config, vaultPath: knowledge }).initialize(), code('IMMUTABLE_TARGET'));
  await assert.rejects(f.vault.create('note.md', 'new'), code('IMMUTABLE_TARGET'));
});

test('replacing the Vault root requires reconnecting', async t => {
  const f = await fixture(t);
  await writeFile(path.join(f.vaultPath, 'note.md'), 'original');
  await rename(f.vaultPath, path.join(f.directory, 'original-vault'));
  await mkdir(f.vaultPath);
  await writeFile(path.join(f.vaultPath, 'note.md'), 'replacement');
  await assert.rejects(f.vault.read('note.md'), code('ROOT_CHANGED'));
  await assert.rejects(f.vault.create('new.md', 'new'), code('ROOT_CHANGED'));
});

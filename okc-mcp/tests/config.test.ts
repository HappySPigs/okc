import assert from 'node:assert/strict';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { configSchema, defaultConfig, loadConfig } from '../src/config.js';

test('configuration rejects implicit paths, unknown options and unbounded limits', () => {
  assert.throws(() => defaultConfig('relative'));
  const config = defaultConfig(path.join(tmpdir(), 'source-vault'));
  assert.equal(config.readOnly, false);
  assert.equal(config.maxNoteBytes, 1024 * 1024);
  for (const change of [{ vaultPath: 'relative' }, { statePath: 'relative' }, { providerApiKey: 'not-supported' },
    { maxFiles: Infinity }, { maxNoteBytes: 999_999_999 }, { maxResponseBytes: 1 }]) {
    assert.throws(() => configSchema.parse({ ...config, ...change }));
  }
});

test('configuration is bounded and kept outside the authoring Vault', async t => {
  const root = await mkdtemp(path.join(tmpdir(), 'okc-mcp-config-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const vaultPath = path.join(root, 'vault');
  await mkdir(vaultPath);
  const content = JSON.stringify({ vaultPath, statePath: path.join(root, 'state') });
  const outside = path.join(root, 'okc-mcp.json');
  await writeFile(outside, content);
  assert.equal((await loadConfig(outside)).vaultPath, vaultPath);
  const inside = path.join(vaultPath, 'okc-mcp.json');
  await writeFile(inside, content);
  await assert.rejects(loadConfig(inside), /outside/);
  await writeFile(outside, ' '.repeat(65_537));
  await assert.rejects(loadConfig(outside), /64 KiB/);
  await writeFile(outside, Buffer.from([0xff]));
  await assert.rejects(loadConfig(outside));
});

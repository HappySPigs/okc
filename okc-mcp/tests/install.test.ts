import assert from 'node:assert/strict';
import { mkdtemp, rm, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test, { type TestContext } from 'node:test';
import { runSetup, runUnregister, type CommandRunner, type SetupOptions } from '../src/install.js';
import { loadConfig } from '../src/config.js';
import { VaultError } from '../src/vault.js';

const NODE = '/usr/local/bin/node';
const CLI = '/opt/okc-mcp/dist/cli.js';
const TOKEN = 'SECRET-read-token-abc123XYZ';

function fakeRunner(available: string[]) {
  const present = new Set(available);
  const calls: { command: string; args: string[] }[] = [];
  const run: CommandRunner = async (command, args) => {
    calls.push({ command, args });
    if (!present.has(command)) throw Object.assign(new Error('spawn ENOENT'), { code: 'ENOENT' });
    return { code: 0, stdout: '', stderr: '' };
  };
  return { run, calls };
}

async function workspace(t: TestContext, config: unknown) {
  const root = await mkdtemp(path.join(tmpdir(), 'okc-mcp-install-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const source = path.join(root, 'source.json');
  await writeFile(source, JSON.stringify(config));
  const configHome = path.join(root, 'xdg');
  const dest = path.join(configHome, 'okc-mcp', 'config.json');
  const out: string[] = [], err: string[] = [];
  const base: SetupOptions = { configPath: source, configHome, cliPath: CLI, nodePath: NODE,
    validateWeb: async () => {}, out: (s: string) => { out.push(s); }, err: (s: string) => { err.push(s); } };
  return { source, dest, out, err, base };
}

const webConfig = (agents: string[]) => ({ readOnly: true, agents,
  web: { baseUrl: 'https://example.com', projectId: 'p1', token: TOKEN } });

test('setup writes a 0600 config, registers exactly the selected agents, and never echoes the token', async t => {
  const { dest, out, err, base } = await workspace(t, webConfig(['claude', 'codex']));
  const { run, calls } = fakeRunner(['claude', 'codex']);
  await runSetup({ ...base, run });

  assert.equal((await stat(dest)).mode & 0o777, 0o600);
  assert.equal((await loadConfig(dest)).agents?.length, 2); // reloadable canonical config

  assert.deepEqual(calls.find(c => c.command === 'claude' && c.args[1] === 'add'),
    { command: 'claude', args: ['mcp', 'add', '--scope', 'user', 'okc-mcp', '--', NODE, CLI, 'serve', '--config', dest] });
  assert.deepEqual(calls.find(c => c.command === 'codex' && c.args[1] === 'add'),
    { command: 'codex', args: ['mcp', 'add', 'okc-mcp', '--', NODE, CLI, 'serve', '--config', dest] });

  const printed = out.join('') + err.join('');
  assert.ok(!printed.includes(TOKEN), 'token must never reach stdout/stderr');
});

test('setup falls back to printing the snippet when an agent CLI is absent', async t => {
  const { dest, out, err, base } = await workspace(t, webConfig(['claude']));
  const { run, calls } = fakeRunner(['codex']); // claude missing
  await runSetup({ ...base, run });

  assert.ok(!calls.some(c => c.command === 'claude' && c.args[1] === 'add'), 'no add attempted for the absent CLI');
  const snippet = out.join('');
  assert.ok(snippet.includes('mcpServers') && snippet.includes('okc-mcp') && snippet.includes(dest));
  assert.ok(err.join('').includes("'claude' CLI not found"));
});

test('setup warns on web-validation failure but still writes config and registers', async t => {
  const { dest, err, base } = await workspace(t, webConfig(['claude']));
  const { run, calls } = fakeRunner(['claude']);
  await runSetup({ ...base, run, validateWeb: async () => { throw new VaultError('WEB_UNAVAILABLE', 'unreachable'); } });

  assert.equal((await stat(dest)).mode & 0o777, 0o600);
  assert.ok(calls.some(c => c.command === 'claude' && c.args[1] === 'add'), 'registration still runs after a web warning');
  assert.ok(err.join('').includes('web validation failed'));
});

test('setup is idempotent: a re-run upserts (remove then add) without error', async t => {
  const { dest, base } = await workspace(t, webConfig(['claude']));
  await runSetup({ ...base, run: fakeRunner(['claude']).run });
  const { run, calls } = fakeRunner(['claude']);
  await runSetup({ ...base, run });

  const claude = calls.filter(c => c.command === 'claude');
  assert.ok(claude.some(c => c.args[1] === 'remove'), 'stale entry removed first');
  assert.ok(claude.some(c => c.args[1] === 'add'), 'then re-added');
  assert.equal((await stat(dest)).mode & 0o777, 0o600);
});

test('unregister removes the server from configured agents and --purge deletes the generated config', async t => {
  const { dest, base } = await workspace(t, webConfig(['claude', 'codex']));
  await runSetup({ ...base, run: fakeRunner(['claude', 'codex']).run }); // create dest
  const { run, calls } = fakeRunner(['claude', 'codex']);
  await runUnregister({ ...base, run, purge: true });

  assert.deepEqual(calls.find(c => c.command === 'claude' && c.args[1] === 'remove'),
    { command: 'claude', args: ['mcp', 'remove', 'okc-mcp', '--scope', 'user'] });
  assert.deepEqual(calls.find(c => c.command === 'codex' && c.args[1] === 'remove'),
    { command: 'codex', args: ['mcp', 'remove', 'okc-mcp'] });
  await assert.rejects(stat(dest), 'purge removed the generated config');
});

import assert from 'node:assert/strict';
import { createServer as httpServer, type IncomingMessage, type ServerResponse } from 'node:http';
import { once } from 'node:events';
import { mkdir, mkdtemp, readFile, readdir, rm, symlink, writeFile } from 'node:fs/promises';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test, { type TestContext } from 'node:test';
import fc from 'fast-check';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { InMemoryTransport } from '@modelcontextprotocol/sdk/inMemory.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';
import { configSchema, type Config } from '../src/config.js';
import { createServer } from '../src/server.js';
import { initializeSourceVault } from '../src/setup.js';
import { Vault, VaultError } from '../src/vault.js';
import { validateWebPath, WebVault } from '../src/web.js';

const R1 = 'a'.repeat(64);
const R2 = 'b'.repeat(64);
const identity = (revision = R1) => ({ project_id: 'project-1', revision, status: 'stale', stale: true });
const code = (value: string) => (error: unknown) => error instanceof VaultError && error.code === value;
const json = (response: ServerResponse, value: unknown) => { response.setHeader('content-type', 'application/json'); response.end(JSON.stringify(value)); };

async function remote(t: TestContext, handler?: (request: IncomingMessage, response: ServerResponse, url: URL) => boolean) {
  const requests: { url: URL; authorization: string | undefined }[] = [];
  const server = httpServer((request, response) => {
    const url = new URL(request.url!, 'http://localhost');
    requests.push({ url, authorization: request.headers.authorization });
    if (handler?.(request, response, url)) return;
    const revision = url.searchParams.get('revision') ?? R1;
    if (url.pathname.endsWith('/contract')) json(response, identity(revision));
    else if (url.pathname.endsWith('/files')) json(response, { ...identity(revision), files: ['knowledge/지식 #?.md', 'legacy/두번째.md', '.okc/manifest.json'] });
    else if (url.pathname.endsWith('/verify')) json(response, { ...identity(revision), valid: true });
    else if (url.pathname.endsWith('/explain')) json(response, { ...identity(revision), record: { sources: ['source-1'], contradictions: ['A', 'B'] } });
    else response.end(`# ${url.searchParams.get('path')}\n\n통합 지식 ${revision}\n`);
  });
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  t.after(async () => { server.closeAllConnections(); await new Promise<void>((resolve, reject) => server.close(error => error ? reject(error) : resolve())); });
  const address = server.address();
  assert.ok(address && typeof address !== 'string');
  return { baseUrl: `http://127.0.0.1:${address.port}`, requests };
}

async function connect(t: TestContext, config: Config, vault?: Vault) {
  const server = createServer(config, vault);
  const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
  const client = new Client({ name: 'knowledge-test', version: '1' });
  await server.connect(serverTransport);
  await client.connect(clientTransport);
  t.after(async () => { await client.close(); await server.close(); });
  return client;
}

async function call(client: Client, name: string, args: Record<string, unknown> = {}) {
  const result = await client.callTool({ name, arguments: args });
  return result.structuredContent as { ok: boolean; data: Record<string, any>; error?: { code: string; message: string } };
}

test('web-only config requires read-only; URL credentials, unknown fields and incomplete local targets are rejected', () => {
  const web = { baseUrl: 'https://okc.example.test', projectId: 'project-1' };
  assert.equal(configSchema.parse({ web, readOnly: true }).vaultPath, '');
  const parsed = configSchema.parse({ web, readOnly: true });
  assert.deepEqual(configSchema.parse(JSON.parse(JSON.stringify(parsed))), parsed);
  for (const input of [{ web }, {}, { web, vaultPath: '/vault', readOnly: true },
    ...['file:///vault', 'https://name:password@okc.test', 'https://okc.test?token=secret', 'https://okc.test#fragment']
      .map(baseUrl => ({ readOnly: true, web: { ...web, baseUrl } })),
    { readOnly: true, web: { ...web, projectId: '../project' } },
    { readOnly: true, web: { ...web, token: 'bad\nheader' } }]) {
    assert.equal(configSchema.safeParse(input).success, false, JSON.stringify(input));
  }
});

test('web-only JSON config starts the real stdio CLI and doctor without a local Vault', async t => {
  const fixture = await remote(t);
  const directory = await mkdtemp(path.join(tmpdir(), 'okc-web-cli-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const configPath = path.join(directory, 'mcp.json');
  const config = configSchema.parse({ readOnly: true, web: { baseUrl: fixture.baseUrl, projectId: 'project-1', token: 'private-read-token' } });
  await writeFile(configPath, JSON.stringify(config));
  const projectRoot = fileURLToPath(new URL('../', import.meta.url));
  const cli = path.join(projectRoot, 'src/cli.ts');
  const doctor = await promisify(execFile)(process.execPath, ['--import', 'tsx', cli, 'doctor', '--config', configPath], { cwd: projectRoot });
  const diagnostics = JSON.parse(doctor.stdout);
  assert.equal(diagnostics.ok, true);
  assert.equal(diagnostics.defaultSource, 'web');
  assert.ok(!doctor.stdout.includes('private-read-token'));
  const client = new Client({ name: 'web-stdio-test', version: '1' });
  const transport = new StdioClientTransport({ command: process.execPath,
    args: ['--import', 'tsx', cli, 'serve', '--config', configPath], cwd: projectRoot });
  await client.connect(transport);
  t.after(() => client.close());
  assert.equal((await call(client, 'read_note', { path: 'knowledge/note.md' })).data.source.kind, 'web');
});

test('web is default, pins all reads, preserves provenance/stale and supports web-only sessions', async t => {
  const fixture = await remote(t);
  const config = configSchema.parse({ readOnly: true, web: { baseUrl: fixture.baseUrl, projectId: 'project-1', token: 'read-token' } });
  const client = await connect(t, config);
  const result = await call(client, 'search_notes', { query: '통합', revision: R2 });
  assert.equal(result.ok, true);
  assert.equal(result.data.matches.length, 2);
  assert.deepEqual(result.data.source, { kind: 'web', projectId: 'project-1', revision: R2, status: 'stale', stale: true,
    readOnly: true, boundIntegrationPlanId: null, boundCorpusHash: null, boundTaxonomyHash: null });
  assert.ok(result.data.matches.every((match: any) => new URL(match.provenanceUrl).searchParams.get('revision') === R2));
  assert.ok(fixture.requests.every(request => request.authorization === 'Bearer read-token'));
  assert.ok(fixture.requests.every(request => request.url.searchParams.get('revision') === R2));
  const listed = await client.listTools();
  assert.ok(!listed.tools.some(tool => tool.name === 'create_note'));
  assert.ok(listed.tools.some(tool => tool.name === 'explain_note'));
  const explain = await call(client, 'explain_note', { path: 'knowledge/지식 #?.md', revision: R2 });
  assert.deepEqual(explain.data.evidence.record.contradictions, ['A', 'B']);
  assert.equal((await call(client, 'verify_vault')).data.evidence.valid, true);
  assert.equal((await call(client, 'list_notes', { source: 'local' })).error?.code, 'LOCAL_NOT_CONFIGURED');
});

test('web errors never fall back; explicit local reads and authoring remain independent', async t => {
  const fixture = await remote(t, (_request, response) => { response.statusCode = 403; response.end('private remote body'); return true; });
  const directory = await mkdtemp(path.join(tmpdir(), 'okc-web-local-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const initialized = await initializeSourceVault(path.join(directory, 'vault'));
  const config = configSchema.parse({ ...initialized.config, statePath: path.join(directory, 'state'), web: { baseUrl: fixture.baseUrl, projectId: 'project-1' } });
  const vault = new Vault(config);
  await vault.initialize();
  await vault.create('notes/local.md', '# Local knowledge');
  const client = await connect(t, config, vault);
  const failed = await call(client, 'search_notes', { query: 'Local' });
  assert.equal(failed.error?.code, 'WEB_AUTH_REQUIRED');
  assert.ok(!failed.error.message.includes('private remote body'));
  const local = await call(client, 'read_note', { source: 'local', path: 'notes/local.md' });
  assert.equal(local.data.content, '# Local knowledge');
  assert.equal(local.data.source.kind, 'local');
  const written = await call(client, 'update_note', { path: 'notes/local.md', expectedHash: local.data.sha256,
    changes: { body: '# Changed locally' }, dryRun: false });
  assert.equal(written.ok, true);
  assert.equal(written.data.source.kind, 'local');
  assert.equal(await readFile(path.join(config.vaultPath, 'notes/local.md'), 'utf8'), '# Changed locally');
  assert.equal(fixture.requests.length, 1, 'authoring/local reads do not contact web');
});

test('absent web selects local; requesting unconfigured web returns a typed error', async t => {
  const directory = await mkdtemp(path.join(tmpdir(), 'okc-local-default-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const initialized = await initializeSourceVault(path.join(directory, 'vault'));
  const config = { ...initialized.config, statePath: path.join(directory, 'state') };
  const vault = new Vault(config);
  await vault.initialize();
  await vault.create('notes/local.md', '# Local');
  const client = await connect(t, config, vault);
  assert.equal((await call(client, 'list_notes')).data.source.kind, 'local');
  assert.equal((await call(client, 'list_notes', { source: 'web' })).error?.code, 'WEB_NOT_CONFIGURED');
});

test('a list revision mismatch rejects the entire operation before file reads', async t => {
  const fixture = await remote(t, (_request, response, url) => {
    if (!url.pathname.endsWith('/files')) return false;
    json(response, { ...identity(R2), files: ['knowledge/note.md'] }); return true;
  });
  const client = new WebVault(configSchema.parse({ readOnly: true, web: { baseUrl: fixture.baseUrl, projectId: 'project-1' } }));
  await assert.rejects((await client.snapshot()).list(), code('WEB_REVISION_MISMATCH'));
  assert.equal(fixture.requests.length, 2);
});

test('reader accepts core legacy spelling, uppercase Markdown, NFD and deep paths without changing traversal rules', async t => {
  const files = ['legacy/source/.notes/Cafe\u0301.MD', 'legacy/source/long.markdown',
    `legacy/source/${'folder/'.repeat(24)}note.md`, 'legacy/source/unsupported.canvas'];
  const fixture = await remote(t, (_request, response, url) => {
    if (!url.pathname.endsWith('/files')) return false;
    json(response, { ...identity(), files }); return true;
  });
  const client = new WebVault(configSchema.parse({ readOnly: true, web: { baseUrl: fixture.baseUrl, projectId: 'project-1' } }));
  const snapshot = await client.snapshot();
  assert.deepEqual((await snapshot.list()).notes, files.slice(0, 3).sort());
  assert.deepEqual((await snapshot.list()).otherFiles, [files[3]]);
  await snapshot.read(files[0]!);
  assert.equal(fixture.requests.at(-1)!.url.searchParams.get('path'), files[0]);
});

test('malformed contracts, wrong project identity and unsafe remote file lists are rejected', async t => {
  for (const body of ['not json', JSON.stringify({ ...identity(), project_id: 'other-project' })]) {
    const fixture = await remote(t, (_request, response) => { response.end(body); return true; });
    const client = new WebVault(configSchema.parse({ readOnly: true, web: { baseUrl: fixture.baseUrl, projectId: 'project-1' } }));
    await assert.rejects(client.snapshot(), code('WEB_CONTRACT_INVALID'));
  }
  const fixture = await remote(t, (_request, response, url) => {
    if (!url.pathname.endsWith('/files')) return false;
    json(response, { ...identity(), files: ['knowledge/../../secret.md'] }); return true;
  });
  const client = new WebVault(configSchema.parse({ readOnly: true, web: { baseUrl: fixture.baseUrl, projectId: 'project-1' } }));
  await assert.rejects((await client.snapshot()).list(), code('INVALID_PATH'));
});

test('remote 404, 409, redirect, invalid metadata, and streamed oversized bodies fail with bounded errors', async t => {
  for (const [status, expected] of [[404, 'WEB_NOT_FOUND'], [409, 'WEB_REVISION_MISMATCH'], [302, 'WEB_REDIRECT_REFUSED']] as const) {
    const fixture = await remote(t, (_request, response) => { response.statusCode = status; response.setHeader('location', 'https://not-contacted.invalid'); response.end(); return true; });
    const client = new WebVault(configSchema.parse({ readOnly: true, web: { baseUrl: fixture.baseUrl, projectId: 'project-1' } }));
    await assert.rejects(client.snapshot(), code(expected));
    assert.equal(fixture.requests.length, 1);
  }
  const fixture = await remote(t, (_request, response, url) => {
    if (!url.pathname.endsWith('/file')) return false;
    response.write('x'.repeat(700)); response.end('x'.repeat(700)); return true;
  });
  const config = configSchema.parse({ readOnly: true, maxNoteBytes: 1024, web: { baseUrl: fixture.baseUrl, projectId: 'project-1' } });
  await assert.rejects((await new WebVault(config).snapshot()).read('knowledge/note.md'), code('RESPONSE_LIMIT'));
});

test('timeouts and abort are distinguished and do not leak response bodies or tokens', async t => {
  const fixture = await remote(t, () => true);
  const client = new WebVault(configSchema.parse({ readOnly: true, web: { baseUrl: fixture.baseUrl, projectId: 'project-1', timeoutMs: 100, token: 'secret' } }));
  await assert.rejects(client.snapshot(), code('WEB_TIMEOUT'));
  await assert.rejects(client.snapshot(AbortSignal.abort()), code('WEB_CANCELLED'));
});

test('snapshot caching is scoped per operation and total scanned bytes stay bounded', async t => {
  const fixture = await remote(t, (_request, response, url) => {
    if (!url.pathname.endsWith('/file')) return false;
    response.end('a'.repeat(600)); return true;
  });
  const client = new WebVault(configSchema.parse({ readOnly: true, maxScanBytes: 1024, web: { baseUrl: fixture.baseUrl, projectId: 'project-1' } }));
  const snapshot = await client.snapshot();
  await snapshot.read('knowledge/one.md');
  await snapshot.read('knowledge/one.md');
  assert.equal(fixture.requests.filter(request => request.url.pathname.endsWith('/file')).length, 1);
  await assert.rejects(snapshot.read('knowledge/two.md'), code('SCAN_LIMIT'));
  await (await client.snapshot()).read('knowledge/one.md');
  assert.equal(fixture.requests.filter(request => request.url.pathname.endsWith('/file')).length, 3);
});

test('source initialization is explicit, empty, conventional, and refuses existing/compiled/symlink destinations', async t => {
  const directory = await mkdtemp(path.join(tmpdir(), 'okc-source-init-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const target = path.join(directory, 'source');
  const initialized = await initializeSourceVault(target);
  assert.deepEqual(await readdir(target), ['inbox', 'maps', 'notes', 'sources']);
  for (const folder of initialized.folders) assert.deepEqual(await readdir(path.join(target, folder)), []);
  await assert.rejects(initializeSourceVault(target), code('NOTE_EXISTS'));
  const alias = path.join(directory, 'alias');
  await symlink(target, alias);
  await assert.rejects(initializeSourceVault(path.join(alias, 'forbidden')), code('INVALID_PATH'));
  await mkdir(path.join(directory, '.okc'));
  await assert.rejects(initializeSourceVault(path.join(directory, 'forbidden')), code('IMMUTABLE_TARGET'));
});

test('PBT: generated Unicode paths round-trip as query data without changing fixed origin/path', () => {
  const client = new WebVault(configSchema.parse({ readOnly: true, web: { baseUrl: 'https://okc.example.test', projectId: 'project-1' } }));
  const segment = fc.array(fc.constantFrom('한글', 'Emoji😀', '#fragment', '?query', '%2fencoded', 'space name'), { minLength: 1, maxLength: 10 }).map(parts => parts.join(''));
  fc.assert(fc.property(segment, name => {
    const value = `knowledge/${name}.md`;
    assert.equal(validateWebPath(value), value);
    const url = new URL(client.url('file', { path: value, revision: R1 }));
    assert.equal(url.origin, 'https://okc.example.test');
    assert.equal(url.pathname, '/api/serving/project-1/file');
    assert.equal(url.searchParams.get('path'), value);
  }), { seed: 98765, numRuns: 200 });
  for (const value of ['../private.md', 'knowledge/../private.md', 'https://other.test/file.md', 'knowledge/./secret.md', 'knowledge/back\\slash.md', 'knowledge/bad\u0000.md']) {
    assert.throws(() => validateWebPath(value), code('INVALID_PATH'));
  }
});

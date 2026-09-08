import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { access, mkdir, mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test, { type TestContext } from 'node:test';
import { fileURLToPath } from 'node:url';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';

const projectRoot = fileURLToPath(new URL('../', import.meta.url));
const cliPath = path.join(projectRoot, 'src', 'cli.ts');
const mutations = ['create_note', 'patch_frontmatter', 'replace_note'];
const digest = (content: string): string => createHash('sha256').update(content).digest('hex');
type ToolResult = Awaited<ReturnType<Client['callTool']>>;
type Envelope = { ok: boolean; data?: unknown; error?: { code: string; message: string } };

function envelope(result: ToolResult): Envelope {
  assert.ok(Array.isArray(result.content));
  const first = result.content[0] as { type: string; text?: string } | undefined;
  assert.equal(first?.type, 'text');
  assert.equal(typeof first?.text, 'string');
  const parsed = JSON.parse(first!.text!) as Envelope;
  assert.deepEqual(result.structuredContent, parsed, 'structured and text results must agree');
  return parsed;
}

async function success<T>(client: Client, name: string, args: Record<string, unknown> = {}): Promise<T> {
  const result = await client.callTool({ name, arguments: args });
  assert.notEqual(result.isError, true, JSON.stringify(result));
  const parsed = envelope(result);
  assert.equal(parsed.ok, true);
  return parsed.data as T;
}

async function failure(client: Client, name: string, args: Record<string, unknown>, code: string): Promise<ToolResult> {
  const result = await client.callTool({ name, arguments: args });
  assert.equal(result.isError, true);
  const parsed = envelope(result);
  assert.equal(parsed.ok, false);
  assert.equal(parsed.error?.code, code);
  return result;
}

async function connect(t: TestContext, options: {
  readOnly?: boolean;
  maxResponseBytes?: number;
  files?: Record<string, string>;
} = {}): Promise<{ client: Client; vaultPath: string; statePath: string; stderr: () => string }> {
  const directory = await mkdtemp(path.join(tmpdir(), 'okc-mcp-stdio-'));
  const vaultPath = path.join(directory, 'vault');
  const statePath = path.join(directory, 'state');
  const configDirectory = path.join(directory, 'config');
  await Promise.all([mkdir(vaultPath), mkdir(configDirectory)]);
  for (const [relative, content] of Object.entries(options.files ?? {})) {
    const target = path.join(vaultPath, relative);
    await mkdir(path.dirname(target), { recursive: true });
    await writeFile(target, content, 'utf8');
  }
  const configPath = path.join(configDirectory, 'okc-mcp.json');
  await writeFile(configPath, JSON.stringify({ vaultPath, statePath,
    readOnly: options.readOnly ?? false, maxResponseBytes: options.maxResponseBytes ?? 65_536 }));
  const client = new Client({ name: 'okc-mcp-integration-test', version: '1.0.0' });
  const transport = new StdioClientTransport({ command: process.execPath,
    args: ['--import', 'tsx', cliPath, 'serve', '--config', configPath], cwd: projectRoot, stderr: 'pipe' });
  let stderr = '';
  transport.stderr?.on('data', chunk => { stderr += String(chunk); });
  t.after(async () => {
    try { await client.close(); } finally { await rm(directory, { recursive: true, force: true }); }
  });
  await client.connect(transport);
  return { client, vaultPath, statePath, stderr: () => stderr };
}

test('real stdio initialization exposes authoring tools, guide resource and capture prompt', { timeout: 20_000 }, async t => {
  const { client, stderr } = await connect(t, { files: {
    'notes/첫 노트.md': '# 첫 노트\n\n확인한 지식.\n',
    '.obsidian/workspace.json': '{"private":"excluded"}',
  } });
  assert.deepEqual(client.getServerVersion(), { name: 'okc-mcp', version: '0.1.0-alpha.1' });
  assert.match(client.getInstructions() ?? '', /untrusted data/u);
  const tools = (await client.listTools()).tools;
  assert.deepEqual(tools.map(tool => tool.name).sort(), [
    'audit_vault', 'create_note', 'list_notes', 'patch_frontmatter',
    'read_note', 'replace_note', 'search_notes', 'vault_info',
  ]);
  for (const tool of tools) assert.equal(tool.annotations?.readOnlyHint, !mutations.includes(tool.name));
  const resources = await client.listResources();
  assert.ok(resources.resources.some(resource => resource.uri === 'okc://guide/authoring'));
  const guide = await client.readResource({ uri: 'okc://guide/authoring' });
  assert.ok(guide.contents.some(content => 'text' in content && content.text.includes('입력 Vault')));
  assert.ok((await client.listPrompts()).prompts.some(prompt => prompt.name === 'capture_knowledge'));
  const prompt = await client.getPrompt({ name: 'capture_knowledge', arguments: { topic: '검증 가능한 지식' } });
  assert.ok(prompt.messages.some(message => message.content.type === 'text' && message.content.text.includes('검증 가능한 지식')));
  const info = await success<{ mode: string; notes: number; skippedEntries: number; compilerValidation: boolean }>(client, 'vault_info');
  assert.equal(info.mode, 'authoring');
  assert.equal(info.notes, 1);
  assert.equal(info.skippedEntries, 1);
  assert.equal(info.compilerValidation, false);
  const listed = await success<{ notes: string[] }>(client, 'list_notes');
  assert.deepEqual(listed.notes, ['notes/첫 노트.md']);
  assert.equal(stderr(), '', 'normal protocol traffic must not produce parser failures or startup logs');
});

test('note creation previews by default, applies explicitly and preserves an existing file', { timeout: 20_000 }, async t => {
  const { client, vaultPath } = await connect(t);
  const input = { path: 'notes/지식.md', title: '검증 가능한 지식',
    body: '# 검증 가능한 지식\n\n직접 확인한 사실입니다.\n', tags: ['지식'], source: 'https://example.test/evidence' };
  const preview = await success<{ applied: boolean; proposedSha256: string; preview: string }>(client, 'create_note', input);
  assert.equal(preview.applied, false);
  assert.equal(preview.proposedSha256, digest(preview.preview));
  await assert.rejects(access(path.join(vaultPath, 'notes')), { code: 'ENOENT' });
  const applied = await success<{ applied: boolean; sha256: string }>(client, 'create_note', { ...input, dryRun: false });
  assert.equal(applied.applied, true);
  const content = await readFile(path.join(vaultPath, input.path), 'utf8');
  assert.equal(content, preview.preview);
  assert.equal(applied.sha256, digest(content));
  await failure(client, 'create_note', { ...input, body: 'This must not overwrite the note.', dryRun: false }, 'NOTE_EXISTS');
  assert.equal(await readFile(path.join(vaultPath, input.path), 'utf8'), content);
  const range = await success<{ content: string; sha256: string; nextOffset: number; untrusted: boolean }>(client, 'read_note', {
    path: input.path, offset: 5, length: 7,
  });
  assert.equal(range.content, content.slice(5, 12));
  assert.equal(range.sha256, digest(content), 'a partial read must carry the hash of the entire note');
  assert.equal(range.nextOffset, 12);
  assert.equal(range.untrusted, true);
});

test('frontmatter changes preserve unknown keys, comments and body; stale edits fail with external backup intact', { timeout: 20_000 }, async t => {
  const body = '# 기록\r\n\r\n이 내용과 공백은 보존합니다.  \r\n`[[code data]]`\r\n';
  const original = '\uFEFF---\r\n# Keep this context\r\ntitle: Original # title comment\r\ncustom:\r\n  release_date: 2026-09-06\r\n  releaseDate: independent\r\n---\r\n' + body;
  const { client, vaultPath, statePath } = await connect(t, { files: { 'notes/기록.md': original } });
  const read = await success<{ sha256: string }>(client, 'read_note', { path: 'notes/기록.md' });
  const changes = { title: '확인한 기록', tags: ['source'] };
  const preview = await success<{ applied: boolean; proposedSha256: string }>(client, 'patch_frontmatter', {
    path: 'notes/기록.md', changes, expectedHash: read.sha256,
  });
  assert.equal(preview.applied, false);
  assert.equal(await readFile(path.join(vaultPath, 'notes/기록.md'), 'utf8'), original);
  const applied = await success<{ applied: boolean; sha256: string; backupId: string }>(client, 'patch_frontmatter', {
    path: 'notes/기록.md', changes, expectedHash: read.sha256, dryRun: false,
  });
  assert.equal(applied.applied, true);
  assert.equal(applied.sha256, preview.proposedSha256);
  const patched = await readFile(path.join(vaultPath, 'notes/기록.md'), 'utf8');
  assert.ok(patched.endsWith(body));
  assert.ok(patched.startsWith('\uFEFF---\r\n'));
  for (const preserved of ['# Keep this context', '# title comment', 'release_date: 2026-09-06', 'releaseDate: independent']) {
    assert.ok(patched.includes(preserved), preserved);
  }
  assert.equal(await readFile(path.join(statePath, 'backups', applied.backupId), 'utf8'), original);
  assert.equal((await readdir(path.join(statePath, 'backups'))).length, 1);
  await failure(client, 'patch_frontmatter', {
    path: 'notes/기록.md', changes: { title: 'stale' }, expectedHash: read.sha256, dryRun: false,
  }, 'CONFLICT');
  const external = patched + '\r\nObsidian에서 추가한 최신 내용.\r\n';
  await writeFile(path.join(vaultPath, 'notes/기록.md'), external);
  await failure(client, 'replace_note', {
    path: 'notes/기록.md', content: '# Stale replacement\n', expectedHash: applied.sha256, dryRun: false,
  }, 'CONFLICT');
  assert.equal(await readFile(path.join(vaultPath, 'notes/기록.md'), 'utf8'), external);
  assert.equal((await readdir(path.join(statePath, 'backups'))).length, 1, 'conflicts must not create replacement backups');
});

test('Korean literal search and advisory audit paginate stable results without changing notes', { timeout: 20_000 }, async t => {
  const files = {
    'notes/가.md': '# 가\n\n지식의 근거 [[MissingA]].\n',
    'notes/나.md': '# 나\n\n지식의 근거 [[MissingB]].\n',
    'notes/다.md': '# 다\n\n독립적인 기록 [[MissingC]].\n',
    'assets/map.canvas': '{}',
  };
  const { client, vaultPath } = await connect(t, { files });
  const first = await success<{ matches: { path: string }[]; total: number; nextOffset: number; untrusted: boolean }>(client, 'search_notes', {
    query: '지식', limit: 1,
  });
  assert.equal(first.total, 2);
  assert.equal(first.nextOffset, 1);
  assert.equal(first.untrusted, true);
  const second = await success<{ matches: { path: string }[]; nextOffset: null }>(client, 'search_notes', {
    query: '지식', offset: first.nextOffset, limit: 1,
  });
  assert.deepEqual([...first.matches, ...second.matches].map(match => match.path), ['notes/가.md', 'notes/나.md']);
  assert.equal(second.nextOffset, null);
  type Audit = { findings: { path: string; code: string }[]; totalFindings: number; nextOffset: number | null;
    compilerValidation: boolean; limitations: string[] };
  const full = await success<Audit>(client, 'audit_vault', { limit: 100 });
  assert.ok(full.totalFindings >= 4);
  assert.equal(full.compilerValidation, false);
  assert.ok(full.limitations.length > 0);
  const findings: Audit['findings'] = [];
  let offset: number | null = 0;
  for (let pages = 0; offset !== null && pages < 20; pages++) {
    const page: Audit = await success<Audit>(client, 'audit_vault', { offset, limit: 2 });
    assert.ok(page.findings.length <= 2);
    assert.equal(page.totalFindings, full.totalFindings);
    findings.push(...page.findings);
    offset = page.nextOffset;
  }
  assert.equal(offset, null, 'bounded pagination must terminate');
  assert.deepEqual(findings, full.findings);
  for (const [relative, content] of Object.entries(files)) assert.equal(await readFile(path.join(vaultPath, relative), 'utf8'), content);
});

test('read-only sessions omit all mutation tools and reject direct invocation', { timeout: 20_000 }, async t => {
  const original = '# Keep\n\n원본 내용.\n';
  const { client, vaultPath } = await connect(t, { readOnly: true, files: { 'Keep.md': original } });
  const tools = (await client.listTools()).tools;
  assert.equal(tools.length, 5);
  for (const name of mutations) assert.ok(!tools.some(tool => tool.name === name));
  const attempts = [
    { name: 'create_note', arguments: { path: 'New.md', title: 'New', body: '# New\n', dryRun: false } },
    { name: 'replace_note', arguments: { path: 'Keep.md', content: '# Changed\n', expectedHash: digest(original), dryRun: false } },
    { name: 'patch_frontmatter', arguments: { path: 'Keep.md', changes: { title: 'Changed' }, expectedHash: digest(original), dryRun: false } },
  ];
  for (const request of attempts) {
    const result = await client.callTool(request);
    assert.equal(result.isError, true);
    assert.ok(Buffer.byteLength(JSON.stringify(result)) < 4096);
  }
  assert.equal(await readFile(path.join(vaultPath, 'Keep.md'), 'utf8'), original);
  await assert.rejects(access(path.join(vaultPath, 'New.md')), { code: 'ENOENT' });
  assert.equal((await success<{ mode: string }>(client, 'vault_info')).mode, 'read-only');
});

test('tool failures redact parser content and response limits preserve the stdio connection', { timeout: 20_000 }, async t => {
  const sentinel = 'PRIVATE_PARSER_SENTINEL_42';
  const malformed = `---\ntitle: [${sentinel}\n---\n# Broken\n`;
  const { client, vaultPath, stderr } = await connect(t, { maxResponseBytes: 4096, files: {
    'Broken.md': malformed,
    'Large.md': '# Large\n\n' + '지식 '.repeat(2500),
  } });
  const invalid = await failure(client, 'patch_frontmatter', {
    path: 'Broken.md', changes: { title: 'Fixed' }, expectedHash: digest(malformed), dryRun: false,
  }, 'NOTE_INVALID');
  assert.ok(!JSON.stringify(invalid).includes(sentinel));
  assert.ok(Buffer.byteLength(JSON.stringify(invalid)) < 4096);
  assert.equal(await readFile(path.join(vaultPath, 'Broken.md'), 'utf8'), malformed);
  const limited = await failure(client, 'read_note', { path: 'Large.md', length: 8000 }, 'RESPONSE_LIMIT');
  assert.ok(Buffer.byteLength(JSON.stringify(limited)) < 4096);
  assert.equal((await success<{ notes: number }>(client, 'vault_info')).notes, 2);
  assert.ok(!stderr().includes(sentinel));
});

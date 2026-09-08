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
const mutations = ['create_note', 'update_note', 'standardize_frontmatter', 'fix_yaml', 'reinforce_sources_links'];
const readTools = ['audit_vault', 'list_backlinks', 'list_notes', 'outline_note', 'read_note', 'search_notes'];
const digest = (content: string): string => createHash('sha256').update(content).digest('hex');
type ToolResult = Awaited<ReturnType<Client['callTool']>>;
type Envelope = { ok: boolean; data?: unknown; error?: { kind?: string; code: string; message: string } };

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

async function failure(client: Client, name: string, args: Record<string, unknown>, code: string, kind?: string): Promise<ToolResult> {
  const result = await client.callTool({ name, arguments: args });
  assert.equal(result.isError, true);
  const parsed = envelope(result);
  assert.equal(parsed.ok, false);
  assert.equal(parsed.error?.code, code);
  if (kind !== undefined) assert.equal(parsed.error?.kind, kind, `expected canonical kind ${kind}`);
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

test('initialization exposes exactly the design tool surface, guide resource and capture prompt (US-IN-06)', { timeout: 20_000 }, async t => {
  const { client, stderr } = await connect(t, { files: {
    'notes/첫 노트.md': '# 첫 노트\n\n확인한 지식.\n',
    '.obsidian/workspace.json': '{"private":"excluded"}',
  } });
  assert.deepEqual(client.getServerVersion(), { name: 'okc-mcp', version: '0.1.0-alpha.1' });
  assert.match(client.getInstructions() ?? '', /untrusted data/u);
  const tools = (await client.listTools()).tools;
  assert.deepEqual(tools.map(tool => tool.name).sort(), [...readTools, ...mutations].sort());
  // No shell/http/delete/approve/AI tool exists (REQ-011 / BR-TRUST-1).
  for (const forbidden of ['delete_note', 'run_shell', 'http_get', 'approve', 'invoke_ai']) {
    assert.ok(!tools.some(tool => tool.name === forbidden), forbidden);
  }
  for (const tool of tools) assert.equal(tool.annotations?.readOnlyHint, !mutations.includes(tool.name));
  const resources = await client.listResources();
  assert.ok(resources.resources.some(resource => resource.uri === 'okc://guide/authoring'));
  const guide = await client.readResource({ uri: 'okc://guide/authoring' });
  assert.ok(guide.contents.some(content => 'text' in content && content.text.includes('입력 Vault')));
  assert.ok((await client.listPrompts()).prompts.some(prompt => prompt.name === 'capture_knowledge'));
  const prompt = await client.getPrompt({ name: 'capture_knowledge', arguments: { topic: '검증 가능한 지식' } });
  assert.ok(prompt.messages.some(message => message.content.type === 'text' && message.content.text.includes('검증 가능한 지식')));
  const listed = await success<{ notes: string[] }>(client, 'list_notes');
  assert.deepEqual(listed.notes, ['notes/첫 노트.md']);
  assert.equal(stderr(), '', 'normal protocol traffic must not produce parser failures or startup logs');
});

test('create_note previews by default, applies explicitly, refuses overwrite; read returns full hash (US-AU-06, US-IN-08, US-RV-01)', { timeout: 20_000 }, async t => {
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
  await failure(client, 'create_note', { ...input, body: 'This must not overwrite the note.', dryRun: false }, 'NOTE_EXISTS', 'overwrite-refused');
  assert.equal(await readFile(path.join(vaultPath, input.path), 'utf8'), content);
  const range = await success<{ content: string; sha256: string; nextOffset: number; untrusted: boolean }>(client, 'read_note', {
    path: input.path, offset: 5, length: 7,
  });
  assert.equal(range.content, content.slice(5, 12));
  assert.equal(range.sha256, digest(content), 'a partial read must carry the hash of the entire note');
  assert.equal(range.untrusted, true);
});

test('standardize_frontmatter preserves unknown keys/comments/BOM/body; conflict and backup behavior (US-AU-02/05, US-RV-02/03/05)', { timeout: 20_000 }, async t => {
  const body = '# 기록\r\n\r\n이 내용과 공백은 보존합니다.  \r\n`[[code data]]`\r\n';
  const original = '\uFEFF---\r\n# Keep this context\r\ntitle: Original # title comment\r\ncustom:\r\n  release_date: 2026-09-06\r\n  releaseDate: independent\r\n---\r\n' + body;
  const { client, vaultPath, statePath } = await connect(t, { files: { 'notes/기록.md': original } });
  const read = await success<{ sha256: string }>(client, 'read_note', { path: 'notes/기록.md' });
  const frontmatterPatch = { title: '확인한 기록', tags: ['source'] };
  const preview = await success<{ applied: boolean; proposedSha256: string; changedKeys: string[] }>(client, 'standardize_frontmatter', {
    path: 'notes/기록.md', frontmatterPatch, expectedHash: read.sha256,
  });
  assert.equal(preview.applied, false);
  assert.deepEqual(preview.changedKeys.sort(), ['tags', 'title']);
  assert.equal(await readFile(path.join(vaultPath, 'notes/기록.md'), 'utf8'), original);
  const applied = await success<{ applied: boolean; sha256: string; backupId: string }>(client, 'standardize_frontmatter', {
    path: 'notes/기록.md', frontmatterPatch, expectedHash: read.sha256, dryRun: false,
  });
  assert.equal(applied.applied, true);
  assert.equal(applied.sha256, preview.proposedSha256);
  const patched = await readFile(path.join(vaultPath, 'notes/기록.md'), 'utf8');
  assert.ok(patched.endsWith(body));
  assert.ok(patched.startsWith('\uFEFF---\r\n'));
  for (const preserved of ['# Keep this context', '# title comment', 'release_date: 2026-09-06', 'releaseDate: independent']) {
    assert.ok(patched.includes(preserved), preserved);
  }
  // Exactly one external pre-change backup, with a source-traceable sidecar.
  assert.equal(await readFile(path.join(statePath, 'backups', applied.backupId), 'utf8'), original);
  const meta = JSON.parse(await readFile(path.join(statePath, 'backups', `${applied.backupId}.meta.json`), 'utf8')) as { sourcePath: string };
  assert.equal(meta.sourcePath, 'notes/기록.md');
  const backupFiles = (await readdir(path.join(statePath, 'backups'))).filter(name => name.endsWith('.md'));
  assert.equal(backupFiles.length, 1);
  // Stale hash rejected as a conflict; file untouched; no replacement backup.
  await failure(client, 'standardize_frontmatter', {
    path: 'notes/기록.md', frontmatterPatch: { title: 'stale' }, expectedHash: read.sha256, dryRun: false,
  }, 'CONFLICT', 'hash-mismatch');
  assert.equal((await readdir(path.join(statePath, 'backups'))).filter(name => name.endsWith('.md')).length, 1);
});

test('update_note applies combined body+frontmatter change through the one pipeline (US-AU-05)', { timeout: 20_000 }, async t => {
  const original = '---\ntitle: Draft\n---\n# Draft\n\nfirst.\n';
  const { client, vaultPath } = await connect(t, { files: { 'n.md': original } });
  const { sha256 } = await success<{ sha256: string }>(client, 'read_note', { path: 'n.md' });
  const applied = await success<{ applied: boolean; sha256: string; backupId: string }>(client, 'update_note', {
    path: 'n.md', expectedHash: sha256, changes: { body: '# Draft\n\nrevised.\n', frontmatter: { tags: ['done'] } }, dryRun: false,
  });
  assert.equal(applied.applied, true);
  const content = await readFile(path.join(vaultPath, 'n.md'), 'utf8');
  assert.ok(content.includes('revised.'));
  assert.ok(content.includes('tags:'));
  assert.equal(applied.sha256, digest(content));
});

test('fix_yaml repairs malformed frontmatter in place and rejects still-invalid corrections (US-AU-03)', { timeout: 20_000 }, async t => {
  const malformed = '---\ntitle: [unterminated\ncustom: keep\n---\n# Body\n\n본문 보존.\n';
  const { client, vaultPath } = await connect(t, { files: { 'broken.md': malformed } });
  const { sha256 } = await success<{ sha256: string }>(client, 'read_note', { path: 'broken.md' });
  // A still-malformed correction is refused; the file is unchanged.
  await failure(client, 'fix_yaml', { path: 'broken.md', expectedHash: sha256, correctedFrontmatter: 'title: [still bad', dryRun: false }, 'NOTE_INVALID', 'malformed-yaml');
  assert.equal(await readFile(path.join(vaultPath, 'broken.md'), 'utf8'), malformed);
  const applied = await success<{ applied: boolean }>(client, 'fix_yaml', {
    path: 'broken.md', expectedHash: sha256, correctedFrontmatter: 'title: Fixed\ncustom: keep', dryRun: false,
  });
  assert.equal(applied.applied, true);
  const fixed = await readFile(path.join(vaultPath, 'broken.md'), 'utf8');
  assert.ok(fixed.includes('title: Fixed'));
  assert.ok(fixed.includes('본문 보존.'), 'the body must be preserved');
});

test('reinforce_sources_links adds literal source and body text only (US-AU-04)', { timeout: 20_000 }, async t => {
  const original = '---\ntitle: Claim\n---\n# Claim\n\n주장.\n';
  const { client, vaultPath } = await connect(t, { files: { 'c.md': original } });
  const { sha256 } = await success<{ sha256: string }>(client, 'read_note', { path: 'c.md' });
  const applied = await success<{ applied: boolean }>(client, 'reinforce_sources_links', {
    path: 'c.md', expectedHash: sha256, source: 'https://example.test/evidence', appendBody: '\n출처: https://example.test/evidence\n', dryRun: false,
  });
  assert.equal(applied.applied, true);
  const content = await readFile(path.join(vaultPath, 'c.md'), 'utf8');
  assert.ok(content.includes('source:'));
  assert.ok(content.includes('출처: https://example.test/evidence'));
  assert.ok(content.includes('# Claim'), 'existing body is retained');
});

test('literal Korean search and categorized audit paginate stable results without changing notes (US-AU-01/07)', { timeout: 20_000 }, async t => {
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
  assert.equal(first.untrusted, true);
  const second = await success<{ matches: { path: string }[]; nextOffset: null }>(client, 'search_notes', {
    query: '지식', offset: first.nextOffset, limit: 1,
  });
  assert.deepEqual([...first.matches, ...second.matches].map(match => match.path), ['notes/가.md', 'notes/나.md']);
  type Audit = { findings: { path: string; code: string; category: string }[]; totalFindings: number; nextOffset: number | null;
    heuristic: boolean; compilerValidation: boolean; limitations: string[] };
  const full = await success<Audit>(client, 'audit_vault', { limit: 100 });
  assert.ok(full.totalFindings >= 4);
  assert.equal(full.heuristic, true);
  assert.equal(full.compilerValidation, false);
  // Every finding carries one of the six fixed categories (BR-AUDIT-2).
  const allowed = new Set(['yaml', 'path', 'link', 'duplicate', 'operational-noise', 'unsupported-format']);
  for (const finding of full.findings) assert.ok(allowed.has(finding.category), finding.category);
  assert.ok(full.findings.some(finding => finding.category === 'link'));
  for (const [relative, content] of Object.entries(files)) assert.equal(await readFile(path.join(vaultPath, relative), 'utf8'), content);
});

test('read-only sessions omit every mutation tool and reject direct invocation (US-IN-09)', { timeout: 20_000 }, async t => {
  const original = '# Keep\n\n원본 내용.\n';
  const { client, vaultPath } = await connect(t, { readOnly: true, files: { 'Keep.md': original } });
  const tools = (await client.listTools()).tools;
  assert.equal(tools.length, readTools.length);
  for (const name of mutations) assert.ok(!tools.some(tool => tool.name === name), name);
  const attempts = [
    { name: 'create_note', arguments: { path: 'New.md', title: 'New', body: '# New\n', dryRun: false } },
    { name: 'update_note', arguments: { path: 'Keep.md', expectedHash: digest(original), changes: { body: '# Changed\n' }, dryRun: false } },
  ];
  for (const request of attempts) {
    const result = await client.callTool(request);
    assert.equal(result.isError, true);
  }
  assert.equal(await readFile(path.join(vaultPath, 'Keep.md'), 'utf8'), original);
  await assert.rejects(access(path.join(vaultPath, 'New.md')), { code: 'ENOENT' });
});

test('errors carry a canonical kind, redact parser content, and response limits preserve the connection (US-IN-09, REQ-008)', { timeout: 20_000 }, async t => {
  const sentinel = 'PRIVATE_PARSER_SENTINEL_42';
  const malformed = `---\ntitle: [${sentinel}\n---\n# Broken\n`;
  const { client, vaultPath, stderr } = await connect(t, { maxResponseBytes: 4096, files: {
    'Broken.md': malformed,
    'Large.md': '# Large\n\n' + '지식 '.repeat(2500),
  } });
  const { sha256 } = await success<{ sha256: string }>(client, 'read_note', { path: 'Broken.md', length: 8000 }).catch(async () => {
    // Broken.md is small; a normal read succeeds and returns the whole-file hash.
    return success<{ sha256: string }>(client, 'read_note', { path: 'Broken.md' });
  });
  const invalid = await failure(client, 'standardize_frontmatter', {
    path: 'Broken.md', frontmatterPatch: { title: 'Fixed' }, expectedHash: sha256, dryRun: false,
  }, 'NOTE_INVALID', 'malformed-yaml');
  assert.ok(!JSON.stringify(invalid).includes(sentinel), 'must not echo untrusted source text');
  assert.ok(Buffer.byteLength(JSON.stringify(invalid)) < 4096);
  assert.equal(await readFile(path.join(vaultPath, 'Broken.md'), 'utf8'), malformed);
  const limited = await failure(client, 'read_note', { path: 'Large.md', length: 8000 }, 'RESPONSE_LIMIT', 'bounds-exceeded');
  assert.ok(Buffer.byteLength(JSON.stringify(limited)) < 4096);
  // Connection still healthy after the bounded refusal.
  assert.equal((await success<{ total: number }>(client, 'list_notes')).total, 2);
  assert.ok(!stderr().includes(sentinel));
});

test('search_notes fold matches case-insensitively only when opted in, echoing fold and leaving notes unchanged (REQ-015)', { timeout: 20_000 }, async t => {
  const files = {
    'notes/http.md': '# HTTP Caching\n\nThe HTTP protocol.\n',
    'notes/plain.md': '# Plain\n\n무관한 내용.\n',
  };
  const { client, vaultPath } = await connect(t, { files });
  // Default is codepoint-faithful and case-sensitive: a lowercase query does NOT match 'HTTP'.
  const literal = await success<{ total: number; fold?: boolean }>(client, 'search_notes', { query: 'http caching' });
  assert.equal(literal.total, 0);
  assert.equal(literal.fold, undefined, 'the default response omits the fold field');
  // fold=true matches case-insensitively and echoes fold.
  const folded = await success<{ total: number; matches: { path: string }[]; fold?: boolean }>(client, 'search_notes', { query: 'http caching', fold: true });
  assert.equal(folded.total, 1);
  assert.equal(folded.matches[0]?.path, 'notes/http.md');
  assert.equal(folded.fold, true);
  // Read-only search does not modify notes.
  for (const [relative, content] of Object.entries(files)) assert.equal(await readFile(path.join(vaultPath, relative), 'utf8'), content);
});

test('search_notes description no longer advertises the retired "No regex, folding" promise (D20 honesty gate)', { timeout: 20_000 }, async t => {
  const { client } = await connect(t);
  const search = (await client.listTools()).tools.find(tool => tool.name === 'search_notes');
  assert.ok(search);
  assert.ok(!(search!.description ?? '').includes('No regex, folding'), 'the stale promise must be removed atomically with the fold capability');
  assert.match(search!.description ?? '', /fold=true/u, 'the description documents the new opt-in fold mode');
});

test('outline_note returns a code-fence-aware ATX heading map whose offsets compose with read_note, available read-only (REQ-017)', { timeout: 20_000 }, async t => {
  const content = '---\ntitle: Doc\n---\n# 개요\n\n첫 문단.\n\n## 세부 A\n\n```\n# 코드 안 제목\n```\n\n## 세부 B\n\n본문.\n';
  const { client } = await connect(t, { readOnly: true, files: { 'notes/문서.md': content } });
  type Outline = { path: string; sha256: string; totalCharacters: number; untrusted: boolean;
    headings: { level: number; title: string; start: number; contentStart: number; end: number }[] };
  const outline = await success<Outline>(client, 'outline_note', { path: 'notes/문서.md' });
  assert.equal(outline.untrusted, true);
  assert.equal(outline.sha256, digest(content), 'the outline carries the full-file hash like read_note');
  assert.equal(outline.totalCharacters, content.length);
  assert.deepEqual(outline.headings.map(h => [h.level, h.title]), [[1, '개요'], [2, '세부 A'], [2, '세부 B']]);
  assert.ok(!outline.headings.some(h => h.title.includes('코드')), 'a # inside a fence is excluded');
  // Composing read_note over [start, end) returns the section beginning at the heading line.
  const first = outline.headings[0]!;
  const section = await success<{ content: string }>(client, 'read_note', { path: 'notes/문서.md', offset: first.start, length: first.end - first.start });
  assert.ok(section.content.startsWith('# 개요'));
});

test('list_backlinks reports inbound wikilinks read-only, resolving the target through the read_note path policy (REQ-016)', { timeout: 20_000 }, async t => {
  const files = {
    'sources/B.md': '# B\n\n## Evidence\n\n근거.\n',
    'notes/A.md': '# A\n\n[[../sources/B]] 그리고 [[../sources/B#Evidence]]\n',
    'notes/lonely.md': '# Lonely\n\n인바운드 링크 없음.\n',
  };
  const { client } = await connect(t, { readOnly: true, files });
  type Backlinks = { target: string; targetSha256: string; total: number; untrusted: boolean; limitations: string[];
    backlinks: { path: string; sha256: string; fragment: string; embed: boolean; ambiguous: boolean }[] };
  const result = await success<Backlinks>(client, 'list_backlinks', { path: 'sources/B.md' });
  assert.equal(result.untrusted, true);
  assert.equal(result.targetSha256, digest(files['sources/B.md']));
  assert.equal(result.total, 2, 'both the plain and the #Evidence link from A are inbound');
  assert.ok(result.backlinks.every(link => link.path === 'notes/A.md'));
  assert.ok(result.backlinks.some(link => link.fragment === 'Evidence'));
  assert.ok(result.limitations.some(text => /complete Obsidian link interpreter/u.test(text)), 'the §8 boundary is disclosed');
  // A note with no inbound links is not an error.
  assert.equal((await success<Backlinks>(client, 'list_backlinks', { path: 'notes/lonely.md' })).total, 0);
  // A missing target is rejected by the same policy as read_note.
  await failure(client, 'list_backlinks', { path: 'notes/missing.md' }, 'NOTE_NOT_FOUND');
});

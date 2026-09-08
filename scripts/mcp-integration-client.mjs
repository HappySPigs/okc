/** Real stdio MCP client used by the umbrella's synthetic integration checks. */
import assert from 'node:assert/strict';
import { parseArgs } from 'node:util';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { Client } from '../okc-mcp/node_modules/@modelcontextprotocol/sdk/dist/esm/client/index.js';
import { StdioClientTransport } from '../okc-mcp/node_modules/@modelcontextprotocol/sdk/dist/esm/client/stdio.js';

const { values } = parseArgs({ options: {
  config: { type: 'string' }, operation: { type: 'string', default: 'read' },
  expected: { type: 'string' }, 'note-path': { type: 'string' },
}, strict: true });
if (!values.config || !path.isAbsolute(values.config) || !values.expected || !values['note-path']
  || !['read', 'author'].includes(values.operation)) {
  throw new Error('Usage: --config /absolute/config.json --operation read|author --expected TEXT --note-path RELATIVE.md');
}

const moduleRoot = fileURLToPath(new URL('../okc-mcp/', import.meta.url));
const transport = new StdioClientTransport({ command: process.execPath,
  args: [path.join(moduleRoot, 'dist/cli.js'), 'serve', '--config', values.config],
  cwd: moduleRoot, stderr: 'pipe' });
// Drain stderr without echoing configuration or source content into test output.
transport.stderr?.on('data', () => {});
const client = new Client({ name: 'okc-integration-client', version: '1.0.0' });

async function call(name, args = {}) {
  const response = await client.callTool({ name, arguments: args });
  const envelope = response.structuredContent;
  if (response.isError || envelope?.ok !== true) {
    throw new Error(`${name} failed: ${envelope?.error?.code ?? 'invalid MCP result'}`);
  }
  return envelope.data;
}

try {
  await client.connect(transport);
  const notePath = values['note-path'];
  if (values.operation === 'author') {
    const created = await call('create_note', { path: notePath, title: 'Integration knowledge',
      body: values.expected, dryRun: false });
    assert.equal(created.applied, true, 'create_note must apply');
    assert.equal(created.source.kind, 'local', 'authoring must remain local');
    const read = await call('read_note', { path: notePath, source: 'local', length: 8000 });
    assert.ok(read.content.includes(values.expected), 'created source note must contain expected text');
    assert.equal(read.sha256, created.sha256, 'authoring/read hash must agree');
    process.stdout.write(JSON.stringify({ ok: true, operation: 'author', path: notePath,
      sha256: read.sha256, source: read.source }) + '\n');
  } else {
    const listed = await call('list_notes', { limit: 100 });
    assert.equal(listed.source.kind, 'web', 'configured default must be web');
    assert.match(listed.source.revision, /^[a-f0-9]{64}$/, 'publication revision is required');
    assert.ok(listed.notes.includes(notePath), 'published note must be listed');
    const revision = listed.source.revision;
    const read = await call('read_note', { path: notePath, revision, length: 8000 });
    assert.ok(read.content.includes(values.expected), 'published note must contain expected text');
    assert.equal(read.source.revision, revision, 'read must pin listing revision');
    assert.equal(new URL(read.provenanceUrl).searchParams.get('revision'), revision);
    const search = await call('search_notes', { query: values.expected.slice(0, 200), revision });
    assert.ok(search.matches.some(match => match.path === notePath), 'remote search must find expected content');
    assert.equal(search.source.revision, revision, 'search must retain revision');
    const verified = await call('verify_vault', { revision });
    assert.equal(verified.evidence.valid, true, 'published artifact integrity must verify');
    assert.equal(verified.source.revision, revision);
    const explained = await call('explain_note', { path: notePath, revision });
    assert.equal(explained.source.revision, revision);
    assert.ok(explained.evidence.record && typeof explained.evidence.record === 'object', 'provenance record is required');
    process.stdout.write(JSON.stringify({ ok: true, operation: 'read', path: notePath,
      sha256: read.sha256, source: read.source, verified: true, hasProvenance: true,
      matches: search.matches.length }) + '\n');
  }
} finally { await client.close(); }

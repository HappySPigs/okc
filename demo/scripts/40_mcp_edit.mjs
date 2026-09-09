#!/usr/bin/env node
// Scenario step 2: use okc-mcp — exactly as a coding agent would in a session —
// to author a new note into the local "mine" vault. Speaks MCP's newline-
// delimited JSON-RPC over stdio to `okc-mcp serve`, then calls
// apply_session_capture(dryRun:false). The new note introduces a THIRD
// conflicting refund window (60일) so the re-merge visibly changes.
import { spawn } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '..', '..');
const MCP_CLI = process.env.OKC_MCP_CLI || resolve(repoRoot, 'okc-mcp', 'dist', 'cli.js');
const CONFIG = process.env.OKC_MCP_CONFIG || resolve(here, '..', 'state', 'mcp-mine.json');

const sessionId = `demo-${new Date().toISOString().slice(0, 19).replace(/[:T]/g, '')}`;
const args = {
  userSelected: true,
  sessionId,
  dryRun: false,
  items: [{
    id: 'refund-policy-update',
    path: '세션-메모-환불정책-변경.md',
    expectedHash: null,
    title: '환불 정책 변경 합의 (세션 메모)',
    content: [
      '오늘 팀 논의에서 환불 정책을 조정하기로 했다.',
      '',
      '- 결정: 환불은 결제일로부터 **60일 이내** 가능하도록 확대한다.',
      '- 배경: 세일즈팀(30일)과 고객지원팀(14일) 기존 안내가 서로 달라 혼선이 있었다.',
      '- 후속: 정책 문서와 고객 안내를 일괄 업데이트 예정.',
      '',
      '관련: [[환불 정책]]',
    ].join('\n'),
    rationale: '세션에서 확정된 환불 기간 변경(60일)을 개인 볼트에 기록',
  }],
};

const child = spawn('node', [MCP_CLI, 'serve', '--config', CONFIG], {
  stdio: ['pipe', 'pipe', 'inherit'],
});

let buf = '';
const pending = new Map();
const send = (o) => child.stdin.write(JSON.stringify(o) + '\n');
const rpc = (id, method, params) =>
  new Promise((res, rej) => { pending.set(id, { res, rej }); send({ jsonrpc: '2.0', id, method, params }); });

child.stdout.on('data', (d) => {
  buf += d.toString('utf8');
  let nl;
  while ((nl = buf.indexOf('\n')) >= 0) {
    const line = buf.slice(0, nl).trim();
    buf = buf.slice(nl + 1);
    if (!line) continue;
    let msg;
    try { msg = JSON.parse(line); } catch { continue; }
    if (msg.id !== undefined && pending.has(msg.id)) {
      const { res, rej } = pending.get(msg.id);
      pending.delete(msg.id);
      msg.error ? rej(new Error(JSON.stringify(msg.error))) : res(msg.result);
    }
  }
});
child.on('error', (e) => { console.error('[mcp-edit] spawn error:', e.message); process.exit(1); });

const fail = (m) => { console.error('[mcp-edit] ' + m); try { child.kill(); } catch {} process.exit(1); };
const timer = setTimeout(() => fail('timeout'), 60000);

try {
  await rpc(1, 'initialize', {
    protocolVersion: '2025-06-18', capabilities: {}, clientInfo: { name: 'okc-demo', version: '1' },
  });
  send({ jsonrpc: '2.0', method: 'notifications/initialized' });
  const result = await rpc(2, 'tools/call', { name: 'apply_session_capture', arguments: args });
  clearTimeout(timer);
  const text = (result?.content || []).map((c) => c.text || '').join('\n');
  console.log('[mcp-edit] apply_session_capture →\n' + text);
  if (result?.isError) fail('tool returned isError');
  child.kill();
  process.exit(0);
} catch (e) {
  clearTimeout(timer);
  fail('failed: ' + e.message);
}

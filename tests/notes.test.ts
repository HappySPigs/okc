import assert from 'node:assert/strict';
import test from 'node:test';
import { analyzeNote, auditNotes, createNoteContent, MAX_NOTE_BYTES, patchFrontmatter, validateNote } from '../src/notes.js';

test('minimal notes and supplied metadata remain useful to ordinary Obsidian users', () => {
  const plain = '# Plain\n\nOriginal claim.\n';
  validateNote('notes/Plain.md', plain);
  assert.equal(analyzeNote('notes/Plain.md', plain).title, 'Plain');
  const content = createNoteContent({ title: 'A: title', body: plain, aliases: ['별명'], tags: ['knowledge'], source: 'https://example.test/source' });
  const analysis = analyzeNote('A.md', content);
  assert.equal(analysis.title, 'A: title');
  assert.deepEqual(analysis.aliases, ['별명']);
  assert.deepEqual(analysis.tags, ['knowledge']);
  assert.ok(!content.includes('updated:'));
  assert.ok(!content.includes('approved:'));
});

test('write validation rejects lossy or dangerous metadata without echoing content', () => {
  for (const yaml of ['title: A\ntitle: B', '[one, two]', 'title: [bad]', 'aliases: [good, 7]', 'tags: {bad: true}', 'data: .inf', 'data: .nan', 'data: 9007199254740993', 'secret: [', '__proto__: malicious', 'custom: {constructor: bad}', 'data: !unknown value', 'data: &a [*a]', 'title: "hidden\\u0000control"']) {
    const content = `---\n${yaml}\n---\n# Body\n`;
    assert.throws(() => validateNote('A.md', content), (error: unknown) => {
      assert.ok(error instanceof Error);
      assert.ok(!error.message.includes('malicious'));
      return true;
    }, yaml);
  }
  assert.throws(() => validateNote('A.md', '---\ntitle: Unterminated\n'));
  assert.throws(() => validateNote('A.md', 'invalid\u0000text'));
  assert.throws(() => validateNote('A.md', 'unpaired\ud800'));
});

test('frontmatter edits preserve comments, unrelated custom keys, BOM and exact CRLF body', () => {
  const body = '# Original\r\n\r\nWhitespace  \r\n`[[literal]]`\r\n';
  const input = '\uFEFF---\r\n# Context comment\r\ntitle: Old # title comment\r\ncustom:\r\n  release_date: 2026-09-06\r\n  releaseDate: distinct\r\n---\r\n' + body;
  const result = patchFrontmatter(input, { title: 'New', tags: ['test'] });
  assert.ok(result.startsWith('\uFEFF---\r\n'));
  assert.ok(result.endsWith(body));
  assert.ok(result.includes('# Context comment'));
  assert.ok(result.includes('# title comment'));
  assert.ok(result.includes('release_date: 2026-09-06'));
  assert.ok(result.includes('releaseDate: distinct'));
  assert.equal(analyzeNote('A.md', result).title, 'New');
  assert.equal(patchFrontmatter(input, {}), input);
});

test('patch rejects prototype keys, undefined, cycles, and invalid special fields', () => {
  const input = '# Note\n';
  const cycle: Record<string, unknown> = {};
  cycle.child = cycle;
  for (const changes of [JSON.parse('{"__proto__":1}') as Record<string, unknown>, { title: 4 }, { tags: [3] }, { value: undefined }, { value: Infinity }, { value: cycle }]) assert.throws(() => patchFrontmatter(input, changes));
  const output = patchFrontmatter(input, { aliases: ['Example'], custom: { release_date: 'today', releaseDate: 'different' } });
  assert.ok(output.endsWith(input));
  assert.deepEqual(analyzeNote('A.md', output).aliases, ['Example']);
});

test('wikilink scanner ignores fenced code, inline code, escaped links and comments', () => {
  const content = '# Links\n\n[[Real|display]] ![[assets/chart.png]]\n`[[Inline]]` ``x ` [[LongInline]]``\n\\[[Escaped]]\n<!-- [[Comment]] -->\n```md\n[[Fence]]\n```\n~~~\n[[Tilde]]\n~~~\n[[Target#Section]]\n';
  assert.deepEqual(analyzeNote('A.md', content).links, ['Real', 'assets/chart.png', 'Target#Section']);
});

test('audit reports ambiguity and missing note, heading, block and unsupported output honestly', () => {
  const result = auditNotes([
    { path: 'notes/A.md', content: '# A\n\n[[../sources/B#Evidence]] [[../sources/B#^proof]] [[../sources/B#Missing]] [[Missing]] [[Topic]] ![[../assets/chart.png]]\n' },
    { path: 'sources/B.md', content: '# B\n\n## Evidence\n\nObserved. ^proof\n' },
    { path: 'one/Topic.md', content: '# Topic\n\nOne.\n' },
    { path: 'two/Topic.md', content: '# Topic\n\nTwo.\n' },
  ], ['assets/chart.png', 'Map.canvas'], ['.obsidian']);
  const codes = result.findings.filter(item => item.path === 'notes/A.md').map(item => item.code);
  assert.equal(codes.filter(code => code === 'OKC_LINK_UNRESOLVED').length, 1);
  assert.equal(codes.filter(code => code === 'OKC_LINK_FRAGMENT_UNRESOLVED').length, 1);
  assert.equal(codes.filter(code => code === 'OKC_LINK_AMBIGUOUS').length, 1);
  assert.ok(result.findings.some(item => item.code === 'OKC_NONMARKDOWN_OUTPUT'));
  assert.equal(result.summary.notes, 4);
  assert.equal(result.summary.info, 1);
  assert.match(result.limitations[0] ?? '', /not OKC compiler validation/u);
});

test('audit duplicate body, ingest noise and sensitive hints do not emit matched values', () => {
  const result = auditNotes([
    { path: 'A.md', content: '---\ntitle: One\n---\nShared body.\n' },
    { path: 'B.md', content: '---\ntitle: Two\n---\nShared body.\n' },
    { path: 'templates/Example.md', content: '# Template\n\napi_key: VERY_SECRET_SENTINEL\n' },
  ], [], []);
  assert.equal(result.findings.filter(item => item.code === 'OKC_DUPLICATE_BODY').length, 2);
  assert.ok(result.findings.some(item => item.code === 'OKC_INGEST_NOISE'));
  assert.ok(result.findings.some(item => item.code === 'OKC_SENSITIVE_CANDIDATE'));
  assert.ok(!JSON.stringify(result).includes('VERY_SECRET_SENTINEL'));
});

test('Obsidian Vault-relative folder links resolve and conflicting relative meanings warn', () => {
  const result = auditNotes([
    { path: 'notes/A.md', content: '# A\n\n[[sources/B]]\n' },
    { path: 'sources/B.md', content: '# B\n' },
  ], [], []);
  assert.ok(!result.findings.some(item => item.code === 'OKC_LINK_UNRESOLVED'));
  const ambiguous = auditNotes([
    { path: 'notes/A.md', content: '# A\n\n[[sources/B]]\n' },
    { path: 'sources/B.md', content: '# Root B\n' },
    { path: 'notes/sources/B.md', content: '# Relative B\n' },
  ], [], []);
  assert.ok(ambiguous.findings.some(item => item.code === 'OKC_LINK_AMBIGUOUS'));
});

test('many cross-note headings and block anchors resolve from one per-note index', () => {
  const count = 2_000;
  const sections = Array.from({ length: count }, (_, index) => `## Evidence ${index}\n\nClaim ${index}. ^proof-${index}\n`).join('\n');
  const links = Array.from({ length: count }, (_, index) => `[[Evidence#Evidence ${index}]] [[Evidence#^proof-${index}]]`).join('\n');
  const result = auditNotes([
    { path: 'Index.md', content: '# Index\n\n' + links + '\n[[Evidence#Missing]]\n' },
    { path: 'Evidence.md', content: '# Evidence\n\n' + sections },
  ], [], []);
  assert.equal(result.summary.warnings, 1);
  assert.equal(result.findings.filter(finding => finding.code === 'OKC_LINK_FRAGMENT_UNRESOLVED').length, 1);
  assert.equal(result.summary.totalOccurrences, 1);
});

test('audit aggregates repeated findings and caps unique results while preserving total counts', () => {
  const report = auditNotes([
    { path: 'A.md', content: '# A\n\n[[MissingOne]] [[MissingTwo]] [[MissingThree]]\n' },
  ], Array.from({ length: 1_100 }, (_, index) => `assets/file-${index}.png`), []);
  assert.equal(report.findings.length, 1_000);
  assert.equal(report.summary.warnings, 1_103);
  assert.equal(report.summary.totalOccurrences, 1_103);
  assert.equal(report.summary.aggregatedOccurrences, 2);
  assert.equal(report.summary.omittedFindings, 101);
  assert.equal(report.summary.totalOccurrences, report.findings.length + report.summary.aggregatedOccurrences + report.summary.omittedFindings);
});

test('create and metadata patch errors consistently expose safe NOTE_INVALID', () => {
  const safeError = (error: unknown): boolean => {
    assert.ok(error instanceof Error);
    assert.equal((error as Error & { code: string }).code, 'NOTE_INVALID');
    assert.ok(!error.message.includes('SECRET_SENTINEL'));
    return true;
  };
  const input = { title: 'Title', body: 'Body', unknown: 'SECRET_SENTINEL' };
  assert.throws(() => createNoteContent(input), safeError);
  assert.throws(() => createNoteContent({ title: '', body: 'Body' }), safeError);
  assert.throws(() => createNoteContent(JSON.parse('{"title":"Title","body":"Body","__proto__":"SECRET_SENTINEL"}') as Parameters<typeof createNoteContent>[0]), safeError);
  assert.throws(() => patchFrontmatter('# Original\n', { constructor: 'SECRET_SENTINEL' }), safeError);
  assert.throws(() => patchFrontmatter('# Original\n', { tags: [4] }), safeError);
  assert.throws(() => patchFrontmatter('# Original\n', { nested: { value: undefined } }), safeError);
  assert.throws(() => patchFrontmatter('# Original\n', { title: '\ud800' }), safeError);
  assert.throws(() => patchFrontmatter('# Original\n', { [Symbol('unsupported')]: 'SECRET_SENTINEL' }), safeError);
});

test('authoring note size hard bound matches the four MiB configuration limit', () => {
  assert.equal(MAX_NOTE_BYTES, 4 * 1024 * 1024);
  const analysis = analyzeNote('Large.md', 'x'.repeat(MAX_NOTE_BYTES + 1));
  assert.equal(analysis.issues[0]?.code, 'OKC_NOTE_TOO_LARGE');
  assert.equal(analysis.issues[0]?.severity, 'error');
  assert.deepEqual(analysis.links, []);
});

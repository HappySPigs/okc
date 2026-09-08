import assert from 'node:assert/strict';
import test from 'node:test';
import { analyzeNote, auditNotes, backlinksOf, createNoteContent, fixYamlContent, MAX_NOTE_BYTES, outlineHeadings, patchFrontmatter, reinforceContent, replaceBody, validateNote } from '../src/notes.js';

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

test('every audit finding is projected onto one of the six fixed categories (BR-AUDIT-2)', () => {
  const result = auditNotes([
    { path: 'A.md', content: '# A\n\n[[Missing]]\n' },
    { path: 'B.md', content: '---\ntitle: One\n---\nShared body.\n' },
    { path: 'C.md', content: '---\ntitle: Two\n---\nShared body.\n' },
  ], ['x.canvas'], ['.obsidian']);
  const allowed = new Set(['yaml', 'path', 'link', 'duplicate', 'operational-noise', 'unsupported-format']);
  assert.ok(result.findings.length > 0);
  for (const finding of result.findings) assert.ok(allowed.has(finding.category), finding.category);
  assert.ok(result.findings.some(finding => finding.category === 'link'));
  assert.ok(result.findings.some(finding => finding.category === 'duplicate'));
  assert.ok(result.findings.some(finding => finding.category === 'unsupported-format'));
  assert.ok(result.findings.some(finding => finding.category === 'path')); // OKC_AUDIT_SKIPPED
});

test('fixYamlContent repairs malformed frontmatter, preserves body, and refuses still-invalid corrections', () => {
  const malformed = '---\ntitle: [unterminated\ncustom: keep\n---\n# Body\n\n본문.\n';
  const fixed = fixYamlContent(malformed, 'title: Fixed\ncustom: keep');
  assert.ok(fixed.includes('title: Fixed'));
  assert.ok(fixed.endsWith('# Body\n\n본문.\n'), 'body preserved verbatim');
  assert.throws(() => fixYamlContent(malformed, 'title: [still bad'));
  const added = fixYamlContent('# No frontmatter\n', 'title: Added');
  assert.ok(added.startsWith('---\ntitle: Added\n---\n'));
  assert.ok(added.endsWith('# No frontmatter\n'));
});

test('reinforceContent writes literal source/body text only and requires at least one change', () => {
  const original = '---\ntitle: Claim\n---\n# Claim\n\n주장.\n';
  const out = reinforceContent(original, { source: 'https://example.test/x', appendBody: '\n출처: https://example.test/x\n' });
  assert.ok(out.includes('source:'));
  assert.ok(out.includes('https://example.test/x'));
  assert.ok(out.includes('# Claim'), 'existing body retained');
  assert.ok(out.endsWith('출처: https://example.test/x\n'));
  assert.throws(() => reinforceContent(original, {}));
});

test('replaceBody swaps only the body, preserving the exact frontmatter block', () => {
  const original = '---\ntitle: Keep # comment\ncustom: v\n---\n# Old body\n';
  const out = replaceBody(original, '# New body\n');
  assert.ok(out.startsWith('---\ntitle: Keep # comment\ncustom: v\n---\n'));
  assert.ok(out.endsWith('# New body\n'));
  assert.throws(() => replaceBody('---\ntitle: [bad\n---\nbody\n', 'x'), 'a malformed current note is rejected');
});

test('outlineHeadings maps ATX headings, excludes code fences, spans sections, and skips the frontmatter block (REQ-017)', () => {
  const content = '---\ntitle: Doc\n---\n# Top\n\nintro\n\n## A\n\ntext\n\n```\n# not a heading\n```\n\n## B ##\n\nmore\n';
  const headings = outlineHeadings(content);
  assert.deepEqual(headings.map(h => [h.level, h.title]), [[1, 'Top'], [2, 'A'], [2, 'B']]);
  assert.ok(!headings.some(h => h.title.includes('not a heading')), 'a # inside a fence is not a heading');
  // The first heading begins after the frontmatter block, at its recorded offset.
  assert.equal(content.slice(headings[0]!.start, headings[0]!.start + 5), '# Top');
  // Section span: [start, contentStart) is the heading line; [contentStart, end) is the body.
  for (const h of headings) assert.ok(content.slice(h.start, h.contentStart).trimStart().startsWith('#'));
  // '# Top' (level 1) spans until end of file; '## A' ends where '## B' begins.
  const a = headings.find(h => h.title === 'A')!;
  const b = headings.find(h => h.title === 'B')!;
  assert.equal(a.end, b.start, 'a section ends at the next same-or-higher-level heading');
  assert.equal(headings[0]!.end, content.length, 'the top heading spans to end of file');
});

test('outlineHeadings ignores # that is not a valid ATX heading (no space, 7+ hashes, 4-space indent)', () => {
  const content = '#notaheading\n\n####### too many\n\n    # indented code\n\n# Real\n';
  assert.deepEqual(outlineHeadings(content).map(h => h.title), ['Real']);
});

test('audit link findings are unaffected by astral characters inside masked regions (BR-VISIBLE-1 regression guard)', () => {
  const content = '# A\n\n`😀 inline` <!-- 😀 comment -->\n```\n😀 fenced\n```\n\n[[Target]]\n';
  const result = auditNotes([{ path: 'A.md', content }, { path: 'Target.md', content: '# Target\n' }], [], []);
  assert.ok(!result.findings.some(f => f.code === 'OKC_LINK_UNRESOLVED'), 'the real [[Target]] link still resolves');
  assert.ok(!result.findings.some(f => f.code === 'OKC_TEXT_ENCODING'), 'masked emoji is not flagged as an encoding error');
  assert.deepEqual(analyzeNote('A.md', content).links, ['Target'], 'only the unmasked link is detected');
});

test('backlinksOf finds inbound wikilinks over the documented subset and reports ambiguity without choosing (REQ-016)', () => {
  const notes = [
    { path: 'sources/B.md', content: '# B\n\n## Evidence\n', sha256: 'hB' },
    { path: 'notes/A.md', content: '# A\n\n[[../sources/B]] [[../sources/B#Evidence]] ![[../sources/B]]\n`[[../sources/B]]`\n', sha256: 'hA' },
    { path: 'notes/none.md', content: '# none\n\nnothing\n', sha256: 'hN' },
  ];
  const toB = backlinksOf('sources/B.md', notes);
  assert.deepEqual([...new Set(toB.map(link => link.path))], ['notes/A.md'], 'only A links to B');
  assert.equal(toB.length, 3, 'plain, #heading and embed count; the inline-code link is excluded (BR-VISIBLE-1 shared masking)');
  assert.ok(toB.some(link => link.embed), 'the ![[embed]] carries embed=true');
  assert.ok(toB.some(link => link.fragment === 'Evidence'), 'the #Evidence fragment is preserved');
  assert.ok(toB.every(link => link.sha256 === 'hA'), 'each backlink carries its source SHA-256');
  assert.deepEqual(backlinksOf('notes/none.md', notes), [], 'a note with no inbound links returns []');

  const shared = [
    { path: 'p/Dup.md', content: '# Dup\n\n[[Shared]]\n', sha256: 'hP' },
    { path: 'q/Shared.md', content: '# Shared\n', sha256: 'hQ' },
    { path: 'r/Shared.md', content: '# Shared\n', sha256: 'hR' },
  ];
  const ambiguous = backlinksOf('q/Shared.md', shared).find(link => link.path === 'p/Dup.md');
  assert.ok(ambiguous?.ambiguous && ambiguous.candidates.length > 1, 'a namesake is reported ambiguous, never auto-chosen');
});

test('backlinksOf never treats an unsafe or external target as a backlink (BR-LINK-2/3)', () => {
  const notes = [
    { path: 'T.md', content: '# T\n', sha256: 'hT' },
    { path: 'U.md', content: '# U\n\n[[https://evil.example/T]] [[/abs/T]] [[../../escape/T]]\n', sha256: 'hU' },
  ];
  assert.deepEqual(backlinksOf('T.md', notes), [], 'URI-like, absolute and Vault-escaping targets are not backlinks');
});

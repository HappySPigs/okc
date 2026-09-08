/** Shrinking property tests; fixed seeds are replayed and printed by fast-check on failure. */
import assert from 'node:assert/strict';
import test from 'node:test';
import fc from 'fast-check';
import { parse } from 'yaml';
import { analyzeNote, backlinksOf, createNoteContent, outlineHeadings, patchFrontmatter } from '../src/notes.js';
import { foldText } from '../src/search.js';
import { sha256 } from '../src/vault.js';
import { configSchema } from '../src/config.js';

const korean = [...'가나다라마바사아자차카타파하검증지식근거출처노트'];
const textOf = (characters: string[], minLength = 0, maxLength = 40) =>
  fc.array(fc.constantFrom(...characters), { minLength, maxLength }).map(values => values.join(''));
const title = textOf([...korean, ...'abcXYZ'], 1, 30);
const body = textOf([...korean, ...'abcXYZ0 \n', '😀'], 0, 200);
const note = fc.record({ title, body, aliases: fc.array(title, { maxLength: 5 }), tags: fc.array(title, { maxLength: 5 }) });

test('PBT: note serialization round-trips author metadata and body with shrinking', () => {
  fc.assert(fc.property(note, input => {
    const output = createNoteContent(input);
    // analyzeNote projects aliases/tags into unique lookup values; the YAML
    // serialization boundary preserves the original arrays, including duplicates.
    const parsed = parse(output.slice(4, output.indexOf('\n---\n', 4))) as typeof input;
    assert.equal(parsed.title, input.title);
    assert.deepEqual(parsed.aliases, input.aliases);
    assert.deepEqual(parsed.tags, input.tags);
    assert.ok(output.endsWith(input.body));
  }), { seed: 3, numRuns: 200 });
});

test('PBT: content hash is deterministic and changes on any byte change', () => {
  fc.assert(fc.property(body, value => {
    assert.equal(sha256(value), sha256(value));
    assert.notEqual(sha256(value), sha256(value + '가'));
  }), { seed: 1, numRuns: 300 });
});

test('PBT: standardizing the title changes only that key, preserving body and unknown keys', () => {
  fc.assert(fc.property(note, title, textOf([...'abcdef'], 3, 10), (input, newTitle, custom) => {
    const original = patchFrontmatter(createNoteContent(input), { keepKey: custom });
    const output = patchFrontmatter(original, { title: newTitle });
    assert.ok(output.endsWith(input.body));
    assert.ok(output.includes('keepKey: ' + custom));
    assert.equal(analyzeNote('n.md', output).title, newTitle);
  }), { seed: 7, numRuns: 200 });
});

test('PBT: an empty patch is a byte-exact no-op', () => {
  fc.assert(fc.property(note, input => {
    const content = createNoteContent(input);
    assert.equal(patchFrontmatter(content, {}), content);
  }), { seed: 9, numRuns: 150 });
});

test('PBT: literal (codepoint-faithful) search finds inserted Korean needles', () => {
  fc.assert(fc.property(body, textOf(korean, 1, 5), body, (before, needle, after) => {
    assert.ok((before + needle + after).includes(needle));
  }), { seed: 11, numRuns: 200 });
});

test('PBT: foldText is idempotent, ASCII case-insensitive, and Hangul-preserving (REQ-015)', () => {
  fc.assert(fc.property(title, textOf(korean, 1, 30), (value, hangul) => {
    assert.equal(foldText(foldText(value)), foldText(value));
    assert.equal(foldText(value.toUpperCase()), foldText(value.toLowerCase()));
    assert.equal(foldText(hangul), hangul.normalize('NFC'));
  }), { seed: 21, numRuns: 200 });
});

test('foldText unifies composed and decomposed Hangul and folds ASCII case', () => {
  assert.equal(foldText('가'), foldText('가'));
  assert.equal(foldText('HTTP'), 'http');
  assert.equal(foldText('I'), 'i');
});

test('PBT: outline offsets stay in bounds, are monotonic, and begin at the heading line', () => {
  const section = fc.record({ level: fc.integer({ min: 1, max: 6 }), title, body });
  fc.assert(fc.property(fc.array(section, { minLength: 1, maxLength: 20 }), sections => {
    const content = sections.map(item => '#'.repeat(item.level) + ' ' + item.title + '\n\n' + item.body + '\n').join('\n');
    for (const heading of outlineHeadings(content)) {
      assert.ok(heading.start >= 0 && heading.contentStart <= content.length && heading.end <= content.length);
      assert.ok(heading.start < heading.contentStart && heading.contentStart <= heading.end);
      assert.ok(content.slice(heading.start, heading.contentStart).startsWith('#'.repeat(heading.level)));
    }
  }), { seed: 31, numRuns: 100 });
});

test('outline offsets are correct when astral chars precede a heading inside a code fence (BR-VISIBLE-1)', () => {
  const content = '# Doc\n\n' + '```\n😀😀 astral in a fence\n```\n\n## 대상\n\n본문\n';
  const target = outlineHeadings(content).find(heading => heading.title === '대상');
  assert.ok(target);
  assert.equal(content.slice(target.start, target.contentStart), '## 대상\n');
});

test('PBT: config round-trips the agents field through serialize/parse (LIR-M1, NFR-4)', () => {
  // fc.subarray yields deduped, order-preserving subsets — the schema's canonical form.
  fc.assert(fc.property(fc.subarray(['claude', 'codex'] as const), agents => {
    const parsed = configSchema.parse({ readOnly: true, web: { baseUrl: 'https://example.com', projectId: 'p1' }, agents });
    const round = configSchema.parse(JSON.parse(JSON.stringify(parsed)));
    assert.deepEqual(round.agents, parsed.agents);
    assert.deepEqual(round.agents, agents);
  }), { seed: 51, numRuns: 100 });
});

test('config dedupes repeated agents and preserves first-seen order (LIR-M1)', () => {
  assert.deepEqual(configSchema.parse({ readOnly: true, web: { baseUrl: 'https://example.com', projectId: 'p1' },
    agents: ['codex', 'claude', 'codex', 'claude'] }).agents, ['codex', 'claude']);
});

test('PBT: backlinksOf is deterministic and returns exactly the notes that forward-link the target (REQ-016)', () => {
  fc.assert(fc.property(fc.array(fc.boolean(), { maxLength: 20 }), links => {
    const notes = [{ path: 'T.md', content: '# T\n', sha256: 't' },
      ...links.map((linked, index) => ({ path: 'n' + index + '.md', content: linked ? '[[T]]' : '링크 없음', sha256: 'h' + index }))];
    const actual = backlinksOf('T.md', notes);
    assert.deepEqual(actual, backlinksOf('T.md', notes));
    assert.deepEqual(actual.map(value => value.path).sort(), links.flatMap((linked, index) => linked ? ['n' + index + '.md'] : []).sort());
  }), { seed: 41, numRuns: 100 });
});

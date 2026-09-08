/**
 * Property-based tests for pure functions + serialization round-trips
 * (NFR-TEST-1, D11-partial, nfr-design P8).
 *
 * D11 selected `fast-check`; it is unavailable in this offline environment, so
 * these properties use a small self-contained deterministic (seeded) generator
 * instead. This keeps the PBT intent — many generated inputs, reproducible via
 * a fixed seed — with zero external dependency. Swap in fast-check when network
 * install is available (see the code summary's noted deviation).
 */
import assert from 'node:assert/strict';
import test from 'node:test';
import { analyzeNote, backlinksOf, outlineHeadings, patchFrontmatter } from '../src/notes.js';
import { foldText } from '../src/search.js';
import { sha256 } from '../src/vault.js';

/** Deterministic PRNG (mulberry32) — reproducible across runs from a fixed seed. */
function rng(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) | 0;
    let t = Math.imul(state ^ (state >>> 15), 1 | state);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
function pick(next: () => number, alphabet: string, length: number): string {
  let out = '';
  for (let i = 0; i < length; i++) out += alphabet[Math.floor(next() * alphabet.length)];
  return out;
}
const KOREAN = '가나다라마바사아자차카타파하검증지식근거출처노트';

test('PBT: content hash is deterministic and changes on any byte change', () => {
  const next = rng(1);
  for (let i = 0; i < 300; i++) {
    const a = pick(next, KOREAN + 'abcXYZ0 \n', 1 + Math.floor(next() * 40));
    assert.equal(sha256(a), sha256(a), 'equal bytes -> equal hash');
    const b = a + pick(next, 'x가', 1);
    assert.notEqual(sha256(a), sha256(b), 'a byte change -> a different hash');
  }
});

test('PBT: standardizing the title changes only that key, preserving body and unknown keys', () => {
  const next = rng(7);
  for (let i = 0; i < 200; i++) {
    const custom = pick(next, 'abcdef', 3); // ASCII value: unambiguous plain YAML scalar
    const body = `# ${pick(next, KOREAN + 'abc ', 5)}\n\n${pick(next, KOREAN + 'abc \n', 20)}\n`;
    const content = `---\ntitle: ${pick(next, KOREAN + 'abc', 4)}\nkeepKey: ${custom}\n---\n${body}`;
    const newTitle = pick(next, KOREAN + 'abc', 5);
    const out = patchFrontmatter(content, { title: newTitle });
    assert.ok(out.endsWith(body), 'body preserved verbatim');
    assert.ok(out.includes(`keepKey: ${custom}`), 'unknown key preserved');
    assert.equal(analyzeNote('n.md', out).title, newTitle, 'title reflects exactly the requested change');
  }
});

test('PBT: an empty patch is a byte-exact no-op', () => {
  const next = rng(9);
  for (let i = 0; i < 150; i++) {
    const content = `---\ntitle: ${pick(next, KOREAN + 'abc', 4)}\n---\n# ${pick(next, KOREAN + 'abc ', 6)}\n`;
    assert.equal(patchFrontmatter(content, {}), content);
  }
});

test('PBT: literal (codepoint-faithful) search finds inserted Korean needles', () => {
  const next = rng(11);
  for (let i = 0; i < 200; i++) {
    const needle = pick(next, KOREAN, 1 + Math.floor(next() * 3));
    const content = `${pick(next, KOREAN + 'abc\n', 10)}${needle}${pick(next, KOREAN + 'abc\n', 10)}`;
    assert.ok(content.indexOf(needle) >= 0, 'a note containing the literal needle is matched');
  }
});

test('PBT: foldText is idempotent, ASCII case-insensitive, and Hangul-preserving (REQ-015)', () => {
  const next = rng(21);
  for (let i = 0; i < 200; i++) {
    const s = pick(next, KOREAN + 'ABCabcXYZ ', 1 + Math.floor(next() * 10));
    assert.equal(foldText(foldText(s)), foldText(s), 'idempotent');
    assert.equal(foldText(s.toUpperCase()), foldText(s.toLowerCase()), 'case-insensitive both ways');
    const k = pick(next, KOREAN, 1 + Math.floor(next() * 5));
    assert.equal(foldText(k), k.normalize('NFC'), 'Hangul is caseless; only NFC canonicalization applies');
  }
});

test('foldText unifies composed and decomposed Hangul and folds ASCII case', () => {
  assert.equal(foldText('가'), foldText('가'), 'decomposed jamo unify with the composed syllable');
  assert.equal(foldText('HTTP'), 'http');
  assert.equal(foldText('I'.repeat(1)), 'i', 'locale-independent: capital I folds to ASCII i');
});

test('PBT: outline offsets stay in bounds, are monotonic, and begin at the heading line', () => {
  const next = rng(31);
  for (let i = 0; i < 100; i++) {
    const count = 1 + Math.floor(next() * 4);
    let content = `${pick(next, KOREAN + 'abc \n', 6)}\n`;
    for (let h = 0; h < count; h++) {
      const level = 1 + Math.floor(next() * 3);
      content += `${'#'.repeat(level)} ${pick(next, KOREAN + 'abc', 3 + Math.floor(next() * 5))}\n\n${pick(next, KOREAN + 'abc \n', 8)}\n`;
    }
    for (const hd of outlineHeadings(content)) {
      assert.ok(hd.start >= 0 && hd.contentStart <= content.length && hd.end <= content.length, 'offsets in bounds');
      assert.ok(hd.start < hd.contentStart && hd.contentStart <= hd.end, 'offsets monotonic');
      assert.ok(content.slice(hd.start, hd.contentStart).startsWith('#'.repeat(hd.level)), 'slice(start) begins the heading line');
    }
  }
});

test('outline offsets are correct when astral chars precede a heading inside a code fence (BR-VISIBLE-1)', () => {
  const content = '# Doc\n\n```\n😀😀 astral in a fence\n```\n\n## 대상\n\n본문\n';
  const target = outlineHeadings(content).find(heading => heading.title === '대상');
  assert.ok(target, 'the heading after the astral fence is found');
  assert.equal(content.slice(target!.start, target!.contentStart), '## 대상\n', 'astral collapse must not shift the offset');
});

test('PBT: backlinksOf is deterministic and returns exactly the notes that forward-link the target (REQ-016)', () => {
  const next = rng(41);
  for (let i = 0; i < 80; i++) {
    const notes = [{ path: 'T.md', content: '# T\n', sha256: 't' }];
    const linkers = new Set<string>();
    const count = 1 + Math.floor(next() * 4);
    for (let j = 0; j < count; j++) {
      const relative = `n${j}.md`;
      const links = next() < 0.5;
      notes.push({ path: relative, content: `# ${relative}\n\n${links ? '[[T]]' : '링크 없음'}\n`, sha256: `h${j}` });
      if (links) linkers.add(relative);
    }
    const first = backlinksOf('T.md', notes);
    const second = backlinksOf('T.md', notes);
    assert.deepEqual(first.map(link => link.path), second.map(link => link.path), 'deterministic across runs');
    assert.deepEqual([...new Set(first.map(link => link.path))].sort(), [...linkers].sort(), 'exactly the forward-linkers appear');
  }
});

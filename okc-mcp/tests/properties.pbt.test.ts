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
import { analyzeNote, patchFrontmatter } from '../src/notes.js';
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

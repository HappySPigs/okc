import { createHash } from 'node:crypto';
import { posix } from 'node:path';
import { Document, isMap, parseDocument } from 'yaml';

export interface Finding {
  code: string;
  severity: 'error' | 'warning' | 'info';
  path: string;
  message: string;
}

export interface NoteAnalysis {
  title: string;
  aliases: string[];
  tags: string[];
  links: string[];
  issues: Finding[];
}

export const MAX_NOTE_BYTES = 4 * 1024 * 1024;
const MAX_AUDIT_FINDINGS = 1_000;
const UNSAFE_KEYS = new Set(['__proto__', 'constructor', 'prototype']);
const CONTROL = /[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/u;
const UNPAIRED_SURROGATE = /[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/u;

class InvalidNote extends Error {
  readonly code = 'NOTE_INVALID';
  constructor(readonly issues: Finding[]) {
    super(issues.map(issue => `${issue.code}: ${issue.message}`).join('; '));
    this.name = 'NoteValidationError';
  }
}

function fail(message: string): never {
  throw new Error(message);
}

function mutationResult(operation: () => string): string {
  try { return operation(); }
  catch (error) {
    if (error instanceof InvalidNote) throw error;
    throw new InvalidNote([{
      code: 'OKC_METADATA_INVALID', severity: 'error', path: 'note.md',
      message: 'The note operation requires supported fields and bounded JSON-compatible metadata with valid title, aliases, and tags.',
    }]);
  }
}

function jsonValue(value: unknown, depth = 0, seen = new Set<object>()): unknown {
  if (depth > 32) fail('Metadata nesting exceeds the authoring limit.');
  if (typeof value === 'string' && (CONTROL.test(value) || UNPAIRED_SURROGATE.test(value))) fail('Metadata contains unsupported Unicode or control characters.');
  if (value === null || typeof value === 'string' || typeof value === 'boolean') return value;
  if (typeof value === 'number' && Number.isFinite(value) && (!Number.isInteger(value) || Number.isSafeInteger(value))) return value;
  if (!value || typeof value !== 'object' || seen.has(value)) fail('Metadata must contain finite, safe JSON values without cycles.');
  seen.add(value);
  let result: unknown;
  if (Array.isArray(value)) {
    if (value.length > 10_000) fail('Metadata collection exceeds the authoring limit.');
    result = value.map(item => jsonValue(item, depth + 1, seen));
  } else {
    if (!(value instanceof Map) && ![Object.prototype, null].includes(Object.getPrototypeOf(value))) fail('Metadata must be plain JSON data.');
    if (!(value instanceof Map) && Reflect.ownKeys(value).some(field => typeof field !== 'string')) fail('Metadata must have string keys.');
    const entries: [unknown, unknown][] = value instanceof Map ? [...value.entries()] : Object.entries(value);
    if (entries.length > 10_000) fail('Metadata collection exceeds the authoring limit.');
    const record: Record<string, unknown> = Object.create(null) as Record<string, unknown>;
    for (const [key, item] of entries) {
      if (typeof key !== 'string' || UNSAFE_KEYS.has(key) || CONTROL.test(key)) fail('Metadata contains an unsupported mapping key.');
      record[key] = jsonValue(item, depth + 1, seen);
    }
    result = record;
  }
  seen.delete(value);
  return result;
}

function specialFields(metadata: Record<string, unknown>): void {
  if (Object.hasOwn(metadata, 'title') && (typeof metadata.title !== 'string' || !metadata.title.trim() || /[\r\n]/u.test(metadata.title))) {
    fail('The title field must be a non-empty single-line string.');
  }
  for (const key of ['aliases', 'tags']) {
    if (!Object.hasOwn(metadata, key)) continue;
    const value = metadata[key];
    const values = typeof value === 'string' ? [value] : value;
    if (!Array.isArray(values) || values.some(item => typeof item !== 'string' || !item.trim() || /[\r\n]/u.test(item))) {
      fail('Aliases and tags must contain non-empty single-line strings.');
    }
  }
}

interface Frontmatter {
  document: Document | null;
  metadata: Record<string, unknown>;
  body: string;
  bom: string;
  newline: string;
  closing: string;
}

function frontmatter(content: string): Frontmatter {
  const bom = content.startsWith('\uFEFF') ? '\uFEFF' : '';
  const raw = content.slice(bom.length);
  const opening = /^---(\r\n|\n|\r)/u.exec(raw);
  const newline = opening?.[1] ?? (content.includes('\r\n') ? '\r\n' : '\n');
  if (!opening) return { document: null, metadata: {}, body: raw, bom, newline, closing: '---' };
  const start = opening[0].length;
  const closing = /^(---|\.\.\.)(?:\r\n|\n|\r|$)/gmu;
  closing.lastIndex = start;
  const match = closing.exec(raw);
  if (!match) fail('Frontmatter is missing its closing delimiter.');
  const document = parseDocument(raw.slice(start, match.index), {
    uniqueKeys: true, strict: true, prettyErrors: false, keepSourceTokens: true,
  });
  if (document.errors.length || document.warnings.length || !isMap(document.contents)) {
    fail('Frontmatter must be a valid YAML mapping with unique keys and supported tags.');
  }
  const metadata = jsonValue(document.toJS({ mapAsMap: true, maxAliasCount: 20 })) as Record<string, unknown>;
  specialFields(metadata);
  return { document, metadata, body: raw.slice(match.index + match[0].length), bom, newline, closing: match[1] ?? '---' };
}

function list(value: unknown): string[] {
  const values = typeof value === 'string' ? [value] : Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string') : [];
  return [...new Set(values)].sort();
}

/** A bounded authoring scanner, not OKC's parser. It never rewrites link spans. */
function visibleMarkdown(body: string): string {
  let fenced: { marker: string; length: number } | null = null;
  let visible = '';
  for (const line of body.split(/(?<=\n)/u)) {
    const marker = /^ {0,3}(`{3,}|~{3,})(.*)/u.exec(line);
    if (fenced) {
      if (marker && marker[1]?.[0] === fenced.marker && marker[1].length >= fenced.length && !(marker[2] ?? '').trim()) fenced = null;
      visible += line.replace(/[^\r\n]/gu, ' ');
    } else if (marker && (marker[1]?.[0] !== '`' || !(marker[2] ?? '').includes('`'))) {
      fenced = { marker: marker[1]?.[0] ?? '`', length: marker[1]?.length ?? 3 };
      visible += line.replace(/[^\r\n]/gu, ' ');
    } else visible += line;
  }
  visible = visible.replace(/<!--[\s\S]*?(?:-->|$)/gu, value => value.replace(/[^\r\n]/gu, ' '));
  let result = '';
  for (let index = 0; index < visible.length;) {
    if (visible[index] !== '`' || escaped(visible, index)) { result += visible[index]; index++; continue; }
    let length = 1;
    while (visible[index + length] === '`') length++;
    let end = index + length;
    let found = -1;
    while ((end = visible.indexOf('`'.repeat(length), end)) !== -1) {
      if (visible[end - 1] !== '`' && visible[end + length] !== '`') { found = end; break; }
      end += length;
    }
    if (found === -1) { result += visible.slice(index, index + length); index += length; }
    else {
      result += visible.slice(index, found + length).replace(/[^\r\n]/gu, ' ');
      index = found + length;
    }
  }
  return result;
}

function escaped(value: string, index: number): boolean {
  let slashes = 0;
  while (index > 0 && value[--index] === '\\') slashes++;
  return slashes % 2 === 1;
}

function wikiLinks(visible: string): string[] {
  const links: string[] = [];
  for (const match of visible.matchAll(/!?\[\[([^\]\r\n]+)\]\]/gu)) {
    if (escaped(visible, match.index)) continue;
    const target = match[1]?.split('|')[0]?.trim();
    if (target) links.push(target);
  }
  return [...new Set(links)];
}

function key(value: string): string { return value.trim().normalize('NFC').toLowerCase(); }

interface NoteInspection {
  analysis: NoteAnalysis;
  body: string | undefined;
  headings: Set<string>;
  blockIds: Set<string>;
}

function inspectNote(path: string, content: string): NoteInspection {
  const issues: Finding[] = [];
  const add = (code: string, severity: Finding['severity'], message: string): void => { issues.push({ code, severity, path, message }); };
  let metadata: Record<string, unknown> = {};
  let body = content;
  if (Buffer.byteLength(content, 'utf8') > MAX_NOTE_BYTES) {
    add('OKC_NOTE_TOO_LARGE', 'error', 'Note exceeds the 4 MiB authoring analysis limit.');
    return { analysis: { title: posix.basename(path, '.md'), aliases: [], tags: [], links: [], issues }, body: undefined, headings: new Set(), blockIds: new Set() };
  }
  if (CONTROL.test(content) || UNPAIRED_SURROGATE.test(content)) add('OKC_TEXT_ENCODING', 'error', 'Note contains unsupported control characters or unpaired Unicode surrogates.');
  try {
    const parsed = frontmatter(content);
    metadata = parsed.metadata;
    body = parsed.body;
  } catch {
    add('OKC_FRONTMATTER_INVALID', 'error', 'Frontmatter must be a bounded JSON-compatible YAML mapping with unique keys and valid title, aliases, and tags.');
  }
  const visible = visibleMarkdown(body);
  const heading = /^ {0,3}#{1,6}[\t ]+(.+)$/mu.exec(visible)?.[1]?.trim();
  const title = typeof metadata.title === 'string' ? metadata.title : heading ?? posix.basename(path, posix.extname(path));
  if (!body.trim()) add('OKC_EMPTY_NOTE', 'warning', 'Empty notes still become OKC taxonomy and review inputs.');
  if (/(?:^|\/)(?:templates?|readme)(?:\/|\.|$)/iu.test(path)) add('OKC_INGEST_NOISE', 'warning', 'Templates and operating documentation inside a source Vault may be ingested as knowledge; .gitignore does not exclude them from OKC.');
  if (/-----BEGIN (?:[A-Z ]+ )?PRIVATE KEY-----|\b(?:api[_-]?key|access[_-]?token|password)\s*[:=]\s*\S+/iu.test(content)) {
    add('OKC_SENSITIVE_CANDIDATE', 'warning', 'Possible sensitive material detected. Review locally; matched content is omitted from this finding.');
  }
  const valid = !issues.some(issue => issue.severity === 'error');
  return {
    analysis: { title, aliases: list(metadata.aliases), tags: list(metadata.tags), links: wikiLinks(visible), issues },
    body: valid ? body : undefined,
    headings: valid ? new Set([...visible.matchAll(/^ {0,3}#{1,6}[\t ]+(.+)$/gmu)].map(match => key((match[1] ?? '').replace(/[\t ]+#+[\t ]*$/u, '')))) : new Set(),
    blockIds: valid ? new Set([...visible.matchAll(/(?:^|\s)\^([A-Za-z0-9_-]+)(?=\s*$)/gmu)].map(match => match[1] ?? '')) : new Set(),
  };
}

export function analyzeNote(path: string, content: string): NoteAnalysis {
  return inspectNote(path, content).analysis;
}

export function validateNote(path: string, content: string): void {
  const issues = analyzeNote(path, content).issues.filter(issue => issue.severity === 'error');
  if (issues.length) throw new InvalidNote(issues);
}

export function createNoteContent(input: { title: string; body: string; aliases?: string[] | undefined; tags?: string[] | undefined; source?: string | undefined }): string {
  return mutationResult(() => {
    const allowed = new Set(['title', 'body', 'aliases', 'tags', 'source']);
    if (!input || ![Object.prototype, null].includes(Object.getPrototypeOf(input)) || Reflect.ownKeys(input).some(field => typeof field !== 'string' || !allowed.has(field))) fail('Note creation contains unsupported fields.');
    if (typeof input.title !== 'string' || typeof input.body !== 'string') fail('Note title and body must be strings.');
    if ((input.aliases !== undefined && !Array.isArray(input.aliases)) || (input.tags !== undefined && !Array.isArray(input.tags)) || (input.source !== undefined && typeof input.source !== 'string')) fail('Note creation fields have unsupported types.');
    const metadata: Record<string, unknown> = { title: input.title };
    for (const field of ['aliases', 'tags', 'source'] as const) if (input[field] !== undefined) metadata[field] = input[field];
    jsonValue(metadata);
    specialFields(metadata);
    const content = `---\n${new Document(metadata).toString({ lineWidth: 0 })}---\n${input.body}`;
    validateNote('note.md', content);
    return content;
  });
}

export function patchFrontmatter(content: string, changes: Record<string, unknown>): string {
  return mutationResult(() => {
    validateNote('note.md', content);
    const valid = jsonValue(changes) as Record<string, unknown>;
    if (!valid || Array.isArray(valid) || typeof valid !== 'object') fail('Metadata changes must be a JSON mapping.');
    const parsed = frontmatter(content);
    const merged = Object.assign(Object.create(null) as Record<string, unknown>, parsed.metadata, valid);
    specialFields(merged);
    if (Object.keys(valid).length === 0) return content;
    const document = parsed.document ?? new Document({});
    for (const [field, value] of Object.entries(valid)) document.set(field, value);
    const yaml = document.toString({ lineWidth: 0 }).replace(/\n/gu, parsed.newline);
    const result = `${parsed.bom}---${parsed.newline}${yaml}${parsed.closing}${parsed.newline}${parsed.body}`;
    validateNote('note.md', result);
    return result;
  });
}

interface AuditNote { path: string; content: string }

export function auditNotes(notes: AuditNote[], otherFiles: string[], skipped: string[]): {
  findings: Finding[];
  summary: { notes: number; errors: number; warnings: number; info: number; totalOccurrences: number; omittedFindings: number; aggregatedOccurrences: number };
  limitations: string[];
} {
  const findings: Finding[] = [];
  const analyses = notes.map(note => ({ path: note.path, ...inspectNote(note.path, note.content) }));
  const byPath = new Map(analyses.map(note => [key(note.path), note]));
  const lookup = new Map<string, Set<string>>();
  const bodies = new Map<string, string[]>();
  const allFiles = new Set([...notes.map(note => key(note.path)), ...otherFiles.map(key)]);
  const summary = { notes: notes.length, errors: 0, warnings: 0, info: 0, totalOccurrences: 0, omittedFindings: 0, aggregatedOccurrences: 0 };
  const seenFindings = new Set<string>();
  const add = (path: string, code: string, severity: Finding['severity'], message: string): void => {
    summary[severity === 'error' ? 'errors' : severity === 'warning' ? 'warnings' : 'info']++;
    summary.totalOccurrences++;
    const identity = JSON.stringify([path, code, message]);
    if (seenFindings.has(identity)) { summary.aggregatedOccurrences++; return; }
    seenFindings.add(identity);
    if (findings.length >= MAX_AUDIT_FINDINGS) { summary.omittedFindings++; return; }
    findings.push({ path, code, severity, message });
  };
  for (const note of analyses) {
    for (const issue of note.analysis.issues) add(issue.path, issue.code, issue.severity, issue.message);
    for (const name of [note.analysis.title, posix.basename(note.path, '.md'), ...note.analysis.aliases]) {
      const names = lookup.get(key(name)) ?? new Set<string>();
      names.add(note.path);
      lookup.set(key(name), names);
    }
    if (note.body?.trim()) {
      const digest = createHash('sha256').update(note.body).digest('hex');
      const duplicates = bodies.get(digest) ?? [];
      duplicates.push(note.path);
      bodies.set(digest, duplicates);
    }
  }
  for (const paths of bodies.values()) if (paths.length > 1) for (const path of paths) add(path, 'OKC_DUPLICATE_BODY', 'warning', 'Another note has the same exact body bytes; review without automatically deleting either source.');
  const ambiguousPaths = new Set<string>();
  for (const paths of lookup.values()) if (paths.size > 1) for (const path of paths) ambiguousPaths.add(path);
  for (const path of ambiguousPaths) add(path, 'OKC_NAME_AMBIGUOUS', 'warning', 'A title, filename, or alias shares a lookup name with another note; qualify links where needed.');
  for (const note of analyses) {
    for (const target of note.analysis.links) {
      const hash = target.indexOf('#');
      const pathPart = (hash < 0 ? target : target.slice(0, hash)).trim();
      const fragment = hash < 0 ? '' : target.slice(hash + 1);
      if (/^[a-z][a-z\d+.-]*:/iu.test(pathPart) || pathPart.startsWith('/') || pathPart.includes('\\')) {
        add(note.path, 'OKC_LINK_UNSAFE', 'warning', 'A wikilink uses an absolute, URI-like, or nonportable target; review locally.');
        continue;
      }
      const relative = posix.normalize(posix.join(posix.dirname(note.path), pathPart));
      if (relative === '..' || relative.startsWith('../')) {
        add(note.path, 'OKC_LINK_UNSAFE', 'warning', 'A wikilink may escape the source Vault.');
        continue;
      }
      let candidates = new Set<string>();
      if (!pathPart) candidates.add(note.path);
      else {
        const locations = [...new Set([relative, pathPart])];
        for (const location of locations) {
          for (const candidate of [location, `${location}.md`]) {
            const found = byPath.get(key(candidate));
            if (found) candidates.add(found.path);
          }
        }
        if (candidates.size === 0 && locations.some(location => allFiles.has(key(location)))) continue;
        if (candidates.size === 0) candidates = lookup.get(key(posix.basename(pathPart, posix.extname(pathPart)))) ?? new Set<string>();
      }
      if (candidates.size === 0) add(note.path, 'OKC_LINK_UNRESOLVED', 'warning', 'A wikilink target could not be found by the authoring heuristic.');
      else if (candidates.size > 1) add(note.path, 'OKC_LINK_AMBIGUOUS', 'warning', 'A wikilink has multiple possible targets; no target was chosen.');
      else if (fragment) {
        const candidate = byPath.get(key([...candidates][0] ?? ''));
        if (!candidate || candidate.body === undefined) continue;
        const found = fragment.startsWith('^')
          ? candidate.blockIds.has(fragment.slice(1))
          : candidate.headings.has(key(fragment));
        if (!found) add(note.path, 'OKC_LINK_FRAGMENT_UNRESOLVED', 'warning', 'A heading or block anchor could not be confirmed by the authoring heuristic.');
      }
    }
  }
  for (const path of otherFiles) add(path, /\.(canvas|base)$/iu.test(path) ? 'OKC_NONMARKDOWN_OUTPUT' : 'OKC_ATTACHMENT_OUTPUT', 'warning', 'Current OKC compiled output is Markdown-only; preserve important attachment or Canvas/Base evidence in Markdown too.');
  for (const path of skipped) add(path, 'OKC_AUDIT_SKIPPED', 'info', 'This path was excluded or could not be analyzed; this audit is incomplete for it.');
  findings.sort((left, right) => left.path.localeCompare(right.path, 'en') || left.code.localeCompare(right.code, 'en'));
  return {
    findings,
    summary,
    limitations: [
      'This is an authoring heuristic, not OKC compiler validation, evidence approval, or a guarantee of successful integration.',
      'Link checks cover basic wikilinks, ATX headings, and ASCII block anchors. Ordinary Markdown links, complex escaped syntax, Canvas links, and all Obsidian plugin syntax are not fully resolved.',
      'Lookup uses JavaScript NFC and lowercasing, not the compiler’s pinned full Unicode case folding; source-relative and Vault-relative link interpretation may differ.',
      'Sensitive-content hints are incomplete and do not replace OKC disclosure preflight. No provider or network is contacted by this audit.',
      'Templates, drafts, and operating Markdown inside an OKC source may still be ingested even when ignored by Git.',
      'Findings with the same path, code, and message are aggregated and at most 1,000 unique findings are returned. Summary severity counts include all occurrences; aggregatedOccurrences counts repeated occurrences and omittedFindings counts unique findings excluded by the limit.',
    ],
  };
}

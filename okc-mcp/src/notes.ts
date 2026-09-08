import { createHash } from 'node:crypto';
import { posix } from 'node:path';
import { Document, isMap, parseDocument } from 'yaml';

/** The fixed heuristic audit categories (business-rules.md BR-AUDIT-2). */
export type AuditCategory = 'yaml' | 'path' | 'link' | 'duplicate' | 'operational-noise' | 'unsupported-format';

/** Project a rich internal finding code onto one of the six design categories. */
function categoryFor(code: string): AuditCategory {
  switch (code) {
    case 'OKC_FRONTMATTER_INVALID':
    case 'OKC_TEXT_ENCODING':
    case 'OKC_METADATA_INVALID':
    case 'NOTE_INVALID':
      return 'yaml';
    case 'OKC_LINK_UNSAFE':
    case 'OKC_LINK_UNRESOLVED':
    case 'OKC_LINK_AMBIGUOUS':
    case 'OKC_LINK_FRAGMENT_UNRESOLVED':
      return 'link';
    case 'OKC_DUPLICATE_BODY':
      return 'duplicate';
    case 'OKC_ATTACHMENT_OUTPUT':
    case 'OKC_NONMARKDOWN_OUTPUT':
    case 'OKC_NOTE_TOO_LARGE':
      return 'unsupported-format';
    case 'OKC_AUDIT_SKIPPED':
      return 'path';
    default:
      return 'operational-noise';
  }
}

export interface Finding {
  code: string;
  category: AuditCategory;
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
      code: 'OKC_METADATA_INVALID', category: 'yaml', severity: 'error', path: 'note.md',
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
      visible += line.replace(/[^\r\n]/g, ' ');
    } else if (marker && (marker[1]?.[0] !== '`' || !(marker[2] ?? '').includes('`'))) {
      fenced = { marker: marker[1]?.[0] ?? '`', length: marker[1]?.length ?? 3 };
      visible += line.replace(/[^\r\n]/g, ' ');
    } else visible += line;
  }
  visible = visible.replace(/<!--[\s\S]*?(?:-->|$)/gu, value => value.replace(/[^\r\n]/g, ' '));
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
      result += visible.slice(index, found + length).replace(/[^\r\n]/g, ' ');
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

export interface WikiLinkOccurrence {
  raw: string;    // full text inside [[...]] (before splitting on '|')
  target: string; // the part before '|', trimmed
  index: number;  // offset of the match in `visible` (== body offset; BR-VISIBLE-1)
  embed: boolean; // leading '!' (embed/transclusion)
}

/** Documented wikilink-subset occurrences in a note's visible (fence-masked) body. */
export function wikiLinkOccurrences(visible: string): WikiLinkOccurrence[] {
  const occurrences: WikiLinkOccurrence[] = [];
  for (const match of visible.matchAll(/!?\[\[([^\]\r\n]+)\]\]/gu)) {
    if (escaped(visible, match.index)) continue;
    const raw = match[1] ?? '';
    const target = raw.split('|')[0]?.trim() ?? '';
    if (target) occurrences.push({ raw, target, index: match.index, embed: match[0].startsWith('!') });
  }
  return occurrences;
}

function wikiLinks(visible: string): string[] {
  return [...new Set(wikiLinkOccurrences(visible).map(occurrence => occurrence.target))];
}

export interface ResolvedWikiLink {
  pathPart: string;
  fragment: string;
  status: 'unsafe' | 'external' | 'unresolved' | 'resolved' | 'ambiguous';
  reason?: 'nonportable' | 'escape';
  candidates: string[]; // sorted note paths (empty unless resolved/ambiguous)
}

/**
 * Resolve one wikilink target against the vault index using the SAME rules as
 * audit_vault (shared so the audit and list_backlinks cannot diverge). It never
 * touches the filesystem: link text is matched only against already-listed note
 * paths/names, ambiguity is reported (never auto-chosen), and unsafe/escaping
 * targets are refused (BR-LINK-2/3/4/5).
 */
export function resolveWikiLink(
  sourcePath: string,
  target: string,
  context: { byPath: ReadonlyMap<string, { path: string }>; lookup: ReadonlyMap<string, ReadonlySet<string>>; allFiles: ReadonlySet<string> },
): ResolvedWikiLink {
  const hash = target.indexOf('#');
  const pathPart = (hash < 0 ? target : target.slice(0, hash)).trim();
  const fragment = hash < 0 ? '' : target.slice(hash + 1);
  if (/^[a-z][a-z\d+.-]*:/iu.test(pathPart) || pathPart.startsWith('/') || pathPart.includes('\\')) {
    return { pathPart, fragment, status: 'unsafe', reason: 'nonportable', candidates: [] };
  }
  const relative = posix.normalize(posix.join(posix.dirname(sourcePath), pathPart));
  if (relative === '..' || relative.startsWith('../')) {
    return { pathPart, fragment, status: 'unsafe', reason: 'escape', candidates: [] };
  }
  const found = new Set<string>();
  if (!pathPart) found.add(sourcePath);
  else {
    const locations = [...new Set([relative, pathPart])];
    for (const location of locations) {
      for (const candidate of [location, `${location}.md`]) {
        const hit = context.byPath.get(key(candidate));
        if (hit) found.add(hit.path);
      }
    }
    if (found.size === 0 && locations.some(location => context.allFiles.has(key(location)))) {
      return { pathPart, fragment, status: 'external', candidates: [] };
    }
    if (found.size === 0) for (const path of context.lookup.get(key(posix.basename(pathPart, posix.extname(pathPart)))) ?? []) found.add(path);
  }
  if (found.size === 0) return { pathPart, fragment, status: 'unresolved', candidates: [] };
  const candidates = [...found].sort((left, right) => (left < right ? -1 : left > right ? 1 : 0));
  return { pathPart, fragment, status: candidates.length > 1 ? 'ambiguous' : 'resolved', candidates };
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
  const add = (code: string, severity: Finding['severity'], message: string): void => { issues.push({ code, category: categoryFor(code), severity, path, message }); };
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

export function createNoteContent(input: { title: string; body: string; aliases?: string[] | undefined; tags?: string[] | undefined; source?: string | string[] | undefined }): string {
  return mutationResult(() => {
    const allowed = new Set(['title', 'body', 'aliases', 'tags', 'source']);
    if (!input || ![Object.prototype, null].includes(Object.getPrototypeOf(input)) || Reflect.ownKeys(input).some(field => typeof field !== 'string' || !allowed.has(field))) fail('Note creation contains unsupported fields.');
    if (typeof input.title !== 'string' || typeof input.body !== 'string') fail('Note title and body must be strings.');
    if ((input.aliases !== undefined && !Array.isArray(input.aliases)) || (input.tags !== undefined && !Array.isArray(input.tags)) || (input.source !== undefined && typeof input.source !== 'string' && !Array.isArray(input.source))) fail('Note creation fields have unsupported types.');
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

/** Textual split that tolerates malformed frontmatter YAML (used by fix_yaml). */
function splitRaw(content: string): { bom: string; newline: string; body: string; hadFrontmatter: boolean } {
  const bom = content.startsWith('\uFEFF') ? '\uFEFF' : '';
  const raw = content.slice(bom.length);
  const opening = /^---(\r\n|\n|\r)/u.exec(raw);
  const newline = opening?.[1] ?? (content.includes('\r\n') ? '\r\n' : '\n');
  if (!opening) return { bom, newline, body: raw, hadFrontmatter: false };
  const closing = /^(---|\.\.\.)(?:\r\n|\n|\r|$)/gmu;
  closing.lastIndex = opening[0].length;
  const match = closing.exec(raw);
  if (!match) return { bom, newline, body: raw, hadFrontmatter: false };
  return { bom, newline, body: raw.slice(match.index + match[0].length), hadFrontmatter: true };
}

/** One heading in an {@link outlineHeadings} result (REQ-017). */
export interface OutlineHeading {
  level: number;
  kind: 'atx';
  title: string;
  titleTruncated: boolean;
  start: number;        // offset in `content` where the heading line begins
  contentStart: number; // offset in `content` where this heading's section body begins
  end: number;          // offset of the next same-or-higher-level heading, else content.length
}

const OUTLINE_TITLE_MAX = 300;

/**
 * Bounded, code-fence-aware ATX heading map for one note (REQ-017 / BR-OUTLINE-*).
 * A heuristic structural parse, NOT a full CommonMark/Obsidian parser: setext
 * headings are intentionally out of scope in this Unit, and a `#` inside fenced
 * code, an inline-code span, or an HTML comment is excluded via visibleMarkdown.
 * Offsets are UTF-16 code-unit indices into the full `content`, identical to the
 * basis of read_note's slice, so a section is read by composing with read_note.
 * Correctness of the offsets relies on visibleMarkdown being length-preserving
 * (BR-VISIBLE-1).
 */
export function outlineHeadings(content: string): OutlineHeading[] {
  if (typeof content !== 'string') fail('outline requires string content.');
  const { body } = splitRaw(content);
  const bodyStart = content.length - body.length;
  const visible = visibleMarkdown(body);
  const vlines = visible.split(/(?<=\n)/u);
  const blines = body.split(/(?<=\n)/u);
  const partial: Omit<OutlineHeading, 'end'>[] = [];
  let offset = 0;
  for (let index = 0; index < vlines.length; index++) {
    const vline = vlines[index] ?? '';
    const lineStart = offset;
    offset += vline.length;
    const detect = /^( {0,3}#{1,6})(?:([\t ]+)(.*?))?[\t ]*$/u.exec(vline.replace(/\r?\n$/u, ''));
    if (!detect) continue;
    const level = (detect[1] ?? '').trim().length;
    const prefix = (detect[1] ?? '').length + (detect[2]?.length ?? 0);
    let title = (blines[index] ?? '').replace(/\r?\n$/u, '').slice(prefix).replace(/[\t ]+#+[\t ]*$/u, '').replace(/[\t ]+$/u, '');
    const points = [...title];
    const titleTruncated = points.length > OUTLINE_TITLE_MAX;
    if (titleTruncated) title = points.slice(0, OUTLINE_TITLE_MAX).join('');
    partial.push({ level, kind: 'atx', title, titleTruncated, start: bodyStart + lineStart, contentStart: bodyStart + offset });
  }
  return partial.map((heading, index) => {
    let end = content.length;
    for (let next = index + 1; next < partial.length; next++) {
      if ((partial[next]?.level ?? 0) <= heading.level) { end = partial[next]!.start; break; }
    }
    return { ...heading, end };
  });
}

/**
 * Replace a note's (possibly malformed) frontmatter with corrected YAML,
 * preserving the body verbatim. Rejects if the corrected block is not a valid
 * bounded YAML mapping (BR-STRUCT-2 — no guessing/auto-repair beyond the
 * supplied, valid correction). Used by S1.fixYaml.
 */
export function fixYamlContent(content: string, correctedFrontmatter: string): string {
  return mutationResult(() => {
    if (typeof content !== 'string' || typeof correctedFrontmatter !== 'string') fail('fix_yaml requires string content and corrected frontmatter.');
    const split = splitRaw(content);
    const normalized = correctedFrontmatter.replace(/\r\n|\r|\n/gu, split.newline).replace(/(?:\r\n|\r|\n)+$/u, '');
    const result = `${split.bom}---${split.newline}${normalized}${split.newline}---${split.newline}${split.body}`;
    // frontmatter() enforces a valid, unique-keyed, bounded JSON-compatible
    // mapping; it throws (-> InvalidNote) if the corrected YAML is still malformed.
    frontmatter(result);
    validateNote('note.md', result);
    return result;
  });
}

/**
 * Replace only the note body, preserving the original frontmatter block byte
 * for byte (BR-STRUCT-1). Requires the current note to be well-formed. Used by
 * S1.updateNote for body changes.
 */
export function replaceBody(content: string, body: string): string {
  return mutationResult(() => {
    if (typeof body !== 'string') fail('body must be a string.');
    const parsed = frontmatter(content); // validates the current note is well-formed
    const prefix = content.slice(0, content.length - parsed.body.length); // exact bom+fences+yaml
    const result = `${prefix}${body}`;
    validateNote('note.md', result);
    return result;
  });
}

/**
 * Literal in-note source/link reinforcement (BR-STRUCT-4): sets the `source`
 * frontmatter key and/or appends literal body text. No link-graph resolution,
 * rename-time relinking, or full Obsidian link interpretation. Used by
 * S1.reinforceSourcesLinks. Requires the current note to already be valid.
 */
export function reinforceContent(content: string, changes: { source?: string | string[]; appendBody?: string }): string {
  return mutationResult(() => {
    if (typeof content !== 'string') fail('reinforce requires string content.');
    if (changes.source === undefined && changes.appendBody === undefined) fail('reinforce requires a source and/or appendBody.');
    let result = content;
    if (changes.source !== undefined) {
      result = patchFrontmatter(result, { source: changes.source });
    }
    if (changes.appendBody !== undefined) {
      if (typeof changes.appendBody !== 'string') fail('appendBody must be a string.');
      if (changes.appendBody.length > 0) {
        const nl = result.includes('\r\n') ? '\r\n' : '\n';
        const sep = /\r|\n/u.test(result.slice(-1)) ? '' : nl;
        result = `${result}${sep}${changes.appendBody}`;
      }
    }
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
    findings.push({ path, code, category: categoryFor(code), severity, message });
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
      const resolved = resolveWikiLink(note.path, target, { byPath, lookup, allFiles });
      if (resolved.status === 'unsafe') {
        add(note.path, 'OKC_LINK_UNSAFE', 'warning', resolved.reason === 'escape'
          ? 'A wikilink may escape the source Vault.'
          : 'A wikilink uses an absolute, URI-like, or nonportable target; review locally.');
        continue;
      }
      if (resolved.status === 'external') continue;
      if (resolved.status === 'unresolved') { add(note.path, 'OKC_LINK_UNRESOLVED', 'warning', 'A wikilink target could not be found by the authoring heuristic.'); continue; }
      if (resolved.status === 'ambiguous') { add(note.path, 'OKC_LINK_AMBIGUOUS', 'warning', 'A wikilink has multiple possible targets; no target was chosen.'); continue; }
      if (resolved.fragment) {
        const candidate = byPath.get(key(resolved.candidates[0] ?? ''));
        if (!candidate || candidate.body === undefined) continue;
        const found = resolved.fragment.startsWith('^')
          ? candidate.blockIds.has(resolved.fragment.slice(1))
          : candidate.headings.has(key(resolved.fragment));
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

/** Body used for link extraction: after valid frontmatter, else the whole note. */
function bodyForLinks(content: string): string {
  try { return frontmatter(content).body; } catch { return content; }
}

export interface Backlink {
  path: string;
  sha256: string;
  linkText: string;
  fragment: string;
  embed: boolean;
  ambiguous: boolean;
  candidates: string[];
  excerpt: string;
}

/**
 * Inbound wikilinks to `targetPath` (REQ-016 / BR-LINK-*). Pure and deterministic:
 * builds the vault index (byPath/lookup/allFiles) exactly like auditNotes, then
 * reuses the shared resolveWikiLink over each source note's documented-subset
 * occurrences, keeping an edge only when a resolved/ambiguous candidate IS the
 * target. Ambiguous namesakes are reported (never auto-chosen); nothing is read
 * from the filesystem. Excerpts are verbatim body slices and rely on
 * visibleMarkdown being length-preserving (BR-VISIBLE-1).
 */
export function backlinksOf(
  targetPath: string,
  notes: { path: string; content: string; sha256: string }[],
  otherFiles: string[] = [],
): Backlink[] {
  const targetKey = key(targetPath);
  const byPath = new Map<string, { path: string }>();
  const lookup = new Map<string, Set<string>>();
  const allFiles = new Set([...notes.map(note => key(note.path)), ...otherFiles.map(key)]);
  for (const note of notes) {
    byPath.set(key(note.path), { path: note.path });
    const analysis = analyzeNote(note.path, note.content);
    for (const name of [analysis.title, posix.basename(note.path, '.md'), ...analysis.aliases]) {
      const names = lookup.get(key(name)) ?? new Set<string>();
      names.add(note.path);
      lookup.set(key(name), names);
    }
  }
  const backlinks: Backlink[] = [];
  for (const note of notes) {
    const body = bodyForLinks(note.content);
    const visible = visibleMarkdown(body);
    const seen = new Set<string>();
    for (const occurrence of wikiLinkOccurrences(visible)) {
      const dedupe = `${occurrence.embed ? '!' : ''}${occurrence.raw}`;
      if (seen.has(dedupe)) continue;
      seen.add(dedupe);
      const resolved = resolveWikiLink(note.path, occurrence.target, { byPath, lookup, allFiles });
      if (resolved.status !== 'resolved' && resolved.status !== 'ambiguous') continue;
      if (!resolved.candidates.some(candidate => key(candidate) === targetKey)) continue;
      backlinks.push({
        path: note.path, sha256: note.sha256, linkText: occurrence.raw, fragment: resolved.fragment,
        embed: occurrence.embed, ambiguous: resolved.status === 'ambiguous', candidates: resolved.candidates,
        excerpt: body.slice(Math.max(0, occurrence.index - 60), Math.max(0, occurrence.index - 60) + 180),
      });
    }
  }
  return backlinks.sort((left, right) =>
    left.path < right.path ? -1 : left.path > right.path ? 1 : left.linkText < right.linkText ? -1 : left.linkText > right.linkText ? 1 : 0);
}

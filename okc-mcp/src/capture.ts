import { randomUUID } from 'node:crypto';
import { posix } from 'node:path';
import { z } from 'zod';
import { applyMutation } from './authoring.js';
import type { Config } from './config.js';
import { analyzeNote, createNoteContent, outlineHeadings, validateNote } from './notes.js';
import { foldText } from './search.js';
import { Vault, VaultError, sha256 } from './vault.js';

const id = z.string().min(1).max(64).regex(/^[A-Za-z0-9][A-Za-z0-9_-]*$/u);
const sessionId = z.string().min(1).max(200).refine(value =>
  !/[\x00-\x1f\x7f]/u.test(value) && Buffer.from(value).toString('utf8') === value, 'Use a stable, single-line session identifier.');
const headingPath = z.array(z.string().min(1).max(300)).min(1).max(6);
// The host must declare a current user selection. This is not an authentication
// claim: the MCP server cannot independently inspect the user's conversation.
const userSelected = z.boolean().default(false).describe('True only when the user explicitly selected this session/content to save in the current request. Never infer from startup, progress, compaction or a previous capture.');
export const prepareCaptureSchema = z.object({
  userSelected,
  sessionId: sessionId.optional(),
  topics: z.array(z.object({
    id, summary: z.string().min(1).max(500),
    queries: z.array(z.string().min(1).max(160)).min(1).max(6),
  }).strict()).min(1).max(8),
  limit: z.number().int().min(1).max(5).default(3),
}).strict();
export const applyCaptureSchema = z.object({
  userSelected,
  sessionId,
  items: z.array(z.object({
    id, path: z.string().min(1).max(1024),
    expectedHash: z.string().regex(/^[a-f0-9]{64}$/u).nullable(),
    title: z.string().min(1).max(300).optional(),
    section: headingPath.optional(),
    content: z.string().min(1).max(1_000_000),
    rationale: z.string().min(1).max(300),
  }).strict()).min(1).max(12),
  dryRun: z.boolean().default(true),
}).strict();
export type CaptureTopic = z.infer<typeof prepareCaptureSchema>['topics'][number];
export type CaptureItem = z.infer<typeof applyCaptureSchema>['items'][number];
type LocalNote = Awaited<ReturnType<Vault['read']>>;
export interface CaptureBlock { sessionKey: string; itemId: string; start: number; end: number }
export interface CaptureSection { path: string[]; start: number; contentStart: number; end: number; level: number }
const PREFIX = '<!-- okc-capture:';
const CLOSE_PREFIX = '<!-- /okc-capture:';
const MARKER = /^<!-- (\/?)okc-capture:v1:([a-f0-9]{64}):([A-Za-z0-9][A-Za-z0-9_-]{0,63}) -->$/u;
const INSTRUCTION_FILES = new Set(['agents.md', 'claude.md', '_claude.md']);
const clip = (text: string, length: number): string => [...text].slice(0, length).join('');
const samePath = (a: readonly string[], b: readonly string[]): boolean => JSON.stringify(a) === JSON.stringify(b);

function fail(code: string, message: string): never { throw new VaultError(code, message); }
function cancelled(signal?: AbortSignal): void {
  if (signal?.aborted) fail('CAPTURE_CANCELLED', 'Session capture was cancelled; inspect its receipts before retrying.');
}
export const captureSessionKey = (value: string): string => sha256(`okc-session-v1\0${sessionId.parse(value)}`);

/** Balanced, line-anchored identity markers outside fenced code. No content is executed. */
export function captureBlocks(content: string): CaptureBlock[] {
  const blocks: CaptureBlock[] = [];
  let active: Omit<CaptureBlock, 'end'> | undefined;
  let fence: { char: string; size: number } | undefined;
  let offset = 0;
  for (const raw of content.split(/(?<=\n)/u)) {
    const line = raw.replace(/\r?\n$/u, '');
    const start = offset;
    offset += raw.length;
    const boundary = /^ {0,3}(`{3,}|~{3,})(.*)$/u.exec(line);
    if (fence) {
      if (boundary?.[1]?.[0] === fence.char && boundary[1].length >= fence.size && !boundary[2]?.trim()) fence = undefined;
      continue;
    }
    if (boundary) { fence = { char: boundary[1]![0]!, size: boundary[1]!.length }; continue; }
    if (!line.startsWith(PREFIX) && !line.startsWith(CLOSE_PREFIX)) continue;
    const marker = MARKER.exec(line);
    if (!marker) fail('CAPTURE_MARKER_INVALID', 'Malformed session marker; repair the marked note before capture.');
    const [, closing, key, item] = marker;
    if (!closing) {
      if (active) fail('CAPTURE_MARKER_INVALID', 'Nested session markers cannot be edited safely.');
      active = { sessionKey: key!, itemId: item!, start };
    } else {
      if (!active || active.sessionKey !== key || active.itemId !== item) fail('CAPTURE_MARKER_INVALID', 'Unmatched session marker.');
      blocks.push({ ...active, end: offset });
      active = undefined;
      if (blocks.length > 512) fail('SCAN_LIMIT', 'A note contains too many session records.');
    }
  }
  if (active) fail('CAPTURE_MARKER_INVALID', 'Unclosed session marker.');
  const identities = blocks.map(block => `${block.sessionKey}:${block.itemId}`);
  if (new Set(identities).size !== identities.length) fail('CAPTURE_DUPLICATE', 'A session item occurs more than once in a note.');
  return blocks;
}

/** Paths address direct heading bodies; generated blocks do not become destination headings. */
export function captureSections(content: string): CaptureSection[] {
  const blocks = captureBlocks(content);
  const headings = outlineHeadings(content).filter(heading =>
    !heading.titleTruncated && !blocks.some(block => heading.start >= block.start && heading.start < block.end));
  const stack: { level: number; title: string }[] = [];
  return headings.map((heading, index) => {
    while (stack.length && stack[stack.length - 1]!.level >= heading.level) stack.pop();
    stack.push({ level: heading.level, title: heading.title });
    return { path: stack.map(item => item.title), start: heading.start, contentStart: heading.contentStart,
      end: headings[index + 1]?.start ?? content.length, level: heading.level };
  });
}

export function upsertCaptureBlock(content: string, key: string, item: Pick<CaptureItem, 'id' | 'content' | 'section'>): string {
  id.parse(item.id);
  if (!/^[a-f0-9]{64}$/u.test(key)) fail('CAPTURE_INVALID', 'Invalid session key.');
  if (!item.content.trim() || item.content.includes(PREFIX) || item.content.includes(CLOSE_PREFIX)) {
    fail('CAPTURE_INVALID', 'Capture content must be non-empty and cannot contain reserved session markers.');
  }
  const blocks = captureBlocks(content);
  const previous = blocks.find(block => block.sessionKey === key && block.itemId === item.id);
  let section: CaptureSection | undefined;
  if (item.section) {
    const matches = captureSections(content).filter(candidate => samePath(candidate.path, item.section!));
    if (matches.length !== 1) fail('CAPTURE_SECTION_INVALID', 'Choose one unambiguous full section path returned by preparation.');
    section = matches[0]!;
    if (previous && (previous.start < section.contentStart || previous.end > section.end)) {
      fail('CAPTURE_TARGET_CHANGED', 'This session item is already recorded in a different section; retain its existing location.');
    }
    if (outlineHeadings(item.content).some(heading => heading.level <= section!.level)) {
      fail('CAPTURE_SECTION_INVALID', 'Record content headings must be deeper than the destination heading.');
    }
  }
  const newline = content.includes('\r\n') ? '\r\n' : '\n';
  const body = item.content.replace(/\r\n?|\n/gu, newline).replace(/(?:\r?\n)+$/u, '');
  const block = `${PREFIX}v1:${key}:${item.id} -->${newline}${body}${newline}${CLOSE_PREFIX}v1:${key}:${item.id} -->${newline}`;
  if (previous) {
    const result = content.slice(0, previous.start) + block + content.slice(previous.end);
    if (!captureBlocks(result).some(value => value.sessionKey === key && value.itemId === item.id)) {
      fail('CAPTURE_MARKER_INVALID', 'The record would be hidden inside an unclosed code fence.');
    }
    return result;
  }
  const position = section?.end ?? content.length;
  const before = content.slice(0, position);
  const after = content.slice(position);
  const gap = before.endsWith(newline + newline) ? '' : before.endsWith(newline) ? newline : newline + newline;
  const result = before + gap + block + (after ? newline : '') + after;
  if (!captureBlocks(result).some(value => value.sessionKey === key && value.itemId === item.id)) {
    fail('CAPTURE_MARKER_INVALID', 'The record would be hidden inside an unclosed code fence.');
  }
  return result;
}

export function rankCaptureCandidates(notes: readonly LocalNote[], topic: CaptureTopic, limit: number) {
  const terms = [...new Set(topic.queries.flatMap(query => [foldText(query), ...foldText(query).match(/[\p{L}\p{N}_-]{2,}/gu) ?? []]))].slice(0, 48);
  return notes.map(note => {
    const analysis = analyzeNote(note.path, note.content);
    const sections = captureSections(note.content);
    const fields: [string, number, string][] = [
      [foldText(note.path), 5, 'path'], [foldText(analysis.title), 6, 'title'],
      [foldText(analysis.aliases.join(' ')), 6, 'alias'], [foldText(analysis.tags.join(' ')), 4, 'tag'],
      [foldText(sections.map(section => section.path.join(' ')).join(' ')), 4, 'heading'],
      [foldText(note.content), 1, 'body'],
    ];
    let score = 0;
    const matched = new Set<string>();
    for (const term of terms) {
      for (const [value, weight, field] of fields) if (term.trim() && value.includes(term)) { score += weight; matched.add(field); }
    }
    return { path: note.path, sha256: note.sha256, title: clip(analysis.title, 160), score,
      matchedFields: [...matched], aliases: analysis.aliases.slice(0, 8).map(value => clip(value, 80)),
      tags: analysis.tags.slice(0, 8).map(value => clip(value, 80)),
      sections: sections.slice(0, 20).map(section => section.path), sectionsTruncated: sections.length > 20,
      excerpt: clip(note.content, 280), invalid: analysis.issues.some(issue => issue.severity === 'error') };
  }).filter(note => note.score > 0 && !INSTRUCTION_FILES.has(posix.basename(note.path).toLowerCase()))
    .sort((a, b) => b.score - a.score || (a.path < b.path ? -1 : a.path > b.path ? 1 : 0)).slice(0, limit);
}

interface FilePlan { path: string; before: LocalNote | null; content: string; items: CaptureItem[]; action: 'create' | 'update' | 'unchanged' }
export interface CaptureFileReceipt {
  path: string; action: FilePlan['action']; itemIds: string[]; rationales: string[];
  beforeSha256: string | null; sha256: string; applied: boolean; verified: boolean; backupId?: string;
}
export interface CaptureReceipt {
  sessionId: string; sessionKey: string; status: 'preview' | 'applied' | 'unchanged' | 'partial' | 'failed';
  trigger: 'user-selected';
  source: { kind: 'local'; publication: 'pending-upload-and-integration' };
  files: CaptureFileReceipt[];
  failure?: { path: string; code: string; message: string };
  remainingPaths?: string[];
  previews?: { path: string; items: { id: string; section: string[] | null; content: string; truncated: boolean }[] }[];
}

/** Semantic choices stay with the host; this service supplies evidence and safe persistence. */
export class SessionCapture {
  private applying = false;
  constructor(private readonly vault: Vault, private readonly config: Config) {}

  private async scan(signal?: AbortSignal): Promise<LocalNote[]> {
    const listed = await this.vault.list();
    const notes: LocalNote[] = [];
    let bytes = 0;
    for (const path of listed.notes) {
      cancelled(signal);
      const note = await this.vault.read(path);
      bytes += Buffer.byteLength(note.content);
      if (bytes > this.config.maxScanBytes) fail('SCAN_LIMIT', 'Local capture scan exceeds maxScanBytes.');
      notes.push(note);
    }
    return notes;
  }

  private locations(notes: readonly LocalNote[], key: string) {
    const found = new Map<string, { path: string; sha256: string; section?: string[] }>();
    for (const note of notes) {
      for (const block of captureBlocks(note.content).filter(block => block.sessionKey === key)) {
        if (found.has(block.itemId)) fail('CAPTURE_DUPLICATE', `Session item ${block.itemId} occurs in multiple notes.`);
        const section = captureSections(note.content).find(value => block.start >= value.contentStart && block.end <= value.end)?.path;
        found.set(block.itemId, { path: note.path, sha256: note.sha256, ...(section ? { section } : {}) });
      }
    }
    if (found.size > 128) fail('SCAN_LIMIT', 'Too many records in this session; split future work into separate sessions.');
    return found;
  }

  async prepare(input: z.input<typeof prepareCaptureSchema>, signal?: AbortSignal) {
    const request = prepareCaptureSchema.parse(input);
    if (!request.userSelected) fail('CAPTURE_SELECTION_REQUIRED', 'Capture only the session/content explicitly selected by the user; no automatic session recording.');
    if (new Set(request.topics.map(topic => topic.id)).size !== request.topics.length) fail('CAPTURE_INVALID', 'Topic IDs must be unique.');
    const session = request.sessionId ?? randomUUID();
    const key = captureSessionKey(session);
    const notes = await this.scan(signal);
    const previous = this.locations(notes, key);
    const folders = new Map<string, number>();
    for (const note of notes) {
      const folder = posix.dirname(note.path);
      folders.set(folder, (folders.get(folder) ?? 0) + 1);
    }
    cancelled(signal);
    return {
      sessionId: session, sessionKey: key, trigger: 'user-selected' as const, source: { kind: 'local' as const }, untrusted: true,
      guide: 'okc://guide/session-capture',
      retrieval: 'bounded-lexical-candidates; the host must read candidates and decide semantic relevance',
      coverage: { notesScanned: notes.length, foldersTotal: folders.size, foldersTruncated: folders.size > 32 },
      folders: [...folders].sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : 1)).slice(0, 32).map(([path, notes]) => ({ path, notes })),
      organizationHints: notes.filter(note => INSTRUCTION_FILES.has(posix.basename(note.path).toLowerCase())).slice(0, 3)
        .map(note => ({ path: note.path, excerpt: clip(note.content, 2000), truncated: [...note.content].length > 2000 })),
      existingItems: [...previous].map(([id, location]) => ({ id, ...location })),
      topics: request.topics.map(topic => ({ id: topic.id, summary: topic.summary,
        candidates: rankCaptureCandidates(notes, topic, request.limit) })),
      next: 'For this user-selected session only: reuse sessionId/item IDs, read local candidates and choose destinations. Preview apply_session_capture and apply with userSelected=true under this save request. Report receipts, then stop capturing; a future capture needs a new user selection.',
    };
  }

  private receipt(session: string, plans: readonly FilePlan[], preview: boolean): CaptureReceipt {
    return { sessionId: session, sessionKey: captureSessionKey(session), trigger: 'user-selected', status: preview ? 'preview' : 'unchanged',
      source: { kind: 'local', publication: 'pending-upload-and-integration' },
      files: plans.map(plan => ({ path: plan.path, action: plan.action, itemIds: plan.items.map(item => item.id),
        rationales: plan.items.map(item => item.rationale), beforeSha256: plan.before?.sha256 ?? null,
        sha256: sha256(plan.content), applied: false, verified: false })) };
  }

  private reserveResponse(receipt: CaptureReceipt): void {
    const worst = { ok: false, data: { ...receipt, status: 'partial',
      files: receipt.files.map(file => ({ ...file, backupId: 'x'.repeat(128) })),
      failure: { path: receipt.files.reduce((longest, file) => file.path.length > longest.length ? file.path : longest, ''),
        code: 'CAPTURE_VERIFICATION_FAILED', message: 'Session capture stopped; prepare again and inspect the saved file receipts before retrying.' },
      remainingPaths: receipt.files.map(file => file.path) },
      error: { code: 'CAPTURE_PARTIAL', message: 'Some notes could not be saved. Inspect the returned file receipts and prepare again.' } };
    if (Buffer.byteLength(JSON.stringify(worst)) * 2 + 1024 > this.config.maxResponseBytes) {
      fail('RESPONSE_LIMIT', 'Capture receipts exceed maxResponseBytes; apply fewer items per call.');
    }
  }

  async apply(input: z.input<typeof applyCaptureSchema>, signal?: AbortSignal): Promise<CaptureReceipt> {
    const request = applyCaptureSchema.parse(input);
    if (!request.userSelected) fail('CAPTURE_SELECTION_REQUIRED', 'Capture only the session/content explicitly selected by the user; no automatic session recording.');
    if (this.config.readOnly) fail('READ_ONLY', 'Session capture requires a writable local Vault.');
    if (this.applying) fail('VAULT_BUSY', 'Another session capture is in progress.');
    this.applying = true;
    try {
      const key = captureSessionKey(request.sessionId);
      if (new Set(request.items.map(item => item.id)).size !== request.items.length) fail('CAPTURE_INVALID', 'Each item ID must appear once per capture call.');
      if (request.items.reduce((size, item) => size + Buffer.byteLength(item.content), 0) > this.config.maxScanBytes) fail('SCAN_LIMIT', 'Capture content exceeds maxScanBytes.');
      const previous = this.locations(await this.scan(signal), key);
      const groups = new Map<string, CaptureItem[]>();
      const aliases = new Map<string, string>();
      for (const item of request.items) {
        if (INSTRUCTION_FILES.has(posix.basename(item.path).toLowerCase())) fail('INVALID_PATH', 'Session capture cannot modify agent instruction files.');
        const recorded = previous.get(item.id);
        if (recorded && recorded.path !== item.path) fail('CAPTURE_TARGET_CHANGED', `Reuse the existing location for item ${item.id}: ${recorded.path}`);
        const alias = item.path.normalize('NFC').toLowerCase();
        if (aliases.has(alias) && aliases.get(alias) !== item.path) fail('PATH_COLLISION', 'Capture destinations collide by case or Unicode normalization.');
        aliases.set(alias, item.path);
        groups.set(item.path, [...groups.get(item.path) ?? [], item]);
      }
      for (const path of groups.keys()) {
        if ([...groups.keys()].some(other => other !== path && other.normalize('NFC').toLowerCase().startsWith(path.normalize('NFC').toLowerCase() + '/'))) {
          fail('PATH_COLLISION', 'A capture note cannot also be a parent directory of another destination.');
        }
      }
      const plans: FilePlan[] = [];
      for (const [path, items] of [...groups].sort(([a], [b]) => a < b ? -1 : 1)) {
        cancelled(signal);
        const before = await this.vault.inspectWriteTarget(path);
        if (new Set(items.map(item => item.expectedHash)).size !== 1) fail('CAPTURE_INVALID', 'Items for one note must use the same read hash.');
        const first = items[0]!;
        if (!before && (first.expectedHash !== null || !first.title)) fail('CAPTURE_INVALID', 'A new note requires a title and expectedHash=null.');
        if (!before && new Set(items.map(item => item.title).filter(Boolean)).size > 1) fail('CAPTURE_INVALID', 'Items creating one note must agree on its title.');
        let content = before?.content ?? createNoteContent({ title: first.title!, body: `# ${first.title!}\n\n` });
        for (const item of [...items].sort((a, b) => a.id < b.id ? -1 : 1)) content = upsertCaptureBlock(content, key, item);
        const action = before ? content === before.content ? 'unchanged' : 'update' : 'create';
        if (before && first.expectedHash === null && action !== 'unchanged') fail('NOTE_EXISTS', 'The selected new-note path already exists; read it and prepare an update.');
        if (action === 'update' && first.expectedHash !== before!.sha256) fail('CONFLICT', 'A destination changed; prepare again and read its current content.');
        if (Buffer.byteLength(content) > this.config.maxNoteBytes) fail('NOTE_TOO_LARGE', 'A capture destination would exceed maxNoteBytes.');
        validateNote(path, content);
        if (action !== 'unchanged') await applyMutation(this.vault, this.config, { path, isCreate: !before,
          dryRun: true, ...(before ? { expectedHash: before.sha256 } : {}), build: () => content });
        plans.push({ path, before, content, items, action });
      }
      const receipt = this.receipt(request.sessionId, plans, request.dryRun);
      this.reserveResponse(receipt);
      if (request.dryRun) {
        receipt.previews = plans.map(plan => ({ path: plan.path, items: plan.items.map(item => ({
          id: item.id, section: item.section ?? null, content: clip(item.content, 1200), truncated: [...item.content].length > 1200,
        })) }));
        // Preview can be refused safely; no note/state directory has been written.
        if (Buffer.byteLength(JSON.stringify({ ok: true, data: receipt })) * 2 + 1024 > this.config.maxResponseBytes) {
          fail('RESPONSE_LIMIT', 'Capture preview exceeds maxResponseBytes; preview fewer items per call.');
        }
        return receipt;
      }
      for (let index = 0; index < plans.length; index++) {
        const plan = plans[index]!;
        const file = receipt.files[index]!;
        try {
          cancelled(signal);
          if (plan.action !== 'unchanged') {
            const saved = await applyMutation(this.vault, this.config, { path: plan.path, isCreate: !plan.before,
              dryRun: false, ...(plan.before ? { expectedHash: plan.before.sha256 } : {}), build: () => plan.content });
            file.applied = saved.applied;
            if (saved.backupId) file.backupId = saved.backupId;
          }
          const readback = await this.vault.read(plan.path);
          if (readback.sha256 !== file.sha256) fail('CAPTURE_VERIFICATION_FAILED', 'The saved note changed before readback verification.');
          file.verified = true;
        } catch (error) {
          receipt.status = receipt.files.some(file => file.applied) ? 'partial' : 'failed';
          receipt.failure = { path: plan.path, code: error instanceof VaultError ? error.code : 'CAPTURE_IO_FAILED',
            message: 'Session capture stopped; prepare again and inspect the saved file receipts before retrying.' };
          receipt.remainingPaths = plans.slice(index).map(plan => plan.path);
          return receipt;
        }
      }
      receipt.status = receipt.files.some(file => file.applied) ? 'applied' : 'unchanged';
      return receipt;
    } finally { this.applying = false; }
  }
}

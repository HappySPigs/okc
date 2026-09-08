/**
 * S1 · AuthoringService — the single, fixed safe write pipeline.
 *
 * Every note mutation funnels through `applyMutation`
 * (services.md, business-logic-model.md W2, nfr-design P2 — "no alternative
 * write path exists"). Each public method is a thin wrapper choosing an
 * operation-specific content `build` function and the `isCreate` flag, so the
 * conflict-check + external-backup + atomic-write guarantees hold uniformly and
 * new tools cannot silently skip them.
 */
import type { Config } from './config.js';
import {
  analyzeNote,
  createNoteContent,
  fixYamlContent,
  patchFrontmatter,
  reinforceContent,
  replaceBody,
  validateNote,
  type Finding,
} from './notes.js';
import { Vault, VaultError, sha256 } from './vault.js';

const PREVIEW_CHARS = 4000;

/** Builds the full new note content. `current` is null only for creates. */
type Build = (current: string | null) => string;

export interface MutationResult {
  applied: boolean;
  path: string;
  sha256?: string;
  proposedSha256?: string;
  previousSha256?: string;
  backupId?: string;
  changedKeys?: string[];
  issues?: Finding[];
  preview?: string;
  previewTruncated?: boolean;
}

interface MutationSpec {
  path: string;
  isCreate: boolean;
  build: Build;
  dryRun: boolean;
  expectedHash?: string; // required when !isCreate
  changedKeys?: string[]; // informational, echoed on dry-run
}

/**
 * The fixed pipeline. Rejections short-circuit BEFORE any write or backup, so a
 * rejected mutation never leaves a file change or a misleading backup
 * (BR-BACKUP-2, BR-REJECT-1). `dryRun` (default true at the surface) previews
 * without writing — REQ-011 no auto-approval.
 */
export async function applyMutation(vault: Vault, config: Config, spec: MutationSpec): Promise<MutationResult> {
  if (spec.isCreate) {
    const rendered = spec.build(null); // may throw malformed-yaml / invalid metadata
    if (Buffer.byteLength(rendered) > config.maxNoteBytes) throw new VaultError('NOTE_LIMIT', 'Rendered note exceeds maxNoteBytes.');
    validateNote(spec.path, rendered);
    if (spec.dryRun) {
      return {
        applied: false, path: spec.path, proposedSha256: sha256(rendered),
        issues: analyzeNote(spec.path, rendered).issues,
        preview: rendered.slice(0, PREVIEW_CHARS), previewTruncated: rendered.length > PREVIEW_CHARS,
      };
    }
    const res = await vault.create(spec.path, rendered); // refuses overwrite (BR-CREATE-1); no backup
    return { applied: true, path: res.path, sha256: res.sha256 };
  }

  // Update path: require a matching expectedHash, then backup + atomic write.
  if (typeof spec.expectedHash !== 'string' || !/^[a-f0-9]{64}$/.test(spec.expectedHash)) {
    throw new VaultError('INVALID_HASH', 'expectedHash (the SHA-256 from read_note) is required for updates.');
  }
  const current = await vault.read(spec.path); // path-denied / not-found / bounds enforced here
  if (current.sha256 !== spec.expectedHash) {
    throw new VaultError('CONFLICT', 'Note changed since it was read. Read the current note and review the edit again.');
  }
  const rendered = spec.build(current.content); // may throw malformed-yaml
  if (Buffer.byteLength(rendered) > config.maxNoteBytes) throw new VaultError('NOTE_LIMIT', 'Note exceeds maxNoteBytes.');
  validateNote(spec.path, rendered);
  if (spec.dryRun) {
    const base: MutationResult = {
      applied: false, path: spec.path, previousSha256: current.sha256, proposedSha256: sha256(rendered),
      issues: analyzeNote(spec.path, rendered).issues,
      preview: rendered.slice(0, PREVIEW_CHARS), previewTruncated: rendered.length > PREVIEW_CHARS,
    };
    return spec.changedKeys ? { ...base, changedKeys: spec.changedKeys } : base;
  }
  const res = await vault.update(spec.path, rendered, spec.expectedHash); // single external backup + atomic write
  return { applied: true, path: res.path, sha256: res.sha256, previousSha256: current.sha256, backupId: res.backupId };
}

// ---------------------------------------------------------------------------
// Thin wrappers — each chooses build + isCreate; none bypasses applyMutation.
// ---------------------------------------------------------------------------

export interface CreateNoteArgs {
  path: string; title: string; body: string;
  aliases?: string[]; tags?: string[]; source?: string | string[];
  dryRun: boolean;
}
export function createNote(vault: Vault, config: Config, args: CreateNoteArgs): Promise<MutationResult> {
  const { path, dryRun, ...input } = args;
  return applyMutation(vault, config, { path, isCreate: true, dryRun, build: () => createNoteContent(input) });
}

export interface UpdateNoteArgs {
  path: string; expectedHash: string;
  changes: { body?: string | undefined; frontmatter?: Record<string, unknown> | undefined };
  dryRun: boolean;
}
export function updateNote(vault: Vault, config: Config, args: UpdateNoteArgs): Promise<MutationResult> {
  const { path, expectedHash, changes, dryRun } = args;
  const hasFrontmatter = changes.frontmatter !== undefined && Object.keys(changes.frontmatter).length > 0;
  const hasBody = changes.body !== undefined;
  if (!hasFrontmatter && !hasBody) throw new VaultError('NOTE_INVALID', 'update_note requires a body and/or frontmatter change.');
  const changedKeys = [...(hasFrontmatter ? Object.keys(changes.frontmatter as object) : []), ...(hasBody ? ['body'] : [])];
  return applyMutation(vault, config, {
    path, isCreate: false, expectedHash, dryRun, changedKeys,
    build: (current) => {
      let result = current as string;
      if (hasFrontmatter) result = patchFrontmatter(result, changes.frontmatter as Record<string, unknown>);
      if (hasBody) result = replaceBody(result, changes.body as string);
      return result;
    },
  });
}

export interface StandardizeArgs {
  path: string; expectedHash: string;
  frontmatterPatch: { title?: string | undefined; aliases?: string[] | undefined; tags?: string[] | undefined };
  dryRun: boolean;
}
export function standardizeFrontmatter(vault: Vault, config: Config, args: StandardizeArgs): Promise<MutationResult> {
  const { path, expectedHash, frontmatterPatch, dryRun } = args;
  const patch: Record<string, unknown> = {};
  for (const key of ['title', 'aliases', 'tags'] as const) {
    if (frontmatterPatch[key] !== undefined) patch[key] = frontmatterPatch[key];
  }
  if (Object.keys(patch).length === 0) throw new VaultError('NOTE_INVALID', 'standardize_frontmatter requires at least one of title/aliases/tags.');
  return applyMutation(vault, config, {
    path, isCreate: false, expectedHash, dryRun, changedKeys: Object.keys(patch),
    build: (current) => patchFrontmatter(current as string, patch),
  });
}

export interface FixYamlArgs {
  path: string; expectedHash: string; correctedFrontmatter: string; dryRun: boolean;
}
export function fixYaml(vault: Vault, config: Config, args: FixYamlArgs): Promise<MutationResult> {
  const { path, expectedHash, correctedFrontmatter, dryRun } = args;
  return applyMutation(vault, config, {
    path, isCreate: false, expectedHash, dryRun,
    build: (current) => fixYamlContent(current as string, correctedFrontmatter),
  });
}

export interface ReinforceArgs {
  path: string; expectedHash: string; source?: string | string[]; appendBody?: string; dryRun: boolean;
}
export function reinforceSourcesLinks(vault: Vault, config: Config, args: ReinforceArgs): Promise<MutationResult> {
  const { path, expectedHash, source, appendBody, dryRun } = args;
  const changes: { source?: string | string[]; appendBody?: string } = {};
  if (source !== undefined) changes.source = source;
  if (appendBody !== undefined) changes.appendBody = appendBody;
  return applyMutation(vault, config, {
    path, isCreate: false, expectedHash, dryRun,
    build: (current) => reinforceContent(current as string, changes),
  });
}

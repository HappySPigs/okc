import { createHash, randomUUID } from 'node:crypto';
import { constants, type Dirent, type Stats } from 'node:fs';
import { link, lstat, mkdir, open, opendir, realpath, rename, unlink } from 'node:fs/promises';
import path from 'node:path';

export class VaultError extends Error {
  constructor(public readonly code: string, message: string) {
    super(message);
    this.name = 'VaultError';
  }
}

export function sha256(content: string | Buffer): string {
  return createHash('sha256').update(content).digest('hex');
}

export interface VaultConfig {
  vaultPath: string;
  statePath: string;
  maxNoteBytes: number;
  maxFiles: number;
  maxScanBytes: number;
}

interface Note {
  path: string;
  content: string;
  sha256: string;
}

const MAX_DEPTH = 20;
const MAX_PATH_BYTES = 1024;
const RESERVED = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i;
const EXCLUDED = new Set(['node_modules', '.git', '.obsidian', '.okc', '.trash']);
const NOFOLLOW = constants.O_NOFOLLOW ?? 0;

function fail(code: string, message: string): never {
  throw new VaultError(code, message);
}

function errorCode(error: unknown): string | undefined {
  return (error as NodeJS.ErrnoException | undefined)?.code;
}

function inside(root: string, candidate: string): boolean {
  const relative = path.relative(root, candidate);
  return relative === '' || (!path.isAbsolute(relative) && relative !== '..' && !relative.startsWith(`..${path.sep}`));
}

function sameIdentity(a: Stats, b: Stats): boolean {
  return a.dev === b.dev && a.ino === b.ino;
}

function regularFile(stat: Stats, name: string): void {
  if (stat.isSymbolicLink()) fail('SYMLINK', `Symbolic links are not allowed: ${name}`);
  if (!stat.isFile()) fail('NOT_REGULAR_FILE', `Expected a regular file: ${name}`);
  if (stat.nlink !== 1) fail('HARDLINK', `Hard-linked files are not allowed: ${name}`);
}

function portableRelative(input: string, markdown: boolean): string[] {
  if (typeof input !== 'string' || input.length === 0 || path.isAbsolute(input) || input.includes('\\')) {
    fail('INVALID_PATH', 'Use an explicit Vault-relative path with forward slashes.');
  }
  if (input !== input.normalize('NFC') || Buffer.byteLength(input) > MAX_PATH_BYTES) {
    fail('INVALID_PATH', 'Paths must be NFC-normalized and at most 1024 UTF-8 bytes.');
  }
  const parts = input.split('/');
  if (parts.length > MAX_DEPTH) fail('PATH_DEPTH', `Paths may have at most ${MAX_DEPTH} components.`);
  for (const part of parts) {
    if (!part || part.startsWith('.') || part.toLowerCase().endsWith('.okc-project') || EXCLUDED.has(part.toLowerCase()) || /[<>:"|?*\x00-\x1f\x7f]/.test(part)
      || /[. ]$/.test(part) || Buffer.byteLength(part) > 255 || RESERVED.test(part)) {
      fail('INVALID_PATH', `Unsafe or reserved path component: ${part}`);
    }
    if (Buffer.from(part).toString('utf8') !== part) fail('INVALID_PATH', 'Paths must contain valid Unicode.');
  }
  if (markdown && !input.endsWith('.md')) fail('INVALID_PATH', 'Note paths must end in lowercase .md.');
  return parts;
}

/**
 * Local authoring only. Hash checks detect observed edits; Node's path-based
 * rename is not a cross-process compare-and-swap against external editors.
 */
export class Vault {
  private root = '';
  private state = '';
  private rootIdentity: Stats | undefined;
  private stateIdentity: Stats | undefined;
  private stateAncestor = '';
  private stateAncestorIdentity: Stats | undefined;
  private stateMissing: string[] = [];
  private vaultId = '';
  private initialized = false;

  constructor(private readonly config: VaultConfig) {}

  async initialize(): Promise<void> {
    if (this.initialized) {
      await this.assertRoots();
      return;
    }
    if (!path.isAbsolute(this.config.vaultPath) || !path.isAbsolute(this.config.statePath)) {
      fail('INVALID_CONFIG', 'vaultPath and statePath must be absolute.');
    }
    for (const key of ['maxNoteBytes', 'maxFiles', 'maxScanBytes'] as const) {
      if (!Number.isSafeInteger(this.config[key]) || this.config[key] <= 0) {
        fail('INVALID_CONFIG', `${key} must be a positive safe integer.`);
      }
    }
    if (this.config.maxNoteBytes > 64 * 1024 * 1024) fail('INVALID_CONFIG', 'maxNoteBytes cannot exceed 64 MiB.');
    const rootStat = await lstat(this.config.vaultPath);
    if (rootStat.isSymbolicLink() || !rootStat.isDirectory()) fail('INVALID_VAULT', 'Vault root must be a real directory.');
    this.root = await realpath(this.config.vaultPath);
    this.rootIdentity = await lstat(this.root);
    await this.assertAuthoringRoot();

    // Resolve existing OS aliases before containment checks, including when
    // the configured state directory does not exist yet.
    let ancestor = path.resolve(this.config.statePath);
    const missing: string[] = [];
    for (;;) {
      try {
        const stat = await lstat(ancestor);
        if (stat.isSymbolicLink() || !stat.isDirectory()) fail('INVALID_STATE', 'State directory ancestry must use real directories.');
        break;
      } catch (error) {
        if (errorCode(error) !== 'ENOENT') throw error;
        const parent = path.dirname(ancestor);
        if (parent === ancestor) throw error;
        missing.unshift(path.basename(ancestor));
        ancestor = parent;
      }
    }
    ancestor = await realpath(ancestor);
    const stateCandidate = path.join(ancestor, ...missing);
    if (inside(this.root, stateCandidate) || inside(stateCandidate, this.root)) {
      fail('STATE_OVERLAP', 'State and Vault directories must be separate, without containment or aliases.');
    }
    this.stateAncestor = ancestor;
    this.stateAncestorIdentity = await lstat(ancestor);
    this.stateMissing = missing;
    this.state = stateCandidate;
    if (missing.length === 0) this.stateIdentity = await lstat(this.state);
    this.vaultId = sha256(this.root);
    this.initialized = true;
    await this.assertRoots();
  }

  private async assertRoots(): Promise<void> {
    if (!this.initialized || !this.rootIdentity) fail('NOT_INITIALIZED', 'Initialize the Vault before use.');
    const roots: [string, string, Stats][] = [['Vault', this.root, this.rootIdentity]];
    if (this.stateIdentity) roots.push(['state', this.state, this.stateIdentity]);
    for (const [name, directory, identity] of roots) {
      const current = await lstat(directory);
      if (current.isSymbolicLink() || !current.isDirectory() || !sameIdentity(identity, current)
        || await realpath(directory) !== directory) {
        fail('ROOT_CHANGED', `${name} directory identity changed; reconnect after inspecting the filesystem.`);
      }
    }
    await this.checkStatePath(false);
    await this.assertAuthoringRoot();
  }

  private async assertAuthoringRoot(): Promise<void> {
    let ancestor = this.root;
    for (;;) {
      if (ancestor.toLowerCase().endsWith('.okc-project')) fail('IMMUTABLE_TARGET', 'An OKC project or its subdirectory is not an authoring Vault.');
      try {
        await lstat(path.join(ancestor, '.okc'));
        fail('IMMUTABLE_TARGET', 'A compiled OKC artifact or its subdirectory is not an authoring Vault.');
      } catch (error) {
        if (errorCode(error) !== 'ENOENT') throw error;
      }
      const parent = path.dirname(ancestor);
      if (parent === ancestor) return;
      ancestor = parent;
    }
  }

  private async checkStatePath(create: boolean): Promise<void> {
    if (!this.stateAncestorIdentity) fail('NOT_INITIALIZED', 'Initialize the Vault before use.');
    const ancestor = await lstat(this.stateAncestor);
    if (ancestor.isSymbolicLink() || !ancestor.isDirectory() || !sameIdentity(ancestor, this.stateAncestorIdentity)
      || await realpath(this.stateAncestor) !== this.stateAncestor) {
      fail('INVALID_STATE', 'State directory ancestry changed.');
    }
    let current = this.stateAncestor;
    for (const component of this.stateMissing) {
      current = path.join(current, component);
      let stat: Stats;
      try { stat = await lstat(current); } catch (error) {
        if (errorCode(error) !== 'ENOENT') throw error;
        if (!create) return;
        try { await mkdir(current, { mode: 0o700 }); } catch (mkdirError) {
          if (errorCode(mkdirError) !== 'EEXIST') throw mkdirError;
        }
        stat = await lstat(current);
      }
      if (stat.isSymbolicLink() || !stat.isDirectory() || await realpath(current) !== current) {
        fail('INVALID_STATE', 'State directory ancestry must use real directories.');
      }
    }
    if (create && !this.stateIdentity) this.stateIdentity = await lstat(this.state);
  }

  private async target(notePath: string, createParents = false): Promise<{ absolute: string; parent: Stats }> {
    const parts = portableRelative(notePath, true);
    await this.assertRoots();
    let current = this.root;
    for (let index = 0; index < parts.length; index++) {
      const component = parts[index]!;
      const entries = (await this.entries(current)).map(entry => entry.name);
      const matches = entries.filter(name => name.normalize('NFC').toLowerCase() === component.toLowerCase());
      if (matches.length > 1 || (matches.length === 1 && matches[0] !== component)) {
        fail('PATH_COLLISION', `Case or Unicode-normalization collision: ${notePath}`);
      }
      const next = path.join(current, component);
      if (index === parts.length - 1) return { absolute: next, parent: await lstat(current) };
      try { await lstat(next); } catch (error) {
        if (errorCode(error) !== 'ENOENT' || !createParents) throw error;
        try { await mkdir(next); } catch (mkdirError) {
          if (errorCode(mkdirError) !== 'EEXIST') throw mkdirError;
        }
      }
      const stat = await lstat(next);
      if (stat.isSymbolicLink() || !stat.isDirectory()) fail('SYMLINK', `Path ancestors must be real directories: ${notePath}`);
      if (!inside(this.root, await realpath(next))) fail('PATH_ESCAPE', 'Path escaped the Vault.');
      current = next;
    }
    fail('INVALID_PATH', 'Missing note path.');
  }

  private async entries(directory: string): Promise<Dirent[]> {
    const entries: Dirent[] = [];
    const handle = await opendir(directory);
    for await (const entry of handle) {
      if (entries.length >= this.config.maxFiles) fail('SCAN_LIMIT', 'Directory entry count exceeds maxFiles.');
      entries.push(entry);
    }
    return entries;
  }

  async list(): Promise<{ notes: string[]; otherFiles: string[]; skipped: string[] }> {
    await this.assertRoots();
    const result = { notes: [] as string[], otherFiles: [] as string[], skipped: [] as string[] };
    let count = 0;
    let bytes = 0;
    const visit = async (directory: string, relative: string, depth: number): Promise<void> => {
      if (depth > MAX_DEPTH) fail('PATH_DEPTH', `Vault exceeds ${MAX_DEPTH} directory levels.`);
      const directoryStat = await lstat(directory);
      if (!directoryStat.isDirectory() || directoryStat.isSymbolicLink() || !inside(this.root, await realpath(directory))) {
        fail('PATH_ESCAPE', 'A directory changed during the scan.');
      }
      const entries = (await this.entries(directory)).sort((a, b) => a.name < b.name ? -1 : a.name > b.name ? 1 : 0);
      const names = new Map<string, number>();
      for (const entry of entries) {
        const key = entry.name.normalize('NFC').toLowerCase();
        names.set(key, (names.get(key) ?? 0) + 1);
      }
      for (const entry of entries) {
        count++;
        if (count > this.config.maxFiles) fail('SCAN_LIMIT', 'Vault entry count exceeds maxFiles; narrow the Vault or adjust the limit.');
        const name = relative ? `${relative}/${entry.name}` : entry.name;
        if (entry.name.startsWith('.') || EXCLUDED.has(entry.name.toLowerCase())) {
          result.skipped.push(`${name}${entry.isDirectory() ? '/' : ''}`);
          continue;
        }
        try { portableRelative(name, false); } catch (error) {
          if (!(error instanceof VaultError)) throw error;
          result.skipped.push(`${name} (${error.code})`);
          continue;
        }
        if ((names.get(entry.name.normalize('NFC').toLowerCase()) ?? 0) > 1) {
          result.skipped.push(`${name} (PATH_COLLISION)`);
          continue;
        }
        const absolute = path.join(directory, entry.name);
        const stat = await lstat(absolute);
        if (stat.isSymbolicLink()) { result.skipped.push(`${name} (SYMLINK)`); continue; }
        if (stat.isDirectory()) { await visit(absolute, name, depth + 1); continue; }
        if (!stat.isFile() || stat.nlink !== 1) {
          result.skipped.push(`${name} (${stat.nlink !== 1 ? 'HARDLINK' : 'NOT_REGULAR_FILE'})`);
          continue;
        }
        bytes += stat.size;
        if (bytes > this.config.maxScanBytes) fail('SCAN_LIMIT', 'Vault bytes exceed maxScanBytes; narrow the Vault or adjust the limit.');
        if (name.endsWith('.md') && stat.size > this.config.maxNoteBytes) {
          result.skipped.push(`${name} (NOTE_TOO_LARGE)`);
        } else if (name.endsWith('.md')) result.notes.push(name);
        else result.otherFiles.push(name);
      }
      if (!sameIdentity(directoryStat, await lstat(directory))) fail('PATH_CHANGED', 'Directory changed during scan.');
    };
    await visit(this.root, '', 0);
    await this.assertRoots();
    result.notes.sort(); result.otherFiles.sort(); result.skipped.sort();
    return result;
  }

  private async readInternal(notePath: string): Promise<Note & { stat: Stats; parent: Stats }> {
    const target = await this.target(notePath);
    let before: Stats;
    try { before = await lstat(target.absolute); } catch (error) {
      if (errorCode(error) === 'ENOENT') fail('NOTE_NOT_FOUND', `Note does not exist: ${notePath}`);
      throw error;
    }
    regularFile(before, notePath);
    if (before.size > this.config.maxNoteBytes) fail('NOTE_TOO_LARGE', `Note exceeds maxNoteBytes: ${notePath}`);
    const handle = await open(target.absolute, constants.O_RDONLY | NOFOLLOW);
    try {
      const acquired = await handle.stat();
      regularFile(acquired, notePath);
      if (!sameIdentity(before, acquired)) fail('FILE_CHANGED', `File changed while opening: ${notePath}`);
      const buffer = Buffer.alloc(this.config.maxNoteBytes + 1);
      let length = 0;
      while (length < buffer.length) {
        const { bytesRead } = await handle.read(buffer, length, buffer.length - length, length);
        if (bytesRead === 0) break;
        length += bytesRead;
      }
      if (length > this.config.maxNoteBytes) fail('NOTE_TOO_LARGE', `Note exceeds maxNoteBytes: ${notePath}`);
      const after = await handle.stat();
      const observed = await this.target(notePath);
      const final = await lstat(observed.absolute);
      regularFile(after, notePath); regularFile(final, notePath);
      if (!sameIdentity(acquired, final) || !sameIdentity(target.parent, observed.parent)
        || acquired.size !== after.size || acquired.mtimeMs !== after.mtimeMs || acquired.ctimeMs !== after.ctimeMs) {
        fail('FILE_CHANGED', `File changed while reading: ${notePath}`);
      }
      let content: string;
      try { content = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true }).decode(buffer.subarray(0, length)); }
      catch { fail('INVALID_UTF8', `Note is not valid UTF-8: ${notePath}`); }
      return { path: notePath, content, sha256: sha256(buffer.subarray(0, length)), stat: after, parent: observed.parent };
    } finally { await handle.close(); }
  }

  async read(notePath: string): Promise<Note> {
    const { stat: _stat, parent: _parent, ...note } = await this.readInternal(notePath);
    return note;
  }

  private contentBytes(content: string): Buffer {
    if (typeof content !== 'string') fail('INVALID_CONTENT', 'Note content must be a string.');
    const bytes = Buffer.from(content, 'utf8');
    if (bytes.toString('utf8') !== content) fail('INVALID_UTF8', 'Content contains invalid Unicode.');
    if (bytes.length > this.config.maxNoteBytes) fail('NOTE_TOO_LARGE', 'Content exceeds maxNoteBytes.');
    return bytes;
  }

  private async stateDirectory(name: string): Promise<string> {
    await this.assertRoots();
    await this.checkStatePath(true);
    const directory = path.join(this.state, name);
    try { await mkdir(directory, { mode: 0o700 }); } catch (error) {
      if (errorCode(error) !== 'EEXIST') throw error;
    }
    const stat = await lstat(directory);
    if (stat.isSymbolicLink() || !stat.isDirectory() || await realpath(directory) !== directory) {
      fail('INVALID_STATE', 'State subdirectory must be a real directory.');
    }
    return directory;
  }

  private async locked<T>(operation: () => Promise<T>): Promise<T> {
    const locks = await this.stateDirectory('locks');
    const filename = path.join(locks, `${this.vaultId}.lock`);
    let handle;
    try { handle = await open(filename, constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL | NOFOLLOW, 0o600); }
    catch (error) {
      if (errorCode(error) === 'EEXIST') fail('VAULT_BUSY', 'Another writer or a retained lock exists. Inspect the owner before manually recovering a stale lock.');
      throw error;
    }
    const identity = await handle.stat();
    let completed = false;
    try {
      await handle.writeFile(JSON.stringify({ pid: process.pid, startedAt: new Date().toISOString(), vaultHash: this.vaultId }));
      await handle.sync();
      await this.assertRoots();
      const result = await operation();
      completed = true;
      return result;
    } finally {
      await handle.close();
      try {
        const current = await lstat(filename);
        if (!current.isSymbolicLink() && sameIdentity(current, identity)) await unlink(filename);
      } catch (error) {
        if (errorCode(error) !== 'ENOENT') {
          if (completed) fail('WRITE_PUBLISHED_CLEANUP_FAILED', 'The write succeeded, but its lock could not be removed. Read the note before retrying; inspect the retained lock.');
          throw error;
        }
      }
    }
  }

  async create(notePath: string, content: string): Promise<{ path: string; sha256: string }> {
    portableRelative(notePath, true);
    const bytes = this.contentBytes(content);
    return this.locked(async () => {
      const target = await this.target(notePath, true);
      const stagingPath = path.join(path.dirname(target.absolute), `.okc-mcp-${randomUUID()}.tmp`);
      const handle = await open(stagingPath, constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL | NOFOLLOW, 0o600);
      const identity = await handle.stat();
      let published = false;
      try {
        try { await handle.writeFile(bytes); await handle.sync(); }
        finally { await handle.close(); }
        const checked = await this.target(notePath);
        if (!sameIdentity(target.parent, checked.parent)) fail('PATH_CHANGED', 'Parent directory changed before creating the note.');
        await this.assertRoots();
        const stageNow = await lstat(stagingPath);
        regularFile(stageNow, 'staging file');
        if (!sameIdentity(stageNow, identity)) fail('FILE_CHANGED', 'Staging file was replaced.');
        try {
          // link publishes complete staged bytes atomically and refuses to
          // replace any existing directory entry. The temporary second link
          // exists only until staging cleanup; external hardlinks stay denied.
          await link(stagingPath, target.absolute);
        } catch (error) {
          if (errorCode(error) === 'EEXIST') fail('NOTE_EXISTS', `Note already exists: ${notePath}`);
          throw error;
        }
        published = true;
        return { path: notePath, sha256: sha256(bytes) };
      } finally {
        try {
          const current = await lstat(stagingPath);
          if (current.isSymbolicLink() || !sameIdentity(current, identity)) fail('FILE_CHANGED', 'Staging file identity changed; retained for inspection.');
          await unlink(stagingPath);
        } catch (error) {
          if (errorCode(error) !== 'ENOENT') {
            if (published) fail('WRITE_PUBLISHED_CLEANUP_FAILED', `Note ${notePath} was created, but staging cleanup failed. Inspect the note and retained staging file before retrying.`);
            throw error;
          }
        }
      }
    });
  }

  async update(notePath: string, content: string, expectedHash: string): Promise<{ path: string; sha256: string; backupId: string }> {
    portableRelative(notePath, true);
    if (!/^[a-f0-9]{64}$/.test(expectedHash)) fail('INVALID_HASH', 'expectedHash must be the SHA-256 returned by read.');
    const bytes = this.contentBytes(content);
    return this.locked(async () => {
      const original = await this.readInternal(notePath);
      if (original.sha256 !== expectedHash) fail('CONFLICT', 'Note changed since it was read. Read the current note and review the edit again.');
      const backups = await this.stateDirectory('backups');
      const backupId = `${this.vaultId.slice(0, 12)}-${randomUUID()}.md`;
      const backup = await open(path.join(backups, backupId), constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL | NOFOLLOW, 0o600);
      try { await backup.writeFile(original.content, 'utf8'); await backup.sync(); }
      finally { await backup.close(); }

      const target = await this.target(notePath);
      const stagingPath = path.join(path.dirname(target.absolute), `.okc-mcp-${randomUUID()}.tmp`);
      const staging = await open(stagingPath, constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL | NOFOLLOW, 0o600);
      const stageIdentity = await staging.stat();
      let published = false;
      try {
        try {
          await staging.writeFile(bytes);
          await staging.chmod(original.stat.mode & 0o777);
          await staging.sync();
        } finally { await staging.close(); }
        const latest = await this.readInternal(notePath);
        if (latest.sha256 !== expectedHash || !sameIdentity(original.stat, latest.stat)
          || !sameIdentity(original.parent, latest.parent)) {
          fail('CONFLICT', 'Note or parent changed before publication. The original backup is retained; no edit was published.');
        }
        await this.assertRoots();
        const stageNow = await lstat(stagingPath);
        regularFile(stageNow, 'staging file');
        if (!sameIdentity(stageIdentity, stageNow)) fail('FILE_CHANGED', 'Staging file was replaced.');
        await rename(stagingPath, target.absolute);
        published = true;
        return { path: notePath, sha256: sha256(bytes), backupId };
      } finally {
        if (!published) {
          const current = await lstat(stagingPath).catch(() => undefined);
          if (current && !current.isSymbolicLink() && sameIdentity(current, stageIdentity)) await unlink(stagingPath);
        }
      }
    });
  }
}

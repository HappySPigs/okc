import { z } from 'zod';
import type { Config } from './config.js';
import { VaultError, sha256 } from './vault.js';

const identity = z.object({
  project_id: z.string(), revision: z.string().regex(/^[a-f0-9]{64}$/),
  status: z.enum(['live', 'stale']), stale: z.boolean(),
  bound_integration_plan_id: z.string().nullable().optional(),
  bound_corpus_hash: z.string().nullable().optional(),
  bound_taxonomy_hash: z.string().nullable().optional(),
});
export type WebIdentity = z.infer<typeof identity>;
export interface Reader {
  list(): Promise<{ notes: string[]; otherFiles: string[]; skipped: string[] }>;
  read(path: string): Promise<{ path: string; content: string; sha256: string }>;
}

/** Paths are query data, never arbitrary URLs or filesystem destinations. */
export function validateWebPath(value: string, markdown = true): string {
  const parts = value.split('/');
  // Core preserves original legacy spelling (including NFD, hidden directories
  // and .MD); the 1024-byte path bound already bounds component count. These are
  // read-only query paths, separate from the local author's stricter policy.
  if (Buffer.byteLength(value) > 1024 || !['knowledge', 'legacy'].includes(parts[0] ?? '')
    || parts.length < 2 || (markdown && !/\.(md|markdown)$/iu.test(value))
    || parts.some(part => !part || part === '.' || part === '..' || /[\\\x00-\x1f\x7f]/u.test(part))
    || Buffer.from(value).toString('utf8') !== value) {
    throw new VaultError('INVALID_PATH', 'Use a relative Markdown path under knowledge/ or legacy/.');
  }
  return value;
}

/** Configured OKC serving only. No arbitrary-HTTP tool, redirects, or disk cache. */
export class WebVault {
  readonly base: string;
  private readonly web: NonNullable<Config['web']>;

  constructor(private readonly config: Config) {
    if (!config.web) throw new VaultError('WEB_NOT_CONFIGURED', 'No web knowledge source is configured.');
    this.web = config.web;
    this.base = `${this.web.baseUrl.replace(/\/+$/u, '')}/api/serving/${encodeURIComponent(this.web.projectId)}`;
  }

  url(endpoint: string, query: Record<string, string> = {}): string {
    const url = new URL(`${this.base}/${endpoint}`);
    for (const [key, value] of Object.entries(query)) url.searchParams.set(key, value);
    return url.href;
  }

  async request(endpoint: string, query: Record<string, string>, maxBytes: number, signal?: AbortSignal): Promise<Buffer> {
    const timeout = AbortSignal.timeout(this.web.timeoutMs);
    const combined = signal ? AbortSignal.any([signal, timeout]) : timeout;
    try {
      const response = await fetch(this.url(endpoint, query), {
        method: 'GET', redirect: 'manual', signal: combined,
        headers: this.web.token ? { Authorization: `Bearer ${this.web.token}` } : {},
      });
      if (!response.ok) {
        await response.body?.cancel();
        const code = response.status === 401 || response.status === 403 ? 'WEB_AUTH_REQUIRED'
          : response.status === 404 || response.status === 410 ? 'WEB_NOT_FOUND'
          : response.status === 409 ? 'WEB_REVISION_MISMATCH'
          : response.status >= 300 && response.status < 400 ? 'WEB_REDIRECT_REFUSED' : 'WEB_HTTP_ERROR';
        throw new VaultError(code, `Web knowledge request failed (HTTP ${response.status}). No local fallback was used.`);
      }
      const length = response.headers.get('content-length');
      if (length && Number(length) > maxBytes) {
        await response.body?.cancel();
        throw new VaultError('RESPONSE_LIMIT', 'Web response exceeds its byte limit.');
      }
      const reader = response.body?.getReader();
      if (!reader) return Buffer.alloc(0);
      const chunks: Uint8Array[] = [];
      let bytes = 0;
      try {
        for (;;) {
          const item = await reader.read();
          if (item.done) break;
          bytes += item.value.byteLength;
          if (bytes > maxBytes) throw new VaultError('RESPONSE_LIMIT', 'Web response exceeds its byte limit.');
          chunks.push(item.value);
        }
      } finally { await reader.cancel().catch(() => {}); }
      return Buffer.concat(chunks, bytes);
    } catch (error) {
      if (error instanceof VaultError) throw error;
      if (signal?.aborted) throw new VaultError('WEB_CANCELLED', 'Web knowledge request cancelled.');
      if (timeout.aborted) throw new VaultError('WEB_TIMEOUT', 'Web knowledge request timed out. No local fallback was used.');
      throw new VaultError('WEB_UNAVAILABLE', 'Web knowledge is unavailable. Check the configured server. No local fallback was used.');
    }
  }

  async json(endpoint: string, query: Record<string, string>, maxBytes: number, signal?: AbortSignal): Promise<unknown> {
    const bytes = await this.request(endpoint, query, maxBytes, signal);
    try { return JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes)) as unknown; }
    catch { throw new VaultError('WEB_CONTRACT_INVALID', 'Web knowledge returned invalid UTF-8 JSON.'); }
  }

  async snapshot(signal?: AbortSignal, revision?: string): Promise<WebSnapshot> {
    // Always check publication/auth status; do not reuse a cached contract after revocation.
    const parsed = identity.safeParse(await this.json('contract', revision ? { revision } : {}, 65_536, signal));
    if (!parsed.success || parsed.data.project_id !== this.web.projectId) {
      throw new VaultError('WEB_CONTRACT_INVALID', 'Web knowledge contract has invalid identity or status.');
    }
    if (revision && parsed.data.revision !== revision) {
      throw new VaultError('WEB_REVISION_MISMATCH', 'Requested web revision is unavailable; no replacement was selected.');
    }
    return new WebSnapshot(this, this.config, parsed.data, signal);
  }
}

export class WebSnapshot implements Reader {
  private listing: { notes: string[]; otherFiles: string[]; skipped: string[] } | undefined;
  private readonly notes = new Map<string, { path: string; content: string; sha256: string }>();
  private bytes = 0;

  constructor(private readonly client: WebVault, private readonly config: Config,
    readonly identity: WebIdentity, private readonly signal?: AbortSignal) {}

  get source() {
    return { kind: 'web' as const, projectId: this.identity.project_id, revision: this.identity.revision,
      status: this.identity.status, stale: this.identity.stale, readOnly: true,
      boundIntegrationPlanId: this.identity.bound_integration_plan_id ?? null,
      boundCorpusHash: this.identity.bound_corpus_hash ?? null,
      boundTaxonomyHash: this.identity.bound_taxonomy_hash ?? null };
  }

  reference(path: string) {
    const query = { path: validateWebPath(path), revision: this.identity.revision };
    return { url: this.client.url('file', query), provenanceUrl: this.client.url('explain', query) };
  }

  async list() {
    if (this.listing) return this.listing;
    const raw = await this.client.json('files', { revision: this.identity.revision },
      Math.min(this.config.maxScanBytes, this.config.maxFiles * 2048 + 65_536), this.signal);
    const parsed = identity.extend({ files: z.array(z.string()).max(this.config.maxFiles) }).safeParse(raw);
    if (!parsed.success) throw new VaultError('WEB_CONTRACT_INVALID', 'Web file list is invalid or exceeds maxFiles.');
    if (parsed.data.project_id !== this.identity.project_id || parsed.data.revision !== this.identity.revision) {
      throw new VaultError('WEB_REVISION_MISMATCH', 'Web file list does not match the pinned revision.');
    }
    const files = parsed.data.files.filter(value => !value.startsWith('.okc/')).map(value => validateWebPath(value, false));
    if (new Set(files).size !== files.length) throw new VaultError('WEB_CONTRACT_INVALID', 'Web file list contains duplicate paths.');
    const notes = files.filter(value => /\.(md|markdown)$/iu.test(value));
    const otherFiles = files.filter(value => !/\.(md|markdown)$/iu.test(value));
    this.listing = { notes: notes.sort(), otherFiles: otherFiles.sort(), skipped: [] };
    return this.listing;
  }

  async read(path: string) {
    validateWebPath(path);
    const cached = this.notes.get(path);
    if (cached) return cached;
    const bytes = await this.client.request('file', { path, revision: this.identity.revision }, this.config.maxNoteBytes, this.signal);
    this.bytes += bytes.length;
    if (this.bytes > this.config.maxScanBytes) throw new VaultError('SCAN_LIMIT', 'Web knowledge scan exceeds maxScanBytes.');
    let content: string;
    try { content = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true }).decode(bytes); }
    catch { throw new VaultError('INVALID_UTF8', 'Web note is not valid UTF-8.'); }
    const note = { path, content, sha256: sha256(bytes) };
    this.notes.set(path, note);
    return note;
  }

  async details(endpoint: 'verify' | 'explain', path?: string): Promise<unknown> {
    const query: Record<string, string> = { revision: this.identity.revision };
    if (path !== undefined) query.path = validateWebPath(path);
    const raw = await this.client.json(endpoint, query, this.config.maxResponseBytes, this.signal);
    const parsed = identity.safeParse(raw);
    if (!parsed.success || parsed.data.project_id !== this.identity.project_id || parsed.data.revision !== this.identity.revision) {
      throw new VaultError('WEB_REVISION_MISMATCH', 'Web evidence does not match the pinned revision.');
    }
    return raw;
  }
}

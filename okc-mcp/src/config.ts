import { open, realpath } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { homedir } from 'node:os';
import path from 'node:path';
import { z } from 'zod';

const absolute = z.string().min(1).refine(value => path.isAbsolute(value), 'Use an absolute path.');
export const webConfigSchema = z.object({
  baseUrl: z.string().url().refine(value => {
    const url = new URL(value);
    return ['http:', 'https:'].includes(url.protocol) && !url.username && !url.password && !url.search && !url.hash;
  }, 'Use an HTTP(S) server URL without credentials, query, or fragment.'),
  projectId: z.string().regex(/^[A-Za-z0-9][A-Za-z0-9_-]{0,127}$/),
  token: z.string().min(1).max(4096).regex(/^[\x21-\x7e]+$/).optional(),
  timeoutMs: z.number().int().min(100).max(60_000).default(10_000),
}).strict();
export const configSchema = z.object({
  vaultPath: absolute.or(z.literal('')).optional().transform(value => value ?? ''),
  statePath: absolute.or(z.literal('')).optional().transform(value => value ?? ''),
  web: webConfigSchema.optional(),
  // Coding agents to register the MCP server into (v1: Claude Code, Codex).
  // Deduped, order-preserving; absent means setup registers nothing.
  agents: z.array(z.enum(['claude', 'codex'])).transform(values => [...new Set(values)]).optional(),
  readOnly: z.boolean().default(false),
  maxNoteBytes: z.number().int().min(1024).max(4 * 1024 * 1024).default(1024 * 1024),
  maxFiles: z.number().int().min(1).max(100_000).default(10_000),
  maxScanBytes: z.number().int().min(1024).max(512 * 1024 * 1024).default(64 * 1024 * 1024),
  maxResponseBytes: z.number().int().min(4096).max(1024 * 1024).default(65_536),
}).strict().superRefine((config, ctx) => {
  if (Boolean(config.vaultPath) !== Boolean(config.statePath)) {
    ctx.addIssue({ code: 'custom', message: 'Local vaultPath and statePath must be provided together.' });
  }
  if (!config.vaultPath && !(config.web && config.readOnly)) {
    ctx.addIssue({ code: 'custom', message: 'Configure a local Vault, or web with readOnly: true.' });
  }
});

export type Config = z.infer<typeof configSchema>;

export function defaultConfig(vaultPath: string): Config {
  if (!path.isAbsolute(vaultPath)) throw new Error('The Vault path must be absolute.');
  const resolved = path.resolve(vaultPath);
  const id = createHash('sha256').update(resolved).digest('hex').slice(0, 20);
  return configSchema.parse({
    vaultPath: resolved,
    statePath: path.join(homedir(), '.local', 'state', 'okc-mcp', id),
  });
}

export async function loadConfig(configPath: string): Promise<Config> {
  if (!path.isAbsolute(configPath)) throw new Error('The config path must be absolute.');
  const file = await open(configPath, 'r');
  let bytes: Buffer;
  try {
    if (!(await file.stat()).isFile()) throw new Error('Configuration must be a regular file.');
    const buffer = Buffer.alloc(65_537);
    let length = 0;
    while (length < buffer.length) {
      const next = await file.read(buffer, length, buffer.length - length, null);
      if (next.bytesRead === 0) break;
      length += next.bytesRead;
    }
    if (length > 65_536) throw new Error('Configuration exceeds 64 KiB.');
    bytes = buffer.subarray(0, length);
  } finally { await file.close(); }
  const config = configSchema.parse(JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes)));
  if (!config.vaultPath) return config;
  const vaultRoot = await realpath(config.vaultPath);
  const actualConfig = await realpath(configPath);
  const relative = path.relative(vaultRoot, actualConfig);
  if (relative === '' || (!relative.startsWith(`..${path.sep}`) && relative !== '..' && !path.isAbsolute(relative))) {
    throw new Error('Keep the MCP configuration outside the source Vault.');
  }
  return config;
}

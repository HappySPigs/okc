import { open, realpath } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { homedir } from 'node:os';
import path from 'node:path';
import { z } from 'zod';

const absolute = z.string().min(1).refine(value => path.isAbsolute(value), 'Use an absolute path.');
export const configSchema = z.object({
  vaultPath: absolute,
  statePath: absolute,
  readOnly: z.boolean().default(false),
  maxNoteBytes: z.number().int().min(1024).max(4 * 1024 * 1024).default(1024 * 1024),
  maxFiles: z.number().int().min(1).max(100_000).default(10_000),
  maxScanBytes: z.number().int().min(1024).max(512 * 1024 * 1024).default(64 * 1024 * 1024),
  maxResponseBytes: z.number().int().min(4096).max(1024 * 1024).default(65_536),
}).strict();

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
  const vaultRoot = await realpath(config.vaultPath);
  const actualConfig = await realpath(configPath);
  const relative = path.relative(vaultRoot, actualConfig);
  if (relative === '' || (!relative.startsWith(`..${path.sep}`) && relative !== '..' && !path.isAbsolute(relative))) {
    throw new Error('Keep the MCP configuration outside the source Vault.');
  }
  return config;
}

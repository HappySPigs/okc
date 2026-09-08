import { chmod, mkdir, rename, rm, writeFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { homedir } from 'node:os';
import path from 'node:path';
import { loadConfig, type Config } from './config.js';
import { WebVault } from './web.js';
import { VaultError } from './vault.js';

export type AgentName = 'claude' | 'codex';
const AGENT_BIN: Record<AgentName, string> = { claude: 'claude', codex: 'codex' };

/** Injectable subprocess boundary: real spawns in production, a fake in tests.
 *  Resolves with the child's exit code; rejects only on spawn failure (e.g. ENOENT). */
export interface RunResult { code: number | null; stdout: string; stderr: string }
export type CommandRunner = (command: string, args: string[]) => Promise<RunResult>;

export const spawnRunner: CommandRunner = (command, args) => new Promise((resolve, reject) => {
  const child = spawn(command, args, { stdio: ['ignore', 'pipe', 'pipe'], shell: false });
  let stdout = '', stderr = '';
  child.stdout.on('data', chunk => { stdout += chunk; });
  child.stderr.on('data', chunk => { stderr += chunk; });
  child.on('error', reject);
  child.on('close', code => resolve({ code, stdout, stderr }));
});

export interface SetupOptions {
  configPath?: string | undefined; // --config source; defaults to the user-level config path
  configHome?: string; // base config dir; defaults to XDG_CONFIG_HOME or ~/.config (test override)
  purge?: boolean; // unregister only: also remove the generated config
  nodePath?: string; // node executable for the registered command; defaults to process.execPath
  cliPath: string; // absolute path to the installed cli.js the agent will spawn
  run?: CommandRunner; // subprocess runner; defaults to spawnRunner
  validateWeb?: (config: Config) => Promise<void>; // web reachability probe; defaults to a read-only contract GET
  out?: (text: string) => void; // actionable stdout (fallback snippets); defaults to process.stdout
  err?: (text: string) => void; // human/warning stderr; defaults to process.stderr
}

/** ~/.config/okc-mcp/config.json (or platform/XDG equivalent). */
export function userConfigPath(configHome?: string): string {
  const base = configHome ?? process.env.XDG_CONFIG_HOME ?? path.join(homedir(), '.config');
  return path.join(base, 'okc-mcp', 'config.json');
}

/** The single source of truth for how an agent launches the MCP server. */
function serverInvocation(nodePath: string, cliPath: string, configPath: string): string[] {
  return [nodePath, cliPath, 'serve', '--config', configPath];
}

/** Local MCP client snippet; never contains a token (it points at the 0600 config). */
export function clientConfigSnippet(nodePath: string, cliPath: string, configPath: string) {
  const [command, ...args] = serverInvocation(nodePath, cliPath, configPath);
  return { mcpServers: { 'okc-mcp': { command, args } } };
}

// Centralized agent CLI argv. add: register; remove: idempotent upsert + teardown.
function addArgs(agent: AgentName, invocation: string[]): string[] {
  return agent === 'claude'
    ? ['mcp', 'add', '--scope', 'user', 'okc-mcp', '--', ...invocation]
    : ['mcp', 'add', 'okc-mcp', '--', ...invocation];
}
function removeArgs(agent: AgentName): string[] {
  return agent === 'claude' ? ['mcp', 'remove', 'okc-mcp', '--scope', 'user'] : ['mcp', 'remove', 'okc-mcp'];
}

/** Present iff the binary can be spawned; a non-zero --version still means installed. */
async function detect(bin: string, run: CommandRunner): Promise<boolean> {
  try { await run(bin, ['--version']); return true; } catch { return false; }
}

/** Write config with 0600, creating the 0700 parent; atomic rename, temp cleaned on failure. */
async function writeConfig(dest: string, config: Config): Promise<void> {
  await mkdir(path.dirname(dest), { recursive: true, mode: 0o700 });
  const tmp = `${dest}.${process.pid}.tmp`;
  try {
    await writeFile(tmp, `${JSON.stringify(config, null, 2)}\n`, { mode: 0o600 });
    await rename(tmp, dest);
  } catch (error) { await rm(tmp, { force: true }); throw error; }
  await chmod(dest, 0o600); // enforce 0600 regardless of umask
}

export async function runSetup(options: SetupOptions): Promise<void> {
  const run = options.run ?? spawnRunner;
  const nodePath = options.nodePath ?? process.execPath;
  const out = options.out ?? (text => process.stdout.write(text));
  const err = options.err ?? (text => process.stderr.write(text));
  const validateWeb = options.validateWeb ?? (async config => { await new WebVault(config).snapshot(); });
  const dest = userConfigPath(options.configHome);
  const source = options.configPath ?? dest;

  let config: Config;
  try { config = await loadConfig(source); }
  catch (error) {
    if (!options.configPath && (error as NodeJS.ErrnoException).code === 'ENOENT') {
      throw new Error(`No okc-mcp configuration found at ${dest}. Provide --config <path> or create it first.`);
    }
    throw error; // fail closed on invalid/unreadable config; the token is never in these messages
  }

  await writeConfig(dest, config);

  // Best-effort, non-blocking web probe; warns without the token, then continues.
  let webStatus = 'skipped';
  if (config.web) {
    try { await validateWeb(config); webStatus = 'ok'; }
    catch (error) {
      webStatus = 'warning';
      err(`okc-mcp: web validation failed (${error instanceof VaultError ? error.code : 'unavailable'}); the config was written and runtime auth fails closed.\n`);
    }
  }

  const invocation = serverInvocation(nodePath, options.cliPath, dest);
  const agents = config.agents ?? [];
  const registered: AgentName[] = [];
  for (const agent of agents) {
    const bin = AGENT_BIN[agent];
    if (!(await detect(bin, run))) { fallback(agent, bin); continue; }
    try {
      await run(bin, removeArgs(agent)).catch(() => undefined); // upsert: drop any stale entry first
      const result = await run(bin, addArgs(agent, invocation));
      if (result.code === 0) registered.push(agent);
      else err(`okc-mcp: ${agent} registration returned a non-zero status; verify with '${bin} mcp list'.\n`);
    } catch { fallback(agent, bin); } // CLI vanished mid-run: fall back to manual snippet
  }

  err(`${JSON.stringify({ configPath: dest, mode: '0600', agents, registered, web: webStatus }, null, 2)}\n`);

  function fallback(agent: AgentName, bin: string): void {
    out(`${JSON.stringify(clientConfigSnippet(nodePath, options.cliPath, dest), null, 2)}\n`);
    err(`okc-mcp: '${bin}' CLI not found on PATH; add okc-mcp to ${agent} manually using the snippet above.\n`);
  }
}

export async function runUnregister(options: SetupOptions): Promise<void> {
  const run = options.run ?? spawnRunner;
  const out = options.out ?? (text => process.stdout.write(text));
  const err = options.err ?? (text => process.stderr.write(text));
  const dest = userConfigPath(options.configHome);
  const source = options.configPath ?? dest;

  let agents: AgentName[] = ['claude', 'codex']; // if the config is gone, try every known agent
  try { agents = (await loadConfig(source)).agents ?? []; }
  catch { err('okc-mcp: no readable config; attempting to remove okc-mcp from all known agents.\n'); }

  for (const agent of agents) {
    const bin = AGENT_BIN[agent];
    if (!(await detect(bin, run))) { err(`okc-mcp: '${bin}' CLI not found on PATH; remove okc-mcp from ${agent} manually.\n`); continue; }
    const result = await run(bin, removeArgs(agent)).catch(() => ({ code: 1, stdout: '', stderr: '' }));
    if (result.code === 0) out(`okc-mcp: removed okc-mcp from ${agent}.\n`);
    else err(`okc-mcp: '${bin} mcp remove okc-mcp' returned non-zero; it may already be absent.\n`);
  }

  if (options.purge) {
    await rm(dest, { force: true });
    err(`okc-mcp: removed generated config ${dest}. The vault was not touched.\n`);
  }
}

#!/usr/bin/env node
import { parseArgs } from 'node:util';
import { fileURLToPath } from 'node:url';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { defaultConfig, loadConfig } from './config.js';
import { createServer, VERSION } from './server.js';
import { Vault, VaultError } from './vault.js';
import { WebVault } from './web.js';
import { initializeSourceVault } from './setup.js';
import { clientConfigSnippet, runSetup, runUnregister } from './install.js';

const HELP = `okc-mcp ${VERSION} — coding-session capture, local authoring and published OKC knowledge

  okc-mcp config --vault /absolute/Vault
  okc-mcp init --vault /absolute/NewVault
  okc-mcp doctor --config /absolute/okc-mcp.json
  okc-mcp client-config --config /absolute/okc-mcp.json
  okc-mcp setup [--config /absolute/okc-mcp.json]
  okc-mcp unregister [--config /absolute/okc-mcp.json] [--purge]
  okc-mcp serve --config /absolute/okc-mcp.json

config prints a starting configuration; save it OUTSIDE the Vault.
init creates a NEW source Vault with empty conventional folders; refuses existing paths.
client-config prints a local MCP client snippet; it never edits client settings.
setup writes the config to ~/.config/okc-mcp/config.json (0600) and registers okc-mcp
  into the agents named in the config (Claude Code, Codex) via their official CLIs.
unregister removes okc-mcp from those agents; --purge also removes the generated config.
serve uses stdio only. It never launches Obsidian or an AI provider.
`;

async function main(): Promise<void> {
  const { values, positionals } = parseArgs({ options: {
    config: { type: 'string' }, vault: { type: 'string' }, purge: { type: 'boolean' },
    help: { type: 'boolean' }, version: { type: 'boolean' },
  }, allowPositionals: true, strict: true });
  if (values.help || (!positionals.length && !values.version)) { process.stdout.write(HELP); return; }
  if (values.version) { process.stdout.write(`${VERSION}\n`); return; }
  if (positionals.length !== 1) throw new Error('Expected one command. Use --help.');
  const command = positionals[0];
  if (command === 'init') {
    if (!values.vault) throw new Error('init requires --vault /absolute/new/path.');
    process.stdout.write(`${JSON.stringify(await initializeSourceVault(values.vault), null, 2)}\n`);
    return;
  }
  if (command === 'config') {
    if (!values.vault) throw new Error('config requires --vault /absolute/path.');
    process.stdout.write(`${JSON.stringify(defaultConfig(values.vault), null, 2)}\n`);
    return;
  }
  if (command === 'setup') {
    await runSetup({ configPath: values.config, cliPath: fileURLToPath(import.meta.url) });
    return;
  }
  if (command === 'unregister') {
    await runUnregister({ configPath: values.config, purge: values.purge ?? false, cliPath: fileURLToPath(import.meta.url) });
    return;
  }
  if (!['serve', 'doctor', 'client-config'].includes(command ?? '')) throw new Error('Unknown command. Use --help.');
  if (!values.config) throw new Error('This command requires --config /absolute/path.');
  const config = await loadConfig(values.config);
  if (command === 'client-config') {
    process.stdout.write(`${JSON.stringify(clientConfigSnippet(process.execPath, fileURLToPath(import.meta.url), values.config), null, 2)}\n`);
    return;
  }
  if (command === 'doctor') {
    // Shallow local self-check reported per check (no network). Vault failures
    // are reported as failing checks, not thrown, so diagnostics stays actionable.
    const checks: { name: string; pass: boolean; detail: string }[] = [];
    const major = Number(process.versions.node.split('.')[0] ?? '0');
    const nodeOk = major >= 22;
    checks.push({ name: 'node-runtime', pass: nodeOk, detail: `Node ${process.version}${nodeOk ? '' : ' (requires >= 22.13.0)'}` });
    checks.push({ name: 'config', pass: true, detail: `Loaded ${values.config}; vault=${config.vaultPath}; state=${config.statePath}` });
    const vault = config.vaultPath ? new Vault(config) : undefined;
    let vaultOk = false;
    try {
      if (!vault) throw new VaultError('LOCAL_NOT_CONFIGURED', 'No local source configured (web-only read-only mode).');
      await vault.initialize();
      vaultOk = true;
      checks.push({ name: 'vault-reachable', pass: true, detail: `Vault root resolved and validated: ${config.vaultPath}` });
    } catch (error) {
      checks.push({ name: 'vault-reachable', pass: false, detail: error instanceof VaultError ? `${error.code}: ${error.message}` : 'Vault could not be initialized; check the path, permissions, and that it is a real directory outside any .okc / .okc-project.' });
    }
    if (vaultOk && vault) {
      try {
        const files = await vault.list();
        checks.push({ name: 'vault-scan', pass: true, detail: `${files.notes.length} notes, ${files.otherFiles.length} other files, ${files.skipped.length} skipped (within bounds)` });
      } catch (error) {
        checks.push({ name: 'vault-scan', pass: false, detail: error instanceof VaultError ? `${error.code}: ${error.message}` : 'Vault scan failed within configured bounds.' });
      }
    } else {
      checks.push({ name: 'vault-scan', pass: false, detail: 'Skipped: the Vault is not reachable.' });
    }
    if (!vault && config.web) {
      for (const check of checks) if (check.name.startsWith('vault-')) check.pass = true;
    }
    if (config.web) {
      try {
        const snapshot = await new WebVault(config).snapshot();
        await snapshot.list();
        checks.push({ name: 'web-knowledge', pass: true,
          detail: `project=${snapshot.source.projectId}; revision=${snapshot.source.revision}; stale=${snapshot.source.stale}` });
      } catch (error) {
        checks.push({ name: 'web-knowledge', pass: false, detail: error instanceof VaultError ? `${error.code}: ${error.message}` : 'Web knowledge check failed.' });
      }
    }
    const ok = checks.every(check => check.pass);
    process.stdout.write(`${JSON.stringify({ ok, version: VERSION, node: process.version,
      mode: config.readOnly ? 'read-only' : 'authoring', transport: 'stdio', network: Boolean(config.web), compilerRequired: false,
      defaultSource: config.web ? 'web' : 'local',
      vaultPath: config.vaultPath, statePath: config.statePath, checks,
      note: 'Path/scan and configured web reachability only; not a compiler or write-permission certification.' }, null, 2)}\n`);
    process.exitCode = ok ? 0 : 1;
    return;
  }
  const vault = config.vaultPath ? new Vault(config) : undefined;
  await vault?.initialize();
  const server = createServer(config, vault);
  await server.connect(new StdioServerTransport());
}

main().catch(error => {
  if (error instanceof VaultError) process.stderr.write(`${error.code}: ${error.message}\n`);
  else process.stderr.write('okc-mcp: Check command arguments, JSON configuration and filesystem permissions. Use --help.\n');
  process.exitCode = 1;
});

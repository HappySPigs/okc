#!/usr/bin/env node
import { parseArgs } from 'node:util';
import { fileURLToPath } from 'node:url';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { defaultConfig, loadConfig } from './config.js';
import { createServer, VERSION } from './server.js';
import { Vault, VaultError } from './vault.js';

const HELP = `okc-mcp ${VERSION} — local Obsidian source Vault authoring

  okc-mcp config --vault /absolute/Vault
  okc-mcp doctor --config /absolute/okc-mcp.json
  okc-mcp client-config --config /absolute/okc-mcp.json
  okc-mcp serve --config /absolute/okc-mcp.json

config prints a starting configuration; save it OUTSIDE the Vault.
client-config prints a local MCP client snippet; it never edits client settings.
serve uses stdio only. It never launches Obsidian or an AI provider.
`;

async function main(): Promise<void> {
  const { values, positionals } = parseArgs({ options: {
    config: { type: 'string' }, vault: { type: 'string' }, help: { type: 'boolean' }, version: { type: 'boolean' },
  }, allowPositionals: true, strict: true });
  if (values.help || (!positionals.length && !values.version)) { process.stdout.write(HELP); return; }
  if (values.version) { process.stdout.write(`${VERSION}\n`); return; }
  if (positionals.length !== 1) throw new Error('Expected one command. Use --help.');
  const command = positionals[0];
  if (command === 'config') {
    if (!values.vault) throw new Error('config requires --vault /absolute/path.');
    process.stdout.write(`${JSON.stringify(defaultConfig(values.vault), null, 2)}\n`);
    return;
  }
  if (!['serve', 'doctor', 'client-config'].includes(command ?? '')) throw new Error('Unknown command. Use --help.');
  if (!values.config) throw new Error('This command requires --config /absolute/path.');
  const config = await loadConfig(values.config);
  if (command === 'client-config') {
    process.stdout.write(`${JSON.stringify({ mcpServers: { 'okc-mcp': {
      command: process.execPath, args: [fileURLToPath(import.meta.url), 'serve', '--config', values.config],
    } } }, null, 2)}\n`);
    return;
  }
  const vault = new Vault(config);
  await vault.initialize();
  if (command === 'doctor') {
    const files = await vault.list();
    process.stdout.write(`${JSON.stringify({ ok: true, version: VERSION, node: process.version,
      mode: config.readOnly ? 'read-only' : 'authoring', vaultPath: config.vaultPath,
      statePath: config.statePath, notes: files.notes.length, skippedEntries: files.skipped.length,
      transport: 'stdio', compilerRequired: false, note: 'Path/scan check only; not a compiler or write-permission certification.' }, null, 2)}\n`);
    return;
  }
  const server = createServer(config, vault);
  await server.connect(new StdioServerTransport());
}

main().catch(error => {
  if (error instanceof VaultError) process.stderr.write(`${error.code}: ${error.message}\n`);
  else process.stderr.write('okc-mcp: Check command arguments, JSON configuration and filesystem permissions. Use --help.\n');
  process.exitCode = 1;
});

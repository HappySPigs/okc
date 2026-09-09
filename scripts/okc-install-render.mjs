// Render the single combined okc-install config into per-module config files.
//
// Usage: node scripts/okc-install-render.mjs <combined-config.json> <out-dir>
// Writes <out-dir>/hooks.json and/or <out-dir>/mcp.json (each 0600) for the enabled modules.
// Tokens are written only into those files — never printed to stdout/stderr (SECURITY-03).
// Consumed by install.sh / install.ps1; the presence of each output file signals "install this module".
import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const [configPath, outDir] = process.argv.slice(2);
if (!configPath || !outDir) { console.error('usage: okc-install-render.mjs <config> <out-dir>'); process.exit(2); }

let raw;
try { raw = JSON.parse(readFileSync(configPath, 'utf8')); }
catch (error) { console.error(`설정 파일을 읽을 수 없습니다: ${error.message}`); process.exit(1); }

const base = String(raw.okc_web_base_url ?? '').replace(/\/+$/, '');
if (!base) { console.error('okc_web_base_url 값을 채워주세요 (예: https://okc.example.com).'); process.exit(1); }

function req(obj, key, ctx) {
  const value = obj?.[key];
  if (value === undefined || value === null || value === '') { console.error(`${ctx}.${key} 값을 채워주세요.`); process.exit(1); }
  return value;
}

// okc-hooks: foundation config (snake_case). server_endpoint = <base>/api/sync (https 강제됨).
const hooks = raw.hooks ?? {};
if (hooks.enabled !== false) {
  const endpoint = /\/api\/sync$/.test(base) ? base : `${base}/api/sync`;
  const out = {
    vault_path: req(hooks, 'vault_path', 'hooks'),
    server_endpoint: endpoint,
    token: req(hooks, 'upload_token', 'hooks'),
  };
  if (hooks.data_dir) out.data_dir = hooks.data_dir;
  writeFileSync(path.join(outDir, 'hooks.json'), `${JSON.stringify(out, null, 2)}\n`, { mode: 0o600 });
  console.error(`okc-hooks: config 준비됨 (endpoint=${endpoint})`); // 토큰 미출력
} else {
  console.error('okc-hooks: 건너뜀 (enabled=false)');
}

// okc-mcp: zod config (camelCase). web.enabled=true → 웹 지식(read-only), 아니면 로컬 vault.
const mcp = raw.mcp ?? {};
if (mcp.enabled !== false) {
  const agents = Array.isArray(mcp.agents) ? mcp.agents : [];
  const web = mcp.web ?? {};
  let out;
  if (web.enabled) {
    out = {
      readOnly: web.read_only ?? true,
      agents,
      web: { baseUrl: base, projectId: req(web, 'project_id', 'mcp.web') },
    };
    if (web.read_token) out.web.token = web.read_token;
  } else {
    out = {
      agents,
      vaultPath: req(mcp, 'vault_path', 'mcp'),
      statePath: req(mcp, 'state_path', 'mcp'),
      readOnly: mcp.read_only ?? false,
    };
  }
  writeFileSync(path.join(outDir, 'mcp.json'), `${JSON.stringify(out, null, 2)}\n`, { mode: 0o600 });
  console.error(`okc-mcp: config 준비됨 (mode=${web.enabled ? 'web' : 'local'}, agents=${JSON.stringify(agents)})`);
} else {
  console.error('okc-mcp: 건너뜀 (enabled=false)');
}

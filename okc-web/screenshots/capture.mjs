// Screenshot capture for okc-web (run with a local browser: see screenshots/README.md).
//   cd frontend && npx playwright install chromium && node ../screenshots/capture.mjs
//
// Logs in as the bootstrap admin, then captures the offline-reachable demo screens.
// AI happy-path screens (populated taxonomy/clusters/compiled/provenance) only render
// after a real run with a configured provider — re-run this after completing one.
//
// Env: BASE_URL (default http://localhost:8000), ADMIN_EMAIL, ADMIN_PASSWORD.
import { chromium } from 'playwright'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const OUT = dirname(fileURLToPath(import.meta.url))
const BASE = process.env.BASE_URL ?? 'http://localhost:8000'
const EMAIL = process.env.ADMIN_EMAIL ?? 'admin@example.com'
const PASSWORD = process.env.ADMIN_PASSWORD ?? 'change-me'

async function shot(page, path, name) {
  await page.goto(`${BASE}${path}`, { waitUntil: 'networkidle' }).catch(() => {})
  await page.waitForTimeout(400)
  await page.screenshot({ path: join(OUT, `${name}.png`), fullPage: true })
  console.log('captured', name, '←', path)
}

const browser = await chromium.launch()
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } })

// --- login (via the SPA form; falls back to the API if the form selectors differ) ---
await page.goto(`${BASE}/login`, { waitUntil: 'networkidle' })
await shot(page, '/login', '01-login')
try {
  await page.fill('input[type="email"], [data-testid$="email-input"]', EMAIL, { timeout: 3000 })
  await page.fill('input[type="password"], [data-testid$="password-input"]', PASSWORD, { timeout: 3000 })
  await page.click('button[type="submit"], [data-testid$="submit-button"]', { timeout: 3000 })
  await page.waitForTimeout(800)
} catch {
  // API fallback: set the session cookie, then continue.
  const res = await page.request.post(`${BASE}/api/auth/login`, { data: { email: EMAIL, password: PASSWORD } })
  if (!res.ok()) { console.error('login failed', res.status(), await res.text()); }
}

// --- offline-reachable admin screens (empty/gated states render without a provider) ---
await shot(page, '/projects', '02-projects')
// Deep-links below assume at least one project exists; adjust the id or create one first.
// await shot(page, `/projects/${PID}`, '03-project-overview')
// await shot(page, `/projects/${PID}/sources`, '04-sources-freeze')
// await shot(page, `/projects/${PID}/tokens`, '05-tokens')
// await shot(page, `/projects/${PID}/integration`, '07-integration-monitor')   // ★ focal
// await shot(page, `/projects/${PID}/review`, '08-review-gate')
// await shot(page, `/projects/${PID}/review/taxonomy`, '09-taxonomy')
// await shot(page, `/projects/${PID}/serving`, '12-serving-publish')
// await shot(page, `/projects/${PID}/serving/contract`, '14-mcp-contract')

// contributor upload portal (needs a live token from the Tokens screen):
// await shot(page, `/upload/${TOKEN}`, '06-upload-portal')

console.log('\nDone. For the AI happy-path focal screens (10-cluster-review, 11-compiled-vault,')
console.log('13-provenance-verify), configure a provider, complete a run in the UI, then re-run.')
await browser.close()

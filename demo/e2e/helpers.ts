import { type Page, type APIRequestContext, expect } from "@playwright/test";
import { readFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
export const STATE_DIR = resolve(here, "..", "state");
export const SHOT_DIR = resolve(STATE_DIR, "screenshots");
export const BASE = process.env.OKC_DEMO_BASE_URL || "http://localhost:8000";
export const ADMIN_EMAIL = process.env.OKC_DEMO_ADMIN_EMAIL || "admin@okc.demo";
export const ADMIN_PASSWORD = process.env.OKC_DEMO_ADMIN_PASSWORD || "okc-demo-1234";

export function projectId(): string {
  return readFileSync(resolve(STATE_DIR, "project.id"), "utf8").trim();
}

let shotN = 0;
/** Full-page screenshot into demo/state/screenshots with an ordered prefix. */
export async function shot(page: Page, name: string): Promise<void> {
  mkdirSync(SHOT_DIR, { recursive: true });
  const n = String(++shotN).padStart(2, "0");
  await page.screenshot({ path: resolve(SHOT_DIR, `${n}-${name}.png`), fullPage: true });
}

/** Log in through the real /login form; leaves the browser on /projects. */
export async function login(page: Page): Promise<void> {
  await page.goto("/login");
  await page.getByTestId("login-email").fill(ADMIN_EMAIL);
  await page.getByTestId("login-password").fill(ADMIN_PASSWORD);
  await page.getByTestId("login-submit").click();
  await page.waitForURL("**/projects", { timeout: 30_000 });
}

/** An APIRequestContext that carries the browser's session cookie, for robust
 *  read-side polling (gate/status/jobs) alongside the visible UI actions. */
export async function apiCtx(page: Page): Promise<APIRequestContext> {
  return page.request;
}

/** Poll GET /review/gate until state === "Ready" (or throw with the blockers). */
export async function waitGateReady(page: Page, pid: string, timeoutMs = 15 * 60_000) {
  const deadline = Date.now() + timeoutMs;
  let last: any = null;
  while (Date.now() < deadline) {
    const r = await page.request.get(`${BASE}/api/projects/${pid}/review/gate`);
    if (r.ok()) {
      last = await r.json();
      if (last.state === "Ready") return last;
    }
    await page.waitForTimeout(3000);
  }
  throw new Error(`gate not Ready in time: ${JSON.stringify(last)}`);
}

/** Poll GET /status.checkpoint until it matches one of `wanted`. */
export async function waitCheckpoint(page: Page, pid: string, wanted: string[], timeoutMs = 4 * 60_000) {
  const deadline = Date.now() + timeoutMs;
  let cp = "";
  while (Date.now() < deadline) {
    const r = await page.request.get(`${BASE}/api/projects/${pid}/status`);
    if (r.ok()) {
      cp = (await r.json()).checkpoint;
      if (wanted.includes(cp)) return cp;
    }
    await page.waitForTimeout(1500);
  }
  throw new Error(`checkpoint '${cp}' never reached ${JSON.stringify(wanted)}`);
}

export { expect };

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

// Korean narration burned into the demo film as an on-page caption overlay,
// keyed by beat name. Only shown when OKC_DEMO_CAPTIONS=1 (normal runs unaffected).
const CAPTIONS: Record<string, string> = {
  overview: "① 너울(Neoul) — 5개 팀 볼트를 하나의 '브레인'으로 병합합니다",
  sources: "② 소스 고정(freeze) — 엔지니어링 · 프로덕트 · 세일즈 · 고객지원 · 내 볼트",
  integration: "③ 통합 시작 — 오거나이저가 노트를 정규화하고 유사 항목을 클러스터링",
  "integration-organizer-done": "④ 오거나이저 완료 — 병합된 분류 체계(taxonomy) 제안",
  taxonomy: "⑤ 관리자가 병합된 분류 체계를 검토하고 승인",
  "review-gate": "⑥ 합성 + 크리틱(비평) 후 — 클러스터 리뷰 게이트",
  "cluster-contradictions-no-winner": "⑦ 팀 간 상충(예: 환불 30일 vs 14일)은 '승자 없음'으로 보존됩니다",
  compile: "⑧ 모든 지적 처리 완료 → 컴파일(compile)",
  "compiled-vault": "⑨ 병합된 '브레인' 컴파일 결과",
  "served-published": "⑩ 게시(publish) — okc-mcp가 읽는 최신 브레인 리비전",
  "stale-banner": "🔄 okc-mcp로 '내 볼트' 편집 → 워처가 감지 → 프로젝트 '변경됨(stale)' → 재병합",
};
const CAPTIONS_ON = !!process.env.OKC_DEMO_CAPTIONS;
const CAPTION_DWELL_MS = Number(process.env.OKC_DEMO_CAPTION_DWELL_MS || 2600);

/** Inject/replace a fixed bottom caption bar on the current page (for the film). */
async function injectCaption(page: Page, text: string): Promise<void> {
  await page
    .evaluate((t) => {
      let el = document.getElementById("okc-demo-caption");
      if (!el) {
        el = document.createElement("div");
        el.id = "okc-demo-caption";
        document.body.appendChild(el);
      }
      el.style.cssText = [
        "position:fixed", "left:0", "right:0", "bottom:0", "z-index:2147483647",
        "padding:20px 40px", "box-sizing:border-box",
        "background:linear-gradient(to top, rgba(6,10,20,0.92), rgba(6,10,20,0.66))",
        "color:#fff", "font-size:27px", "line-height:1.45", "font-weight:600",
        "text-align:center", "letter-spacing:0.2px",
        "font-family:'Apple SD Gothic Neo','Noto Sans KR',system-ui,sans-serif",
        "text-shadow:0 1px 4px rgba(0,0,0,0.7)", "pointer-events:none",
        "border-top:2px solid rgba(120,170,255,0.85)",
      ].join(";");
      el.textContent = t;
    }, text)
    .catch(() => {});
}

let shotN = 0;
/** Full-page screenshot into demo/state/screenshots with an ordered prefix.
 *  When OKC_DEMO_CAPTIONS=1, also burns a Korean caption onto the page (after the
 *  clean screenshot) and dwells so the recording captures a readable subtitle. */
export async function shot(page: Page, name: string): Promise<void> {
  mkdirSync(SHOT_DIR, { recursive: true });
  const n = String(++shotN).padStart(2, "0");
  await page.screenshot({ path: resolve(SHOT_DIR, `${n}-${name}.png`), fullPage: true });
  if (CAPTIONS_ON && CAPTIONS[name]) {
    await injectCaption(page, CAPTIONS[name]);
    await page.waitForTimeout(CAPTION_DWELL_MS);
  }
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

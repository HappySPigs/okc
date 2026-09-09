import { type Page } from "@playwright/test";
import { BASE, shot, waitCheckpoint } from "./helpers";

// The okc-core merge contracts are strict, so every state mutation is driven via
// the browser's authenticated session (page.request carries the okc_session
// cookie) — reliable and fast. The UI is navigated + screenshotted at each beat
// for the visible walkthrough. `integrate` is RE-ENTRANT: called again after each
// human approval to advance one stage (organizer → [approve taxonomy] → synthesis
// +critic → [approve clusters] → ready → compile).
export async function getJSON(page: Page, path: string, retryBusyMs = 3 * 60_000): Promise<any> {
  const deadline = Date.now() + retryBusyMs;
  for (;;) {
    const r = await page.request.get(`${BASE}${path}`);
    if (r.ok()) return r.json();
    // Reserving reads (status/clusters/gate) return 409 PROJECT_BUSY while a job
    // holds the project — wait it out rather than failing.
    if (r.status() === 409 && Date.now() < deadline) {
      await page.waitForTimeout(1500);
      continue;
    }
    throw new Error(`GET ${path} → ${r.status()} ${await r.text()}`);
  }
}
export async function postJSON(page: Page, path: string, body: unknown = {}): Promise<any> {
  const r = await page.request.post(`${BASE}${path}`, { data: body });
  if (!r.ok()) throw new Error(`POST ${path} → ${r.status()} ${await r.text()}`);
  return r.json();
}
async function checkpoint(page: Page, pid: string): Promise<string> {
  return (await getJSON(page, `/api/projects/${pid}/status`)).checkpoint;
}
export async function waitJob(page: Page, pid: string, jobId: string, timeoutMs = 4 * 60_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const s = await getJSON(page, `/api/projects/${pid}/jobs/${jobId}`);
    if (s.state === "completed") return s;
    if (s.state === "failed" || s.state === "cancelled")
      throw new Error(`job ${jobId} ${s.state}: ${JSON.stringify(s.error)}`);
    await page.waitForTimeout(1500);
  }
  throw new Error(`job ${jobId} timed out`);
}
/** One re-entrant integrate pass (drives the next stage). */
export async function driveIntegrate(page: Page, pid: string) {
  const r = await postJSON(page, `/api/projects/${pid}/integrate`, {
    allow_remote_provider: false, remote_disclosure_confirmed: false,
  });
  await waitJob(page, pid, r.job_id);
}

/** Freeze the source set (shows the Sources screen; freezes via API). */
export async function ensureFrozen(page: Page, pid: string) {
  await page.goto(`/projects/${pid}/sources`);
  await shot(page, "sources");
  const st = await getJSON(page, `/api/projects/${pid}/status`);
  if (!st.frozen || st.stale) await postJSON(page, `/api/projects/${pid}/freeze`, {});
}

/** Organizer pass: show the integration monitor, drive integrate → taxonomy. */
export async function integrateOrganizer(page: Page, pid: string) {
  await page.goto(`/projects/${pid}/integration`);
  await shot(page, "integration");
  if (!["needs_taxonomy", "needs_clusters", "ready_to_compile", "verified"].includes(await checkpoint(page, pid)))
    await driveIntegrate(page, pid);
  await waitCheckpoint(page, pid, ["needs_taxonomy", "needs_clusters", "ready_to_compile", "verified"]);
  await shot(page, "integration-organizer-done");
}

/** Approve taxonomy then drive synthesis+critic → cluster review. */
export async function approveTaxonomyAndSynthesize(page: Page, pid: string) {
  if ((await checkpoint(page, pid)) === "needs_taxonomy") {
    await page.goto(`/projects/${pid}/review/taxonomy`);
    await shot(page, "taxonomy");
    await postJSON(page, `/api/projects/${pid}/taxonomy/approve`, { rationale: "병합된 분류 체계를 검토했고 승인합니다." });
    await driveIntegrate(page, pid); // synthesis + critic
    await waitCheckpoint(page, pid, ["needs_clusters", "ready_to_compile", "verified"]);
  }
}

/** Open a cluster that carries a preserved contradiction; screenshot "no winner". */
export async function showcaseContradictions(page: Page, pid: string) {
  await page.goto(`/projects/${pid}/review`);
  await shot(page, "review-gate");
  const clusters = await getJSON(page, `/api/projects/${pid}/clusters`);
  let targetId: string | undefined;
  for (const c of clusters) {
    if ((c.blocking_count ?? 0) > 0) { targetId = c.cluster_id; break; }
    const d = await getJSON(page, `/api/projects/${pid}/clusters/${c.cluster_id}`);
    if ((d.contradictions?.length ?? 0) > 0) { targetId = c.cluster_id; break; }
  }
  targetId = targetId ?? clusters[0]?.cluster_id;
  if (!targetId) return;
  await page.goto(`/projects/${pid}/review/clusters/${targetId}`);
  const tab = page.getByTestId("tab-contradictions");
  if (await tab.isVisible().catch(() => false)) await tab.click().catch(() => {});
  await page.waitForTimeout(500);
  await shot(page, "cluster-contradictions-no-winner");
}

const REGEN_FEEDBACK =
  "상충되는 두 주장을 모두 보존하고 어느 한쪽도 승자로 고르지 마라. " +
  "서로 다른 사실은 명시적 모순으로 기록하고 각 주장의 출처를 함께 남겨라.";

/** Dispose of every finding until the compile gate is Ready: regenerate blocking
 *  clusters, waive minors, and re-drive integrate after approvals. */
export async function resolveToReady(page: Page, pid: string, maxRounds = 8) {
  const approved = new Set<string>();
  for (let round = 0; round < maxRounds; round++) {
    const gate = await getJSON(page, `/api/projects/${pid}/review/gate`);
    if (gate.state === "Ready") return gate;

    const blocking = [...new Set((gate.blocking_items ?? []).map((b: any) => b.cluster_id))];
    for (const cid of blocking) {
      const receipt = await postJSON(page, `/api/projects/${pid}/clusters/${cid}/regenerate`, {
        feedback: REGEN_FEEDBACK, allow_remote_provider: false, remote_disclosure_confirmed: false,
      }).catch((e) => { console.warn("regenerate:", String(e)); return null; });
      if (receipt?.job_id) await waitJob(page, pid, receipt.job_id);
    }
    const clusters = await getJSON(page, `/api/projects/${pid}/clusters`);
    for (const c of clusters) {
      if ((c.blocking_count ?? 0) === 0 && !approved.has(c.cluster_id)) {
        const d = await getJSON(page, `/api/projects/${pid}/clusters/${c.cluster_id}`);
        const minorWaivers: Record<string, string> = {};
        for (const f of d.findings ?? []) {
          const sev = String(f.severity ?? "").toLowerCase();
          if (sev === "minor") minorWaivers[f.id ?? f.finding_id] = "데모: 경미한 지적 확인 후 수용";
        }
        await postJSON(page, `/api/projects/${pid}/clusters/${c.cluster_id}/approve`, {
          minor_waivers: minorWaivers, omission_rationales: {},
        }).then(() => approved.add(c.cluster_id)).catch((e) => console.warn("approve:", String(e)));
      }
    }
    await driveIntegrate(page, pid).catch(() => {});
  }
  return getJSON(page, `/api/projects/${pid}/review/gate`);
}

/** Compile the merged vault and publish it; returns the published revision. */
export async function compileAndPublish(page: Page, pid: string) {
  if ((await checkpoint(page, pid)) !== "verified") {
    await page.goto(`/projects/${pid}/compiled`);
    await shot(page, "compile");
    await postJSON(page, `/api/projects/${pid}/compile`, {});
    await waitCheckpoint(page, pid, ["verified"]);
  }
  await page.goto(`/projects/${pid}/compiled`);
  await page.waitForTimeout(800);
  await shot(page, "compiled-vault");
  await postJSON(page, `/api/projects/${pid}/serving/publish`, {}).catch(() => {});
  await page.goto(`/projects/${pid}/serving`);
  await page.waitForTimeout(800);
  await shot(page, "served-published");
  return (await getJSON(page, `/api/projects/${pid}/serving`)).revision as string;
}

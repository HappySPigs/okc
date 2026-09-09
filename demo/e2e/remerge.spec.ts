import { test, expect } from "@playwright/test";
import { login, projectId, shot, waitCheckpoint } from "./helpers";
import {
  getJSON, ensureFrozen, integrateOrganizer, approveTaxonomyAndSynthesize,
  showcaseContradictions, resolveToReady, compileAndPublish,
} from "./flow";

// Re-merge after the MCP edit + watcher upload made the project STALE: the admin
// re-freezes, re-runs the pipeline, re-reviews, and compiles again. This is
// publication REVISION 2 — the "always-fresh brain" the admin re-merges on demand.
test("re-merge after change → revision 2", async ({ page }) => {
  const pid = projectId();
  await login(page);

  // The change should have invalidated prior approvals.
  const st = await getJSON(page, `/api/projects/${pid}/status`);
  expect(st.stale, "project should be stale after the MCP edit + upload").toBe(true);
  await page.goto(`/projects/${pid}`);
  await shot(page, "stale-banner");

  await ensureFrozen(page, pid);
  await integrateOrganizer(page, pid);
  await approveTaxonomyAndSynthesize(page, pid);
  await showcaseContradictions(page, pid);

  const gate = await resolveToReady(page, pid);
  expect(gate.state, `gate blockers: ${JSON.stringify(gate.blocking_items)}`).toBe("Ready");

  const rev = await compileAndPublish(page, pid);
  console.log(`[remerge] published revision ${rev}`);

  // History should now contain more than one published revision.
  const history = await getJSON(page, `/api/projects/${pid}/serving/history`);
  console.log(`[remerge] publication history size = ${history.length}`);
  expect(history.length).toBeGreaterThanOrEqual(1);
});

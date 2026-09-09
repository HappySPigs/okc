import { test, expect } from "@playwright/test";
import { login, projectId, shot } from "./helpers";
import {
  ensureFrozen, integrateOrganizer, approveTaxonomyAndSynthesize,
  showcaseContradictions, resolveToReady, compileAndPublish,
} from "./flow";

// Baseline merge: the 5 seeded vaults → integrate → review (preserve
// contradictions; waive minors / regenerate blocking) → compile → publish.
// This is publication REVISION 1.
test("baseline merge → revision 1", async ({ page }) => {
  const pid = projectId();
  await login(page);
  await page.goto(`/projects/${pid}`);
  await shot(page, "overview");

  await ensureFrozen(page, pid);
  await integrateOrganizer(page, pid);
  await approveTaxonomyAndSynthesize(page, pid);
  await showcaseContradictions(page, pid);

  const gate = await resolveToReady(page, pid);
  expect(gate.state, `gate blockers: ${JSON.stringify(gate.blocking_items)}`).toBe("Ready");

  const rev = await compileAndPublish(page, pid);
  console.log(`[baseline] published revision ${rev}`);
});

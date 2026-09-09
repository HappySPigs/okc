import { defineConfig } from "@playwright/test";

// Integration + regenerate are real local-LLM ops (qwen2.5:14b), so tests run
// serially with a generous timeout. Set OKC_DEMO_HEADED=1 to watch it live.
export default defineConfig({
  testDir: ".",
  timeout: 20 * 60_000,
  expect: { timeout: 30_000 },
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [["list"]],
  // For a demo recording (OKC_DEMO_VIDEO=1) each `playwright test` invocation gets
  // its own timestamped output dir so the baseline video is not clobbered when the
  // re-merge run starts. Normal runs keep Playwright's default.
  outputDir: process.env.OKC_DEMO_VIDEO ? `../state/pw-video/${Date.now()}` : undefined,
  use: {
    baseURL: process.env.OKC_DEMO_BASE_URL || "http://localhost:8000",
    headless: !process.env.OKC_DEMO_HEADED,
    // Pace the actions so a human (or a recording) can follow (OKC_DEMO_SLOWMO ms).
    // Defaults: 700ms headed, 0 headless — set OKC_DEMO_SLOWMO to pace a headless recording.
    launchOptions: {
      slowMo: Number(process.env.OKC_DEMO_SLOWMO || (process.env.OKC_DEMO_HEADED ? 700 : 0)),
    },
    viewport: { width: 1440, height: 900 },
    // Record the browser session as video only when OKC_DEMO_VIDEO is set (for the demo film).
    video: process.env.OKC_DEMO_VIDEO
      ? { mode: "on", size: { width: 1440, height: 900 } }
      : "off",
    actionTimeout: 30_000,
    navigationTimeout: 30_000,
    locale: "ko-KR",
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
});

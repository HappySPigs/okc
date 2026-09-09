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
  use: {
    baseURL: process.env.OKC_DEMO_BASE_URL || "http://localhost:8000",
    headless: !process.env.OKC_DEMO_HEADED,
    viewport: { width: 1440, height: 900 },
    actionTimeout: 30_000,
    navigationTimeout: 30_000,
    locale: "ko-KR",
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
});

import { defineConfig, devices } from '@playwright/test';

// Runs against `vite preview` (the production build); check-app.sh
// builds before invoking Playwright.
// EPIORA_PERF=1 swaps the test dir to ./perf for the Task 16
// measurement spec — it never runs in the normal e2e gate, and the
// normal gate never sees it.
const perf = process.env.EPIORA_PERF === '1';

export default defineConfig({
  testDir: perf ? './perf' : './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : 'list',
  use: {
    baseURL: 'http://localhost:4173',
    trace: 'retain-on-failure',
  },
  webServer: {
    command: 'npm run preview -- --port 4173 --strictPort',
    url: 'http://localhost:4173',
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'firefox', use: { ...devices['Desktop Firefox'] } },
    { name: 'webkit', use: { ...devices['Desktop WebKit'] } },
  ],
});

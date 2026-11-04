// Playwright config for the Chromium performance gate (spec §5.3).
// webServer serves crates/epiora-wasm via serve.mjs so bench.html can
// import the release build at /pkg/epiora_wasm.js. The bundled
// Chromium (`npx playwright install chromium`) is pinned by the exact
// @playwright/test version in package.json.

import { defineConfig } from "@playwright/test";

const PORT = 8765;

export default defineConfig({
  testDir: ".",
  testMatch: "bench.spec.mjs",
  timeout: 300_000,
  workers: 1,
  reporter: [["list"]],
  use: { baseURL: `http://127.0.0.1:${PORT}` },
  webServer: {
    command: `node serve.mjs ${PORT}`,
    url: `http://127.0.0.1:${PORT}/`,
    reuseExistingServer: !process.env.CI,
  },
});

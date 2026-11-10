// WebdriverIO config for the desktop smoke test (v0.3 plan Task 15,
// spec §10.1): the mocha spec drives the built Epiora app through
// tauri-driver, which forwards WebDriver to the platform's native
// driver (WebKitWebDriver on Linux, msedgedriver on Windows).
// scripts/desktop-smoke.sh starts tauri-driver on :4444 and exports
// EPIORA_APP_BINARY; both are overridable for local runs.

const application =
  process.env.EPIORA_APP_BINARY ??
  new URL('../src-tauri/target/release/epiora-app', import.meta.url).pathname;

export const config = {
  runner: 'local',
  specs: ['./smoke.spec.mjs'],
  maxInstances: 1,

  hostname: '127.0.0.1',
  port: Number(process.env.EPIORA_WDIO_PORT ?? 4444),
  path: '/',
  connectionRetryTimeout: 120_000,
  connectionRetryCount: 5,

  capabilities: [
    {
      // tauri-driver only needs the binary under test; it injects the
      // platform driver's own fields into the session request.
      'tauri:options': { application },
      // The first worker/store call migrates and opens the encrypted
      // database; engine wasm compiles inside the webview. Both can
      // outlast the WebDriver default script timeout.
      timeouts: { script: 120_000, pageLoad: 120_000, implicit: 0 },
    },
  ],

  logLevel: 'warn',
  bail: 1,
  waitforTimeout: 60_000,
  waitforInterval: 500,

  framework: 'mocha',
  // No reporter package: the spec logs its own progress lines and the
  // launcher prints the pass/fail summary.
  reporters: [],
  mochaOpts: {
    ui: 'bdd',
    timeout: 300_000,
  },
};

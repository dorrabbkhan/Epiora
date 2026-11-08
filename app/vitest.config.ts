import { mkdirSync, rmSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import react from '@vitejs/plugin-react';
import { playwright } from '@vitest/browser-playwright';
import { searchForWorkspaceRoot } from 'vite';
import { configDefaults, defineConfig } from 'vitest/config';

// `?raw` imports of ../store/migrations/*.sql live outside the app root;
// Vite requires the workspace root to stay in `allow` once it is set.
// The wasm `pkg/` outputs and ts-rs `bindings/` the data worker aliases
// below live under ../crates, and the worker's browser tests fetch
// fixture files under ../fixtures — all three must be servable too.
const migrationsDir = fileURLToPath(new URL('../store/migrations', import.meta.url));
const cratesDir = fileURLToPath(new URL('../crates', import.meta.url));
const fixturesDir = fileURLToPath(new URL('../fixtures', import.meta.url));
const fsAllow = [searchForWorkspaceRoot(process.cwd()), migrationsDir, cratesDir, fixturesDir];

// Mirrors vite.config.ts (vitest does not read it): the worker reaches
// the wasm packages and generated types through these aliases.
const enginePkg = fileURLToPath(new URL('../crates/epiora-wasm/pkg', import.meta.url));
const engineTypes = fileURLToPath(new URL('../crates/epiora-wasm/bindings', import.meta.url));
const importPkg = fileURLToPath(new URL('../crates/epiora-import-wasm/pkg', import.meta.url));
const importTypes = fileURLToPath(new URL('../crates/epiora-import-wasm/bindings', import.meta.url));

// WebKit needs a persistent (on-disk) profile for OPFS, and a fresh one
// per run — a reused profile can leave a stale WebKit lock and time out
// the next launch. Profiles live under one parent so the stale dirs do
// not accumulate: clear it at config load; Playwright creates the fresh
// <uuid> leaf itself on launch (the config is evaluated more than once
// per run, so mkdir'ing the leaf here would leave an empty sibling dir).
const webkitProfileParent = fileURLToPath(
  new URL('./node_modules/.cache/vitest-webkit', import.meta.url),
);
rmSync(webkitProfileParent, { recursive: true, force: true });
mkdirSync(webkitProfileParent, { recursive: true });
const webkitProfile = `${webkitProfileParent}/${crypto.randomUUID()}`;

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@engine': enginePkg,
      '@engine-types': engineTypes,
      '@import': importPkg,
      '@import-types': importTypes,
    },
  },
  test: {
    projects: [
      {
        test: {
          name: 'unit',
          environment: 'jsdom',
          include: ['src/**/*.test.{ts,tsx}', 'lint-rules.test.ts'],
          exclude: [...configDefaults.exclude, 'src/**/*.browser.test.ts'],
        },
      },
      {
        // Mirrors vite.config.ts (vitest does not read it): the `?raw`
        // migration imports live outside the app root, sqlite-wasm must
        // not be pre-bundled, and workers stay ES modules.
        optimizeDeps: { exclude: ['@sqlite.org/sqlite-wasm'] },
        server: { fs: { allow: fsAllow } },
        worker: { format: 'es' },
        test: {
          name: 'store-opfs',
          include: ['src/**/*.browser.test.ts'],
          // The suite is one test file per backend; serial also lets the
          // WebKit instance use a persistent context (see below).
          fileParallelism: false,
          browser: {
            enabled: true,
            headless: true,
            instances: [
              { browser: 'chromium', provider: playwright() },
              { browser: 'firefox', provider: playwright() },
              {
                browser: 'webkit',
                // WebKit's OPFS root does not exist in Playwright's
                // ephemeral contexts — navigator.storage.getDirectory()
                // rejects with UnknownError. A persistent context gives
                // WebKit a real on-disk profile where OPFS works.
                provider: playwright({ persistentContext: webkitProfile }),
              },
            ],
          },
        },
      },
    ],
  },
});

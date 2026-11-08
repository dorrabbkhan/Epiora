import react from '@vitejs/plugin-react';
import { playwright } from '@vitest/browser-playwright';
import { configDefaults, defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [react()],
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
        server: { fs: { allow: ['..'] } },
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
                provider: playwright({
                  // Fresh profile per run: reusing a profile can leave a
                  // stale WebKit lock and time out the next launch.
                  persistentContext: `node_modules/.cache/vitest-webkit-${crypto.randomUUID()}`,
                }),
              },
            ],
          },
        },
      },
    ],
  },
});

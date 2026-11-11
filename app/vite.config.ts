import { fileURLToPath } from 'node:url';
import { readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import react from '@vitejs/plugin-react';
import { defineConfig, searchForWorkspaceRoot } from 'vite';
import { VitePWA } from 'vite-plugin-pwa';

// `?raw` imports of ../store/migrations/*.sql live outside the app
// root; allow the dev server to serve them. Vite requires the workspace
// root to stay in `allow` once it is set. The wasm `pkg/` outputs and
// ts-rs `bindings/` the data worker aliases below live under ../crates,
// and the worker's browser tests fetch fixture files under ../fixtures —
// all three must be servable too.
const migrationsDir = fileURLToPath(new URL('../store/migrations', import.meta.url));
const cratesDir = fileURLToPath(new URL('../crates', import.meta.url));
const fixturesDir = fileURLToPath(new URL('../fixtures', import.meta.url));
const fsAllow = [searchForWorkspaceRoot(process.cwd()), migrationsDir, cratesDir, fixturesDir];

// The worker reaches the wasm packages and their generated types
// through these aliases (v0.3 plan Task 9); pkg/ is a build artifact —
// scripts/check-app.sh builds both before any gate step needs it.
const enginePkg = fileURLToPath(new URL('../crates/epiora-wasm/pkg', import.meta.url));
const engineTypes = fileURLToPath(new URL('../crates/epiora-wasm/bindings', import.meta.url));
const importPkg = fileURLToPath(new URL('../crates/epiora-import-wasm/pkg', import.meta.url));
const importTypes = fileURLToPath(new URL('../crates/epiora-import-wasm/bindings', import.meta.url));

const MIB = 1024 * 1024;

/** Largest file under `dir` in bytes (recursive). */
function largestFile(dir: string): number {
  let max = 0;
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      max = Math.max(max, largestFile(path));
    } else if (entry.isFile()) {
      max = Math.max(max, statSync(path).size);
    }
  }
  return max;
}

export default defineConfig({
  plugins: [
    react(),
    // PWA (v0.3 spec §7.1, plan Task 13): Workbox precaches the shell
    // and every emitted js/css/html/wasm/svg/png/webmanifest asset —
    // core-wasm, import-wasm and sqlite-wasm included. Updates prompt
    // ("New version available") instead of swapping mid-session, and
    // registration is manual (`UpdatePrompt`) so the strict CSP in
    // public/_headers never has to allow inline scripts. The web
    // manifest ships from public/ and index.html links it.
    VitePWA({
      registerType: 'prompt',
      injectRegister: null,
      manifest: false,
      workbox: {
        globPatterns: ['**/*.{js,css,html,wasm,svg,png,webmanifest}'],
        // Updates must wait for the prompt: with clientsClaim on, a
        // waiting update auto-activated mid-session (the controller
        // changed under a live page), silently swapping versions.
        clientsClaim: false,
        skipWaiting: false,
        // Filled in by `beforeBuildServiceWorker` below, once every
        // asset is on disk — no hand-typed constant.
        maximumFileSizeToCacheInBytes: 0,
      },
      integration: {
        beforeBuildServiceWorker(options) {
          const dir =
            typeof options.workbox.globDirectory === 'string'
              ? options.workbox.globDirectory
              : options.outDir;
          const largest = largestFile(dir);
          const cap = Math.ceil(largest / MIB) * MIB;
          options.workbox.maximumFileSizeToCacheInBytes = cap;
          console.log(
            `[pwa] largest built asset ${largest} bytes → ` +
              `maximumFileSizeToCacheInBytes ${cap} bytes`,
          );
        },
      },
    }),
  ],
  resolve: {
    alias: {
      '@engine': enginePkg,
      '@engine-types': engineTypes,
      '@import': importPkg,
      '@import-types': importTypes,
    },
  },
  server: {
    fs: { allow: fsAllow },
  },
  // sqlite-wasm manages its own .wasm asset and OPFS plumbing; bundling
  // it through optimizeDeps breaks that resolution.
  optimizeDeps: {
    exclude: ['@sqlite.org/sqlite-wasm'],
  },
  worker: {
    format: 'es',
  },
  build: {
    // .vite/manifest.json names the entry's JS/CSS files — the
    // first-load budget gate (app/scripts/budget.mjs, v0.3 Task 17)
    // reads it.
    manifest: true,
  },
});

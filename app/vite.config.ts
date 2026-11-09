import { fileURLToPath } from 'node:url';
import react from '@vitejs/plugin-react';
import { defineConfig, searchForWorkspaceRoot } from 'vite';

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
});

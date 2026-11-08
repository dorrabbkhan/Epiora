import { fileURLToPath } from 'node:url';
import react from '@vitejs/plugin-react';
import { defineConfig, searchForWorkspaceRoot } from 'vite';

// `?raw` imports of ../store/migrations/*.sql live outside the app
// root; allow the dev server to serve them. Vite requires the workspace
// root to stay in `allow` once it is set.
const migrationsDir = fileURLToPath(new URL('../store/migrations', import.meta.url));

export default defineConfig({
  plugins: [react()],
  server: {
    fs: { allow: [searchForWorkspaceRoot(process.cwd()), migrationsDir] },
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

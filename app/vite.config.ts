import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [react()],
  // `?raw` imports of ../store/migrations/*.sql live outside the app
  // root; allow the dev server to serve them.
  server: {
    fs: { allow: ['..'] },
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

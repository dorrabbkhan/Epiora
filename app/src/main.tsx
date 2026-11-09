import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { RouterProvider } from '@tanstack/react-router';
import './design/tokens.css';
import {
  applyTheme,
  THEME_STORAGE_KEY,
  type Theme,
} from './design/theme';
import { router } from './ui/router';
import { dataApi } from './worker/client';
import type { DataApi } from './worker/api';
import type { Remote } from 'comlink';

declare global {
  interface Window {
    /** The app's typed RPC client, also the e2e seeding handle: specs
     *  drive imports through the same calls the UI makes. */
    dataApi: Remote<DataApi>;
  }
}
window.dataApi = dataApi;

const stored = localStorage.getItem(THEME_STORAGE_KEY);
const theme: Theme =
  stored === 'light' ||
  stored === 'dark' ||
  stored === 'hc-light' ||
  stored === 'hc-dark' ||
  stored === 'system'
    ? stored
    : 'system';
applyTheme(theme);

const rootEl = document.getElementById('root');
if (rootEl === null) {
  throw new Error('missing #root element');
}
const queryClient = new QueryClient();
createRoot(rootEl).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  </StrictMode>,
);

import { StrictMode, useEffect, useState } from 'react';
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
import { dataApi, workerName } from './worker/client';
import type { DataApi } from './worker/api';
import type { Remote } from 'comlink';
import { LockScreen } from './ui/LockScreen';
import { UpdatePrompt } from './ui/UpdatePrompt';

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

type LockReason = 'held-elsewhere' | 'lost';

/** The tab-lock gate (Task 13): the data worker reports its lock
 *  state on the 'epiora-tab-lock' channel, tagged with its worker
 *  name so other tabs' messages are ignored. 'ui-ready' asks it to
 *  re-post when this listener missed the startup announcement. */
function App() {
  const [lock, setLock] = useState<LockReason | null>(null);
  useEffect(() => {
    const channel = new BroadcastChannel('epiora-tab-lock');
    const onMessage = (e: MessageEvent) => {
      const data = e.data as { worker?: unknown; type?: unknown };
      if (
        data.worker === workerName &&
        (data.type === 'held-elsewhere' || data.type === 'lost')
      ) {
        setLock(data.type);
      }
    };
    channel.addEventListener('message', onMessage);
    channel.postMessage({ type: 'ui-ready' });
    return () => channel.close();
  }, []);

  if (lock !== null) {
    return <LockScreen reason={lock} />;
  }
  return (
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  );
}

const rootEl = document.getElementById('root');
if (rootEl === null) {
  throw new Error('missing #root element');
}
const queryClient = new QueryClient();
createRoot(rootEl).render(
  <StrictMode>
    <App />
    <UpdatePrompt />
  </StrictMode>,
);

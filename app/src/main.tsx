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
import { dataApi, dataWorker, inTauri, workerName } from './worker/client';
import type { DataApi } from './worker/api';
import type { Remote } from 'comlink';
import { RpcError } from './worker/errors';
import { Heading } from './ui/components';
import { cx } from './ui/components/classes';
import { LockScreen } from './ui/LockScreen';
import { UpdatePrompt } from './ui/UpdatePrompt';
import { t } from './ui/strings';
import lockStyles from './ui/LockScreen.module.css';

declare global {
  interface Window {
    /** The app's typed RPC client, also the e2e seeding handle: specs
     *  drive imports through the same calls the UI makes. */
    dataApi: Remote<DataApi>;
    /** Present only inside the Tauri desktop shell. */
    __TAURI_INTERNALS__?: unknown;
  }
}
window.dataApi = dataApi;

// Task 14: inside Tauri the worker's store ops go through the
// main-thread bridge to the native encrypted store. Started before the
// first render so the listener precedes any relay message.
if (inTauri) {
  void import('./tauri/bridge').then((m) => m.startTauriBridge(dataWorker));
}

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
type FatalReason = 'keychain_unavailable' | 'key_invalid';

/** The tab-lock gate (Task 13): the data worker reports its lock
 *  state on the 'epiora-tab-lock' channel, tagged with its worker
 *  name so other tabs' messages are ignored. 'ui-ready' asks it to
 *  re-post when this listener missed the startup announcement. */
function App() {
  const [lock, setLock] = useState<LockReason | null>(null);
  const [fatal, setFatal] = useState<FatalReason | null>(null);
  // Task 14: a `keychain_unavailable`/`key_invalid` at startup shows a
  // full-screen error — never an empty store. The probe also forces the
  // store open (and migrations) at startup under Tauri.
  useEffect(() => {
    if (!inTauri) return;
    let mounted = true;
    dataApi.listProjects().then(
      () => {},
      (e: unknown) => {
        const detail = e instanceof RpcError ? e.detail : '';
        if (
          mounted &&
          (detail === 'keychain_unavailable' || detail === 'key_invalid')
        ) {
          setFatal(detail);
        }
      },
    );
    return () => {
      mounted = false;
    };
  }, []);
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

  if (fatal !== null) {
    return (
      <main className={lockStyles.screen}>
        <Heading level={1} className={cx(lockStyles.title)}>
          {t.appName}
        </Heading>
        <p className={lockStyles.body} role="alert">
          {t.errors[fatal]}
        </p>
      </main>
    );
  }
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

// The data worker (v0.3 spec §5.2): one dedicated worker owns the
// store, core-wasm and import-wasm; the main thread only ever talks to
// `DataApi` over Comlink. All wiring — the store mutex and RPC error
// mapping — lives in api.ts.
//
// Task 13: the worker takes the tab lock before the store can open
// (openStore re-checks it too), and when the lock is stolen it closes
// the store and tells its tab to show the LockScreen.

import * as Comlink from 'comlink';
import { api } from './api';
import { engine } from './engines';
import { closeStore } from './store-backend';
import {
  acquireStoreLock,
  reportLockLost,
  storeLockReleased,
} from './tab-lock';

// Acquire eagerly so a second tab learns 'held-elsewhere' at startup,
// not on its first store RPC.
void acquireStoreLock();
void storeLockReleased().then(() =>
  closeStore().finally(() => reportLockLost()),
);

// Core-wasm starts loading with the worker (plan Task 9 Step 4) so the
// first analysis RPC does not pay for it. `engine()` caches the same
// promise for later calls and drops it on failure, so an init error is
// still reported — as an RpcError — by the first RPC that needs the
// engine; this catch only keeps the startup call off the unhandled-
// rejection log.
void engine().catch(() => {});

Comlink.expose(api);

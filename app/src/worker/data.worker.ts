// The data worker (v0.3 spec §5.2): one dedicated worker owns the
// store, core-wasm and import-wasm; the main thread only ever talks to
// `DataApi` over Comlink. All wiring — the store mutex and RPC error
// mapping — lives in api.ts.

import * as Comlink from 'comlink';
import { api } from './api';
import { engine } from './engines';

// Core-wasm starts loading with the worker (plan Task 9 Step 4) so the
// first analysis RPC does not pay for it. `engine()` caches the same
// promise for later calls and drops it on failure, so an init error is
// still reported — as an RpcError — by the first RPC that needs the
// engine; this catch only keeps the startup call off the unhandled-
// rejection log.
void engine().catch(() => {});

Comlink.expose(api);

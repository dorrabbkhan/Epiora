// The data worker (v0.3 spec §5.2): one dedicated worker owns the
// store, core-wasm and import-wasm; the main thread only ever talks to
// `DataApi` over Comlink. All wiring — the store mutex and RPC error
// mapping — lives in api.ts.

import * as Comlink from 'comlink';
import { api } from './api';

Comlink.expose(api);

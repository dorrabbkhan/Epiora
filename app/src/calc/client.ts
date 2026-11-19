// Main-thread client for the calculator worker (v1.0 plan Task 14,
// spec §7.1). Same pattern as `worker/client.ts` — wired error
// objects are revived into `RpcError` — but the worker it spawns is
// the store-less `calc.worker.ts`: no OPFS, no `epiora-store` Web
// Lock, so calculator links keep working in any number of tabs next
// to a locked or even lock-blocked app tab.

import * as Comlink from 'comlink';
import type { Remote } from 'comlink';
import { isWiredRpcError, RpcError } from '../worker/errors';
import type { CalcApi } from './api';

/** Awaits `p`, reviving a wired RpcError into the real class. */
async function call<T>(p: Promise<T>): Promise<T> {
  try {
    return await p;
  } catch (e) {
    if (isWiredRpcError(e)) throw new RpcError(e.code, e.detail, e.message);
    throw e;
  }
}

export function wrapCalcApi(worker: Worker): Remote<CalcApi> {
  const r = Comlink.wrap<CalcApi>(worker);
  return {
    listCalculators: () => call(r.listCalculators()),
    run: (id, inputs) => call(r.run(id, inputs)),
    explain: (id, inputs, statistic) => call(r.explain(id, inputs, statistic)),
    exportR: (id, inputs, receipt) => call(r.exportR(id, inputs, receipt)),
    newSeed: () => call(r.newSeed()),
  } as Remote<CalcApi>;
}

let cached: Remote<CalcApi> | null = null;

/** The tab's calculator worker (lazily spawned). */
export function calcApi(): Remote<CalcApi> {
  cached ??= wrapCalcApi(
    new Worker(new URL('./calc.worker.ts', import.meta.url), {
      type: 'module',
      name: 'epiora-calc',
    }),
  );
  return cached;
}

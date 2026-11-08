// Conformance harness worker: the OPFS sahpool VFS needs synchronous
// access handles, which only exist in a Worker, so the store lives here
// and the test file proxies it with Comlink.
//
// Comlink drops custom properties off thrown Errors (only name/message/
// stack cross), so StoreError is re-thrown as a marked plain object and
// revived into a real StoreError on the test side.

import * as Comlink from 'comlink';
import { openOpfsStore } from '../opfs';
import { StoreError, type SqlValue, type Tx } from '../types';

const wireError = (e: unknown): never => {
  if (e instanceof StoreError) {
    throw { __epioraStoreError: true, code: e.code, message: e.message };
  }
  throw e;
};

const wire =
  <A extends unknown[], R>(f: (...args: A) => Promise<R>) =>
  (...args: A): Promise<R> =>
    f(...args).catch(wireError);

const api = {
  async open(name: string) {
    try {
      const store = await openOpfsStore(name);
      return Comlink.proxy({
        exec: wire((sql: string, params?: SqlValue[]) => store.exec(sql, params)),
        query: wire((sql: string, params?: SqlValue[]) => store.query(sql, params)),
        transaction: wire((fn: (tx: Tx) => Promise<unknown>) =>
          store.transaction(async (innerTx) => {
            // Errors raised inside fn (running on the main side) cross
            // the boundary twice; mark StoreErrors before they leave the
            // worker so the test side can revive them.
            const proxyTx: Tx = {
              exec: (sql, params) => innerTx.exec(sql, params).catch(wireError),
              query: (sql, params) => innerTx.query(sql, params).catch(wireError),
            };
            return fn(Comlink.proxy(proxyTx));
          }),
        ),
        close: wire(() => store.close()),
        __simulateQuotaOnNextWrite: () => store.__simulateQuotaOnNextWrite?.(),
      });
    } catch (e) {
      wireError(e);
    }
  },
};

export type HarnessApi = typeof api;

Comlink.expose(api);

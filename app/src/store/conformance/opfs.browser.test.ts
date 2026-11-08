// Runs the shared conformance suite against the OPFS backend in each
// browser Vitest instance (chromium, firefox, webkit). The store itself
// runs inside a dedicated worker — sahpool's synchronous access handles
// are worker-only — and is proxied back with Comlink.
//
// One OPFS database per suite run: every open() returns a new connection
// to the same database so close/reopen semantics are actually exercised.

import * as Comlink from 'comlink';
import { storeConformance } from './suite';
import { StoreError, type SqlValue, type Store, type Tx } from '../types';
import type { HarnessApi } from './harness.worker';

const DB_NAME = `conformance-${crypto.randomUUID()}`;

interface RemoteStore {
  exec(sql: string, params?: SqlValue[]): Promise<void>;
  query(sql: string, params?: SqlValue[]): Promise<Record<string, SqlValue>[]>;
  transaction(fn: (tx: Tx) => Promise<unknown>): Promise<unknown>;
  close(): Promise<void>;
  __simulateQuotaOnNextWrite(): void;
}

interface WiredStoreError {
  __epioraStoreError: true;
  code: StoreError['code'];
  message: string;
}

function isWiredStoreError(e: unknown): e is WiredStoreError {
  return (
    typeof e === 'object' &&
    e !== null &&
    (e as WiredStoreError).__epioraStoreError === true
  );
}

/** Awaits the call and revives wired StoreErrors into real ones. */
async function call<T>(p: Promise<T>): Promise<T> {
  try {
    return await p;
  } catch (e) {
    throw isWiredStoreError(e) ? new StoreError(e.code, e.message) : e;
  }
}

async function openRemoteOpfsStore(): Promise<Store> {
  const worker = new Worker(new URL('./harness.worker.ts', import.meta.url), {
    type: 'module',
  });
  const api = Comlink.wrap<HarnessApi>(worker);
  const remote = (await call(api.open(DB_NAME))) as unknown as RemoteStore;
  return {
    exec: (sql, params) => call(remote.exec(sql, params)),
    query: (sql, params) => call(remote.query(sql, params)) as Promise<never>,
    transaction: (fn) => call(remote.transaction(Comlink.proxy(fn))) as Promise<never>,
    close: async () => {
      await call(remote.close());
      worker.terminate();
    },
    __simulateQuotaOnNextWrite: () => remote.__simulateQuotaOnNextWrite(),
  };
}

storeConformance('opfs', () => openRemoteOpfsStore());

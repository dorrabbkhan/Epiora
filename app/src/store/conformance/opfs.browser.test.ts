// Runs the shared conformance suite against the OPFS backend in each
// browser Vitest instance (chromium, firefox, webkit). The store itself
// runs inside a dedicated worker — sahpool's synchronous access handles
// are worker-only — and is proxied back with Comlink.
//
// One OPFS database per suite run: every open() returns a new connection
// to the same database so close/reopen semantics are actually exercised.

import * as Comlink from 'comlink';
import { describe, expect, it } from 'vitest';
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

function spawnHarness(): Worker {
  return new Worker(new URL('./harness.worker.ts', import.meta.url), { type: 'module' });
}

async function openRemoteOpfsStore(): Promise<Store> {
  const worker = spawnHarness();
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

describe('opfs backend specifics', () => {
  it(
    'held_pool_maps_to_locked_elsewhere_and_recovers',
    { timeout: 60_000 },
    async () => {
      // OPFS-specific (not in suite.ts): a second context cannot create
      // sync access handles for the sahpool's files while the first holds
      // them, so open() must reject with locked_elsewhere — and the cached
      // rejection must not poison the retry once the holder is gone.
      const holder = spawnHarness();
      const contender = spawnHarness();
      try {
        const holderApi = Comlink.wrap<HarnessApi>(holder);
        const contenderApi = Comlink.wrap<HarnessApi>(contender);
        await call(holderApi.open(DB_NAME));

        const err = await call(contenderApi.open(DB_NAME)).then(
          () => null,
          (e: unknown) => e,
        );
        expect(err).toBeInstanceOf(StoreError);
        expect((err as StoreError).code).toBe('locked_elsewhere');

        holder.terminate();

        // The handles die with the holder's worker, but not necessarily on
        // the same tick — retry until the open takes.
        let remote: RemoteStore | null = null;
        for (let i = 0; i < 100 && remote === null; i++) {
          remote = (await call(contenderApi.open(DB_NAME)).then(
            (r) => r as unknown as RemoteStore,
            () => null,
          )) as RemoteStore | null;
          if (remote === null) await new Promise((r) => setTimeout(r, 100));
        }
        expect(remote).not.toBeNull();
        await call(remote!.close());
      } finally {
        holder.terminate();
        contender.terminate();
      }
    },
  );
});

// Worker side of the Tauri store relay (v0.3 plan Task 14, spec §5.2):
// Tauri's IPC does not reach Web Workers, so this Store posts ops as
// `{ epiora: 'store-relay', id, op, sql, params }` messages on the
// worker channel; the main-thread bridge (app/src/tauri/bridge.ts)
// invokes the Rust store commands and posts results back. Blobs cross
// as `Uint8Array` both ways, never JSON.

import initSql from '../../../store/migrations/0001_init.sql?raw';
import { migrate } from './migrate';
import { StoreError, type Row, type SqlValue, type Store, type Tx } from './types';

const MIGRATIONS: { version: number; sql: string }[] = [
  { version: 1, sql: initSql },
];

export const RELAY_TAG = 'epiora-store-relay';
export type RelayOp = 'exec' | 'query' | 'begin' | 'commit' | 'rollback';

export interface RelayRequest {
  epiora: typeof RELAY_TAG;
  id: string;
  op: RelayOp;
  sql?: string;
  params?: SqlValue[];
}

export interface RelayResponse {
  epiora: typeof RELAY_TAG;
  id: string;
  ok: boolean;
  code?: string;
  message?: string;
  rows?: Row[];
}

const pending = new Map<
  string,
  { resolve: (r: RelayResponse) => void; reject: (e: Error) => void }
>();
let counter = 0;
let listening = false;

function listen(): void {
  if (listening) return;
  listening = true;
  // Shares the worker's message channel with Comlink, which ignores
  // these responses (unknown `type`/`id`); we ignore Comlink's traffic
  // the same way — the `epiora` tag decides.
  self.addEventListener('message', (e: MessageEvent) => {
    const d = e.data as Partial<RelayResponse>;
    if (d?.epiora !== RELAY_TAG || typeof d.id !== 'string') return;
    const p = pending.get(d.id);
    if (p === undefined) return;
    pending.delete(d.id);
    if (d.ok === true) {
      p.resolve(d as RelayResponse);
    } else {
      p.reject(
        new StoreError(
          (d.code as StoreError['code']) ?? 'sql',
          d.message ?? 'store relay error',
        ),
      );
    }
  });
}

function send(op: RelayOp, sql?: string, params?: SqlValue[]): Promise<RelayResponse> {
  listen();
  const id = `sr${counter++}`;
  return new Promise<RelayResponse>((resolve, reject) => {
    pending.set(id, { resolve, reject });
    const msg: RelayRequest = {
      epiora: RELAY_TAG,
      id,
      op,
      ...(sql === undefined ? {} : { sql }),
      ...(params === undefined ? {} : { params }),
    };
    self.postMessage(msg);
  });
}

function makeStore(): Store {
  let closed = false;
  const guard = () => {
    if (closed) throw new StoreError('sql', 'store is closed');
  };
  const tx: Tx = {
    exec: async (sql, params) => {
      guard();
      await send('exec', sql, params);
    },
    query: async (sql, params) => {
      guard();
      const res = await send('query', sql, params);
      return res.rows ?? [];
    },
  };
  return {
    exec: tx.exec,
    query: tx.query,
    transaction: async <T>(fn: (tx: Tx) => Promise<T>): Promise<T> => {
      guard();
      await send('begin');
      let result: T;
      try {
        result = await fn(tx);
      } catch (e) {
        await send('rollback').catch(() => {});
        throw e;
      }
      try {
        await send('commit');
      } catch (e) {
        await send('rollback').catch(() => {});
        throw e;
      }
      return result;
    },
    close: () => {
      // The native connection lives for the app's lifetime; close is
      // local-only (process exit rolls back anything uncommitted).
      closed = true;
      return Promise.resolve();
    },
    // No __simulateQuotaOnNextWrite: the quota hook is implemented by
    // the store-stdio test backend (stdio feature).
  };
}

/** Opens the native store through the relay: foreign keys on, then the
 *  same migrations the OPFS backend applies. */
export async function openRelayStore(): Promise<Store> {
  const store = makeStore();
  try {
    await store.exec('PRAGMA foreign_keys = ON');
    await migrate(store, MIGRATIONS);
  } catch (e) {
    await store.close();
    throw e;
  }
  return store;
}

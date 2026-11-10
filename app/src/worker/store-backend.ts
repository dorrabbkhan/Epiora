// Store backend selection (v0.3 spec §5.2): the data worker owns the
// store — OPFS sqlite-wasm in the browser and, once Task 14 lands, the
// native SQLite3MultipleCiphers relay in Tauri. It is one connection,
// so every RPC handler's store work runs through `withStore`, a
// promise-chain mutex: a `transaction()` call issues BEGIN and awaits
// its callback, and any other access joining that await would share or
// break the open transaction.
//
// The database name comes from the worker's `name` option (falling
// back to `epiora`) so tests can give each spawned worker its own db.
// The app's worker is named `epiora:<tab id>` (client.ts) — the part
// after ':' is the tab identity used by the lock channel.

import { openOpfsStore } from '../store/opfs';
import { openRelayStore } from '../store/relay';
import { StoreError, type Store } from '../store/types';
import { acquireStoreLock, storeLockState } from './tab-lock';

let storePromise: Promise<Store> | null = null;
let queue: Promise<unknown> = Promise.resolve();

// Task 14: Tauri's IPC does not reach workers, so the main thread marks
// the shell by naming this worker 'epiora-tauri:<tab>' (client.ts, when
// window.__TAURI_INTERNALS__ exists); bridge.ts also sets
// self.__EPIORA_TAURI__ on the main thread. Either marker picks the
// native relay store.
const TAINTED =
  (globalThis as Record<string, unknown>).__EPIORA_TAURI__ === true ||
  (typeof (globalThis as { name?: unknown }).name === 'string' &&
    ((globalThis as { name?: string }).name as string).startsWith('epiora-tauri:'));

function dbName(): string {
  const name = (globalThis as { name?: unknown }).name;
  return typeof name === 'string' && name !== '' ? name.split(':')[0]! : 'epiora';
}

/** A previous worker's sahpool handles take a beat to die after its
 *  termination; opening is retried for a short window on the lock
 *  errors that produces (`locked_elsewhere`, and CANTOPEN surfacing as
 *  `sql` when the pool file itself is still held). */
const OPEN_RETRY_MS = 10_000;
const OPEN_RETRY_STEP = 100;

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

function isTransientOpen(e: unknown): boolean {
  return (
    e instanceof StoreError &&
    (e.code === 'locked_elsewhere' ||
      (e.code === 'sql' && /CANTOPEN|unable to open/i.test(e.message)))
  );
}

/** Opens (creating if needed) this worker's one store connection. */
export function openStore(): Promise<Store> {
  storePromise ??= (async () => {
    // The store only ever opens under the tab lock (Task 13): a
    // second tab's worker reports 'held-elsewhere', and a worker
    // whose lock was stolen never re-opens it.
    await acquireStoreLock();
    if (storeLockState() !== 'acquired') {
      throw new StoreError(
        'locked_elsewhere',
        'Epiora is open in another tab or window',
      );
    }
    if (TAINTED) return openRelayStore();
    let last: unknown;
    for (let waited = 0; waited <= OPEN_RETRY_MS; waited += OPEN_RETRY_STEP) {
      try {
        return await openOpfsStore(dbName());
      } catch (e) {
        if (!isTransientOpen(e)) throw e;
        last = e;
        await sleep(OPEN_RETRY_STEP);
      }
    }
    throw last;
  })();
  storePromise.catch(() => {
    storePromise = null;
  });
  return storePromise;
}

/**
 * Closes the shared connection. Called when the tab lock is stolen:
 * the store must be released before the new holder opens it. A later
 * `openStore` still refuses — the lock state gates every open.
 */
export async function closeStore(): Promise<void> {
  const p = storePromise;
  storePromise = null;
  const store = await p?.catch(() => null);
  try {
    await store?.close();
  } catch {
    // The connection may already be broken; the tab is locking anyway.
  }
}

/**
 * Serialises store access across RPC handlers: `fn` gets the shared
 * connection once every earlier caller finished — writes can never
 * interleave inside one transaction. Handlers that never touch the
 * store (cancel, sessions) must not queue here, or a cancel could not
 * reach a commit holding the mutex.
 */
export function withStore<T>(fn: (store: Store) => Promise<T>): Promise<T> {
  const run = queue.then(async () => fn(await openStore()));
  queue = run.then(
    () => undefined,
    () => undefined,
  );
  return run;
}

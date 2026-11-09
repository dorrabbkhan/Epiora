// Shared harness for the data-worker browser tests
// (integration.browser.test.ts): spawn a real data worker over the
// real OPFS store, fetch Vite-served fixtures, and — once a scenario
// is done — reopen the same database through the conformance harness
// worker for raw SQL and repository calls (sahpool allows one holder
// at a time, so the data worker is terminated first).
//
// Every spawned worker is tracked in `workers`; a test's afterEach
// calls `terminateSpawned` so nothing outlives it.

import * as Comlink from 'comlink';
import { wrapDataApi } from './client';
import { RpcError } from './errors';
import type { HarnessApi } from '../store/conformance/harness.worker';
import { StoreError, type Row, type SqlValue, type Store, type Tx } from '../store/types';
import type { DataApi } from './api';
import type { ImportSettings } from '@import-types/ImportSettings';
import type { FreqOptions } from '@engine-types/FreqOptions';
import type { TablesOptions } from '@engine-types/TablesOptions';
import type { Remote } from 'comlink';

const workers: Worker[] = [];

export function spawn(db: string): Remote<DataApi> {
  const worker = new Worker(new URL('./data.worker.ts', import.meta.url), {
    type: 'module',
    name: db,
  });
  workers.push(worker);
  return wrapDataApi(worker);
}

export async function fixture(url: string): Promise<ArrayBuffer> {
  return (await fetch(url)).arrayBuffer();
}

export function csv(text: string): ArrayBuffer {
  return new TextEncoder().encode(text).buffer as ArrayBuffer;
}

export const KEY_ID: ImportSettings = {
  encoding: null,
  delimiter: null,
  header: true,
  decimal: null,
  missing_codes: ['', '.', 'NA'],
  date_format: 'iso',
  sheet: null,
  column_types: {},
  key: { column: { name: 'ID' } },
};

export const FREQ_OPTIONS: FreqOptions = {
  variables: ['E'],
  stratify_by: null,
  include_missing: false,
  ci_method: 'clopper_pearson',
};

export const TABLES_OPTIONS: TablesOptions = {
  // Text 2×2 variables need explicit orientation (engine's
  // AmbiguousLevels rule — it never guesses).
  exposure: {
    name: 'E',
    levels: [
      { type: 'text', value: 'a' },
      { type: 'text', value: 'b' },
    ],
  },
  outcome: {
    name: 'O',
    levels: [
      { type: 'text', value: 'x' },
      { type: 'text', value: 'y' },
    ],
  },
  stratify_by: [],
  include_missing: false,
};

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

interface WiredStoreError {
  __epioraStoreError: true;
  code: StoreError['code'];
  message: string;
}

function isWiredStoreError(e: unknown): e is WiredStoreError {
  return (
    typeof e === 'object' && e !== null && (e as WiredStoreError).__epioraStoreError === true
  );
}

async function call<T>(p: Promise<T>): Promise<T> {
  try {
    return await p;
  } catch (e) {
    throw isWiredStoreError(e) ? new StoreError(e.code, e.message) : e;
  }
}

interface RemoteStore {
  exec(sql: string, params?: SqlValue[]): Promise<void>;
  query(sql: string, params?: SqlValue[]): Promise<Row[]>;
  transaction(fn: (tx: Tx) => Promise<unknown>): Promise<unknown>;
  close(): Promise<void>;
}

/** Opens the conformance harness worker on `db`, retrying while the
 *  terminated data worker's sahpool handles are still dying. */
export async function openHarness(db: string): Promise<Store> {
  const worker = new Worker(new URL('../store/conformance/harness.worker.ts', import.meta.url), {
    type: 'module',
  });
  workers.push(worker);
  const api = Comlink.wrap<HarnessApi>(worker);
  let remote: RemoteStore | null = null;
  for (let i = 0; i < 100 && remote === null; i++) {
    remote = (await call(api.open(db)).then(
      (r) => r as unknown as RemoteStore,
      () => null,
    )) as RemoteStore | null;
    if (remote === null) await sleep(100);
  }
  if (remote === null) throw new Error('harness store did not open');
  const r = remote;
  return {
    exec: (sql, params) => call(r.exec(sql, params)),
    query: (sql, params) => call(r.query(sql, params)) as Promise<Row[]>,
    transaction: (fn) => call(r.transaction(Comlink.proxy(fn))) as Promise<never>,
    close: async () => {
      await call(r.close());
      worker.terminate();
    },
  };
}

/** Terminates the workers the test spawned so far (sahpool holder). */
export function terminateSpawned(): void {
  while (workers.length > 0) workers.pop()!.terminate();
}

/** Retries an RPC while a previous worker's sahpool is still closing. */
export async function untilStore<T>(fn: () => Promise<T>): Promise<T> {
  for (let i = 0; i < 100; i++) {
    try {
      return await fn();
    } catch (e) {
      if (!(e instanceof RpcError) || e.detail !== 'locked_elsewhere') throw e;
      await sleep(100);
    }
  }
  return fn();
}

/** Imports a file end to end and returns its CommitResult. */
export async function importFile(
  api: Remote<DataApi>,
  projectId: string,
  name: string,
  bytes: ArrayBuffer,
  settings?: ImportSettings,
) {
  const { sessionId } = await api.inspectFile(name, bytes);
  return api.commitImport(
    sessionId,
    { projectId, datasetName: name.replace(/\..*$/, '') },
    settings ?? ({} as ImportSettings),
    true,
    () => {},
  );
}

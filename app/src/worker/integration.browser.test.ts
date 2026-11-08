// Worker integration tests (v0.3 plan Task 9, spec §5.2–5.3, §4.3):
// a real data worker over the real OPFS store and both wasm engines,
// driven through the same Comlink facade the UI uses. Each test gets a
// fresh OPFS database (the worker's `name` option is the db name).
//
// The plan asks for `changes.countByDataset` counts and for stored
// snapshots to be corrupted/re-checked — neither crosses the typed RPC
// surface — so once a scenario is done the data worker is terminated
// and the conformance harness worker opens the same database for raw
// SQL and repository calls (sahpool allows one holder at a time).

import * as Comlink from 'comlink';
import { afterEach, describe, expect, it } from 'vitest';
import { wrapDataApi } from './client';
import { isWiredRpcError, RpcError } from './errors';
import * as changes from '../store/repo/changes';
import * as records from '../store/repo/records';
import * as versions from '../store/repo/versions';
import type { HarnessApi } from '../store/conformance/harness.worker';
import { StoreError, type Row, type SqlValue, type Store, type Tx } from '../store/types';
import type { DataApi } from './api';
import type { ImportSettings } from '@import-types/ImportSettings';
import type { FreqOptions } from '@engine-types/FreqOptions';
import type { TablesOptions } from '@engine-types/TablesOptions';
import type { Remote } from 'comlink';

import utf8CommaUrl from '../../../fixtures/import/utf8_comma/input.csv?url';
import raggedUrl from '../../../fixtures/import/ragged_row_error/input.csv?url';

const workers: Worker[] = [];
afterEach(() => {
  while (workers.length > 0) workers.pop()!.terminate();
});

function spawn(db: string): Remote<DataApi> {
  const worker = new Worker(new URL('./data.worker.ts', import.meta.url), {
    type: 'module',
    name: db,
  });
  workers.push(worker);
  return wrapDataApi(worker);
}

async function fixture(url: string): Promise<ArrayBuffer> {
  return (await fetch(url)).arrayBuffer();
}

function csv(text: string): ArrayBuffer {
  return new TextEncoder().encode(text).buffer as ArrayBuffer;
}

const KEY_ID: ImportSettings = {
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

const FREQ_OPTIONS: FreqOptions = {
  variables: ['E'],
  stratify_by: null,
  include_missing: false,
  ci_method: 'clopper_pearson',
};

const TABLES_OPTIONS: TablesOptions = {
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
async function openHarness(db: string): Promise<Store> {
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
function terminateSpawned(): void {
  while (workers.length > 0) workers.pop()!.terminate();
}

/** Retries an RPC while a previous worker's sahpool is still closing. */
async function untilStore<T>(fn: () => Promise<T>): Promise<T> {
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
async function importFile(
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

describe('data worker integration', () => {
  it('import_creates_version_1_with_matching_hash', async () => {
    const api = spawn(`t-${crypto.randomUUID()}`);
    const project = await api.createProject('Oswego');
    const commit = await importFile(api, project.id, 'input.csv', await fixture(utf8CommaUrl));
    expect(commit.version).toBe(1);

    const dataset = await api.getDataset(commit.datasetId);
    expect(dataset.currentVersion).toBe(1);
    const vs = await api.listVersions(commit.datasetId);
    expect(vs).toHaveLength(1);
    expect(vs[0]!.rowCount).toBe(2);
    expect(vs[0]!.contentHash).toMatch(/^[0-9a-f]{64}$/);

    const run = await api.runFreq(project.id, commit.datasetId, 1, {
      ...FREQ_OPTIONS,
      variables: ['AGE', 'NAME'],
    });
    // The stored content hash is the engine's dataset hash.
    expect(run.receipt.dataset.sha256).toBe(vs[0]!.contentHash);
    expect(run.receipt.dataset.version).toBe(1);

    const page = await api.getVersionPage(commit.datasetId, 1, 0, 10);
    expect(page.total).toBe(2);
    expect(page.columns).toEqual([
      { name: 'AGE', type: 'integer', missing: 1 },
      { name: 'NAME', type: 'text', missing: 0 },
    ]);
    expect(page.rows).toEqual([
      ['20', 'alice'],
      [null, 'bob'],
    ]);
  });

  it('reimport_identical_returns_null_version', async () => {
    const api = spawn(`t-${crypto.randomUUID()}`);
    const project = await api.createProject('p');
    const bytes = await fixture(utf8CommaUrl);
    const first = await importFile(api, project.id, 'input.csv', bytes);

    const { sessionId } = await api.inspectFile('input.csv', bytes);
    const second = await api.commitImport(
      sessionId,
      { datasetId: first.datasetId },
      {} as ImportSettings,
      true,
      () => {},
    );
    expect(second.version).toBeNull();
    expect(await api.listVersions(first.datasetId)).toHaveLength(1);
  });

  it('reimport_changed_writes_insert_update_delete_entries', async () => {
    const db = `t-${crypto.randomUUID()}`;
    const api = spawn(db);
    const project = await api.createProject('p');
    const v1 = 'ID,AGE\n1,20\n2,30\n3,40\n';
    const first = await importFile(api, project.id, 'd.csv', csv(v1), KEY_ID);

    // Record 2's AGE changes, record 3 is removed, record 4 is added.
    const v2 = 'ID,AGE\n1,20\n2,35\n4,50\n';
    const { sessionId } = await api.inspectFile('d.csv', csv(v2));
    const diff = await api.diffAgainst(sessionId, first.datasetId, KEY_ID);
    expect(diff.added).toBe(1);
    expect(diff.removed).toBe(1);
    expect(diff.changed).toBe(1);
    const commit = await api.commitImport(
      sessionId,
      { datasetId: first.datasetId },
      KEY_ID,
      true,
      () => {},
    );
    expect(commit.version).toBe(2);

    // Raw change counts by kind need the store — held by the worker —
    // so it is terminated and the harness worker reopens the same db.
    terminateSpawned();
    const store = await openHarness(db);
    expect(await changes.countByDataset(store, first.datasetId, 'insert')).toBe(4);
    expect(await changes.countByDataset(store, first.datasetId, 'update')).toBe(1);
    expect(await changes.countByDataset(store, first.datasetId, 'delete')).toBe(1);
    const updateRows = await store.query(
      "SELECT field, base_version FROM changes WHERE dataset_id = ? AND kind = 'update'",
      [first.datasetId],
    );
    expect(updateRows[0]!.field).toBe('AGE');
    expect(Number(updateRows[0]!.base_version)).toBe(1);
    // The rewritten record set is the v2 dataset, in file order.
    expect(await records.pageJson(store, first.datasetId, 0, 10)).toEqual([
      '{"AGE":{"type":"integer","value":20},"ID":{"type":"integer","value":1}}',
      '{"AGE":{"type":"integer","value":35},"ID":{"type":"integer","value":2}}',
      '{"AGE":{"type":"integer","value":50},"ID":{"type":"integer","value":4}}',
    ]);
    await store.close();
  });

  it('schema_change_needs_acceptance', async () => {
    const api = spawn(`t-${crypto.randomUUID()}`);
    const project = await api.createProject('p');
    const first = await importFile(api, project.id, 'd.csv', csv('ID,AGE\n1,20\n'), KEY_ID);

    const { sessionId } = await api.inspectFile('d.csv', csv('ID,AGE,NEW\n1,20,x\n'));
    const err = await api
      .commitImport(sessionId, { datasetId: first.datasetId }, KEY_ID, false, () => {})
      .then(
        () => null,
        (e: unknown) => e,
      );
    expect(err).toBeInstanceOf(RpcError);
    expect((err as RpcError).code).toBe('ImportError');
    expect((err as RpcError).detail).toBe('schema_change_unaccepted');
    expect(await api.listVersions(first.datasetId)).toHaveLength(1);

    const { sessionId: s2 } = await api.inspectFile('d.csv', csv('ID,AGE,NEW\n1,20,x\n'));
    const accepted = await api.commitImport(
      s2,
      { datasetId: first.datasetId },
      KEY_ID,
      true,
      () => {},
    );
    expect(accepted.version).toBe(2);
  });

  it('cancel_mid_write_leaves_store_unchanged', async () => {
    const db = `t-${crypto.randomUUID()}`;
    const api = spawn(db);
    const project = await api.createProject('p');
    const { sessionId } = await api.inspectFile('input.csv', await fixture(utf8CommaUrl));

    const commit = api.commitImport(
      sessionId,
      { projectId: project.id, datasetName: 'input' },
      {} as ImportSettings,
      true,
      () => {},
    );
    await api.cancelImport(sessionId);
    const err = await commit.then(
      () => null,
      (e: unknown) => e,
    );
    expect(err).toBeInstanceOf(RpcError);
    expect((err as RpcError).code).toBe('Cancelled');

    expect(await api.listDatasets(project.id)).toHaveLength(0);
    terminateSpawned();
    const store = await openHarness(db);
    expect(await store.query('SELECT COUNT(*) AS n FROM records')).toEqual([
      expect.objectContaining({ n: 0 }),
    ]);
    expect(await store.query('SELECT COUNT(*) AS n FROM changes')).toEqual([
      expect.objectContaining({ n: 0 }),
    ]);
    await store.close();
  });

  it('run_tables_saves_run_with_v2_receipt_naming_project', async () => {
    const api = spawn(`t-${crypto.randomUUID()}`);
    const project = await api.createProject('Oswego');
    const commit = await importFile(
      api,
      project.id,
      'eo.csv',
      csv('E,O\na,x\na,y\nb,x\nb,y\n'),
    );
    const run = await api.runTables(project.id, commit.datasetId, 1, TABLES_OPTIONS);
    expect(run.procedure).toBe('tables');
    expect(run.version).toBe(1);
    expect(run.receipt.receipt_version).toBe(2);
    expect(run.receipt.project).toEqual({ id: project.id, name: 'Oswego' });
    expect(run.receipt.dataset.sha256).toBe(
      (await api.listVersions(commit.datasetId))[0]!.contentHash,
    );
    expect(run.receiptSha256).toMatch(/^[0-9a-f]{64}$/);

    const listed = await api.listRuns(project.id);
    expect(listed).toHaveLength(1);
    expect(listed[0]!.id).toBe(run.id);
    expect(listed[0]!.receipt.receipt_version).toBe(2);
  });

  it('corrupt_snapshot_is_rebuilt_before_run', async () => {
    const db = `t-${crypto.randomUUID()}`;
    const api = spawn(db);
    const project = await api.createProject('p');
    const commit = await importFile(
      api,
      project.id,
      'eo.csv',
      csv('E,O\na,x\na,y\nb,x\nb,y\n'),
    );
    const wanted = (await api.listVersions(commit.datasetId))[0]!.contentHash;

    terminateSpawned();
    let store = await openHarness(db);
    await store.exec(
      'UPDATE dataset_versions SET snapshot = ? WHERE dataset_id = ? AND version = 1',
      [new TextEncoder().encode('corrupted-corrupted-corrupted'), commit.datasetId],
    );
    await store.close();

    // A fresh worker must rebuild the snapshot from records before the
    // run; the receipt carries the original content hash.
    const api2 = spawn(db);
    const run = await untilStore(() =>
      api2.runFreq(project.id, commit.datasetId, 1, { ...FREQ_OPTIONS, variables: ['E'] }),
    );
    expect(run.receipt.dataset.sha256).toBe(wanted);

    terminateSpawned();
    store = await openHarness(db);
    const snap = await versions.snapshot(store, commit.datasetId, 1);
    expect(snap).not.toBeNull();
    expect(
      Array.from(snap!.slice(0, 4))
        .map((b) => String.fromCharCode(b))
        .join(''),
    ).toBe('EPSN');
    await store.close();
  });

  it('older_version_mismatch_is_hash_mismatch', async () => {
    const db = `t-${crypto.randomUUID()}`;
    const api = spawn(db);
    const project = await api.createProject('p');
    const first = await importFile(api, project.id, 'd.csv', csv('ID,AGE\n1,20\n'), KEY_ID);
    const { sessionId } = await api.inspectFile('d.csv', csv('ID,AGE\n1,21\n'));
    const second = await api.commitImport(
      sessionId,
      { datasetId: first.datasetId },
      KEY_ID,
      true,
      () => {},
    );
    expect(second.version).toBe(2);

    terminateSpawned();
    let store = await openHarness(db);
    await store.exec(
      'UPDATE dataset_versions SET snapshot = ? WHERE dataset_id = ? AND version = 1',
      [new TextEncoder().encode('corrupted'), first.datasetId],
    );
    await store.close();

    const api2 = spawn(db);
    const err = await untilStore(() =>
      api2
        .runFreq(project.id, first.datasetId, 1, { ...FREQ_OPTIONS, variables: ['AGE'] })
        .then(
          () => null,
          (e: unknown) => {
            if (e instanceof RpcError && e.detail === 'locked_elsewhere') throw e;
            return e;
          },
        ),
    );
    expect(err).toBeInstanceOf(RpcError);
    expect((err as RpcError).code).toBe('HashMismatch');

    terminateSpawned();
    store = await openHarness(db);
    const v1 = await versions.get(store, first.datasetId, 1);
    expect(v1!.snapshotAvailable).toBe(false);
    await store.close();
  });

  it('rerun_on_newer_version_adds_a_run', async () => {
    const api = spawn(`t-${crypto.randomUUID()}`);
    const project = await api.createProject('p');
    const commit = await importFile(
      api,
      project.id,
      'eo.csv',
      csv('E,O\na,x\na,y\nb,x\nb,y\n'),
    );
    const first = await api.runTables(project.id, commit.datasetId, 1, TABLES_OPTIONS);
    expect(first.newerVersion).toBeNull();

    const { sessionId } = await api.inspectFile(
      'eo.csv',
      csv('E,O\na,x\na,y\nb,x\nb,y\nb,x\n'),
    );
    const v2 = await api.commitImport(
      sessionId,
      { datasetId: commit.datasetId },
      {} as ImportSettings,
      true,
      () => {},
    );
    expect(v2.version).toBe(2);

    const rerun = await api.rerun(first.id, 2);
    expect(rerun.version).toBe(2);
    expect(rerun.procedure).toBe('tables');
    const listed = await api.listRuns(project.id);
    expect(listed).toHaveLength(2);
    // The older card now knows a newer version exists.
    expect(listed.find((r) => r.id === first.id)!.newerVersion).toBe(2);
  });

  it('errors_are_rpc_errors_with_user_messages', async () => {
    const api = spawn(`t-${crypto.randomUUID()}`);
    const err = await api.inspectFile('input.csv', await fixture(raggedUrl)).then(
      () => null,
      (e: unknown) => e,
    );
    expect(err).not.toBeNull();
    expect(isWiredRpcError(err)).toBe(false);
    expect(err).toBeInstanceOf(RpcError);
    expect((err as RpcError).code).toBe('ImportError');
    expect((err as RpcError).detail).toBe('ragged_row');
    expect((err as RpcError).userMessage.length).toBeGreaterThan(0);
  });

  it('concurrent_writes_do_not_join_one_transaction', async () => {
    // The store is one connection: two commits issued together must
    // serialise behind the worker's store queue, and both must commit.
    const api = spawn(`t-${crypto.randomUUID()}`);
    const project = await api.createProject('p');
    const bytes = await fixture(utf8CommaUrl);
    const a = await api.inspectFile('a.csv', bytes);
    const b = await api.inspectFile('b.csv', bytes);
    const [ra, rb] = await Promise.all([
      api.commitImport(
        a.sessionId,
        { projectId: project.id, datasetName: 'a' },
        {} as ImportSettings,
        true,
        () => {},
      ),
      api.commitImport(
        b.sessionId,
        { projectId: project.id, datasetName: 'b' },
        {} as ImportSettings,
        true,
        () => {},
      ),
    ]);
    expect(ra.version).toBe(1);
    expect(rb.version).toBe(1);
    expect(await api.listDatasets(project.id)).toHaveLength(2);
  });
});

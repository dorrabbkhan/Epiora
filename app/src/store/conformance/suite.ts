// The single store conformance suite (v0.3 plan Task 8, spec §4.5).
// Every backend — OPFS sqlite-wasm today, native SQLite3MultipleCiphers
// later — runs these tests unchanged via
// `storeConformance('<backend>', () => open<Backend>Store(...))`.
//
// `open()` must return a Store on the same underlying database for every
// call within the suite (the backend's test entry point fixes one
// database per suite run): several tests close and reopen the store to
// prove persistence semantics.

import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { Store } from '../types';
import * as changes from '../repo/changes';
import * as datasets from '../repo/datasets';
import * as meta from '../repo/meta';
import * as projects from '../repo/projects';
import * as records from '../repo/records';
import * as runs from '../repo/runs';
import * as versions from '../repo/versions';
import { StoreError } from '../types';

const now = () => new Date().toISOString();

function utf8(s: string): Uint8Array {
  return new TextEncoder().encode(s);
}

/** Creates a project + dataset and returns their ids. */
async function makeDataset(store: Store) {
  const project = await projects.create(store, `p-${crypto.randomUUID()}`, now());
  const dataset = await datasets.create(
    store,
    { projectId: project.id, name: 'd', key: 'row_position', settings: '{}' },
    now(),
  );
  return { project, dataset };
}

export function storeConformance(name: string, open: () => Promise<Store>): void {
  describe(`store conformance: ${name}`, () => {
    let store: Store;

    beforeEach(async () => {
      store = await open();
    });

    afterEach(async () => {
      await store?.close();
    });

    it('migrates_fresh_and_is_idempotent', async () => {
      // open() already ran migrations at connect: the schema version and
      // the generated-once keys must be in meta, and a second open of the
      // same database must succeed without re-migrating.
      const metaRows = await store.query('SELECT key, value FROM meta ORDER BY key');
      const keys = metaRows.map((r) => r.key);
      expect(keys).toEqual(['device_id', 'lamport', 'schema_version']);
      expect(String(metaRows[2]!.value)).toBe('1');

      const tables = (
        await store.query("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
      ).map((r) => r.name);
      for (const t of [
        'changes',
        'dataset_versions',
        'datasets',
        'imports',
        'meta',
        'projects',
        'records',
        'runs',
      ]) {
        expect(tables).toContain(t);
      }

      // Idempotent: a second connect applies nothing and keeps the data.
      await store.close();
      store = await open();
      const again = await store.query("SELECT value FROM meta WHERE key = 'schema_version'");
      expect(String(again[0]!.value)).toBe('1');
    });

    it('device_id_is_stable', async () => {
      const first = await meta.deviceId(store);
      expect(first).toMatch(/^[0-9a-f-]{36}$/);
      await store.close();
      store = await open();
      expect(await meta.deviceId(store)).toBe(first);
    });

    it('foreign_keys_cascade_project_delete', async () => {
      const { project, dataset } = await makeDataset(store);
      await versions.insert(store, {
        datasetId: dataset.id,
        version: 1,
        sourceName: 'a.csv',
        sourceSha256: 'aa',
        rowCount: 1,
        contentHash: 'bb',
        snapshot: utf8('snap'),
      });
      await records.upsertBatch(store, dataset.id, 1, [{ id: 'r1', json: '{}' }]);
      await changes.appendBatch(store, [
        {
          changeId: crypto.randomUUID(),
          datasetId: dataset.id,
          recordId: 'r1',
          kind: 'insert',
          field: null,
          value: null,
          baseVersion: null,
          author: 'local',
          deviceId: 'dev',
          lamport: 1,
          importId: null,
        },
      ]);
      await store.exec(
        'INSERT INTO imports (id, dataset_id, version, settings, coercions, created_at) VALUES (?, ?, ?, ?, ?, ?)',
        [crypto.randomUUID(), dataset.id, 1, '{}', '{}', now()],
      );
      await runs.insert(store, {
        projectId: project.id,
        procedure: 'freq',
        options: '{}',
        datasetId: dataset.id,
        datasetVersion: 1,
        result: '{}',
        receipt: '{}',
        receiptHash: 'cc',
        now: now(),
      });

      await projects.remove(store, project.id);

      for (const table of [
        'datasets',
        'dataset_versions',
        'records',
        'changes',
        'imports',
        'runs',
      ]) {
        const rows = await store.query(`SELECT COUNT(*) AS n FROM ${table}`);
        expect(Number(rows[0]!.n), `${table} must be empty after cascade`).toBe(0);
      }
    });

    it('transaction_commits', async () => {
      const { project } = await makeDataset(store);
      await store.transaction(async (tx) => {
        await tx.exec('INSERT INTO projects (id, name, created_at, updated_at) VALUES (?, ?, ?, ?)', [
          'px',
          'x',
          now(),
          now(),
        ]);
      });
      const all = await projects.list(store);
      expect(all.map((p) => p.id)).toContain('px');
      expect(all.map((p) => p.id)).toContain(project.id);
    });

    it('transaction_rolls_back_on_throw', async () => {
      // The thrown error is re-thrown to the caller (message survives the
      // Comlink boundary to the worker; object identity cannot).
      await expect(
        store.transaction(async (tx) => {
          await tx.exec(
            'INSERT INTO projects (id, name, created_at, updated_at) VALUES (?, ?, ?, ?)',
            ['rolled-back', 'x', now(), now()],
          );
          throw new Error('boom');
        }),
      ).rejects.toThrow('boom');
      const rows = await store.query("SELECT id FROM projects WHERE id = 'rolled-back'");
      expect(rows).toHaveLength(0);
    });

    it('nested_transaction_is_a_store_error', async () => {
      // BEGIN while a transaction is open fails at the engine level; the
      // contract surfaces it as a StoreError, not a raw driver error.
      let release!: () => void;
      const gate = new Promise<void>((resolve) => {
        release = resolve;
      });
      const first = store.transaction(() => gate);
      const err = await store.transaction(async () => {}).then(
        () => null,
        (e: unknown) => e,
      );
      expect(err).toBeInstanceOf(StoreError);
      release();
      await first;
    });

    it('uncommitted_transaction_is_invisible_after_reopen', async () => {
      // Review Focus 4: a reload mid-import must leave no partial writes.
      const { dataset } = await makeDataset(store);
      let written!: () => void;
      const writeDone = new Promise<void>((resolve) => {
        written = resolve;
      });
      const pending = store.transaction(async (tx) => {
        await tx.exec(
          'INSERT INTO records (dataset_id, record_id, record_values, version) VALUES (?, ?, ?, ?)',
          [dataset.id, 'r-uncommitted', '{}', 1],
        );
        written();
        // Never resolve: simulates the tab dying mid-transaction.
        return new Promise<never>(() => {});
      });
      void pending.catch(() => {});
      await writeDone;
      await store.close();

      store = await open();
      expect(await records.pageJson(store, dataset.id, 0, 10)).toHaveLength(0);
    });

    it('blob_roundtrip_1mb', async () => {
      const blob = new Uint8Array(1024 * 1024);
      for (let i = 0; i < blob.length; i++) blob[i] = i % 251;
      await store.exec('CREATE TEMP TABLE blob_test (b BLOB)');
      await store.exec('INSERT INTO blob_test (b) VALUES (?)', [blob]);
      const rows = await store.query('SELECT b FROM blob_test');
      expect(rows[0]!.b).toEqual(blob);
    });

    it('bigint_roundtrip_above_2_53', async () => {
      const big = 2n ** 53n + 3n;
      await store.exec('CREATE TEMP TABLE big_test (n INTEGER)');
      await store.exec('INSERT INTO big_test (n) VALUES (?)', [big]);
      const rows = await store.query('SELECT n FROM big_test');
      expect(rows[0]!.n).toBe(big);
    });

    it('lamport_reserves_contiguous_ranges', async () => {
      await store.transaction(async (tx) => {
        const a = await meta.nextLamport(tx, 5);
        const b = await meta.nextLamport(tx, 3);
        const c = await meta.nextLamport(tx, 1);
        expect(b).toBe(a + 5);
        expect(c).toBe(a + 8);
      });
    });

    it('records_upsert_delete_and_page', async () => {
      const { dataset } = await makeDataset(store);
      await records.upsertBatch(store, dataset.id, 1, [
        { id: 'r1', json: '{"a":1}' },
        { id: 'r2', json: '{"a":2}' },
        { id: 'r3', json: '{"a":3}' },
      ]);
      expect(await records.pageJson(store, dataset.id, 0, 2)).toEqual(['{"a":1}', '{"a":2}']);
      expect(await records.pageJson(store, dataset.id, 2, 2)).toEqual(['{"a":3}']);

      // Upsert replaces the row at the same key.
      await records.upsertBatch(store, dataset.id, 2, [{ id: 'r2', json: '{"a":20}' }]);
      expect(await records.pageJson(store, dataset.id, 0, 10)).toEqual([
        '{"a":1}',
        '{"a":20}',
        '{"a":3}',
      ]);

      await records.deleteBatch(store, dataset.id, ['r1']);
      expect(await records.pageJson(store, dataset.id, 0, 10)).toEqual(['{"a":20}', '{"a":3}']);

      const batches: string[][] = [];
      for await (const batch of records.allJson(store, dataset.id)) batches.push(batch);
      expect(batches.flat()).toEqual(['{"a":20}', '{"a":3}']);
    });

    it('changes_append_order_is_seq', async () => {
      const { dataset } = await makeDataset(store);
      const mk = (i: number): changes.ChangeEntry => ({
        changeId: `c${i}`,
        datasetId: dataset.id,
        recordId: `r${i}`,
        kind: 'insert',
        field: null,
        value: null,
        baseVersion: null,
        author: 'local',
        deviceId: 'dev',
        lamport: i,
        importId: null,
      });
      await changes.appendBatch(store, [mk(1), mk(2), mk(3)]);
      const rows = await store.query(
        'SELECT change_id FROM changes WHERE dataset_id = ? ORDER BY seq',
        [dataset.id],
      );
      expect(rows.map((r) => r.change_id)).toEqual(['c1', 'c2', 'c3']);
      expect(await changes.countByDataset(store, dataset.id)).toBe(3);
    });

    it('runs_list_excludes_deleted', async () => {
      const { project, dataset } = await makeDataset(store);
      const base = {
        projectId: project.id,
        procedure: 'freq',
        options: '{}',
        datasetId: dataset.id,
        datasetVersion: 1,
        result: '{}',
        receipt: '{}',
        receiptHash: 'h',
      };
      const a = await runs.insert(store, { ...base, now: '2026-01-01T00:00:00.000Z' });
      await runs.insert(store, { ...base, now: '2026-01-02T00:00:00.000Z' });
      await runs.markDeleted(store, a.id, '2026-01-03T00:00:00.000Z');
      const listed = await runs.listByProject(store, project.id);
      expect(listed).toHaveLength(1);
      expect(listed[0]!.createdAt).toBe('2026-01-02T00:00:00.000Z');
    });

    it('quota_error_rolls_back', async () => {
      // Review Focus 3: a mid-import quota failure surfaces quota_exceeded
      // and the whole transaction is rolled back.
      const { dataset } = await makeDataset(store);
      expect(store.__simulateQuotaOnNextWrite).toBeTypeOf('function');
      const err = await store
        .transaction(async (tx) => {
          await records.upsertBatch(tx, dataset.id, 1, [{ id: 'kept', json: '{}' }]);
          store.__simulateQuotaOnNextWrite!();
          await records.upsertBatch(tx, dataset.id, 1, [{ id: 'doomed', json: '{}' }]);
        })
        .then(
          () => null,
          (e: unknown) => e,
        );
      expect(err).toBeInstanceOf(StoreError);
      expect((err as StoreError).code).toBe('quota_exceeded');
      // The transaction's earlier write is gone too.
      expect(await records.pageJson(store, dataset.id, 0, 10)).toHaveLength(0);
    });
  });
}

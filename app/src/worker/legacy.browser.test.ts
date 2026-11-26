// Legacy `.db` import across EPTB chunk boundaries (v1.1 Task 13,
// spec §4.2 — review finding): a column's `kind` must be classified
// once over ALL its cells before chunking. The old per-chunk
// inference made an all-null/integral first chunk kind 0 and a later
// chunk kind 1 or 2, and `BatchSource::merge_batch` rejected the
// change with `legacy_batch_invalid`. The test appends 50,001 rows to
// the `Oswego12` page table of a real `.db` (its AGE column is
// all-integral in the fixture, then one appended REAL), hands
// `inspectLegacy` a real `File` through the spawned data worker and
// commits the preview: AGE must land typed `real` — the storage-class
// union the native `SqliteSource` reports.

import sqlite3InitModule, { type Sqlite3Static } from '@sqlite.org/sqlite-wasm';
import sqlite3WasmUrl from '@sqlite.org/sqlite-wasm/sqlite3.wasm?url';
import { afterEach, describe, expect, it } from 'vitest';
import {
  fixture,
  openHarness,
  spawn,
  terminateSpawned,
  untilStore,
} from './integration-helpers';
import type { Store } from '../store/types';
import sampleDbUrl from '../../../fixtures/legacy/golden/reimport/SampleDB.db?url';
import originalDbUrl from '../../../fixtures/legacy/samples/ei7/Sample/SampleDB.db?url';
import reimportDbUrl from '../../../fixtures/legacy/golden/reimport/SampleDB.db?url';
import plainMdbUrl from '../../../fixtures/legacy/samples/ei3/HospitalPoints.mdb?url';

afterEach(terminateSpawned);

interface DbExport {
  sqlite3_js_db_export(db: unknown, schema: string): Uint8Array;
}

/** The fixture `.db` plus 50,001 appended records: spine rows in
 *  `Oswego` (live: RECSTATUS defaults to 1) and page rows in
 *  `Oswego12`, joined on `GlobalRecordId`. The first 50,000 appended
 *  rows leave AGE NULL; the last carries the only REAL value — the
 *  chunk-mix that used to fail the merge. */
async function makeChunkMixDb(base: ArrayBuffer): Promise<Uint8Array> {
  const sqlite3 = (await (
    sqlite3InitModule as unknown as (cfg?: {
      locateFile?: () => string;
    }) => Promise<Sqlite3Static>
  )({ locateFile: () => sqlite3WasmUrl })) as Sqlite3Static;
  const src = new Uint8Array(base);
  const db = new sqlite3.oo1.DB(':memory:');
  try {
    sqlite3.capi.sqlite3_deserialize(
      db.pointer!,
      'main',
      sqlite3.wasm.allocFromTypedArray(src),
      src.length,
      src.length,
      sqlite3.capi.SQLITE_DESERIALIZE_FREEONCLOSE |
        sqlite3.capi.SQLITE_DESERIALIZE_RESIZEABLE,
    );
    db.exec(
      `INSERT INTO Oswego (UniqueKey, GlobalRecordId)
       WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 50001)
       SELECT 1000 + x, 'appended-' || x FROM c`,
    );
    db.exec(
      `INSERT INTO Oswego12 (GlobalRecordId)
       WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 50000)
       SELECT 'appended-' || x FROM c`,
    );
    db.exec(`INSERT INTO Oswego12 (GlobalRecordId, AGE) VALUES ('appended-50001', 20.5)`);
    return (sqlite3.capi as unknown as DbExport).sqlite3_js_db_export(db, 'main');
  } finally {
    db.close();
  }
}

async function makeSchemaAndMissingViewDb(base: ArrayBuffer): Promise<Uint8Array> {
  const sqlite3 = (await (
    sqlite3InitModule as unknown as (cfg?: {
      locateFile?: () => string;
    }) => Promise<Sqlite3Static>
  )({ locateFile: () => sqlite3WasmUrl })) as Sqlite3Static;
  const src = new Uint8Array(base);
  const db = new sqlite3.oo1.DB(':memory:');
  try {
    sqlite3.capi.sqlite3_deserialize(
      db.pointer!,
      'main',
      sqlite3.wasm.allocFromTypedArray(src),
      src.length,
      src.length,
      sqlite3.capi.SQLITE_DESERIALIZE_FREEONCLOSE |
        sqlite3.capi.SQLITE_DESERIALIZE_RESIZEABLE,
    );
    db.exec("DELETE FROM metaViews WHERE Name = 'ADDFull'");
    db.exec('ALTER TABLE Oswego12 DROP COLUMN BROWNBREAD');
    db.exec('DELETE FROM metaFields WHERE FieldId = 186');
    db.exec(
      "UPDATE Oswego12 SET AGE = 33.5 WHERE GlobalRecordId = '0672b138-2bb7-44c1-8144-14f26098354d'",
    );
    return (sqlite3.capi as unknown as DbExport).sqlite3_js_db_export(db, 'main');
  } finally {
    db.close();
  }
}

async function countWhere(store: Store, sql: string, params: (string | number)[] = []): Promise<number> {
  const rows = await store.query(sql, params);
  return Number(rows[0]!.n);
}

async function storeShape(store: Store) {
  const tables = [
    'projects',
    'datasets',
    'dataset_versions',
    'records',
    'changes',
    'imports',
    'legacy_projects',
    'dataset_links',
    'programs',
    'check_code',
    'migration_reports',
  ];
  const counts: Record<string, number> = {};
  for (const table of tables) counts[table] = await countWhere(store, `SELECT COUNT(*) AS n FROM ${table}`);
  return {
    counts,
    datasets: await store.query('SELECT id, name, current_version FROM datasets ORDER BY id'),
    versions: await store.query(
      'SELECT dataset_id, version, row_count, content_hash FROM dataset_versions ORDER BY dataset_id, version',
    ),
    changes: await store.query(
      'SELECT dataset_id, kind, COUNT(*) AS n FROM changes GROUP BY dataset_id, kind ORDER BY dataset_id, kind',
    ),
    imports: await store.query('SELECT dataset_id, version FROM imports ORDER BY dataset_id, version'),
    reports: await store.query('SELECT project_id, version, json FROM migration_reports ORDER BY version'),
  };
}

async function importOriginal(db: string) {
  const api = spawn(db);
  const bytes = await fixture(originalDbUrl);
  const preview = await api.inspectLegacy([new File([new Uint8Array(bytes)], 'SampleDB.db')]);
  const projectId = await api.commitLegacy(preview.previewId, () => {});
  const oswego = (await api.listDatasets(projectId)).find((d) => d.name === 'Oswego')!;
  return { api, projectId, oswego };
}

describe('legacy .db import', () => {
  it('column_kind_is_classified_once_over_all_rows', async () => {
    const api = spawn(`t-${crypto.randomUUID()}`);
    const bytes = await makeChunkMixDb(await fixture(sampleDbUrl));
    const preview = await api.inspectLegacy([new File([new Uint8Array(bytes)], 'chunky.db')]);
    // 77 golden records + 50,001 appended.
    expect(
      preview.project.views.find((v) => v.name === 'Oswego')!.records,
    ).toBe(50_078);

    const projectId = await api.commitLegacy(preview.previewId, () => {});
    const oswego = (await api.listDatasets(projectId)).find((d) => d.name === 'Oswego')!;
    // The whole-column classification: REAL union for AGE (integral
    // first chunk + one 20.5) — identical kind on every chunk.
    const page = await api.getVersionPage(oswego.id, 1, 50_077, 1);
    expect(page.columns.find((c) => c.name === 'AGE')).toEqual({
      name: 'AGE',
      type: 'real',
      missing: 50_000,
    });
    expect(page.rows).toHaveLength(1);
    expect(Number(page.rows[0]![page.columns.findIndex((c) => c.name === 'AGE')])).toBeCloseTo(
      20.5,
    );
  });

  it('legacy_import_writes_records_changes_imports_and_a_rebuildable_snapshot', async () => {
    const db = `t-${crypto.randomUUID()}`;
    const { oswego } = await importOriginal(db);

    terminateSpawned();
    let store = await openHarness(db);
    const rows = await countWhere(store, 'SELECT COUNT(*) AS n FROM records WHERE dataset_id = ?', [oswego.id]);
    const inserts = await countWhere(
      store,
      "SELECT COUNT(*) AS n FROM changes WHERE dataset_id = ? AND kind = 'insert'",
      [oswego.id],
    );
    const imports = await countWhere(store, 'SELECT COUNT(*) AS n FROM imports WHERE dataset_id = ?', [oswego.id]);
    expect(rows).toBe(75);
    expect(inserts).toBe(75);
    expect(imports).toBe(1);
    await store.exec('UPDATE dataset_versions SET snapshot = ? WHERE dataset_id = ? AND version = 1', [
      new TextEncoder().encode('corrupted'),
      oswego.id,
    ]);
    await store.close();

    const api2 = spawn(db);
    const page = await untilStore(() => api2.getVersionPage(oswego.id, 1, 0, 1));
    expect(page.total).toBe(75);

    terminateSpawned();
    store = await openHarness(db);
    const snap = await store.query('SELECT snapshot FROM dataset_versions WHERE dataset_id = ? AND version = 1', [oswego.id]);
    expect(String.fromCharCode(...Array.from(snap[0]!.snapshot as Uint8Array).slice(0, 4))).toBe('EPSN');
    await store.close();
  });

  it('plain_table_import_writes_records_changes_imports_and_a_rebuildable_snapshot', async () => {
    const db = `t-${crypto.randomUUID()}`;
    const api = spawn(db);
    const bytes = await fixture(plainMdbUrl);
    const preview = await api.inspectPlainLegacy([
      new File([new Uint8Array(bytes)], 'HospitalPoints.mdb'),
    ]);
    expect(preview.tables).toContain('hosppts');
    const projectId = await api.commitPlainLegacy(preview.previewId, ['hosppts'], () => {});
    const plain = (await api.listDatasets(projectId)).find((d) => d.name === 'hosppts')!;

    terminateSpawned();
    let store = await openHarness(db);
    const version = await store.query(
      'SELECT row_count FROM dataset_versions WHERE dataset_id = ? AND version = 1',
      [plain.id],
    );
    const rowCount = Number(version[0]!.row_count);
    expect(rowCount).toBeGreaterThan(0);
    expect(await countWhere(store, 'SELECT COUNT(*) AS n FROM records WHERE dataset_id = ?', [plain.id])).toBe(rowCount);
    expect(
      await countWhere(
        store,
        "SELECT COUNT(*) AS n FROM changes WHERE dataset_id = ? AND kind = 'insert'",
        [plain.id],
      ),
    ).toBe(rowCount);
    expect(await countWhere(store, 'SELECT COUNT(*) AS n FROM imports WHERE dataset_id = ?', [plain.id])).toBe(1);
    await store.exec('UPDATE dataset_versions SET snapshot = ? WHERE dataset_id = ? AND version = 1', [
      new TextEncoder().encode('corrupted'),
      plain.id,
    ]);
    await store.close();

    const api2 = spawn(db);
    const page = await untilStore(() => api2.getVersionPage(plain.id, 1, 0, 1));
    expect(page.total).toBe(rowCount);

    terminateSpawned();
    store = await openHarness(db);
    const snap = await store.query('SELECT snapshot FROM dataset_versions WHERE dataset_id = ? AND version = 1', [plain.id]);
    expect(String.fromCharCode(...Array.from(snap[0]!.snapshot as Uint8Array).slice(0, 4))).toBe('EPSN');
    await store.close();
  });

  it('legacy_reimport_writes_insert_update_delete_entries_and_a_rebuildable_v2', async () => {
    const db = `t-${crypto.randomUUID()}`;
    const { api, projectId, oswego } = await importOriginal(db);
    const bytes = await fixture(reimportDbUrl);
    const preview = await api.planRefresh(projectId, [new File([new Uint8Array(bytes)], 'SampleDB.db')]);
    await api.commitRefresh(preview.previewId);
    expect((await api.listVersions(oswego.id)).map((v) => v.version)).toEqual([1, 2]);

    terminateSpawned();
    let store = await openHarness(db);
    expect(await countWhere(store, 'SELECT COUNT(*) AS n FROM records WHERE dataset_id = ?', [oswego.id])).toBe(77);
    expect(await countWhere(store, "SELECT COUNT(*) AS n FROM changes WHERE dataset_id = ? AND kind = 'insert'", [oswego.id])).toBe(78);
    expect(await countWhere(store, "SELECT COUNT(*) AS n FROM changes WHERE dataset_id = ? AND kind = 'update'", [oswego.id])).toBe(4);
    expect(await countWhere(store, "SELECT COUNT(*) AS n FROM changes WHERE dataset_id = ? AND kind = 'delete'", [oswego.id])).toBe(1);
    expect(await countWhere(store, 'SELECT COUNT(*) AS n FROM imports WHERE dataset_id = ?', [oswego.id])).toBe(2);
    await store.exec('UPDATE dataset_versions SET snapshot = ? WHERE dataset_id = ? AND version = 2', [
      new TextEncoder().encode('corrupted'),
      oswego.id,
    ]);
    await store.close();

    const api2 = spawn(db);
    const page = await untilStore(() => api2.getVersionPage(oswego.id, 2, 0, 1));
    expect(page.total).toBe(77);
    terminateSpawned();
    store = await openHarness(db);
    await store.close();
  });

  it('legacy_reimport_persists_schema_and_missing_view_report_entries', async () => {
    const db = `t-${crypto.randomUUID()}`;
    const { api, projectId } = await importOriginal(db);
    const bytes = await makeSchemaAndMissingViewDb(await fixture(reimportDbUrl));
    const preview = await api.planRefresh(projectId, [new File([new Uint8Array(bytes)], 'SampleDB.db')]);

    const oswego = preview.plan.views.find((v) => v.name === 'Oswego')!;
    expect(typeof oswego.outcome).not.toBe('string');
    if (typeof oswego.outcome === 'string') throw new Error('Oswego unexpectedly did not change');
    expect(oswego.outcome.changed.schema.added_columns).toContain('ReimportFlag');
    expect(oswego.outcome.changed.schema.removed_columns).toContain('BROWNBREAD');
    expect(oswego.outcome.changed.schema.type_changes.map((c) => c.column)).toContain('AGE');
    expect(preview.plan.views.find((v) => v.name === 'ADDFull')?.outcome).toBe('missing');

    await api.commitRefresh(preview.previewId);
    terminateSpawned();
    const store = await openHarness(db);
    const reports = await store.query(
      'SELECT version, json FROM migration_reports WHERE project_id = ? ORDER BY version',
      [projectId],
    );
    expect(reports).toHaveLength(2);
    const entries = JSON.parse(String(reports[1]!.json)) as {
      kind: string;
      item: string;
      reason: string;
    }[];
    expect(entries).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ kind: 'schema_field_added', item: 'Oswego/ReimportFlag' }),
        expect.objectContaining({ kind: 'schema_field_removed', item: 'Oswego/BROWNBREAD' }),
        expect.objectContaining({ kind: 'schema_field_retyped', item: 'Oswego/AGE' }),
        expect.objectContaining({
          kind: 'view_not_refreshed',
          item: 'ADDFull',
          reason: expect.stringContaining('missing from the newer copy'),
        }),
      ]),
    );
    await store.close();
  });

  it('unchanged_legacy_reimport_writes_nothing', async () => {
    const db = `t-${crypto.randomUUID()}`;
    await importOriginal(db);
    terminateSpawned();
    let store = await openHarness(db);
    const before = await storeShape(store);
    await store.close();

    const api = spawn(db);
    const project = (await api.listProjects())[0]!;
    const bytes = await fixture(originalDbUrl);
    const preview = await api.planRefresh(project.id, [new File([new Uint8Array(bytes)], 'SampleDB.db')]);
    await api.commitRefresh(preview.previewId);

    terminateSpawned();
    store = await openHarness(db);
    expect(await storeShape(store)).toEqual(before);
    await store.close();
  });
});

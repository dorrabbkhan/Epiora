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
import { fixture, spawn, terminateSpawned } from './integration-helpers';
import sampleDbUrl from '../../../fixtures/legacy/golden/reimport/SampleDB.db?url';

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
});

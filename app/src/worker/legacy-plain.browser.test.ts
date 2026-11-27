import sqlite3InitModule, { type Sqlite3Static } from '@sqlite.org/sqlite-wasm';
import sqlite3WasmUrl from '@sqlite.org/sqlite-wasm/sqlite3.wasm?url';
import { afterEach, describe, expect, it } from 'vitest';
import {
  fixture,
  openHarness,
  spawn,
  terminateSpawned,
} from './integration-helpers';
import { RpcError } from './errors';
import plainMdbUrl from '../../../fixtures/legacy/samples/ei3/HospitalPoints.mdb?url';

afterEach(terminateSpawned);

interface DbExport {
  sqlite3_js_db_export(db: unknown, schema: string): Uint8Array;
}

async function rpcDetail(promise: Promise<unknown>): Promise<string | null> {
  try {
    await promise;
    return null;
  } catch (e) {
    return e instanceof RpcError ? e.detail : String(e);
  }
}

async function makePlainBinaryDb(): Promise<Uint8Array> {
  const sqlite3 = (await (
    sqlite3InitModule as unknown as (cfg?: {
      locateFile?: () => string;
    }) => Promise<Sqlite3Static>
  )({ locateFile: () => sqlite3WasmUrl })) as Sqlite3Static;
  const db = new sqlite3.oo1.DB(':memory:');
  try {
    db.exec('CREATE TABLE measurements (id INTEGER, payload BLOB, note TEXT)');
    db.exec("INSERT INTO measurements VALUES (1, x'0102', 'first'), (2, x'0304', 'second')");
    return (sqlite3.capi as unknown as DbExport).sqlite3_js_db_export(db, 'main');
  } finally {
    db.close();
  }
}

describe('plain legacy preview lifecycle', () => {
  it('discard_and_commit_release_the_plain_preview_handle', async () => {
    const db = `t-${crypto.randomUUID()}`;
    const api = spawn(db);
    const bytes = await fixture(plainMdbUrl);
    const discarded = await api.inspectPlainLegacy([
      new File([new Uint8Array(bytes)], 'HospitalPoints.mdb'),
    ]);
    const discardOnce = async () => api.discardPlainPreview(discarded.previewId);
    expect(await rpcDetail(discardOnce())).toBeNull();
    expect(await rpcDetail(discardOnce())).toBe('session_not_found');
    expect(
      await rpcDetail(api.commitPlainLegacy(discarded.previewId, ['hosppts'], () => {})),
    ).toBe('session_not_found');

    const committed = await api.inspectPlainLegacy([
      new File([new Uint8Array(bytes)], 'HospitalPoints.mdb'),
    ]);
    await api.commitPlainLegacy(committed.previewId, ['hosppts'], () => {});
    const discardCommitted = async () => api.discardPlainPreview(committed.previewId);
    expect(await rpcDetail(discardCommitted())).toBe('session_not_found');
    expect(
      await rpcDetail(api.commitPlainLegacy(committed.previewId, ['hosppts'], () => {})),
    ).toBe('session_not_found');
  });

  it('binary_column_is_warned_skipped_and_written_to_the_migration_report', async () => {
    const db = `t-${crypto.randomUUID()}`;
    const api = spawn(db);
    const bytes = await makePlainBinaryDb();
    const preview = await api.inspectPlainLegacy([
      new File([bytes.buffer as ArrayBuffer], 'binary.db'),
    ]);

    expect(preview.tables).toEqual(['measurements']);
    expect(preview.skippedColumns).toEqual([
      {
        table: 'measurements',
        column: 'payload',
        reason: 'binary data not imported',
      },
    ]);

    const projectId = await api.commitPlainLegacy(preview.previewId, ['measurements'], () => {});
    const dataset = (await api.listDatasets(projectId)).find((d) => d.name === 'measurements')!;
    const page = await api.getVersionPage(dataset.id, 1, 0, 10);
    expect(page.columns.map((c) => c.name)).toEqual(['id', 'note']);
    expect(page.rows).toHaveLength(2);

    terminateSpawned();
    const store = await openHarness(db);
    try {
      const reports = await store.query(
        'SELECT json FROM migration_reports WHERE project_id = ?',
        [projectId],
      );
      expect(reports).toHaveLength(1);
      expect(JSON.parse(String(reports[0]!.json))).toEqual([
        {
          kind: 'column_not_imported',
          item: 'measurements/payload',
          reason: 'binary data not imported',
        },
      ]);
    } finally {
      await store.close();
    }
  });
});

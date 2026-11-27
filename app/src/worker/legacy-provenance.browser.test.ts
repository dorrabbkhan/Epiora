// Legacy source provenance and report-only refresh outcomes (fix round
// 4). Kept separate from legacy.browser.test.ts so both files stay below
// the project's size rule; both drive the real worker and OPFS store.
import sqlite3InitModule, { type Sqlite3Static } from '@sqlite.org/sqlite-wasm';
import sqlite3WasmUrl from '@sqlite.org/sqlite-wasm/sqlite3.wasm?url';
import { afterEach, describe, expect, it } from 'vitest';
import {
  fixture,
  openHarness,
  spawn,
  terminateSpawned,
} from './integration-helpers';
import type { Store } from '../store/types';
import originalDbUrl from '../../../fixtures/legacy/samples/ei7/Sample/SampleDB.db?url';
import reimportDbUrl from '../../../fixtures/legacy/golden/reimport/SampleDB.db?url';

afterEach(terminateSpawned);

interface DbExport {
  sqlite3_js_db_export(db: unknown, schema: string): Uint8Array;
}

async function makeMissingViewOnlyDb(base: ArrayBuffer): Promise<Uint8Array> {
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
    return (sqlite3.capi as unknown as DbExport).sqlite3_js_db_export(db, 'main');
  } finally {
    db.close();
  }
}

async function sha256Hex(bytes: ArrayBuffer): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', bytes);
  return Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, '0')).join('');
}

async function countWhere(store: Store, sql: string, params: (string | number)[] = []): Promise<number> {
  const rows = await store.query(sql, params);
  return Number(rows[0]!.n);
}

async function importOriginal(db: string) {
  const api = spawn(db);
  const bytes = await fixture(originalDbUrl);
  const preview = await api.inspectLegacy([
    new File([new Uint8Array(bytes)], 'SampleDB.db'),
  ]);
  const projectId = await api.commitLegacy(preview.previewId, () => {});
  const datasets = await api.listDatasets(projectId);
  const oswego = datasets.find((d) => d.name === 'Oswego')!;
  return { api, projectId, oswego };
}

describe('legacy provenance and report-only refresh', () => {
  it('legacy_versions_store_the_source_file_sha256', async () => {
    const db = `t-${crypto.randomUUID()}`;
    const api = spawn(db);
    const originalBytes = await fixture(originalDbUrl);
    const preview = await api.inspectLegacy([
      new File([new Uint8Array(originalBytes)], 'SampleDB.db'),
    ]);
    const projectId = await api.commitLegacy(preview.previewId, () => {});
    const oswego = (await api.listDatasets(projectId)).find((d) => d.name === 'Oswego')!;
    const v1 = (await api.listVersions(oswego.id)).find((v) => v.version === 1)!;
    expect(v1.sourceSha256).toBe(await sha256Hex(originalBytes));
    expect(v1.sourceSha256).not.toBe(v1.contentHash);

    const reimportBytes = await fixture(reimportDbUrl);
    const refresh = await api.planRefresh(projectId, [
      new File([new Uint8Array(reimportBytes)], 'SampleDB.db'),
    ]);
    await api.commitRefresh(refresh.previewId);
    const v2 = (await api.listVersions(oswego.id)).find((v) => v.version === 2)!;
    expect(v2.sourceSha256).toBe(await sha256Hex(reimportBytes));
    expect(v2.sourceSha256).not.toBe(v2.contentHash);
  });

  it('missing_only_legacy_reimport_writes_the_marker_and_no_versions', async () => {
    const db = `t-${crypto.randomUUID()}`;
    const { api, projectId, oswego } = await importOriginal(db);
    const bytes = await makeMissingViewOnlyDb(await fixture(originalDbUrl));
    const preview = await api.planRefresh(projectId, [new File([new Uint8Array(bytes)], 'SampleDB.db')]);
    expect(preview.plan.views.find((v) => v.name === 'ADDFull')?.outcome).toBe('missing');
    expect(
      preview.plan.views.every(
        (v) => v.outcome === 'unchanged' || v.outcome === 'missing',
      ),
    ).toBe(true);

    await api.commitRefresh(preview.previewId);
    expect((await api.listVersions(oswego.id)).map((v) => v.version)).toEqual([1]);

    terminateSpawned();
    const store = await openHarness(db);
    const reports = await store.query(
      'SELECT version, json FROM migration_reports WHERE project_id = ? ORDER BY version',
      [projectId],
    );
    expect(reports).toHaveLength(2);
    const entries = JSON.parse(String(reports[1]!.json)) as { kind: string; item: string; reason: string }[];
    expect(entries).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          kind: 'view_not_refreshed',
          item: 'ADDFull',
          reason: expect.stringContaining('missing from the newer copy'),
        }),
      ]),
    );
    expect(await countWhere(store, 'SELECT COUNT(*) AS n FROM dataset_versions WHERE dataset_id = ?', [oswego.id])).toBe(1);
    expect(await countWhere(store, 'SELECT COUNT(*) AS n FROM imports WHERE dataset_id = ?', [oswego.id])).toBe(1);
    await store.close();
  });
});

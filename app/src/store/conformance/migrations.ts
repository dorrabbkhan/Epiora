// The schema-migration cases of the store conformance suite (v0.3
// plan Task 8, spec §4.5; split from suite.ts to bring it back under
// the ~400-line file rule — docs/decisions.md). `migrationTests` is
// called inside `storeConformance`'s describe so the cases keep the
// suite's beforeEach/afterEach store lifecycle; `ref` shares the
// suite's `store` binding because the cases close and reopen it.

import { expect, it } from 'vitest';
import type { Store } from '../types';
import * as datasets from '../repo/datasets';
import * as legacy from '../repo/legacy';
import * as projects from '../repo/projects';
import * as runs from '../repo/runs';

const now = () => new Date().toISOString();

/** The tables migration 0004 creates (v1.1 spec §4.2). */
const LEGACY_TABLES = [
  'legacy_projects',
  'dataset_links',
  'programs',
  'check_code',
  'migration_reports',
];

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

/** The suite's `store` binding — the migration cases close and
 *  reopen it, so reads and writes go through `ref`. */
export interface StoreRef {
  get(): Store;
  set(store: Store): void;
}

export function migrationTests(ref: StoreRef, open: () => Promise<Store>): void {
  it('migration_0002_adds_command_on_both_backends', async () => {
    // Rewind to the v1 shape: no `command` column (and no `notes`
    // column — v1 predates both) and a run row written the old way.
    // Reopening replays 0002 and 0003 — the columns are added and the
    // old row survives with `command` NULL, while new rows store it.
    let store = ref.get();
    const { project, dataset } = await makeDataset(store);
    await store.exec('ALTER TABLE runs DROP COLUMN command');
    await store.exec('ALTER TABLE runs DROP COLUMN notes');
    await store.exec("UPDATE meta SET value = '1' WHERE key = 'schema_version'");
    await store.exec(
      'INSERT INTO runs (id, project_id, procedure, options, dataset_id, dataset_version, result, receipt, receipt_hash, created_at) ' +
        'VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)',
      ['r-old', project.id, 'tables', '{}', dataset.id, 1, '{}', '{}', 'h', now()],
    );
    await store.close();
    store = await open();
    ref.set(store);

    const columns = (
      await store.query('PRAGMA table_info(runs)')
    ).map((r) => r.name);
    expect(columns).toContain('command');
    expect(columns).toContain('notes');
    const version = await store.query(
      "SELECT value FROM meta WHERE key = 'schema_version'",
    );
    expect(String(version[0]!.value)).toBe('4');

    const old = await runs.get(store, 'r-old');
    expect(old).not.toBeNull();
    expect(old!.command).toBeNull();

    const added = await runs.insert(store, {
      projectId: project.id,
      procedure: 'means',
      options: '{}',
      datasetId: dataset.id,
      datasetVersion: 1,
      result: '{}',
      receipt: '{}',
      receiptHash: 'h',
      command: 'MEANS AGE SEX',
      notes: '[]',
      now: now(),
    });
    expect((await runs.get(store, added.id))!.command).toBe('MEANS AGE SEX');

    // Leave the database as empty as the test found it — later cases
    // count rows globally.
    await projects.remove(store, project.id);
  });

  it('migration_0003_adds_notes_on_both_backends', async () => {
    // Rewind to the v2 shape: no `notes` column and a run row written
    // the old way. Reopening replays 0003 — the column is added and
    // the old row survives with `notes` NULL (its card shows no
    // notes: they did not exist when it ran), while new rows store
    // the run's assumption notes (v1.0 Task 23, spec §6.1).
    let store = ref.get();
    const { project, dataset } = await makeDataset(store);
    await store.exec('ALTER TABLE runs DROP COLUMN notes');
    await store.exec("UPDATE meta SET value = '2' WHERE key = 'schema_version'");
    await store.exec(
      'INSERT INTO runs (id, project_id, procedure, options, dataset_id, dataset_version, result, receipt, receipt_hash, command, created_at) ' +
        'VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)',
      ['r-old', project.id, 'tables', '{}', dataset.id, 1, '{}', '{}', 'h', 'TABLES E O', now()],
    );
    await store.close();
    store = await open();
    ref.set(store);

    const columns = (
      await store.query('PRAGMA table_info(runs)')
    ).map((r) => r.name);
    expect(columns).toContain('notes');
    const version = await store.query(
      "SELECT value FROM meta WHERE key = 'schema_version'",
    );
    expect(String(version[0]!.value)).toBe('4');

    const old = await runs.get(store, 'r-old');
    expect(old).not.toBeNull();
    expect(old!.notes).toBeNull();

    const json =
      '[{"code":"zero_cell","statistics":["two_by_two.odds_ratio.ci_95_wald.lower"],"values":{"zero_cells":1},"suggests":"two_by_two.odds_ratio.cmle"}]';
    const added = await runs.insert(store, {
      projectId: project.id,
      procedure: 'tables',
      options: '{}',
      datasetId: dataset.id,
      datasetVersion: 1,
      result: '{}',
      receipt: '{}',
      receiptHash: 'h',
      command: 'TABLES E O',
      notes: json,
      now: now(),
    });
    expect((await runs.get(store, added.id))!.notes).toBe(json);

    await projects.remove(store, project.id);
  });

  it('migration_0004_on_both_backends', async () => {
    // Rewind to the v3 shape: none of the five legacy tables. Reopening
    // replays 0004 — the tables a legacy project writes appear
    // (v1.1 plan Task 6, spec §4.2) and repo rows round-trip; deleting
    // the project cascades every row.
    let store = ref.get();
    for (const t of LEGACY_TABLES) {
      await store.exec(`DROP TABLE IF EXISTS ${t}`);
    }
    await store.exec("UPDATE meta SET value = '3' WHERE key = 'schema_version'");
    await store.close();
    store = await open();
    ref.set(store);

    const names = (
      await store.query("SELECT name FROM sqlite_master WHERE type = 'table'")
    ).map((r) => r.name);
    for (const t of LEGACY_TABLES) {
      expect(names, `${t} must exist`).toContain(t);
    }
    const version = await store.query(
      "SELECT value FROM meta WHERE key = 'schema_version'",
    );
    expect(String(version[0]!.value)).toBe('4');

    const { project, dataset } = await makeDataset(store);
    await legacy.upsertProject(store, {
      projectId: project.id,
      legacyId: 'oswego',
      sourceFormat: 'ei7_access',
      fileName: 'Sample.prj',
      importedAt: now(),
    });
    expect(await legacy.project(store, project.id)).toMatchObject({
      legacyId: 'oswego',
      sourceFormat: 'ei7_access',
    });
    await legacy.insertLink(store, {
      projectId: project.id,
      parentDataset: 'parent',
      childDataset: 'child',
      childKey: 'FKEY',
      parentKey: 'GlobalRecordId',
      kind: 'relate',
    });
    expect(await legacy.links(store, project.id)).toHaveLength(1);
    await legacy.insertProgram(store, {
      id: crypto.randomUUID(),
      projectId: project.id,
      name: 'ANALYSIS',
      text: 'FREQ ILL',
      source: 'saved',
    });
    expect((await legacy.programs(store, project.id))[0]!.name).toBe('ANALYSIS');
    await legacy.upsertCheckCode(store, {
      projectId: project.id,
      datasetId: dataset.id,
      text: 'BEFORE\nEND',
      blocks: 1,
    });
    expect((await legacy.checkCode(store, project.id, dataset.id))!.blocks).toBe(1);
    await legacy.insertMigrationReport(store, {
      projectId: project.id,
      version: 1,
      json: '[]',
    });
    expect((await legacy.latestMigrationReport(store, project.id))!.json).toBe('[]');

    await projects.remove(store, project.id);
    for (const t of LEGACY_TABLES) {
      const rows = await store.query(`SELECT COUNT(*) AS n FROM ${t}`);
      expect(Number(rows[0]!.n), `${t} must be empty after cascade`).toBe(0);
    }
  });
}

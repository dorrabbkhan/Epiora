// The canonical records/changes/imports write path (v0.3 spec
// §4.2–§4.5), shared by the CSV commit in `commit.ts` and the legacy
// project import/re-import. The caller owns the transaction; throwing
// here — including cancellation — rolls every write back with it.

import type { ImportProgress } from './api';
import * as changes from '../store/repo/changes';
import * as datasets from '../store/repo/datasets';
import * as imports from '../store/repo/imports';
import * as meta from '../store/repo/meta';
import * as records from '../store/repo/records';
import * as versions from '../store/repo/versions';
import type { Tx } from '../store/types';

const BATCH = 10_000;

/** Appends `entries` in one shot after reserving their Lamport range
 *  inside the same transaction (one `nextLamport` per batch). */
async function appendEntries(
  tx: Tx,
  datasetId: string,
  importId: string,
  deviceId: string,
  entries: Omit<changes.ChangeEntry, 'datasetId' | 'author' | 'deviceId' | 'lamport' | 'importId'>[],
): Promise<void> {
  if (entries.length === 0) return;
  const base = await meta.nextLamport(tx, entries.length);
  await changes.appendBatch(
    tx,
    entries.map((e, i) => ({
      ...e,
      datasetId,
      importId,
      deviceId,
      author: 'local',
      lamport: base + i,
    })),
  );
}

type Entry = Omit<
  changes.ChangeEntry,
  'datasetId' | 'author' | 'deviceId' | 'lamport' | 'importId'
>;

/** Streams a paged source through `collect` in BATCH-sized chunks,
 *  flushing `appendEntries` per chunk. */
async function appendPaged<T>(
  tx: Tx,
  datasetId: string,
  importId: string,
  deviceId: string,
  total: number,
  page: (start: number, len: number) => T[],
  map: (item: T) => Entry,
  checkCancel: () => Promise<void>,
): Promise<void> {
  let pending: Entry[] = [];
  for (let start = 0; start < total; start += BATCH) {
    await checkCancel();
    for (const item of page(start, BATCH)) {
      pending.push(map(item));
      if (pending.length === BATCH) {
        await appendEntries(tx, datasetId, importId, deviceId, pending);
        pending = [];
      }
    }
  }
  await appendEntries(tx, datasetId, importId, deviceId, pending);
}

export interface CanonicalRecordBatch {
  ids: string[];
  json: string[];
}

/** Canonical rows of the version being committed, paged in file order. */
export interface CanonicalRecordSource {
  rowCount: number;
  recordBatch(start: number, len: number): CanonicalRecordBatch;
}

/** The record-level diff a re-import commit turns into change entries. */
export interface CanonicalDiffSource {
  addedCount: number;
  changedCount: number;
  removedIds: string[];
  addedBatch(start: number, len: number): CanonicalRecordBatch;
  changedBatch(
    start: number,
    len: number,
  ): { record_id: string; field: string; new: unknown }[];
}

/** Everything the shared records/changes/imports + snapshot write path
 *  needs. The caller owns the transaction and the datasets row. */
export interface WriteVersionInput {
  tx: Tx;
  datasetId: string;
  version: number;
  now: string;
  sourceName: string;
  sourceSha256: string;
  importSettings: string;
  coercions: string;
  encoding?: string | null;
  delimiter?: string | null;
  sheet?: string | null;
  serializedSettings: string;
  rows: CanonicalRecordSource;
  diff: CanonicalDiffSource | null;
  contentHash: string;
  snapshot: () => Uint8Array;
  checkCancel: () => Promise<void>;
  emit?: (stage: ImportProgress['stage'], done: number, total: number) => void;
}

/** The v0.3 write path shared by CSV and legacy imports (spec §4.2–
 *  §4.5): the `imports` row first, canonical `records` in file order,
 *  insert/update/delete `changes` entries, then the derived snapshot.
 *  It never opens a transaction itself; cancellation propagates to the
 *  caller's transaction and rolls every write back. */
export async function writeCommittedVersion(input: WriteVersionInput): Promise<void> {
  const {
    tx,
    datasetId,
    version,
    now,
    rows,
    diff,
    checkCancel,
  } = input;
  const emit = input.emit ?? (() => {});
  await checkCancel();
  const deviceId = await meta.deviceId(tx);
  const importId = crypto.randomUUID();
  // Inserted first so every change entry can carry its import_id.
  await imports.insert(tx, {
    id: importId,
    datasetId,
    version,
    settings: input.importSettings,
    coercions: input.coercions,
    encoding: input.encoding ?? null,
    delimiter: input.delimiter ?? null,
    sheet: input.sheet ?? null,
    now,
  });

  const changedIds = new Set<string>();
  if (diff !== null) {
    for (let start = 0; ; start += BATCH) {
      const page = diff.changedBatch(start, BATCH);
      for (const fc of page) changedIds.add(fc.record_id);
      if (page.length < BATCH) break;
    }
  }

  // Records: first import inserts every row; re-import rewrites the
  // table in the new file order so `records.allJson` stays in the
  // canonical order snapshot rebuilds hash over (§4.3). Consecutive
  // rows with the same last-changed version flush as one batch without
  // ever reordering a changed row ahead of a later unchanged one.
  const carried = diff === null ? null : await records.idVersions(tx, datasetId);
  if (diff !== null) await records.clearAll(tx, datasetId);
  let done = 0;
  for (let start = 0; start < rows.rowCount; start += BATCH) {
    await checkCancel();
    const page = rows.recordBatch(start, BATCH);
    let runVersion: number | null = null;
    let runRows: { id: string; json: string }[] = [];
    const flush = async () => {
      if (runVersion !== null && runRows.length > 0) {
        await records.upsertBatch(tx, datasetId, runVersion, runRows);
      }
      runRows = [];
    };
    for (let i = 0; i < page.ids.length; i++) {
      const id = page.ids[i]!;
      const rowVersion =
        carried === null || changedIds.has(id) ? version : (carried.get(id) ?? version);
      if (runVersion !== rowVersion) {
        await flush();
        runVersion = rowVersion;
      }
      runRows.push({ id, json: page.json[i]! });
    }
    await flush();
    if (diff === null) {
      // First import: one insert entry per record, whole JSON.
      await appendEntries(
        tx,
        datasetId,
        importId,
        deviceId,
        page.ids.map((id, i) => ({
          changeId: crypto.randomUUID(),
          recordId: id,
          kind: 'insert' as const,
          field: null,
          value: page.json[i]!,
          baseVersion: null,
        })),
      );
    }
    done += page.ids.length;
    emit('write', done, rows.rowCount);
  }

  if (diff !== null) {
    // Re-import entries (§4.2): insert per added record, one update
    // per changed field with base_version, delete per missing record.
    const prevVersion = version - 1;
    await appendPaged(
      tx,
      datasetId,
      importId,
      deviceId,
      diff.addedCount,
      (start, len) => {
        const page = diff.addedBatch(start, len);
        return page.ids.map((id, i) => ({ id, json: page.json[i]! }));
      },
      (a) => ({
        changeId: crypto.randomUUID(),
        recordId: a.id,
        kind: 'insert' as const,
        field: null,
        value: a.json,
        baseVersion: null,
      }),
      checkCancel,
    );
    await appendPaged(
      tx,
      datasetId,
      importId,
      deviceId,
      diff.changedCount,
      (start, len) => diff.changedBatch(start, len),
      (fc) => ({
        changeId: crypto.randomUUID(),
        recordId: fc.record_id,
        kind: 'update' as const,
        field: fc.field,
        value: fc.new === null || fc.new === undefined ? null : JSON.stringify(fc.new),
        baseVersion: prevVersion,
      }),
      checkCancel,
    );
    await appendPaged(
      tx,
      datasetId,
      importId,
      deviceId,
      diff.removedIds.length,
      (start, len) => diff.removedIds.slice(start, start + len),
      (id) => ({
        changeId: crypto.randomUUID(),
        recordId: id,
        kind: 'delete' as const,
        field: null,
        value: null,
        baseVersion: prevVersion,
      }),
      checkCancel,
    );
  }

  emit('snapshot', 0, 1);
  await checkCancel();
  await versions.insert(tx, {
    datasetId,
    version,
    sourceName: input.sourceName,
    sourceSha256: input.sourceSha256,
    rowCount: rows.rowCount,
    contentHash: input.contentHash,
    snapshot: input.snapshot(),
  });
  await datasets.saveSettings(tx, datasetId, input.serializedSettings);
  await datasets.setCurrentVersion(tx, datasetId, version);
  emit('snapshot', 1, 1);
  // Last checkpoint before the transaction commits: a cancel that
  // landed during the snapshot writes still rolls every write back.
  await checkCancel();
}

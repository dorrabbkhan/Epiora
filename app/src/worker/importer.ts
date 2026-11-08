// Import sessions and the version-creating commit (v0.3 spec §4.2,
// §5.3, §6.4). `inspectFile` keeps the file bytes and a cached
// `ImportedHandle` keyed by the exact settings JSON until
// `discardSession` or `commitImport`; the handle is what `recordBatch`,
// `toSnapshot`, `contentHash` and `diffAgainst` page over.
//
// Every commit is ONE transaction. On re-import `records` is rewritten
// in the new file order — not diffed in place — so `records.allJson`
// keeps producing the canonical row order a snapshot rebuild hashes to
// (the content hash is row-order sensitive); `changes` still receives
// the minimal insert/update/delete entries the diff reported. The
// `version` column carries forward "last changed" per record.
//
// The imports row is inserted first inside the transaction (before the
// record/change batches) so every change entry can carry `import_id`.

import type { DiffHandle, ImportedHandle } from '@import/epiora_import_wasm';
import type { DiffSummary } from '@import-types/DiffSummary';
import type { FieldChange } from '@import-types/FieldChange';
import type { ImportSettings } from '@import-types/ImportSettings';
import type { ImportSummary } from '@import-types/ImportSummary';
import type { Inspection } from '@import-types/Inspection';
import type { KeyCandidate } from '@import-types/KeyCandidate';
import type { RecordBatch } from '@import-types/RecordBatch';
import type { CommitResult, ImportProgress } from './api';
import { importer } from './engines';
import { rpc } from './errors';
import { loadEntry, serializeSettings } from './snapshots';
import * as changes from '../store/repo/changes';
import * as datasets from '../store/repo/datasets';
import * as imports from '../store/repo/imports';
import * as meta from '../store/repo/meta';
import * as records from '../store/repo/records';
import * as versions from '../store/repo/versions';
import type { Store, Tx } from '../store/types';

const BATCH = 10_000;

interface Session {
  name: string;
  bytes: Uint8Array;
  imported: { settingsKey: string; handle: ImportedHandle } | null;
  cancelled: boolean;
}

const sessions = new Map<string, Session>();

function getSession(sessionId: string): Session {
  const s = sessions.get(sessionId);
  if (s === undefined) {
    throw rpc('session_not_found', `import session ${sessionId} does not exist`);
  }
  return s;
}

function destroy(sessionId: string): void {
  sessions.get(sessionId)?.imported?.handle.free();
  sessions.delete(sessionId);
}

/** The ImportedHandle for `settings`, reusing the session's cached one
 *  when the settings are byte-identical. */
async function importedFor(s: Session, settings: ImportSettings): Promise<ImportedHandle> {
  const settingsKey = JSON.stringify(settings);
  if (s.imported?.settingsKey === settingsKey) return s.imported.handle;
  const imp = await importer();
  const handle = imp.importFile(s.bytes, s.name, settings);
  s.imported?.handle.free();
  s.imported = { settingsKey, handle };
  return handle;
}

export async function inspectFile(
  name: string,
  bytes: ArrayBuffer,
): Promise<{ sessionId: string; inspection: Inspection; keyCandidates: KeyCandidate[] }> {
  const imp = await importer();
  const data = new Uint8Array(bytes);
  const inspection = imp.inspect(data, name) as Inspection;
  const handle = imp.importFile(data, name, inspection.settings);
  const sessionId = crypto.randomUUID();
  sessions.set(sessionId, {
    name,
    bytes: data,
    imported: { settingsKey: JSON.stringify(inspection.settings), handle },
    cancelled: false,
  });
  return { sessionId, inspection, keyCandidates: handle.keyCandidates() as KeyCandidate[] };
}

export async function previewWithSettings(
  sessionId: string,
  settings: ImportSettings,
): Promise<{ inspection: Inspection; keyCandidates: KeyCandidate[] }> {
  const s = getSession(sessionId);
  const imp = await importer();
  const inspection = imp.inspectWith(s.bytes, s.name, settings) as Inspection;
  const handle = await importedFor(s, inspection.settings);
  return { inspection, keyCandidates: handle.keyCandidates() as KeyCandidate[] };
}

export async function diffAgainst(
  store: Store,
  sessionId: string,
  datasetId: string,
  settings: ImportSettings,
): Promise<DiffSummary> {
  const s = getSession(sessionId);
  const dataset = await datasets.get(store, datasetId);
  if (dataset === null) throw rpc('not_found', `dataset ${datasetId} does not exist`);
  const handle = await importedFor(s, settings);
  const prev = await loadEntry(store, datasetId, dataset.currentVersion);
  const imp = await importer();
  const diff = imp.diffAgainst(prev.bytes, handle);
  const summary = diff.summary() as DiffSummary;
  diff.free();
  return summary;
}

export async function cancelImport(sessionId: string): Promise<void> {
  getSession(sessionId).cancelled = true;
}

export async function discardSession(sessionId: string): Promise<void> {
  const s = getSession(sessionId);
  s.cancelled = true;
  destroy(sessionId);
}

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

export async function commitImport(
  store: Store,
  sessionId: string,
  target: { projectId: string; datasetName: string } | { datasetId: string },
  settings: ImportSettings,
  acceptSchemaChanges: boolean,
  onProgress: (p: ImportProgress) => void,
): Promise<CommitResult> {
  const s = getSession(sessionId);
  let diff: DiffHandle | null = null;
  try {
    const emit = (stage: ImportProgress['stage'], done: number, total: number) => {
      try {
        void onProgress({ stage, done, total });
      } catch {
        // A released progress proxy must not fail the import.
      }
    };
    // sqlite-wasm exec resolves on a microtask, so without an explicit
    // yield the worker's event loop never runs `cancelImport` during a
    // commit — the flag check must yield to be reachable at all.
    const checkCancel = async () => {
      await new Promise((r) => setTimeout(r, 0));
      if (s.cancelled) throw rpc('cancelled', 'the import was cancelled');
    };

    emit('parse', 0, 1);
    await checkCancel();
    const handle = await importedFor(s, settings);
    await checkCancel();
    const summary = handle.summary() as ImportSummary;
    emit('parse', 1, 1);

    // Diff stage: only when the target is an existing dataset.
    let dataset: datasets.Dataset | null = null;
    let diffSummary: DiffSummary | null = null;
    if ('datasetId' in target) {
      dataset = await datasets.get(store, target.datasetId);
      if (dataset === null) {
        throw rpc('not_found', `dataset ${target.datasetId} does not exist`);
      }
      emit('diff', 0, 1);
      const prev = await loadEntry(store, dataset.id, dataset.currentVersion);
      const imp = await importer();
      diff = imp.diffAgainst(prev.bytes, handle);
      diffSummary = diff.summary() as DiffSummary;
      emit('diff', 1, 1);
      if (diffSummary.unchanged) {
        return { datasetId: dataset.id, version: null };
      }
      const sc = diffSummary.schema;
      const hasSchemaChange =
        sc.added_columns.length > 0 || sc.removed_columns.length > 0 || sc.type_changes.length > 0;
      if (hasSchemaChange && !acceptSchemaChanges) {
        throw rpc(
          'schema_change_unaccepted',
          'the file changes the dataset schema; it must be accepted explicitly',
        );
      }
    }

    await checkCancel();
    const now = new Date().toISOString();
    const total = summary.rowCount;
    const changedIds = new Set<string>();
    if (diff !== null) {
      for (let start = 0; ; start += BATCH) {
        const page = diff.changedBatch(start, BATCH) as FieldChange[];
        for (const fc of page) changedIds.add(fc.record_id);
        if (page.length < BATCH) break;
      }
    }

    const result = await store.transaction(async (tx) => {
      const deviceId = await meta.deviceId(tx);
      const importId = crypto.randomUUID();
      let datasetId: string;
      let version: number;
      if ('datasetId' in target) {
        datasetId = target.datasetId;
        version = dataset!.currentVersion + 1;
      } else {
        const created = await datasets.create(
          tx,
          {
            projectId: target.projectId,
            name: target.datasetName,
            key: JSON.stringify(summary.settings.key),
            settings: '{}',
          },
          now,
        );
        datasetId = created.id;
        version = 1;
      }
      // Inserted first so every change entry can carry its import_id.
      await imports.insert(tx, {
        id: importId,
        datasetId,
        version,
        settings: JSON.stringify(summary.settings),
        coercions: JSON.stringify(summary.coercions),
        encoding: summary.detection.encoding,
        delimiter: summary.detection.delimiter,
        sheet: summary.settings.sheet,
        now,
      });

      // Records: first import inserts every row; re-import rewrites the
      // table in the new file order so `records.allJson` stays in the
      // canonical order snapshot rebuilds hash over (§4.3).
      const carried = diff === null ? null : await records.idVersions(tx, datasetId);
      if (diff !== null) await records.clearAll(tx, datasetId);
      let done = 0;
      for (let start = 0; start < total; start += BATCH) {
        await checkCancel();
        const page = handle.recordBatch(start, BATCH) as RecordBatch;
        const byVersion = new Map<number, { id: string; json: string }[]>();
        for (let i = 0; i < page.ids.length; i++) {
          const id = page.ids[i]!;
          const rowVersion =
            carried === null || changedIds.has(id) ? version : (carried.get(id) ?? version);
          const group = byVersion.get(rowVersion);
          const row = { id, json: page.json[i]! };
          if (group === undefined) byVersion.set(rowVersion, [row]);
          else group.push(row);
        }
        for (const [v, rows] of byVersion) {
          await records.upsertBatch(tx, datasetId, v, rows);
        }
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
        emit('write', done, total);
      }

      if (diff !== null && diffSummary !== null) {
        // Re-import entries (§4.2): insert per added record, one update
        // per changed field with base_version, delete per missing record.
        const prevVersion = version - 1;
        await appendPaged(
          tx,
          datasetId,
          importId,
          deviceId,
          diffSummary.added,
          (start, len) => {
            const page = diff!.addedBatch(start, len) as RecordBatch;
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
          diffSummary.changed,
          (start, len) => diff!.changedBatch(start, len) as FieldChange[],
          (fc) => ({
            changeId: crypto.randomUUID(),
            recordId: fc.record_id,
            kind: 'update' as const,
            field: fc.field,
            value: fc.new === null ? null : JSON.stringify(fc.new),
            baseVersion: prevVersion,
          }),
          checkCancel,
        );
        const removedIds = diff.removed() as string[];
        await appendPaged(
          tx,
          datasetId,
          importId,
          deviceId,
          removedIds.length,
          (start, len) => removedIds.slice(start, start + len),
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
      await versions.insert(tx, {
        datasetId,
        version,
        sourceName: s.name,
        sourceSha256: summary.sourceSha256,
        rowCount: total,
        contentHash: handle.contentHash(),
        snapshot: handle.toSnapshot(datasetId, version),
      });
      await datasets.saveSettings(
        tx,
        datasetId,
        serializeSettings(summary.settings, summary.schema),
      );
      await datasets.setCurrentVersion(tx, datasetId, version);
      emit('snapshot', 1, 1);
      return { datasetId, version };
    });
    return result;
  } finally {
    diff?.free();
    destroy(sessionId);
  }
}

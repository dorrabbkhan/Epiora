// The CSV version-creating commit (v0.3 spec §4.2, §5.3, §6.4). Every
// commit is ONE transaction; the canonical records/changes/imports
// writes themselves live in `commit-version.ts` so the legacy import
// uses the same path instead of a snapshot-only copy.
//
// Sessions live in importer.ts; this module only borrows them.

import type { DiffHandle } from '@import/epiora_import_wasm';
import type { DiffSummary } from '@import-types/DiffSummary';
import type { FieldChange } from '@import-types/FieldChange';
import type { ImportSettings } from '@import-types/ImportSettings';
import type { ImportSummary } from '@import-types/ImportSummary';
import type { RecordBatch } from '@import-types/RecordBatch';
import type { CommitResult, ImportProgress } from './api';
import { writeCommittedVersion } from './commit-version';
import { importer } from './engines';
import { rpc } from './errors';
import { destroySession, getSession, importedFor, throwWhenCancelled } from './importer';
import { loadEntry, serializeSettings } from './snapshots';
import * as datasets from '../store/repo/datasets';
import type { Store } from '../store/types';

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
    const checkCancel = () => throwWhenCancelled(() => s.cancelled);

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
    const result = await store.transaction(async (tx) => {
      await checkCancel();
      let datasetId: string;
      let version: number;
      if ('datasetId' in target) {
        datasetId = target.datasetId;
        version = dataset!.currentVersion + 1;
      } else {
        await checkCancel();
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
      const removedIds = diff === null ? [] : (diff.removed() as string[]);
      await writeCommittedVersion({
        tx,
        datasetId,
        version,
        now,
        sourceName: s.name,
        sourceSha256: summary.sourceSha256,
        importSettings: JSON.stringify(summary.settings),
        coercions: JSON.stringify(summary.coercions),
        encoding: summary.detection.encoding,
        delimiter: summary.detection.delimiter,
        sheet: summary.settings.sheet,
        serializedSettings: serializeSettings(summary.settings, summary.schema),
        rows: {
          rowCount: summary.rowCount,
          recordBatch: (start, len) => handle.recordBatch(start, len) as RecordBatch,
        },
        diff:
          diff === null || diffSummary === null
            ? null
            : {
                addedCount: diffSummary.added,
                changedCount: diffSummary.changed,
                removedIds,
                addedBatch: (start, len) => diff!.addedBatch(start, len) as RecordBatch,
                changedBatch: (start, len) => diff!.changedBatch(start, len) as FieldChange[],
              },
        contentHash: handle.contentHash(),
        snapshot: () => handle.toSnapshot(datasetId, version),
        checkCancel,
        emit,
      });
      return { datasetId, version };
    });
    return result;
  } finally {
    diff?.free();
    destroySession(sessionId);
  }
}

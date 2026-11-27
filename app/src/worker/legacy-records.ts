// Bridge between mapped legacy snapshots and the canonical v0.3 write
// path (v1.1 Task 15 review): decode each verified snapshot once with
// epiora-import-wasm, page canonical record ids/JSON out of it, persist
// its schema/meta for rebuilds, and delegate every store write to
// `writeCommittedVersion` — the same function the CSV commit uses.

import type { ColumnMeta } from '@engine-types/ColumnMeta';
import type { LabelSet } from '@engine-types/LabelSet';
import type { ValueType } from '@import-types/ValueType';
import type { RecordBatch } from '@import-types/RecordBatch';
import type { ImportProgress } from './api';
import { writeCommittedVersion, type CanonicalDiffSource } from './commit-version';
import { importer } from './engines';
import { serializeSettings } from './snapshots';
import type { RefreshDiff } from './legacy-shared';
import type { Tx } from '../store/types';

interface SnapshotSchema {
  schema: [string, ValueType][];
  meta: ColumnMeta[];
  label_sets: Record<string, LabelSet>;
}

export interface LegacyVersionWrite {
  tx: Tx;
  datasetId: string;
  version: number;
  now: string;
  sourceName: string;
  sourceSha256: string;
  snapshot: Uint8Array;
  rowCount: number;
  contentHash: string;
  keyColumn: string;
  diff: RefreshDiff | null;
  checkCancel: () => Promise<void>;
  emit?: (stage: ImportProgress['stage'], done: number, total: number) => void;
}

function legacySettings(keyColumn: string): unknown {
  return {
    source: 'legacy',
    key: keyColumn === '' ? 'row_position' : { column: { name: keyColumn } },
  };
}

function refreshDiffSource(
  diff: RefreshDiff,
  rows: { recordRows(rows: number[]): RecordBatch },
): CanonicalDiffSource {
  return {
    addedCount: diff.added.length,
    changedCount: diff.changed.length,
    removedIds: diff.removed,
    addedBatch: (start, len) => rows.recordRows(diff.added.slice(start, start + len)),
    changedBatch: (start, len) => diff.changed.slice(start, start + len),
  };
}

/** Writes one legacy dataset version through the v0.3 canonical write
 *  path. The caller owns both the datasets row and the transaction. */
export async function writeLegacyVersion(input: LegacyVersionWrite): Promise<void> {
  const imp = await importer();
  const rows = imp.snapshotRecords(input.snapshot, input.keyColumn);
  try {
    const stored = rows.schema() as SnapshotSchema;
    const settings = legacySettings(input.keyColumn);
    const snapshotMeta =
      stored.meta.length === 0
        ? undefined
        : { meta: stored.meta, labelSets: stored.label_sets };
    await writeCommittedVersion({
      tx: input.tx,
      datasetId: input.datasetId,
      version: input.version,
      now: input.now,
      sourceName: input.sourceName,
      sourceSha256: input.sourceSha256,
      importSettings: JSON.stringify(settings),
      coercions: '{}',
      serializedSettings: serializeSettings(settings, stored.schema, snapshotMeta),
      rows: {
        rowCount: input.rowCount,
        recordBatch: (start, len) => rows.recordBatch(start, len) as RecordBatch,
      },
      diff: input.diff === null ? null : refreshDiffSource(input.diff, rows),
      contentHash: input.contentHash,
      snapshot: () => input.snapshot,
      checkCancel: input.checkCancel,
      ...(input.emit === undefined ? {} : { emit: input.emit }),
    });
  } finally {
    rows.free();
  }
}

// changes repository (spec §4.1, §4.2): the append-only change log.
// `seq` is AUTOINCREMENT so append order is the storage order.

import type { Tx } from '../types';

export interface ChangeEntry {
  changeId: string;
  datasetId: string;
  recordId: string;
  kind: 'insert' | 'update' | 'delete';
  field: string | null;
  value: string | null;
  baseVersion: number | null;
  author: 'local';
  deviceId: string;
  lamport: number;
  importId: string | null;
}

export async function appendBatch(tx: Tx, entries: ChangeEntry[]): Promise<void> {
  for (const e of entries) {
    await tx.exec(
      'INSERT INTO changes ' +
        '(change_id, dataset_id, record_id, kind, field, value, base_version, author, device_id, lamport, import_id) ' +
        'VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)',
      [
        e.changeId,
        e.datasetId,
        e.recordId,
        e.kind,
        e.field,
        e.value,
        e.baseVersion,
        e.author,
        e.deviceId,
        e.lamport,
        e.importId,
      ],
    );
  }
}

export async function countByDataset(
  tx: Tx,
  datasetId: string,
  kind?: ChangeEntry['kind'],
): Promise<number> {
  const rows = await tx.query(
    `SELECT COUNT(*) AS n FROM changes WHERE dataset_id = ?${kind === undefined ? '' : ' AND kind = ?'}`,
    kind === undefined ? [datasetId] : [datasetId, kind],
  );
  return Number(rows[0]!.n);
}

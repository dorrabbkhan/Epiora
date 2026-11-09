// records repository (spec §4.1): the dataset's current state, one row
// per record holding canonical JSON. Ordered by rowid (insertion
// order) for paging and iteration.

import type { Tx } from '../types';

const PAGE = 10_000;

export async function upsertBatch(
  tx: Tx,
  datasetId: string,
  version: number,
  rows: { id: string; json: string }[],
): Promise<void> {
  for (const r of rows) {
    await tx.exec(
      'INSERT INTO records (dataset_id, record_id, record_values, version) VALUES (?, ?, ?, ?) ' +
        'ON CONFLICT(dataset_id, record_id) DO UPDATE SET ' +
        'record_values = excluded.record_values, version = excluded.version',
      [datasetId, r.id, r.json, version],
    );
  }
}

/** Removes every record of the dataset (re-import rewrite, spec §4.3). */
export async function clearAll(tx: Tx, datasetId: string): Promise<void> {
  await tx.exec('DELETE FROM records WHERE dataset_id = ?', [datasetId]);
}

/** `record_id → version last changed` for every current record. */
export async function idVersions(tx: Tx, datasetId: string): Promise<Map<string, number>> {
  const rows = await tx.query('SELECT record_id, version FROM records WHERE dataset_id = ?', [
    datasetId,
  ]);
  return new Map(rows.map((r) => [String(r.record_id), Number(r.version)]));
}

export async function deleteBatch(tx: Tx, datasetId: string, ids: string[]): Promise<void> {
  if (ids.length === 0) return;
  const marks = ids.map(() => '?').join(', ');
  await tx.exec(`DELETE FROM records WHERE dataset_id = ? AND record_id IN (${marks})`, [
    datasetId,
    ...ids,
  ]);
}

export async function pageJson(
  tx: Tx,
  datasetId: string,
  offset: number,
  limit: number,
): Promise<string[]> {
  const rows = await tx.query(
    'SELECT record_values FROM records WHERE dataset_id = ? ORDER BY rowid LIMIT ? OFFSET ?',
    [datasetId, limit, offset],
  );
  return rows.map((r) => String(r.record_values));
}

/** Streams all record JSON in batches of 10,000, ordered by rowid. */
export async function* allJson(tx: Tx, datasetId: string): AsyncIterable<string[]> {
  let offset = 0;
  for (;;) {
    const batch = await pageJson(tx, datasetId, offset, PAGE);
    if (batch.length === 0) return;
    yield batch;
    if (batch.length < PAGE) return;
    offset += batch.length;
  }
}

// dataset_versions repository (spec §4.1, §4.3): one row per dataset
// version. The snapshot blob is a derived cache; snapshot_available = 0
// marks versions whose snapshot must not be trusted.

import type { Row, Tx } from '../types';

export interface DatasetVersion {
  datasetId: string;
  version: number;
  sourceName: string;
  sourceSha256: string;
  rowCount: number;
  contentHash: string;
  snapshotAvailable: boolean;
}

function toVersion(r: Row): DatasetVersion {
  return {
    datasetId: String(r.dataset_id),
    version: Number(r.version),
    sourceName: String(r.source_name),
    sourceSha256: String(r.source_sha256),
    rowCount: Number(r.row_count),
    contentHash: String(r.content_hash),
    snapshotAvailable: Number(r.snapshot_available) !== 0,
  };
}

const COLUMNS =
  'dataset_id, version, source_name, source_sha256, row_count, content_hash, snapshot_available';

export async function list(tx: Tx, datasetId: string): Promise<DatasetVersion[]> {
  const rows = await tx.query(
    `SELECT ${COLUMNS} FROM dataset_versions WHERE dataset_id = ? ORDER BY version`,
    [datasetId],
  );
  return rows.map(toVersion);
}

export async function get(
  tx: Tx,
  datasetId: string,
  version: number,
): Promise<DatasetVersion | null> {
  const rows = await tx.query(
    `SELECT ${COLUMNS} FROM dataset_versions WHERE dataset_id = ? AND version = ?`,
    [datasetId, version],
  );
  return rows.length > 0 ? toVersion(rows[0]!) : null;
}

export async function insert(
  tx: Tx,
  v: {
    datasetId: string;
    version: number;
    sourceName: string;
    sourceSha256: string;
    rowCount: number;
    contentHash: string;
    snapshot: Uint8Array;
  },
): Promise<DatasetVersion> {
  await tx.exec(
    'INSERT INTO dataset_versions ' +
      '(dataset_id, version, source_name, source_sha256, row_count, content_hash, snapshot, snapshot_available) ' +
      'VALUES (?, ?, ?, ?, ?, ?, ?, 1)',
    [v.datasetId, v.version, v.sourceName, v.sourceSha256, v.rowCount, v.contentHash, v.snapshot],
  );
  return {
    datasetId: v.datasetId,
    version: v.version,
    sourceName: v.sourceName,
    sourceSha256: v.sourceSha256,
    rowCount: v.rowCount,
    contentHash: v.contentHash,
    snapshotAvailable: true,
  };
}

/** The stored snapshot blob, or null when absent or unavailable. */
export async function snapshot(
  tx: Tx,
  datasetId: string,
  version: number,
): Promise<Uint8Array | null> {
  const rows = await tx.query(
    'SELECT snapshot FROM dataset_versions WHERE dataset_id = ? AND version = ? AND snapshot_available != 0',
    [datasetId, version],
  );
  const s = rows[0]?.snapshot;
  return s instanceof Uint8Array ? s : null;
}

export async function replaceSnapshot(
  tx: Tx,
  datasetId: string,
  version: number,
  snapshot: Uint8Array,
): Promise<void> {
  await tx.exec(
    'UPDATE dataset_versions SET snapshot = ?, snapshot_available = 1 WHERE dataset_id = ? AND version = ?',
    [snapshot, datasetId, version],
  );
}

/** Marks every version before `version` as having no trusted snapshot (§4.3). */
export async function markUnavailableBefore(
  tx: Tx,
  datasetId: string,
  version: number,
): Promise<void> {
  await tx.exec(
    'UPDATE dataset_versions SET snapshot_available = 0 WHERE dataset_id = ? AND version < ?',
    [datasetId, version],
  );
}

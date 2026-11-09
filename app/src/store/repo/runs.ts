// runs repository (spec §4.1): the output log. Entries carry result and
// receipt JSON plus the receipt hash; deletion is a tombstone
// (deleted_at), never a row removal.

import type { Row, Tx } from '../types';

export interface Run {
  id: string;
  projectId: string;
  procedure: string;
  options: string;
  datasetId: string;
  datasetVersion: number;
  result: string;
  receipt: string;
  receiptHash: string;
  createdAt: string;
  deletedAt: string | null;
}

function toRun(r: Row): Run {
  return {
    id: String(r.id),
    projectId: String(r.project_id),
    procedure: String(r.procedure),
    options: String(r.options),
    datasetId: String(r.dataset_id),
    datasetVersion: Number(r.dataset_version),
    result: String(r.result),
    receipt: String(r.receipt),
    receiptHash: String(r.receipt_hash),
    createdAt: String(r.created_at),
    deletedAt: r.deleted_at === null ? null : String(r.deleted_at),
  };
}

const COLUMNS =
  'id, project_id, procedure, options, dataset_id, dataset_version, result, receipt, receipt_hash, created_at, deleted_at';

export async function insert(
  tx: Tx,
  r: {
    projectId: string;
    procedure: string;
    options: string;
    datasetId: string;
    datasetVersion: number;
    result: string;
    receipt: string;
    receiptHash: string;
    now: string;
  },
): Promise<Run> {
  const id = crypto.randomUUID();
  await tx.exec(
    'INSERT INTO runs (id, project_id, procedure, options, dataset_id, dataset_version, result, receipt, receipt_hash, created_at) ' +
      'VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)',
    [
      id,
      r.projectId,
      r.procedure,
      r.options,
      r.datasetId,
      r.datasetVersion,
      r.result,
      r.receipt,
      r.receiptHash,
      r.now,
    ],
  );
  return {
    id,
    projectId: r.projectId,
    procedure: r.procedure,
    options: r.options,
    datasetId: r.datasetId,
    datasetVersion: r.datasetVersion,
    result: r.result,
    receipt: r.receipt,
    receiptHash: r.receiptHash,
    createdAt: r.now,
    deletedAt: null,
  };
}

/** One run by id, including a deleted one, or null. */
export async function get(tx: Tx, id: string): Promise<Run | null> {
  const rows = await tx.query(`SELECT ${COLUMNS} FROM runs WHERE id = ?`, [id]);
  return rows.length > 0 ? toRun(rows[0]!) : null;
}

/** All runs for a project except deleted ones, newest first. */
export async function listByProject(tx: Tx, projectId: string): Promise<Run[]> {
  const rows = await tx.query(
    `SELECT ${COLUMNS} FROM runs WHERE project_id = ? AND deleted_at IS NULL ` +
      'ORDER BY created_at DESC, id',
    [projectId],
  );
  return rows.map(toRun);
}

export async function markDeleted(tx: Tx, id: string, now: string): Promise<void> {
  await tx.exec('UPDATE runs SET deleted_at = ? WHERE id = ?', [now, id]);
}

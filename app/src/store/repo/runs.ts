// runs repository (spec §4.1): the output log. Entries carry result and
// receipt JSON plus the receipt hash; deletion is a tombstone
// (deleted_at), never a row removal. `command` (schema v2) is the
// Epi Info command shown on the card — NULL on rows written before the
// migration; the worker generates it on load and never writes it back.
// `notes` (schema v3, v1.0 Task 23, spec §6.1) is the run's assumption
// notes JSON — NULL on rows written before the notes side channel.

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
  /** The stored Epi Info command, or null on pre-v2 rows. */
  command: string | null;
  /** The stored assumption notes JSON, or null on pre-v3 rows. */
  notes: string | null;
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
    command: r.command === null ? null : String(r.command),
    notes: r.notes === null ? null : String(r.notes),
    createdAt: String(r.created_at),
    deletedAt: r.deleted_at === null ? null : String(r.deleted_at),
  };
}

const COLUMNS =
  'id, project_id, procedure, options, dataset_id, dataset_version, result, receipt, receipt_hash, command, notes, created_at, deleted_at';

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
    command: string;
    notes: string;
    now: string;
  },
): Promise<Run> {
  const id = crypto.randomUUID();
  await tx.exec(
    'INSERT INTO runs (id, project_id, procedure, options, dataset_id, dataset_version, result, receipt, receipt_hash, command, notes, created_at) ' +
      'VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)',
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
      r.command,
      r.notes,
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
    command: r.command,
    notes: r.notes,
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

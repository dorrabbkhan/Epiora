// Session repositories (schema v5, v1.1 plan Task 11, spec §5.2): the
// two tables an Analysis session writes. `sessions` keeps one row per
// cleaning session — which dataset/version its recipe replays from,
// the canonical recipe text and the as-of instant; `permanent_vars`
// the project's PERMANENT-scope DEFINEs a session's recipe opens with
// (spec §6.1). Every row belongs to a project and cascades with it.

import type { Row, Tx } from '../types';

/** One row of `sessions` — a saved cleaning session's replay source
 *  and recipe. */
export interface Session {
  id: string;
  projectId: string;
  /** Dataset name (or id) the recipe replays from. */
  sourceDataset: string;
  /** The dataset version the recipe was recorded against. */
  sourceVersion: number;
  /** `Recipe::canonical_text` — every command since READ. */
  recipeText: string;
  /** The session's captured as-of instant (RFC 3339). */
  asOf: string;
  /** Last write timestamp. */
  updatedAt: string;
}

/** One row of `permanent_vars` — a PERMANENT-scope DEFINE: its type
 *  word and, for `DEFINE x = e`, the expression text. */
export interface PermanentVar {
  projectId: string;
  name: string;
  type: string;
  exprText: string | null;
}

/** Inserts or replaces a session row (`id` is the PK). */
export async function upsertSession(tx: Tx, row: Session): Promise<void> {
  await tx.exec(
    `INSERT OR REPLACE INTO sessions
       (id, project_id, source_dataset, source_version, recipe_text, as_of, updated_at)
     VALUES (?, ?, ?, ?, ?, ?, ?)`,
    [
      row.id,
      row.projectId,
      row.sourceDataset,
      row.sourceVersion,
      row.recipeText,
      row.asOf,
      row.updatedAt,
    ],
  );
}

/** One session by id, or null. */
export async function session(tx: Tx, id: string): Promise<Session | null> {
  const rows = await tx.query(
    `SELECT id, project_id, source_dataset, source_version, recipe_text, as_of, updated_at
       FROM sessions WHERE id = ?`,
    [id],
  );
  const r = rows[0];
  if (r === undefined) return null;
  return toSession(r);
}

/** The project's sessions, most recently updated first. */
export async function sessions(tx: Tx, projectId: string): Promise<Session[]> {
  const rows = await tx.query(
    `SELECT id, project_id, source_dataset, source_version, recipe_text, as_of, updated_at
       FROM sessions WHERE project_id = ? ORDER BY updated_at DESC, id`,
    [projectId],
  );
  return rows.map(toSession);
}

/** Deletes a session row. */
export async function removeSession(tx: Tx, id: string): Promise<void> {
  await tx.exec('DELETE FROM sessions WHERE id = ?', [id]);
}

function toSession(r: Row): Session {
  return {
    id: String(r.id),
    projectId: String(r.project_id),
    sourceDataset: String(r.source_dataset),
    sourceVersion: Number(r.source_version),
    recipeText: String(r.recipe_text),
    asOf: String(r.as_of),
    updatedAt: String(r.updated_at),
  };
}

/** Inserts or replaces a project's permanent variable. */
export async function putPermanentVar(tx: Tx, row: PermanentVar): Promise<void> {
  await tx.exec(
    `INSERT OR REPLACE INTO permanent_vars (project_id, name, type, expr_text)
     VALUES (?, ?, ?, ?)`,
    [row.projectId, row.name, row.type, row.exprText],
  );
}

/** The project's permanent variables, in name order. */
export async function permanentVars(tx: Tx, projectId: string): Promise<PermanentVar[]> {
  const rows = await tx.query(
    `SELECT project_id, name, type, expr_text
       FROM permanent_vars WHERE project_id = ? ORDER BY name`,
    [projectId],
  );
  return rows.map((r) => ({
    projectId: String(r.project_id),
    name: String(r.name),
    type: String(r.type),
    exprText: r.expr_text === null ? null : String(r.expr_text),
  }));
}

/** Deletes one permanent variable. */
export async function removePermanentVar(
  tx: Tx,
  projectId: string,
  name: string,
): Promise<void> {
  await tx.exec('DELETE FROM permanent_vars WHERE project_id = ? AND name = ?', [
    projectId,
    name,
  ]);
}

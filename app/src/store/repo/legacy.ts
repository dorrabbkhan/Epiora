// legacy repositories (schema v4, v1.1 plan Task 6, spec §4.2): the
// five tables an imported Epi Info project writes. `legacy_projects`
// marks the project so a re-import finds it; `dataset_links` holds
// the Relate/grid relations (`FKEY` → `GlobalRecordId`); `programs`
// keeps saved `.pgm` documents; `check_code` the per-view check code
// text v1.1 stores read-only; `migration_reports` one JSON report per
// import. Every row belongs to a project and cascades with it.

import type { Row, Tx } from '../types';

/** One row of `legacy_projects` — at most one per project (PK). */
export interface LegacyProject {
  projectId: string;
  legacyId: string;
  /** `ei7_access` | `ei7_sqlite` | `ei7_sql_server` | `ei3_access`. */
  sourceFormat: string;
  fileName: string;
  importedAt: string;
}

/** One row of `dataset_links`: the child's `childKey` column points at
 *  the parent's `parentKey` column. */
export interface DatasetLink {
  projectId: string;
  parentDataset: string;
  childDataset: string;
  childKey: string;
  parentKey: string;
  kind: 'relate' | 'grid';
}

/** One row of `programs` — a saved `.pgm` document. */
export interface Program {
  id: string;
  projectId: string;
  name: string;
  text: string;
  source: string;
}

/** One row of `check_code` — at most one per (project, dataset). */
export interface CheckCode {
  projectId: string;
  datasetId: string;
  text: string;
  blocks: number;
}

/** One row of `migration_reports`; `version` increments per import. */
export interface MigrationReport {
  projectId: string;
  version: number;
  json: string;
}

/** Inserts or replaces the project's legacy identity. */
export async function upsertProject(tx: Tx, row: LegacyProject): Promise<void> {
  await tx.exec(
    `INSERT OR REPLACE INTO legacy_projects
       (project_id, legacy_id, source_format, file_name, imported_at)
     VALUES (?, ?, ?, ?, ?)`,
    [row.projectId, row.legacyId, row.sourceFormat, row.fileName, row.importedAt],
  );
}

/** The project's legacy identity, or null for a non-legacy project. */
export async function project(tx: Tx, projectId: string): Promise<LegacyProject | null> {
  const rows = await tx.query(
    `SELECT project_id, legacy_id, source_format, file_name, imported_at
       FROM legacy_projects WHERE project_id = ?`,
    [projectId],
  );
  const r = rows[0];
  if (r === undefined) return null;
  return {
    projectId: String(r.project_id),
    legacyId: String(r.legacy_id),
    sourceFormat: String(r.source_format),
    fileName: String(r.file_name),
    importedAt: String(r.imported_at),
  };
}

/** Records one parent/child link. */
export async function insertLink(tx: Tx, row: DatasetLink): Promise<void> {
  await tx.exec(
    `INSERT INTO dataset_links
       (project_id, parent_dataset, child_dataset, child_key, parent_key, kind)
     VALUES (?, ?, ?, ?, ?, ?)`,
    [row.projectId, row.parentDataset, row.childDataset, row.childKey, row.parentKey, row.kind],
  );
}

/** The project's links, in write order (rowid). */
export async function links(tx: Tx, projectId: string): Promise<DatasetLink[]> {
  const rows = await tx.query(
    `SELECT project_id, parent_dataset, child_dataset, child_key, parent_key, kind
       FROM dataset_links WHERE project_id = ? ORDER BY rowid`,
    [projectId],
  );
  return rows.map(toLink);
}

function toLink(r: Row): DatasetLink {
  return {
    projectId: String(r.project_id),
    parentDataset: String(r.parent_dataset),
    childDataset: String(r.child_dataset),
    childKey: String(r.child_key),
    parentKey: String(r.parent_key),
    kind: String(r.kind) === 'grid' ? 'grid' : 'relate',
  };
}

/** Stores one saved program document. */
export async function insertProgram(tx: Tx, row: Program): Promise<void> {
  await tx.exec(
    'INSERT INTO programs (id, project_id, name, text, source) VALUES (?, ?, ?, ?, ?)',
    [row.id, row.projectId, row.name, row.text, row.source],
  );
}

/** The project's programs, in write order (rowid). */
export async function programs(tx: Tx, projectId: string): Promise<Program[]> {
  const rows = await tx.query(
    'SELECT id, project_id, name, text, source FROM programs WHERE project_id = ? ORDER BY rowid',
    [projectId],
  );
  return rows.map((r) => ({
    id: String(r.id),
    projectId: String(r.project_id),
    name: String(r.name),
    text: String(r.text),
    source: String(r.source),
  }));
}

/** Inserts or replaces one dataset's check code text. */
export async function upsertCheckCode(tx: Tx, row: CheckCode): Promise<void> {
  await tx.exec(
    `INSERT OR REPLACE INTO check_code (project_id, dataset_id, text, blocks)
     VALUES (?, ?, ?, ?)`,
    [row.projectId, row.datasetId, row.text, row.blocks],
  );
}

/** One dataset's check code, or null when the view had none. */
export async function checkCode(
  tx: Tx,
  projectId: string,
  datasetId: string,
): Promise<CheckCode | null> {
  const rows = await tx.query(
    'SELECT project_id, dataset_id, text, blocks FROM check_code WHERE project_id = ? AND dataset_id = ?',
    [projectId, datasetId],
  );
  const r = rows[0];
  if (r === undefined) return null;
  return {
    projectId: String(r.project_id),
    datasetId: String(r.dataset_id),
    text: String(r.text),
    blocks: Number(r.blocks),
  };
}

/** Appends one import's report; `version` increments per import. */
export async function insertMigrationReport(tx: Tx, row: MigrationReport): Promise<void> {
  await tx.exec(
    'INSERT INTO migration_reports (project_id, version, json) VALUES (?, ?, ?)',
    [row.projectId, row.version, row.json],
  );
}

/** The latest report for the project, or null before any import. */
export async function latestMigrationReport(
  tx: Tx,
  projectId: string,
): Promise<MigrationReport | null> {
  const rows = await tx.query(
    `SELECT project_id, version, json FROM migration_reports
       WHERE project_id = ? ORDER BY version DESC LIMIT 1`,
    [projectId],
  );
  const r = rows[0];
  if (r === undefined) return null;
  return {
    projectId: String(r.project_id),
    version: Number(r.version),
    json: String(r.json),
  };
}

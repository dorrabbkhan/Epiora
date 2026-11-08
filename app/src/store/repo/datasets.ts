// datasets repository (spec §4.1). `key` is the record-key choice
// (a key column or row position) and `settings` the saved import
// settings, both stored as text exactly as the caller serializes them.

import type { Row, Tx } from '../types';

export interface Dataset {
  id: string;
  projectId: string;
  name: string;
  key: string;
  settings: string;
  currentVersion: number;
  createdAt: string;
}

function toDataset(r: Row): Dataset {
  return {
    id: String(r.id),
    projectId: String(r.project_id),
    name: String(r.name),
    key: String(r.record_key),
    settings: String(r.settings),
    currentVersion: Number(r.current_version),
    createdAt: String(r.created_at),
  };
}

const COLUMNS =
  'id, project_id, name, record_key, settings, current_version, created_at';

export async function listByProject(tx: Tx, projectId: string): Promise<Dataset[]> {
  const rows = await tx.query(`SELECT ${COLUMNS} FROM datasets WHERE project_id = ? ORDER BY name, id`, [
    projectId,
  ]);
  return rows.map(toDataset);
}

export async function get(tx: Tx, id: string): Promise<Dataset | null> {
  const rows = await tx.query(`SELECT ${COLUMNS} FROM datasets WHERE id = ?`, [id]);
  return rows.length > 0 ? toDataset(rows[0]!) : null;
}

export async function create(
  tx: Tx,
  d: { projectId: string; name: string; key: string; settings: string },
  now: string,
): Promise<Dataset> {
  const id = crypto.randomUUID();
  await tx.exec(
    `INSERT INTO datasets (${COLUMNS}) VALUES (?, ?, ?, ?, ?, 0, ?)`,
    [id, d.projectId, d.name, d.key, d.settings, now],
  );
  return { id, ...d, currentVersion: 0, createdAt: now };
}

export async function setCurrentVersion(tx: Tx, id: string, version: number): Promise<void> {
  await tx.exec('UPDATE datasets SET current_version = ? WHERE id = ?', [version, id]);
}

export async function saveSettings(tx: Tx, id: string, settings: string): Promise<void> {
  await tx.exec('UPDATE datasets SET settings = ? WHERE id = ?', [settings, id]);
}

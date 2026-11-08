// projects repository (spec §4.1). All functions take a `Tx` so callers
// decide the transaction boundary; ids are crypto.randomUUID(); times
// are ISO strings passed in by the caller.

import type { Row, Tx } from '../types';

export interface Project {
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
}

function toProject(r: Row): Project {
  return {
    id: String(r.id),
    name: String(r.name),
    createdAt: String(r.created_at),
    updatedAt: String(r.updated_at),
  };
}

export async function list(tx: Tx): Promise<Project[]> {
  const rows = await tx.query(
    'SELECT id, name, created_at, updated_at FROM projects ORDER BY created_at, id',
  );
  return rows.map(toProject);
}

export async function create(tx: Tx, name: string, now: string): Promise<Project> {
  const id = crypto.randomUUID();
  await tx.exec(
    'INSERT INTO projects (id, name, created_at, updated_at) VALUES (?, ?, ?, ?)',
    [id, name, now, now],
  );
  return { id, name, createdAt: now, updatedAt: now };
}

export async function rename(tx: Tx, id: string, name: string, now: string): Promise<void> {
  await tx.exec('UPDATE projects SET name = ?, updated_at = ? WHERE id = ?', [name, now, id]);
}

/** Deletes the project; child rows cascade (spec §4.1 FK graph). */
export async function remove(tx: Tx, id: string): Promise<void> {
  await tx.exec('DELETE FROM projects WHERE id = ?', [id]);
}

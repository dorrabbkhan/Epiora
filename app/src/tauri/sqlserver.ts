// Desktop-only SQL Server import bridge (v1.1 Task 9, spec §4.1):
// the two `legacy_sqlserver_*` Tauri commands. `SqlServerConn` is
// passed per call and never stored — the Rust side keeps it in
// memory for one import only (spec §4.5). Errors cross as bare code
// strings (`sqlserver_connect` | `sqlserver_auth` | `sqlserver_query`)
// with no connection detail (spec §10).

import { invoke } from '@tauri-apps/api/core';

export type SqlServerAuth = { sql: { user: string; password: string } } | 'windows';

export interface SqlServerConn {
  host: string;
  port: number;
  database: string;
  auth: SqlServerAuth;
  trustCert: boolean;
}

/** True when running inside the desktop shell (the SQL Server import
 *  UI is desktop-only, spec §4.1). Same marker as `inTauri` in
 *  `worker/client.ts`. */
export function isDesktop(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/** List the project's tables (creation order), for the preview and
 *  the per-table `legacySqlServerBatches` loop. */
export async function legacySqlServerTables(conn: SqlServerConn): Promise<string[]> {
  return invoke<string[]>('legacy_sqlserver_tables', { conn });
}

/** One table as EPTB batch chunks (`chunkRows` rows each), pushed
 *  into the worker's `BatchSource` exactly like the mdb batches. */
export async function legacySqlServerBatches(
  conn: SqlServerConn,
  table: string,
  chunkRows: number,
): Promise<Uint8Array[]> {
  const rows = await invoke<number[][]>('legacy_sqlserver_batches', {
    conn,
    table,
    chunkRows,
  });
  return rows.map((r) => new Uint8Array(r));
}

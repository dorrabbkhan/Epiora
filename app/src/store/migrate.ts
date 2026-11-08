// Forward-only migrations (v0.3 spec §4.1): pending migrations are
// applied in one transaction; the applied version lives in
// meta.schema_version. A first run also mints the per-install
// device_id and starts the Lamport clock at 0.

import { StoreError, type Store } from './types';

export interface Migration {
  version: number;
  sql: string;
}

export async function migrate(store: Store, migrations: Migration[]): Promise<void> {
  const sorted = [...migrations].sort((a, b) => a.version - b.version);
  try {
    await store.transaction(async (tx) => {
      const hasMeta = await tx.query(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'meta'",
      );
      let current = 0;
      if (hasMeta.length > 0) {
        const rows = await tx.query("SELECT value FROM meta WHERE key = 'schema_version'");
        if (rows.length > 0) current = Number(rows[0]!.value);
      }
      const latest = sorted.length > 0 ? sorted[sorted.length - 1]!.version : 0;
      if (current > latest) {
        throw new StoreError(
          'migration_failed',
          `store schema_version ${current} is newer than the latest known migration ${latest}`,
        );
      }
      for (const m of sorted) {
        if (m.version > current) {
          await tx.exec(m.sql);
          current = m.version;
        }
      }
      await tx.exec(
        "INSERT INTO meta (key, value) VALUES ('schema_version', ?) " +
          'ON CONFLICT(key) DO UPDATE SET value = excluded.value',
        [String(current)],
      );
      const device = await tx.query("SELECT value FROM meta WHERE key = 'device_id'");
      if (device.length === 0) {
        await tx.exec('INSERT INTO meta (key, value) VALUES (?, ?)', [
          'device_id',
          crypto.randomUUID(),
        ]);
      }
      const lamport = await tx.query("SELECT value FROM meta WHERE key = 'lamport'");
      if (lamport.length === 0) {
        await tx.exec("INSERT INTO meta (key, value) VALUES ('lamport', '0')");
      }
    });
  } catch (e) {
    if (e instanceof StoreError) throw e;
    throw new StoreError(
      'migration_failed',
      `migration failed: ${e instanceof Error ? e.message : String(e)}`,
      { cause: e },
    );
  }
}

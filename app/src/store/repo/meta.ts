// meta repository (spec §4.1): install-level keys — schema_version,
// device_id (minted once by migrate) and the Lamport clock.

import type { Tx } from '../types';

export async function deviceId(tx: Tx): Promise<string> {
  const rows = await tx.query("SELECT value FROM meta WHERE key = 'device_id'");
  if (rows.length === 0) throw new Error('meta.device_id is not set');
  return String(rows[0]!.value);
}

/**
 * Reserves `n` Lamport values and returns the first of them. Must run
 * inside the caller's transaction so concurrent reservations stay
 * contiguous.
 */
export async function nextLamport(tx: Tx, n: number): Promise<number> {
  const rows = await tx.query("SELECT value FROM meta WHERE key = 'lamport'");
  const current = rows.length === 0 ? 0 : Number(rows[0]!.value);
  await tx.exec(
    "INSERT INTO meta (key, value) VALUES ('lamport', ?) " +
      'ON CONFLICT(key) DO UPDATE SET value = excluded.value',
    [String(current + n)],
  );
  return current;
}

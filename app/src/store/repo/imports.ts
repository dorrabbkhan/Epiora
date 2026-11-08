// imports repository (spec §4.1): one row per committed import,
// recording the resulting version, the settings used, per-column
// coercion counts and the detected format details.

import type { Row, Tx } from '../types';

export interface ImportRecord {
  id: string;
  datasetId: string;
  version: number;
  settings: string;
  coercions: string;
  encoding: string | null;
  delimiter: string | null;
  sheet: string | null;
  createdAt: string;
}

function toImport(r: Row): ImportRecord {
  return {
    id: String(r.id),
    datasetId: String(r.dataset_id),
    version: Number(r.version),
    settings: String(r.settings),
    coercions: String(r.coercions),
    encoding: r.encoding === null ? null : String(r.encoding),
    delimiter: r.delimiter === null ? null : String(r.delimiter),
    sheet: r.sheet === null ? null : String(r.sheet),
    createdAt: String(r.created_at),
  };
}

export async function insert(
  tx: Tx,
  i: {
    /** Caller-chosen id; change entries written earlier in the same
        transaction reference it as `import_id`. */
    id?: string;
    datasetId: string;
    version: number;
    settings: string;
    coercions: string;
    encoding?: string | null;
    delimiter?: string | null;
    sheet?: string | null;
    now: string;
  },
): Promise<ImportRecord> {
  const id = i.id ?? crypto.randomUUID();
  await tx.exec(
    'INSERT INTO imports (id, dataset_id, version, settings, coercions, encoding, delimiter, sheet, created_at) ' +
      'VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)',
    [
      id,
      i.datasetId,
      i.version,
      i.settings,
      i.coercions,
      i.encoding ?? null,
      i.delimiter ?? null,
      i.sheet ?? null,
      i.now,
    ],
  );
  return toImport({
    id,
    dataset_id: i.datasetId,
    version: i.version,
    settings: i.settings,
    coercions: i.coercions,
    encoding: i.encoding ?? null,
    delimiter: i.delimiter ?? null,
    sheet: i.sheet ?? null,
    created_at: i.now,
  });
}

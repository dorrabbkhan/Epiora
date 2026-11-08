// Snapshot loading (v0.3 spec §4.3, §5.1–5.2): `records` + `changes`
// are the source of truth and the stored snapshot is a derived cache.
// Every load compares the decoded dataset's content hash with
// `dataset_versions.content_hash`. A mismatch or `snapshot_decode` on
// the *current* version rebuilds it from `records` via
// `datasetFromRecords` — the rebuild hash must match or the load is a
// typed `HashMismatch`, never a result computed on the wrong bytes —
// and older versions are marked `snapshot_available = 0`. A mismatch
// on an older version is `HashMismatch` plus the same marking.
//
// The schema `datasetFromRecords` needs is persisted inside
// `datasets.settings` at commit time as `{"settings":…, "schema":…}`
// (the column is opaque text — see docs/decisions.md), so a rebuild
// does not depend on parsing the corrupt blob.
//
// Loaded dataset handles live in an LRU of 2 keyed `datasetId:version`.

import type { DatasetHandle } from '@engine/epiora_wasm';
import type { ValueType } from '@import-types/ValueType';
import { engine, importer } from './engines';
import { rpc } from './errors';
import type { Dataset } from '../store/repo/datasets';
import * as datasets from '../store/repo/datasets';
import * as records from '../store/repo/records';
import * as versions from '../store/repo/versions';
import type { Store } from '../store/types';
import type { GridPage } from './api';

const LRU_SIZE = 2;

interface CacheEntry {
  bytes: Uint8Array;
  handle?: DatasetHandle;
}

const cache = new Map<string, CacheEntry>();

function cacheKey(datasetId: string, version: number): string {
  return `${datasetId}:${version}`;
}

function cachePut(key: string, entry: CacheEntry): void {
  cache.delete(key);
  cache.set(key, entry);
  while (cache.size > LRU_SIZE) {
    const oldest = cache.keys().next().value!;
    cache.get(oldest)?.handle?.free();
    cache.delete(oldest);
  }
}

/** Serializes the `datasets.settings` composite stored at commit. */
export function serializeSettings(settings: unknown, schema: [string, ValueType][]): string {
  return JSON.stringify({ settings, schema });
}

/** The `[name, type]` schema persisted at commit, or null when the
 *  row's settings text is not the worker's composite. */
export function storedSchema(dataset: Dataset): [string, ValueType][] | null {
  try {
    const parsed = JSON.parse(dataset.settings) as {
      schema?: [string, ValueType][];
    };
    return Array.isArray(parsed.schema) ? parsed.schema : null;
  } catch {
    return null;
  }
}

/** The ImportSettings half of the composite (or the bare settings JSON
 *  for rows written before the composite existed). */
export function storedSettings(dataset: Dataset): unknown | null {
  try {
    const parsed = JSON.parse(dataset.settings) as { settings?: unknown };
    return parsed.settings ?? parsed;
  } catch {
    return null;
  }
}

async function hashOf(bytes: Uint8Array, datasetId: string, version: number) {
  const eng = await engine();
  const handle = eng.datasetFromSnapshot(datasetId, version, bytes);
  return { handle, hash: eng.datasetHash(handle) };
}

/** Rebuilds the current version's snapshot from `records` (§4.3):
 *  verifies the rebuilt hash before storing, marks every older
 *  version's snapshot unavailable, and returns the verified bytes. */
async function rebuild(
  store: Store,
  dataset: Dataset,
  versionRow: versions.DatasetVersion,
): Promise<CacheEntry> {
  const schema = storedSchema(dataset);
  if (schema === null) {
    throw rpc(
      'hash_mismatch',
      `dataset ${dataset.id} v${versionRow.version}: no stored schema to rebuild the snapshot`,
    );
  }
  const rows: string[] = [];
  for await (const batch of records.allJson(store, dataset.id)) rows.push(...batch);
  const imp = await importer();
  const bytes = imp.datasetFromRecords(dataset.id, versionRow.version, schema, rows);
  const { handle, hash } = await hashOf(bytes, dataset.id, versionRow.version);
  if (hash !== versionRow.contentHash) {
    handle.free();
    throw rpc(
      'hash_mismatch',
      `dataset ${dataset.id} v${versionRow.version}: rebuilt hash ${hash} != stored ${versionRow.contentHash}`,
    );
  }
  await store.transaction(async (tx) => {
    await versions.replaceSnapshot(tx, dataset.id, versionRow.version, bytes);
    await versions.markUnavailableBefore(tx, dataset.id, versionRow.version);
  });
  return { bytes, handle };
}

/**
 * Loads a version's snapshot bytes, verifying the content hash and
 * rebuilding the current version when the cache is corrupt
 * (spec §4.3). Older-version corruption is `HashMismatch`; a missing
 * or unavailable snapshot is `snapshot_unavailable`.
 */
export async function loadEntry(
  store: Store,
  datasetId: string,
  version: number,
): Promise<CacheEntry> {
  const key = cacheKey(datasetId, version);
  const hit = cache.get(key);
  if (hit !== undefined) return hit;

  const dataset = await datasets.get(store, datasetId);
  if (dataset === null) throw rpc('not_found', `dataset ${datasetId} does not exist`);
  const versionRow = await versions.get(store, datasetId, version);
  if (versionRow === null) {
    throw rpc('snapshot_unavailable', `dataset ${datasetId} has no version ${version}`);
  }
  const isCurrent = version === dataset.currentVersion;
  const bytes = await versions.snapshot(store, datasetId, version);

  if (bytes !== null) {
    try {
      const { handle, hash } = await hashOf(bytes, datasetId, version);
      if (hash === versionRow.contentHash) {
        const entry = { bytes, handle };
        cachePut(key, entry);
        return entry;
      }
      handle.free();
    } catch (e) {
      if (!(e instanceof Error && (e as { code?: unknown }).code === 'snapshot_decode')) {
        throw e;
      }
      // snapshot_decode falls through to rebuild/mismatch below.
    }
    if (!isCurrent) {
      await versions.markUnavailable(store, datasetId, version);
      throw rpc(
        'hash_mismatch',
        `dataset ${datasetId} v${version}: snapshot does not match the stored content hash`,
      );
    }
    // Current version: fall through to the rebuild.
  } else if (!isCurrent) {
    throw rpc(
      'snapshot_unavailable',
      `dataset ${datasetId} v${version}: snapshot is unavailable`,
    );
  }

  const entry = await rebuild(store, dataset, versionRow);
  cachePut(key, entry);
  return entry;
}

/** A verified engine handle for the version (LRU of 2). */
export async function loadHandle(
  store: Store,
  datasetId: string,
  version: number,
): Promise<DatasetHandle> {
  const entry = await loadEntry(store, datasetId, version);
  if (entry.handle === undefined) {
    const eng = await engine();
    entry.handle = eng.datasetFromSnapshot(datasetId, version, entry.bytes);
  }
  return entry.handle;
}

// --- The TS half of the snapshot codec (spec §5.1 layout), used to
// --- page grid rows straight off the verified bytes.

const TAG_TYPE: Record<number, ValueType> = {
  1: 'integer',
  2: 'real',
  3: 'text',
  4: 'boolean',
  5: 'date',
};

interface ColumnSlice {
  name: string;
  type: ValueType;
  missing: number;
  bitmap: number;
  values: number;
}

const utf8 = new TextDecoder();

function daysToIso(days: number): string {
  return new Date(days * 86_400_000).toISOString().slice(0, 10);
}

/** Reads the header and per-column regions; the bytes were already
 *  verified against the stored hash by `loadEntry`. */
function scan(bytes: Uint8Array): { rowCount: number; columns: ColumnSlice[] } {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const magic = utf8.decode(bytes.subarray(0, 4));
  const format = view.getUint8(4);
  if (magic !== 'EPSN' || format !== 1) {
    throw rpc('snapshot_unavailable', 'stored snapshot is not an EPSN v1 blob');
  }
  const colCount = view.getUint32(5, true);
  const rowCount = Number(view.getBigUint64(9, true));
  let pos = 17;
  const bitmapBytes = Math.ceil(rowCount / 8);
  const columns: ColumnSlice[] = [];
  for (let c = 0; c < colCount; c++) {
    const nameLen = view.getUint32(pos, true);
    pos += 4;
    const name = utf8.decode(bytes.subarray(pos, pos + nameLen));
    pos += nameLen;
    const tag = view.getUint8(pos);
    pos += 1;
    const type = TAG_TYPE[tag];
    if (type === undefined) {
      throw rpc('snapshot_unavailable', `snapshot column ${name} has unknown type tag ${tag}`);
    }
    const bitmap = pos;
    pos += bitmapBytes;
    const values = pos;
    let missing = 0;
    for (let i = 0; i < bitmapBytes; i++) {
      // Bits past rowCount in the last byte are unused, not missing.
      const validBits = i === bitmapBytes - 1 && rowCount % 8 !== 0 ? rowCount % 8 : 8;
      const b = view.getUint8(bitmap + i) & ((1 << validBits) - 1);
      missing += validBits - popcount(b);
    }
    if (type === 'text') {
      const end = view.getUint32(values + rowCount * 4, true);
      pos += (rowCount + 1) * 4 + end;
    } else {
      const width = { integer: 8, real: 8, boolean: 1, date: 4 }[type];
      pos += width * rowCount;
    }
    columns.push({ name, type, missing, bitmap, values });
  }
  return { rowCount, columns };
}

function present(view: DataView, col: ColumnSlice, row: number): boolean {
  return (view.getUint8(col.bitmap + (row >> 3)) & (1 << row % 8)) !== 0;
}

/** Formats the `[offset, offset+limit)` rows of a verified snapshot
 *  for display (dates ISO, reals via `String`, i64 as decimal). */
export function gridPage(bytes: Uint8Array, offset: number, limit: number): GridPage {
  const { rowCount, columns } = scan(bytes);
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const end = Math.min(offset + limit, rowCount);
  const rows: (string | null)[][] = [];
  for (let r = offset; r < end; r++) {
    const row: (string | null)[] = [];
    for (const col of columns) {
      if (!present(view, col, r)) {
        row.push(null);
        continue;
      }
      switch (col.type) {
        case 'integer':
          row.push(view.getBigInt64(col.values + r * 8, true).toString());
          break;
        case 'real':
          row.push(String(view.getFloat64(col.values + r * 8, true)));
          break;
        case 'text': {
          const base = col.values;
          const from = view.getUint32(base + r * 4, true);
          const to = view.getUint32(base + (r + 1) * 4, true);
          row.push(utf8.decode(bytes.subarray(base + (rowCount + 1) * 4 + from, base + (rowCount + 1) * 4 + to)));
          break;
        }
        case 'boolean':
          row.push(view.getUint8(col.values + r) === 1 ? 'true' : 'false');
          break;
        case 'date':
          row.push(daysToIso(view.getInt32(col.values + r * 4, true)));
          break;
      }
    }
    rows.push(row);
  }
  return {
    columns: columns.map(({ name, type, missing }) => ({ name, type, missing })),
    rows,
    total: rowCount,
  };
}

function popcount(b: number): number {
  let n = 0;
  let x = b;
  while (x !== 0) {
    n += x & 1;
    x >>= 1;
  }
  return n;
}

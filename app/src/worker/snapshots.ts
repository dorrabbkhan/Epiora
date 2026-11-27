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
import type { ColumnMeta } from '@engine-types/ColumnMeta';
import type { LabelSet } from '@engine-types/LabelSet';
import type { ValueType } from '@import-types/ValueType';
import { engine, importer } from './engines';
import { rpc } from './errors';
import type { Dataset } from '../store/repo/datasets';
import * as datasets from '../store/repo/datasets';
import * as records from '../store/repo/records';
import * as versions from '../store/repo/versions';
import type { Store } from '../store/types';

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

/** The snapshot-v2 metadata persisted with a legacy dataset's schema.
 *  `meta` stays in column order; `labelSets` keys the serde map by id. */
export interface StoredSnapshotMeta {
  meta: ColumnMeta[];
  labelSets: Record<string, LabelSet>;
}

/** Serializes the `datasets.settings` composite stored at commit. */
export function serializeSettings(
  settings: unknown,
  schema: [string, ValueType][],
  snapshotMeta?: StoredSnapshotMeta,
): string {
  return JSON.stringify({
    settings,
    schema,
    ...(snapshotMeta === undefined
      ? {}
      : { meta: snapshotMeta.meta, labelSets: snapshotMeta.labelSets }),
  });
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

/** The snapshot-v2 metadata half of the composite, or null for an
 *  ordinary import that never stored any. */
export function storedSnapshotMeta(dataset: Dataset): StoredSnapshotMeta | null {
  try {
    const parsed = JSON.parse(dataset.settings) as {
      meta?: ColumnMeta[];
      labelSets?: Record<string, LabelSet>;
    };
    if (!Array.isArray(parsed.meta)) return null;
    return {
      meta: parsed.meta,
      labelSets:
        typeof parsed.labelSets === 'object' && parsed.labelSets !== null ? parsed.labelSets : {},
    };
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
  const snapshotMeta = storedSnapshotMeta(dataset);
  const bytes =
    snapshotMeta === null
      ? imp.datasetFromRecords(dataset.id, versionRow.version, schema, rows)
      : imp.datasetFromRecordsWithMeta(
          dataset.id,
          versionRow.version,
          schema,
          rows,
          snapshotMeta.meta,
          snapshotMeta.labelSets,
        );
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

/** Test hook (Task 16 perf spec): drops every cached snapshot entry
 *  so the next `loadEntry` is a cold store read + hash check. Frees
 *  the held engine handles like an LRU eviction. */
export function clearSnapshotCache(): void {
  for (const entry of cache.values()) entry.handle?.free();
  cache.clear();
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


export { gridPage, snapshotColumns } from './snapshot-codec';
export type { SnapshotColumnInfo } from './snapshot-codec';

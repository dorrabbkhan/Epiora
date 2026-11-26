// Legacy re-import planning and commit (v1.1 Task 15): a newer copy
// is first proved to be the same legacy project, then Task 7's refresh
// plan diffs every keyed view. The preview is write-free; commit writes
// only changed/new versions in one transaction.

import { legacy } from './engines';
import { dataFile, pushFile } from './legacy-eptb';
import { rpc } from './errors';
import { gridPage, loadEntry } from './snapshots';
import * as datasets from '../store/repo/datasets';
import * as legacyRepo from '../store/repo/legacy';
import * as versions from '../store/repo/versions';
import type { Store, Tx } from '../store/types';
import {
  enrichProject,
  toNameBytes,
  type FileInput,
  type LegacyProjectJson,
  type RefreshPlanJson,
  type RefreshPreview,
} from './legacy-shared';

interface RefreshState {
  projectId: string;
  plan: RefreshPlanJson;
  sourceName: string;
  project: LegacyProjectJson;
  snapshots: Uint8Array[];
}

const refreshPreviews = new Map<string, RefreshState>();

/** `previous` packets for `legacyRefreshPlan`: one
 *  `u32 name_len | name | u32 version | snapshot` per committed view
 *  dataset. */
function previousPacket(name: string, version: number, snapshot: Uint8Array): Uint8Array {
  const nb = new TextEncoder().encode(name);
  const out = new Uint8Array(4 + nb.length + 4 + snapshot.length);
  const dv = new DataView(out.buffer);
  dv.setUint32(0, nb.length, true);
  out.set(nb, 4);
  dv.setUint32(4 + nb.length, version, true);
  out.set(snapshot, 4 + nb.length + 4);
  return out;
}

function sameLegacyProject(
  stored: NonNullable<Awaited<ReturnType<typeof legacyRepo.project>>>,
  mapped: LegacyProjectJson,
  sourceName: string,
): boolean {
  if (stored.legacyId !== '' || mapped.legacy_id !== '') {
    return stored.legacyId !== '' && stored.legacyId === mapped.legacy_id;
  }
  // EI3 files carry no project GUID. The only identity the import can
  // have recorded is therefore the backend plus the source file name.
  return (
    stored.sourceFormat === mapped.source_format &&
    stored.fileName.toLowerCase() === sourceName.toLowerCase()
  );
}

/** Adds `added_records` beside the diff's added row indices by reading
 *  the key column out of the new snapshot. The indices alone are not a
 *  record-level preview; removed and changed entries already carry ids. */
function addAddedRecordIds(plan: RefreshPlanJson, project: LegacyProjectJson, snapshots: Uint8Array[]): void {
  const index = new Map(project.views.map((v, i) => [v.name, i]));
  for (const view of plan.views) {
    if (typeof view.outcome === 'string') continue;
    const diff = view.outcome.changed;
    const i = index.get(view.name);
    if (i === undefined || diff.added.length === 0) continue;
    const meta = project.views[i]!;
    const keyIndex = gridPage(snapshots[i]!, 0, 0).columns.findIndex((c) => c.name === meta.key_column);
    if (keyIndex < 0) continue;
    const maxRow = Math.max(...diff.added);
    const page = gridPage(snapshots[i]!, 0, maxRow + 1);
    diff.added_records = diff.added.map((row) => page.rows[row]?.[keyIndex] ?? null);
  }
}

export async function planRefresh(
  store: Store,
  projectId: string,
  files: FileInput[],
  codePage?: string,
): Promise<RefreshPreview> {
  const stored = await legacyRepo.project(store, projectId);
  if (stored === null) {
    throw rpc('reimport_not_legacy', `project ${projectId} is not a legacy import`);
  }
  const leg = await legacy();
  const h = leg.legacyNew();
  try {
    const f = dataFile(await toNameBytes(files));
    const format = await pushFile(leg, h, f.name, f.bytes, codePage);
    const prev: Uint8Array[] = [];
    for (const d of await datasets.listByProject(store, projectId)) {
      const entry = await loadEntry(store, d.id, d.currentVersion);
      prev.push(previousPacket(d.name, d.currentVersion, entry.bytes));
    }
    const out = leg.legacyRefreshPlan(h, prev, format) as {
      plan: RefreshPlanJson;
      project: LegacyProjectJson;
      snapshots: Uint8Array[];
    };
    const snapshots = out.snapshots.map((s) => new Uint8Array(s));
    const project = enrichProject(out.project, snapshots);
    if (!sameLegacyProject(stored, project, f.name)) {
      throw rpc(
        'reimport_source_mismatch',
        `${f.name} is not a newer copy of legacy project ${stored.legacyId || stored.fileName}`,
      );
    }
    addAddedRecordIds(out.plan, project, snapshots);
    const previewId = crypto.randomUUID();
    refreshPreviews.set(previewId, {
      projectId,
      plan: out.plan,
      sourceName: f.name,
      project,
      snapshots,
    });
    return { previewId, plan: out.plan, project };
  } finally {
    h.free();
  }
}

/** Writes a new version for every `changed`/`new` view of a
 *  `planRefresh` preview. `missing` views keep their committed data
 *  untouched (spec §4.2 re-import semantics). */
export async function commitRefresh(store: Store, previewId: string): Promise<void> {
  const rp = refreshPreviews.get(previewId);
  if (rp === undefined) {
    throw rpc('session_not_found', `refresh preview ${previewId} does not exist`);
  }
  const p = rp.project;
  const now = new Date().toISOString();
  await store.transaction(async (tx: Tx) => {
    const existing = new Map(
      (await datasets.listByProject(tx, rp.projectId)).map((d) => [d.name, d]),
    );
    const index = new Map(p.views.map((v, i) => [v.name, i]));
    for (const v of rp.plan.views) {
      const outcome = typeof v.outcome === 'string' ? v.outcome : 'changed';
      if (outcome !== 'changed' && outcome !== 'new') continue;
      const i = index.get(v.name);
      if (i === undefined) continue;
      const view = p.views[i]!;
      const snap = rp.snapshots[i]!;
      const prev = existing.get(v.name);
      if (outcome === 'new' || prev === undefined) {
        const key =
          view.key_column === ''
            ? '"row_position"'
            : JSON.stringify({ column: { name: view.key_column } });
        const d = await datasets.create(
          tx,
          { projectId: rp.projectId, name: v.name, key, settings: '{}' },
          now,
        );
        await versions.insert(tx, {
          datasetId: d.id,
          version: 1,
          sourceName: rp.sourceName,
          sourceSha256: view.hash,
          rowCount: view.records,
          contentHash: view.hash,
          snapshot: snap,
        });
        await datasets.setCurrentVersion(tx, d.id, 1);
      } else {
        const version = prev.currentVersion + 1;
        await versions.insert(tx, {
          datasetId: prev.id,
          version,
          sourceName: rp.sourceName,
          sourceSha256: view.hash,
          rowCount: view.records,
          contentHash: view.hash,
          snapshot: snap,
        });
        await datasets.setCurrentVersion(tx, prev.id, version);
      }
    }
    const report = await legacyRepo.latestMigrationReport(tx, rp.projectId);
    await legacyRepo.insertMigrationReport(tx, {
      projectId: rp.projectId,
      version: (report?.version ?? 0) + 1,
      json: JSON.stringify(p.report),
    });
  });
  refreshPreviews.delete(previewId);
}

export function discardRefreshPreview(previewId: string): void {
  refreshPreviews.delete(previewId);
}

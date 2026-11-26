// Legacy re-import planning and commit (v1.1 Task 15): a newer copy
// is first proved to be the same legacy project, then Task 7's refresh
// plan diffs every keyed view. The preview is write-free; commit writes
// only changed/new versions in one transaction.

import { legacy } from './engines';
import { dataFile, pushFile } from './legacy-eptb';
import { rpc } from './errors';
import { gridPage, loadEntry } from './snapshots';
import { writeLegacyVersion } from './legacy-records';
import * as datasets from '../store/repo/datasets';
import * as legacyRepo from '../store/repo/legacy';
import type { Store, Tx } from '../store/types';
import {
  cancelScopeRequested,
  clearCancelScope,
  registerCancelScope,
  throwWhenCancelled,
} from './importer';
import {
  enrichProject,
  projectEntries,
  recordKeyJson,
  toNameBytes,
  type FileInput,
  type LegacyProjectJson,
  type RefreshDiff,
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

type ReportEntry = LegacyProjectJson['report'][number];

function schemaReportEntries(view: string, diff: RefreshDiff): ReportEntry[] {
  return [
    ...diff.schema.added_columns.map((column) => ({
      kind: 'schema_field_added',
      item: `${view}/${column}`,
      reason: 'field added in the newer copy; schema change applied',
    })),
    ...diff.schema.removed_columns.map((column) => ({
      kind: 'schema_field_removed',
      item: `${view}/${column}`,
      reason: 'field removed in the newer copy; schema change applied',
    })),
    ...diff.schema.type_changes.map((change) => ({
      kind: 'schema_field_retyped',
      item: `${view}/${change.column}`,
      reason: `field type changed from ${change.from} to ${change.to} in the newer copy; schema change applied`,
    })),
  ];
}

/** Adds `added_records` beside the diff's added row indices by reading
 *  the key column out of the new snapshot. The indices alone are not a
 *  record-level preview; removed and changed entries already carry ids. */
function addAddedRecordIds(plan: RefreshPlanJson, project: LegacyProjectJson, snapshots: Uint8Array[]): void {
  const index = new Map(projectEntries(project).map((entry) => [entry.name, entry]));
  for (const view of plan.views) {
    if (typeof view.outcome === 'string') continue;
    const diff = view.outcome.changed;
    const entry = index.get(view.name);
    if (entry === undefined || diff.added.length === 0) continue;
    const snapshot = snapshots[entry.snapshotIndex]!;
    const keyIndex = gridPage(snapshot, 0, 0).columns.findIndex((c) => c.name === entry.keyColumn);
    if (keyIndex < 0) continue;
    const maxRow = Math.max(...diff.added);
    const page = gridPage(snapshot, 0, maxRow + 1);
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
    registerCancelScope(previewId);
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

/** Writes a new version for every `changed`/`new` view or grid child
 *  of a `planRefresh` preview through the canonical v0.3 write path.
 *  `missing` views keep their committed data untouched; an unchanged
 *  re-import writes nothing at all — no version, imports row or report
 *  row (spec §4.5). Cancellation throws inside the one transaction and
 *  rolls it back. */
export async function commitRefresh(store: Store, previewId: string): Promise<void> {
  const rp = refreshPreviews.get(previewId);
  if (rp === undefined) {
    throw rpc('session_not_found', `refresh preview ${previewId} does not exist`);
  }
  const checkCancel = () => throwWhenCancelled(() => cancelScopeRequested(previewId));
  try {
    await checkCancel();
    const writes = rp.plan.views.some(
      (v) => typeof v.outcome !== 'string' || v.outcome === 'new',
    );
    if (!writes) return;

    const p = rp.project;
    const now = new Date().toISOString();
    await store.transaction(async (tx: Tx) => {
      const existing = new Map(
        (await datasets.listByProject(tx, rp.projectId)).map((d) => [d.name, d]),
      );
      const entries = new Map(projectEntries(p).map((entry) => [entry.name, entry]));
      for (const v of rp.plan.views) {
        const changed = typeof v.outcome === 'string' ? null : v.outcome.changed;
        const isNew = v.outcome === 'new';
        if (changed === null && !isNew) continue;
        const entry = entries.get(v.name);
        if (entry === undefined) continue;
        const prev = existing.get(v.name);
        let datasetId: string;
        let version: number;
        if (isNew || prev === undefined) {
          await checkCancel();
          const d = await datasets.create(
            tx,
            {
              projectId: rp.projectId,
              name: v.name,
              key: recordKeyJson(entry.keyColumn),
              settings: '{}',
            },
            now,
          );
          datasetId = d.id;
          version = 1;
        } else {
          datasetId = prev.id;
          version = prev.currentVersion + 1;
        }
        await writeLegacyVersion({
          tx,
          datasetId,
          version,
          now,
          sourceName: rp.sourceName,
          snapshot: rp.snapshots[entry.snapshotIndex]!,
          rowCount: entry.records,
          contentHash: entry.hash,
          keyColumn: entry.keyColumn,
          diff: changed,
          checkCancel,
        });
      }
      const reportEntries = [...p.report];
      for (const v of rp.plan.views) {
        if (typeof v.outcome !== 'string') {
          reportEntries.push(...schemaReportEntries(v.name, v.outcome.changed));
        }
        if (v.outcome === 'missing') {
          reportEntries.push({
            kind: 'view_not_refreshed',
            item: v.name,
            reason: 'missing from the newer copy; committed data kept unchanged',
          });
        }
        if (v.outcome === 'no_key') {
          reportEntries.push({
            kind: 'view_not_refreshed',
            item: v.name,
            reason: 'no GlobalRecordId or UniqueKey record key; left unchanged',
          });
        }
      }
      const report = await legacyRepo.latestMigrationReport(tx, rp.projectId);
      await legacyRepo.insertMigrationReport(tx, {
        projectId: rp.projectId,
        version: (report?.version ?? 0) + 1,
        json: JSON.stringify(reportEntries),
      });
      await checkCancel();
    });
  } finally {
    refreshPreviews.delete(previewId);
    clearCancelScope(previewId);
  }
}

export function discardRefreshPreview(previewId: string): void {
  refreshPreviews.delete(previewId);
  clearCancelScope(previewId);
}

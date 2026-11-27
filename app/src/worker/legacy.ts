// Legacy project import (v1.1 plan Task 13/15, spec §4.2): picked
// files become EPTB batches — `.mdb`/`.accdb` through the mdbtools
// wasm module, `.db` through sqlite-wasm + the encoder in
// `./legacy-eptb` — then `epiora-legacy-wasm` maps them and
// `commitLegacy` writes project, identity, snapshots, links, programs,
// check code and report in one transaction. Refresh and plain-table
// imports live in their own modules and are re-exported for the RPC
// surface.

import { importer, legacy } from './engines';
import { writeLegacyVersion } from './legacy-records';
import { dataFile, pushFile } from './legacy-eptb';
import { rpc } from './errors';
import { hasPlainPreview } from './legacy-plain';
import {
  awaitImportCommitHold,
  cancelScopeRequested,
  clearCancelScope,
  registerCancelScope,
  throwWhenCancelled,
} from './importer';
import * as datasets from '../store/repo/datasets';
import * as legacyRepo from '../store/repo/legacy';
import * as projects from '../store/repo/projects';
import type { Store, Tx } from '../store/types';
import type { ImportProgress } from './api';
import {
  enrichProject,
  projectEntries,
  recordKeyJson,
  toNameBytes,
  type FileInput,
  type LegacyPreview,
  type LegacyProjectJson,
  type SqlServerTableInput,
} from './legacy-shared';

interface PreviewState {
  sourceName: string;
  sourceSha256: string;
  project: LegacyProjectJson;
  snapshots: Uint8Array[];
}

const previews = new Map<string, PreviewState>();

function getPreview(previewId: string): PreviewState {
  const p = previews.get(previewId);
  if (p === undefined) {
    throw rpc('session_not_found', `legacy preview ${previewId} does not exist`);
  }
  return p;
}

function makePreview(
  sourceName: string,
  sourceSha256: string,
  project: Parameters<typeof enrichProject>[0],
  snapshots: Uint8Array[],
): LegacyPreview {
  const previewId = crypto.randomUUID();
  const enriched = enrichProject(project, snapshots);
  previews.set(previewId, { sourceName, sourceSha256, project: enriched, snapshots });
  return { previewId, project: enriched };
}

export async function inspectLegacy(
  files: FileInput[],
  codePage?: string,
): Promise<LegacyPreview> {
  const leg = await legacy();
  const h = leg.legacyNew();
  try {
    const f = dataFile(await toNameBytes(files));
    const format = await pushFile(leg, h, f.name, f.bytes, codePage);
    const mapped = leg.legacyMap(h, f.name, format) as {
      project: LegacyProjectJson;
      snapshots: Uint8Array[];
    };
    const snapshots = mapped.snapshots.map((s) => new Uint8Array(s));
    const sourceSha256 = (await importer()).sha256(f.bytes);
    return makePreview(f.name, sourceSha256, mapped.project, snapshots);
  } finally {
    h.free();
  }
}

export function beginLegacyImport(previewId: string): void {
  if (!previews.has(previewId) && !hasPlainPreview(previewId)) {
    throw rpc('session_not_found', `legacy preview ${previewId} does not exist`);
  }
  registerCancelScope(previewId);
}

/** One transaction: the project row, its legacy identity, every view
 *  dataset + version 1 snapshot, links, programs, check code and the
 *  migration report. Every checkpoint reads the live cancel flag after
 *  yielding; a throw anywhere before the end rolls the transaction
 *  back. */
export async function commitLegacy(
  store: Store,
  previewId: string,
  onProgress: (p: ImportProgress) => void,
): Promise<string> {
  const state = getPreview(previewId);
  const p = state.project;
  const now = new Date().toISOString();
  const total = p.views.length + p.datasets.length;
  const emit = (stage: ImportProgress['stage'], done: number, count: number) => {
    try {
      void onProgress({ stage, done, total: count });
    } catch {
      // A released progress proxy must not fail the import.
    }
  };
  const checkCancel = () => throwWhenCancelled(() => cancelScopeRequested(previewId));
  try {
    await checkCancel();
    const projectId = await store.transaction(async (tx: Tx) => {
      await checkCancel();
      emit('write', 0, total);
      const project = await projects.create(tx, p.name, now);
      await checkCancel();
      await legacyRepo.upsertProject(tx, {
        projectId: project.id,
        legacyId: p.legacy_id,
        sourceFormat: p.source_format,
        fileName: state.sourceName,
        importedAt: now,
      });
      // View datasets first, then `datasets` (grid children) — the
      // snapshot layout `legacyMap` documented. Every dataset goes
      // through the canonical v0.3 records/changes/imports write path.
      const idByName = new Map<string, string>();
      const entries = projectEntries(p);
      for (const [i, entry] of entries.entries()) {
        await checkCancel();
        const d = await datasets.create(
          tx,
          {
            projectId: project.id,
            name: entry.name,
            key: recordKeyJson(entry.keyColumn),
            settings: '{}',
          },
          now,
        );
        await writeLegacyVersion({
          tx,
          datasetId: d.id,
          version: 1,
          now,
          sourceName: state.sourceName,
          sourceSha256: state.sourceSha256,
          snapshot: state.snapshots[entry.snapshotIndex]!,
          rowCount: entry.records,
          contentHash: entry.hash,
          keyColumn: entry.keyColumn,
          diff: null,
          checkCancel,
        });
        idByName.set(entry.name, d.id);
        emit('write', i + 1, total);
      }
      for (const l of p.links) {
        await checkCancel();
        await legacyRepo.insertLink(tx, {
          projectId: project.id,
          parentDataset: l.parent,
          childDataset: l.child,
          childKey: l.child_key,
          parentKey: l.parent_key,
          kind: l.kind,
        });
      }
      for (const prog of p.programs) {
        await checkCancel();
        await legacyRepo.insertProgram(tx, {
          id: crypto.randomUUID(),
          projectId: project.id,
          name: prog.name,
          text: prog.text,
          source: 'import',
        });
      }
      for (const c of p.check_code) {
        const datasetId = idByName.get(c.view);
        if (datasetId !== undefined) {
          await checkCancel();
          await legacyRepo.upsertCheckCode(tx, {
            projectId: project.id,
            datasetId,
            text: c.text,
            blocks: c.blocks,
          });
        }
      }
      await checkCancel();
      await legacyRepo.insertMigrationReport(tx, {
        projectId: project.id,
        version: 1,
        json: JSON.stringify(p.report),
      });
      await awaitImportCommitHold();
      emit('snapshot', 1, 1);
      // Last checkpoint before commit: cancellation remains a complete
      // rollback even when it lands during the final report write.
      await checkCancel();
      return project.id;
    });
    // A successful commit consumes the preview and its snapshots. A
    // failure keeps it so the user can retry the same inspected import.
    previews.delete(previewId);
    return projectId;
  } finally {
    clearCancelScope(previewId);
  }
}

/** Non-connection provenance for SQL Server imports. No host, database,
 *  user or password is needed to identify the resulting project. The
 *  schema requires non-null text, so the empty string is the no-file-bytes
 *  hash value rather than a reused dataset content hash. */
const SQL_SERVER_SOURCE_NAME = 'SQL Server import';

/** Desktop SQL Server import (v1.1 Task 15): the main thread has
 *  already read every table as EPTB through the Tauri bridge; this
 *  RPC only maps those batches. Credentials never cross the worker. */
export async function inspectLegacySqlServer(
  tables: SqlServerTableInput[],
): Promise<LegacyPreview> {
  const leg = await legacy();
  const h = leg.legacyNew();
  try {
    for (const table of tables) {
      for (const batch of table.batches) leg.legacyPush(h, table.name, batch);
    }
    const mapped = leg.legacyMap(h, SQL_SERVER_SOURCE_NAME, 'ei7_sql_server') as {
      project: Omit<LegacyProjectJson, 'views' | 'datasets'> & {
        views: Omit<LegacyProjectJson['views'][number], 'fields'>[];
        datasets: Omit<LegacyProjectJson['datasets'][number], 'fields'>[];
      };
      snapshots: Uint8Array[];
    };
    const snapshots = mapped.snapshots.map((s) => new Uint8Array(s));
    return makePreview(SQL_SERVER_SOURCE_NAME, '', mapped.project, snapshots);
  } finally {
    h.free();
  }
}

/** Frees a preview once the UI is done with it. `commitLegacy` keeps
 *  the preview so a retry after a store failure can still commit. */
export function discardPreview(previewId: string): void {
  getPreview(previewId);
  previews.delete(previewId);
  clearCancelScope(previewId);
}

export { planRefresh, commitRefresh, discardRefreshPreview } from './legacy-refresh';
export {
  inspectPlainLegacy,
  commitPlainLegacy,
  discardPlainPreview,
} from './legacy-plain';
export type {
  FileInput,
  LegacyPreview,
  LegacyProjectJson,
  PlainPreview,
  PlainSkippedColumn,
  RefreshDiff,
  RefreshPlanJson,
  RefreshPreview,
  SqlServerTableInput,
} from './legacy-shared';

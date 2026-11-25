// Legacy project import and re-import (v1.1 plan Task 13, spec §4.2):
// the worker turns picked files into EPTB batches — `.mdb`/`.accdb`
// through the mdbtools wasm module, `.db` through sqlite-wasm + the
// encoder in `./legacy-eptb` — pushes them into `epiora-legacy-wasm`
// and keeps the mapped project + snapshots in a preview until
// `commitLegacy` writes the five store tables plus the view datasets
// in one transaction. `planRefresh`/`commitRefresh` do the same for a
// newer copy of an already-imported file.

import { legacy } from './engines';
import { dataFile, pushFile } from './legacy-eptb';
import { rpc } from './errors';
import * as datasets from '../store/repo/datasets';
import * as legacyRepo from '../store/repo/legacy';
import * as projects from '../store/repo/projects';
import * as versions from '../store/repo/versions';
import { loadEntry } from './snapshots';
import type { Store, Tx } from '../store/types';
import type { ImportProgress } from './api';

/** One `project` member of `legacyMap`'s return — the golden
 *  `project.json` vocabulary (plus `key_column`, `text` and
 *  `datasets`, which the store needs). */
export interface LegacyProjectJson {
  name: string;
  legacy_id: string;
  source_format: string;
  views: {
    name: string;
    records: number;
    deleted_records: number;
    hash: string;
    key_column: string;
  }[];
  datasets: { name: string; records: number; hash: string }[];
  links: {
    parent: string;
    child: string;
    child_key: string;
    parent_key: string;
    kind: 'relate' | 'grid';
  }[];
  programs: { name: string; text: string; text_sha256: string }[];
  check_code: { view: string; blocks: number; text: string }[];
  report: { kind: string; item: string; reason: string }[];
}

/** `RefreshPlanJson['plan']` — every view's refresh decision. */
export interface RefreshPlanJson {
  views: {
    name: string;
    outcome:
      | 'unchanged'
      | 'missing'
      | 'new'
      | 'no_key'
      | { changed: unknown };
  }[];
}

export interface LegacyPreview {
  previewId: string;
  project: LegacyProjectJson;
}

export interface RefreshPreview {
  previewId: string;
  plan: RefreshPlanJson;
  project: LegacyProjectJson;
}

/** A picked file — the plan's `File[]` spelling — or the equivalent
 *  `{name, bytes}` plain object tests and pre-read callers use. */
export type FileInput = File | { name: string; bytes: Uint8Array };

interface PreviewState {
  sourceName: string;
  project: LegacyProjectJson;
  snapshots: Uint8Array[];
}

interface RefreshState {
  projectId: string;
  plan: RefreshPlanJson;
  sourceName: string;
  project: LegacyProjectJson;
  snapshots: Uint8Array[];
}

const previews = new Map<string, PreviewState>();
const refreshPreviews = new Map<string, RefreshState>();

function getPreview(previewId: string): PreviewState {
  const p = previews.get(previewId);
  if (p === undefined) {
    throw rpc('session_not_found', `legacy preview ${previewId} does not exist`);
  }
  return p;
}

/** `File`s read via `arrayBuffer()`; `{name, bytes}` inputs pass
 *  through. Discriminates on `arrayBuffer` — modern `File`s also
 *  carry a `bytes()` method, so `'bytes' in f` cannot tell them
 *  apart. */
async function toNameBytes(files: FileInput[]): Promise<{ name: string; bytes: Uint8Array }[]> {
  return Promise.all(
    files.map(async (f) =>
      'arrayBuffer' in f ? { name: f.name, bytes: new Uint8Array(await f.arrayBuffer()) } : f,
    ),
  );
}

// ---- inspect / commit / refresh ---------------------------------

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
    const previewId = crypto.randomUUID();
    previews.set(previewId, {
      sourceName: f.name,
      project: mapped.project,
      snapshots: mapped.snapshots.map((s) => new Uint8Array(s)),
    });
    return { previewId, project: mapped.project };
  } finally {
    h.free();
  }
}

/** One transaction: the project row, its legacy identity, every view
 *  dataset + version 1 snapshot, links, programs, check code and the
 *  migration report. */
export async function commitLegacy(
  store: Store,
  previewId: string,
  onProgress: (p: ImportProgress) => void,
): Promise<string> {
  const state = getPreview(previewId);
  const p = state.project;
  const now = new Date().toISOString();
  const total = p.views.length + p.datasets.length;
  return store.transaction(async (tx: Tx) => {
    onProgress({ stage: 'write', done: 0, total });
    const project = await projects.create(tx, p.name, now);
    await legacyRepo.upsertProject(tx, {
      projectId: project.id,
      legacyId: p.legacy_id,
      sourceFormat: p.source_format,
      fileName: state.sourceName,
      importedAt: now,
    });
    // View datasets first, then `datasets` (grid children) — the
    // snapshot layout `legacyMap` documented.
    const idByName = new Map<string, string>();
    const writeDataset = async (
      name: string,
      key: string,
      snapshot: Uint8Array,
      records: number,
      hash: string,
      i: number,
    ) => {
      const d = await datasets.create(
        tx,
        { projectId: project.id, name, key, settings: '{}' },
        now,
      );
      await versions.insert(tx, {
        datasetId: d.id,
        version: 1,
        sourceName: state.sourceName,
        sourceSha256: hash,
        rowCount: records,
        contentHash: hash,
        snapshot,
      });
      await datasets.setCurrentVersion(tx, d.id, 1);
      idByName.set(name, d.id);
      onProgress({ stage: 'write', done: i + 1, total });
    };
    for (const [i, v] of p.views.entries()) {
      const key =
        v.key_column === ''
          ? '"row_position"'
          : JSON.stringify({ column: { name: v.key_column } });
      await writeDataset(v.name, key, state.snapshots[i]!, v.records, v.hash, i);
    }
    for (const [j, d] of p.datasets.entries()) {
      await writeDataset(
        d.name,
        '"row_position"',
        state.snapshots[p.views.length + j]!,
        d.records,
        d.hash,
        p.views.length + j,
      );
    }
    for (const l of p.links) {
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
        await legacyRepo.upsertCheckCode(tx, {
          projectId: project.id,
          datasetId,
          text: c.text,
          blocks: c.blocks,
        });
      }
    }
    await legacyRepo.insertMigrationReport(tx, {
      projectId: project.id,
      version: 1,
      json: JSON.stringify(p.report),
    });
    onProgress({ stage: 'snapshot', done: 1, total: 1 });
    return project.id;
  });
}

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

export async function planRefresh(
  store: Store,
  projectId: string,
  files: FileInput[],
  codePage?: string,
): Promise<RefreshPreview> {
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
    const previewId = crypto.randomUUID();
    refreshPreviews.set(previewId, {
      projectId,
      plan: out.plan,
      sourceName: f.name,
      project: out.project,
      snapshots: out.snapshots.map((s) => new Uint8Array(s)),
    });
    return { previewId, plan: out.plan, project: out.project };
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

/** Frees a preview once the UI is done with it (cancel or refresh
 *  discard). `commitLegacy` keeps the preview so a retry after a
 *  store failure can still commit. */
export function discardPreview(previewId: string): void {
  previews.delete(previewId);
  refreshPreviews.delete(previewId);
}

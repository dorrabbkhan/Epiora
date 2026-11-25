// Legacy project import and re-import (v1.1 plan Task 13, spec §4.2):
// the worker turns picked files into EPTB batches — `.mdb`/`.accdb`
// through the mdbtools wasm module (app/src/legacy/mdb.ts), `.db`
// through sqlite-wasm with the encoder below — pushes them into
// `epiora-legacy-wasm` and keeps the mapped project + snapshots in a
// preview until `commitLegacy` writes the five store tables plus the
// view datasets in one transaction. `planRefresh`/`commitRefresh` do
// the same for a newer copy of an already-imported file.
//
// The EPTB encoder and the storage-class union mirror the native
// `SqliteSource` test source (crates/epiora-legacy/tests/common):
// text wins, then real, then integer; all-blob columns are the
// skipped-binary kind; an all-null column is skipped only when its
// declared type mentions BLOB.

import sqlite3InitModule, {
  type Sqlite3Static,
} from '@sqlite.org/sqlite-wasm';
import sqlite3WasmUrl from '@sqlite.org/sqlite-wasm/sqlite3.wasm?url';
import { openMdb } from '../legacy/mdb';
import { legacy, type Legacy } from './engines';
import type { LegacyHandle } from '@legacy/epiora_legacy_wasm';
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

// ---- EPTB encoder (sqlite path) ---------------------------------

type Cell = null | number | bigint | string | Uint8Array;

interface EncColumn {
  name: string;
  /** sqlite-declared type (lowercased) — the all-null fallback. */
  decl: string;
  cells: Cell[];
}

const CHUNK = 50_000;

/** Encode one sqlite table as an EPTB v1 batch — byte-compatible
 *  with `BatchSource`'s `decode_batch`. Layout: `EPTB` | u16 1 |
 *  u32 rows | u16 cols | u16 ext_len | ext (1 | u16 name_len | name),
 *  then per column u32 name_len + name | u8 source_type | u8 kind |
 *  bitmap | data (i64le / f64le / u8 / u32 blob + offsets). */
function encodeTableEptb(table: string, columns: EncColumn[]): Uint8Array {
  const utf8 = new TextEncoder();
  const rows = columns[0]?.cells.length ?? 0;
  const dvOf = (b: Uint8Array) => new DataView(b.buffer, b.byteOffset, b.byteLength);
  const u32 = (v: number) => {
    const b = new Uint8Array(4);
    dvOf(b).setUint32(0, v >>> 0, true);
    return b;
  };
  const u16 = (v: number) => {
    const b = new Uint8Array(2);
    dvOf(b).setUint16(0, v & 0xffff, true);
    return b;
  };
  const parts: Uint8Array[] = [];

  const nameBytes = utf8.encode(table);
  const ext = new Uint8Array(3 + nameBytes.length);
  ext[0] = 1;
  dvOf(ext).setUint16(1, nameBytes.length, true);
  ext.set(nameBytes, 3);
  parts.push(utf8.encode('EPTB'), u16(1), u32(rows), u16(columns.length), u16(ext.length), ext);

  const bitmapLen = Math.ceil(rows / 8);
  for (const col of columns) {
    parts.push(u32(utf8.encode(col.name).length), utf8.encode(col.name));
    // sqlite columns carry no Access type byte — the native source
    // uses 0 the same way.
    parts.push(new Uint8Array([0]));

    const present = col.cells.filter((c) => c !== null);
    const hasText = present.some((c) => typeof c === 'string');
    const hasReal = present.some((c) => typeof c === 'number' && !Number.isInteger(c));
    const hasInt = present.some((c) => typeof c === 'number' || typeof c === 'bigint');
    const hasBlob = present.some((c) => c instanceof Uint8Array);
    // Storage-class union: text wins, then real, then integer;
    // blob-only (or a BLOB-declared all-null column) is skipped.
    const kind = hasText
      ? 2
      : hasReal
        ? 1
        : hasInt
          ? 0
          : hasBlob || col.decl.includes('blob')
            ? 5
            : 0;
    parts.push(new Uint8Array([kind]));
    if (kind === 5) continue;

    const bitmap = new Uint8Array(bitmapLen);
    col.cells.forEach((c, r) => {
      if (c === null) bitmap[r >> 3]! |= 1 << (r & 7);
    });
    parts.push(bitmap);

    if (kind === 0) {
      const data = new Uint8Array(present.length * 8);
      const dv = dvOf(data);
      present.forEach((c, i) => {
        dv.setBigInt64(i * 8, typeof c === 'bigint' ? c : BigInt(Math.trunc(c as number)), true);
      });
      parts.push(data);
    } else if (kind === 1) {
      const data = new Uint8Array(present.length * 8);
      const dv = dvOf(data);
      present.forEach((c, i) => {
        dv.setFloat64(i * 8, Number(c), true);
      });
      parts.push(data);
    } else {
      // Text: utf8\0 blob, then u32 offsets.
      const blobParts: Uint8Array[] = [];
      const offsets = new Uint8Array(present.length * 4);
      const odv = dvOf(offsets);
      let off = 0;
      present.forEach((c, i) => {
        odv.setUint32(i * 4, off, true);
        const s = typeof c === 'string' ? c : typeof c === 'bigint' ? c.toString() : String(c);
        const b = utf8.encode(`${s}\0`);
        blobParts.push(b);
        off += b.length;
      });
      parts.push(u32(off), ...blobParts, offsets);
    }
  }
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let pos = 0;
  for (const p of parts) {
    out.set(p, pos);
    pos += p.length;
  }
  return out;
}

// ---- file → EPTB pushes ------------------------------------------

let sqliteP: Promise<Sqlite3Static> | null = null;

function getSqlite(): Promise<Sqlite3Static> {
  sqliteP ??= (
    sqlite3InitModule as unknown as (cfg?: { locateFile?: () => string }) => Promise<Sqlite3Static>
  )({ locateFile: () => sqlite3WasmUrl });
  return sqliteP;
}

const SQLITE_LIST =
  "SELECT name FROM sqlite_master WHERE type IN ('table','view') " +
  "AND name NOT LIKE 'sqlite_%' ORDER BY rowid";

/** Push every table of a `.db` file into the legacy handle. */
async function pushSqlite(leg: Legacy, h: LegacyHandle, bytes: Uint8Array): Promise<void> {
  const sqlite3 = await getSqlite();
  const ptr = sqlite3.wasm.allocFromTypedArray(bytes);
  const db = new sqlite3.oo1.DB(':memory:');
  try {
    sqlite3.capi.sqlite3_deserialize(
      db.pointer!,
      'main',
      ptr,
      bytes.length,
      bytes.length,
      sqlite3.capi.SQLITE_DESERIALIZE_FREEONCLOSE |
        sqlite3.capi.SQLITE_DESERIALIZE_RESIZEABLE,
    );
    const names = db.exec({ sql: SQLITE_LIST, rowMode: 'array' }) as unknown as SqlValueLike[][];
    for (const [rawName] of names) {
      const table = String(rawName);
      const quoted = `"${table.replaceAll('"', '""')}"`;
      const declRows = db.exec({
        sql: `PRAGMA table_info(${quoted})`,
        rowMode: 'array',
      }) as unknown as SqlValueLike[][];
      const decls = declRows.map((r) => String(r[2]).toLowerCase());
      const colNames: string[] = [];
      const rows = db.exec({
        sql: `SELECT * FROM ${quoted}`,
        rowMode: 'array',
        columnNames: colNames,
      }) as unknown as SqlValueLike[][];
      const columns: EncColumn[] = colNames.map((name, i) => ({
        name,
        decl: decls[i] ?? '',
        cells: rows.map((r) => (r[i] === undefined ? null : r[i]) as Cell),
      }));
      // Chunked pushes keep a single batch small — same 50_000-row
      // chunking as the mdb path; `legacyPush` accumulates in order.
      for (let start = 0; start < Math.max(rows.length, 1); start += CHUNK) {
        const chunk = columns.map((c) => ({
          ...c,
          cells: c.cells.slice(start, start + CHUNK),
        }));
        leg.legacyPush(h, table, encodeTableEptb(table, chunk));
        if (rows.length <= start + CHUNK) break;
      }
    }
  } finally {
    db.close();
  }
}

type SqlValueLike = string | number | bigint | Uint8Array | null;

/** Push every table of `file` — `.db` via the sqlite encoder,
 *  anything else via the mdbtools module (which emits EPTB
 *  natively). Returns the file's backend format for `legacyMap`:
 *  `detect` is layout-only, so a `.db` must name `ei7_sqlite`. */
async function pushFile(
  leg: Legacy,
  h: LegacyHandle,
  name: string,
  bytes: Uint8Array,
  codePage?: string,
): Promise<string | undefined> {
  if (/\.db$/i.test(name)) {
    await pushSqlite(leg, h, bytes);
    return 'ei7_sqlite';
  }
  const mdb = await openMdb(bytes, codePage);
  try {
    for (const table of mdb.tables()) {
      for await (const batch of mdb.batches(table)) {
        leg.legacyPush(h, table, batch);
      }
    }
  } finally {
    mdb.close();
  }
  return undefined;
}

/** The file whose bytes feed the mapper — `.prj` companions name the
 *  project but carry no tables, so the data file wins. */
function dataFile(files: { name: string; bytes: Uint8Array }[]): {
  name: string;
  bytes: Uint8Array;
} {
  const pick = files.find((f) => /\.(mdb|accdb|db)$/i.test(f.name)) ?? files[0];
  if (pick === undefined) throw rpc('session_not_found', 'no files supplied');
  return pick;
}

// ---- inspect / commit / refresh ---------------------------------

export async function inspectLegacy(
  files: { name: string; bytes: Uint8Array }[],
  codePage?: string,
): Promise<LegacyPreview> {
  const leg = await legacy();
  const h = leg.legacyNew();
  try {
    const f = dataFile(files);
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
  files: { name: string; bytes: Uint8Array }[],
  codePage?: string,
): Promise<RefreshPreview> {
  const leg = await legacy();
  const h = leg.legacyNew();
  try {
    const f = dataFile(files);
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

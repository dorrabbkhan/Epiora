// File bytes → EPTB pushes (v1.1 plan Task 13, spec §4.2): `.db`
// tables are read through sqlite-wasm and written as EPTB v1 chunks;
// `.mdb`/`.accdb` go through the mdbtools wasm module
// (app/src/legacy/mdb.ts), which emits EPTB natively. `pushFile`
// feeds `legacyPush`, `dataFile` picks the file the mapper reads.
//
// A column's `kind` is classified ONCE over all its cells (the native
// `SqliteSource` union: text wins, then real, then integer; all-blob
// columns are the skipped-binary kind; an all-null column is skipped
// only when its declared type mentions BLOB) — never per chunk:
// `BatchSource::merge_batch` rejects a kind change across chunks, so
// a >50,000-row column whose early rows are all-integral/all-null and
// whose later rows hold REAL/text must carry one kind end to end or
// the import dies with `legacy_batch_invalid`.

import sqlite3InitModule, {
  type Sqlite3Static,
} from '@sqlite.org/sqlite-wasm';
import sqlite3WasmUrl from '@sqlite.org/sqlite-wasm/sqlite3.wasm?url';
import { openMdb } from '../legacy/mdb';
import type { Legacy } from './engines';
import type { LegacyHandle } from '@legacy/epiora_legacy_wasm';
import { rpc } from './errors';

export type Cell = null | number | bigint | string | Uint8Array;

export interface EncColumn {
  name: string;
  /** sqlite-declared type (lowercased) — the all-null fallback. */
  decl: string;
  cells: Cell[];
}

export const CHUNK = 50_000;

/** The EPTB `kind` a whole column declares — classified over ALL its
 *  cells so every chunk carries the same kind. Exported (with
 *  `encodeTableEptb`/`pushTableColumns`) for the unit test. */
export function columnKind(col: EncColumn): 0 | 1 | 2 | 5 {
  const present = col.cells.filter((c) => c !== null);
  const hasText = present.some((c) => typeof c === 'string');
  const hasReal = present.some((c) => typeof c === 'number' && !Number.isInteger(c));
  const hasInt = present.some((c) => typeof c === 'number' || typeof c === 'bigint');
  const hasBlob = present.some((c) => c instanceof Uint8Array);
  // Storage-class union: text wins, then real, then integer;
  // blob-only (or a BLOB-declared all-null column) is skipped.
  return hasText
    ? 2
    : hasReal
      ? 1
      : hasInt
        ? 0
        : hasBlob || col.decl.includes('blob')
          ? 5
          : 0;
}

/** Encode one chunk of a sqlite table as an EPTB v1 batch — byte-
 *  compatible with `BatchSource`'s `decode_batch`. Layout: `EPTB` |
 *  u16 1 | u32 rows | u16 cols | u16 ext_len | ext (1 | u16 name_len |
 *  name), then per column u32 name_len + name | u8 source_type |
 *  u8 kind | bitmap | data (i64le / f64le / u8 / u32 blob + offsets).
 *  `kinds[i]` is `columnKind` over the column's full cells. */
export function encodeTableEptb(
  table: string,
  columns: EncColumn[],
  kinds: number[],
): Uint8Array {
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
  for (const [ci, col] of columns.entries()) {
    parts.push(u32(utf8.encode(col.name).length), utf8.encode(col.name));
    // sqlite columns carry no Access type byte — the native source
    // uses 0 the same way.
    parts.push(new Uint8Array([0]));

    const kind = kinds[ci]!;
    parts.push(new Uint8Array([kind]));
    if (kind === 5) continue;

    const present = col.cells.filter((c) => c !== null);
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

/** Push `columns` as EPTB chunks of `CHUNK` rows — `columnKind` is
 *  evaluated once over every cell, so `merge_batch` sees identical
 *  kinds on each chunk. */
export function pushTableColumns(
  leg: Legacy,
  h: LegacyHandle,
  table: string,
  columns: EncColumn[],
): void {
  const kinds = columns.map(columnKind);
  const rows = columns[0]?.cells.length ?? 0;
  // Chunked pushes keep a single batch small — same 50_000-row
  // chunking as the mdb path; `legacyPush` accumulates in order.
  for (let start = 0; start < Math.max(rows, 1); start += CHUNK) {
    const chunk = columns.map((c) => ({
      ...c,
      cells: c.cells.slice(start, start + CHUNK),
    }));
    leg.legacyPush(h, table, encodeTableEptb(table, chunk, kinds));
    if (rows <= start + CHUNK) break;
  }
}

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

type SqlValueLike = string | number | bigint | Uint8Array | null;

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
      pushTableColumns(leg, h, table, columns);
    }
  } finally {
    db.close();
  }
}

/** Push every table of `file` — `.db` via the sqlite encoder,
 *  anything else via the mdbtools module (which emits EPTB
 *  natively). Returns the file's backend format for `legacyMap`:
 *  `detect` is layout-only, so a `.db` must name `ei7_sqlite`. */
export async function pushFile(
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
export function dataFile(files: { name: string; bytes: Uint8Array }[]): {
  name: string;
  bytes: Uint8Array;
} {
  const pick = files.find((f) => /\.(mdb|accdb|db)$/i.test(f.name)) ?? files[0];
  if (pick === undefined) throw rpc('session_not_found', 'no files supplied');
  return pick;
}

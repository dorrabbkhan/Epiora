// The TS half of the snapshot codec (spec §5.1 layout), used to page
// grid rows straight off verified bytes and to read snapshot-v2 column
// metadata without materializing rows. Snapshot loading/rebuild lives
// in `snapshots.ts`.

import type { ValueType } from '@import-types/ValueType';
import { rpc } from './errors';
import type { GridPage } from './api';

const TAG_TYPE: Record<number, ValueType> = {
  1: 'integer',
  2: 'real',
  3: 'text',
  4: 'boolean',
  5: 'date',
  6: 'time',
  7: 'date_time',
};

interface ColumnSlice {
  name: string;
  type: ValueType;
  missing: number;
  bitmap: number;
  values: number;
  prompt?: string;
  legacyType?: string;
  legacyName?: string;
  labelSet?: string;
}

/** The schema and v2 column metadata of a verified snapshot (v1.1
 *  legacy preview: legacy type → Epiora type without decoding rows). */
export interface SnapshotColumnInfo {
  name: string;
  type: ValueType;
  prompt?: string;
  legacyType?: string;
  legacyName?: string;
  labelSet?: string;
}

const utf8 = new TextDecoder();

function daysToIso(days: number): string {
  return new Date(days * 86_400_000).toISOString().slice(0, 10);
}

/** Seconds since midnight → `hh:mm:ss` (the Time display, v1.1 §4.3). */
function secsToClock(secs: number): string {
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${pad(Math.floor(secs / 3600))}:${pad(Math.floor((secs % 3600) / 60))}:${pad(secs % 60)}`;
}

/** Seconds since 1970-01-01 (no zone) → `YYYY-MM-DD hh:mm:ss` (the
 *  DateTime display, v1.1 §4.3). BigInt division keeps i64 inputs
 *  exact; `daysToIso` handles the pre-epoch dates `Date` supports. */
function secsToIso(secs: bigint): string {
  const days = secs / 86_400n;
  const rem = secs - days * 86_400n;
  const s = rem < 0n ? rem + 86_400n : rem;
  const d = rem < 0n ? days - 1n : days;
  const pad = (n: bigint) => String(n).padStart(2, '0');
  return `${daysToIso(Number(d))} ${pad(s / 3600n)}:${pad((s % 3600n) / 60n)}:${pad(s % 60n)}`;
}

/** Reads the header and per-column regions (formats 1 and 2; v1.1
 *  spec §4.3); the bytes were already verified against the stored
 *  hash by `loadEntry`. Format 2 adds a meta block per column and a
 *  trailing label-set section the grid never pages. */
function scan(bytes: Uint8Array): { rowCount: number; columns: ColumnSlice[] } {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const magic = utf8.decode(bytes.subarray(0, 4));
  const format = view.getUint8(4);
  if (magic !== 'EPSN' || (format !== 1 && format !== 2)) {
    throw rpc('snapshot_unavailable', `stored snapshot is not an EPSN blob (format ${format})`);
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
    if (type === undefined || (format === 1 && tag > 5)) {
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
      const width = { integer: 8, real: 8, boolean: 1, date: 4, time: 4, date_time: 8 }[type];
      pos += width * rowCount;
    }
    const info: ColumnSlice = { name, type, missing, bitmap, values };
    if (format === 2) {
      // Meta block: a flag byte (bits 0–3 = prompt, legacy type,
      // legacy name, label-set id) then each flagged `u32`+UTF-8 field.
      const flags = view.getUint8(pos);
      pos += 1;
      const metaKeys = ['prompt', 'legacyType', 'legacyName', 'labelSet'] as const;
      for (let bit = 0; bit < 4; bit++) {
        if ((flags & (1 << bit)) !== 0) {
          const len = view.getUint32(pos, true);
          pos += 4;
          info[metaKeys[bit]!] = utf8.decode(bytes.subarray(pos, pos + len));
          pos += len;
        }
      }
    }
    columns.push(info);
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
        case 'time':
          row.push(secsToClock(view.getInt32(col.values + r * 4, true)));
          break;
        case 'date_time':
          row.push(secsToIso(view.getBigInt64(col.values + r * 8, true)));
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

/** Column names, Epiora types and snapshot-v2 metadata without
 *  materializing any row values. */
export function snapshotColumns(bytes: Uint8Array): SnapshotColumnInfo[] {
  return scan(bytes).columns.map(({ name, type, prompt, legacyType, legacyName, labelSet }) => ({
    name,
    type,
    ...(prompt !== undefined ? { prompt } : {}),
    ...(legacyType !== undefined ? { legacyType } : {}),
    ...(legacyName !== undefined ? { legacyName } : {}),
    ...(labelSet !== undefined ? { labelSet } : {}),
  }));
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

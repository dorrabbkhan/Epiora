// Emission for the golden imports (v1.1 Task 4): the dataset-JSON v2
// documents the corpus loader reads (crates/epiora-core/src/corpus/
// dataset_json.rs), the project.json document Task 7 compares the
// mapper against, and the SHA-256 content hash — a port of
// crates/epiora-core/src/dataset/hash.rs so the golden records the
// same hash the engine computes (spec §8.2).

import { createHash } from 'node:crypto';

// ------------------------------------------------------------------
// canonical bytes (mirrors hash.rs exactly)

const TAG = { integer: 1, real: 2, text: 3, boolean: 4, date: 5, time: 6, date_time: 7 };

function u32(n) {
  const b = Buffer.alloc(4);
  b.writeUInt32LE(n >>> 0);
  return b;
}
function u64(n) {
  const b = Buffer.alloc(8);
  b.writeBigUInt64LE(BigInt(n));
  return b;
}
function i32(n) {
  const b = Buffer.alloc(4);
  b.writeInt32LE(n);
  return b;
}
function i64(n) {
  const b = Buffer.alloc(8);
  b.writeBigInt64LE(BigInt(n));
  return b;
}
function f64(x) {
  const b = Buffer.alloc(8);
  b.writeDoubleLE(x === 0 ? 0 : x); // -0.0 normalises to 0.0
  return b;
}
function str(s) {
  const u = Buffer.from(s, 'utf8');
  return Buffer.concat([u32(u.length), u]);
}

// One present value's canonical bytes: tag + the column encoding's
// value bytes (label-set entries use this too).
function writeValue(v) {
  switch (v.type) {
    case 'integer':
      return Buffer.concat([Buffer.from([1]), i64(v.value)]);
    case 'real':
      return Buffer.concat([Buffer.from([2]), f64(v.value)]);
    case 'text':
      return Buffer.concat([Buffer.from([3]), str(v.value)]);
    case 'boolean':
      return Buffer.concat([Buffer.from([4]), Buffer.from([v.value ? 1 : 0])]);
    case 'date':
      return Buffer.concat([Buffer.from([5]), i32(v.value)]);
    case 'time':
      return Buffer.concat([Buffer.from([6]), i32(v.value)]);
    case 'date_time':
      return Buffer.concat([Buffer.from([7]), i64(v.value)]);
    default:
      throw new Error(`unknown label value type ${v.type}`);
  }
}

// The golden dataset model keeps typed column storage exactly like
// ColumnData: {name, type, values:[Option<typed>], meta?} where typed
// is the JSON surface value (number|string|boolean) EXCEPT date/time/
// date_time which travel as "YYYY-MM-DD" / "hh:mm:ss" /
// "YYYY-MM-DD hh:mm:ss" strings and are converted to the integer
// counts canonical bytes want.
function dateToDays(s) {
  const [y, m, d] = s.split('-').map(Number);
  // Howard Hinnant's days_from_civil (same as hash.rs).
  const yy = m <= 2 ? y - 1 : y;
  const era = Math.floor((yy >= 0 ? yy : yy - 399) / 400);
  const yoe = yy - era * 400;
  const mp = (m + 9) % 12;
  const doy = Math.floor((153 * mp + 2) / 5) + d - 1;
  const doe = yoe * 365 + Math.floor(yoe / 4) - Math.floor(yoe / 100) + doy;
  return era * 146097 + doe - 719468;
}
function timeToSecs(s) {
  const [h, m, sec] = s.split(':').map(Number);
  return h * 3600 + m * 60 + sec;
}
function dateTimeToSecs(s) {
  return dateToDays(s.slice(0, 10)) * 86400 + timeToSecs(s.slice(11));
}

function cellBytes(type, v) {
  switch (type) {
    case 'integer':
      return i64(v);
    case 'real':
      return f64(v);
    case 'text':
      return str(v);
    case 'boolean':
      return Buffer.from([v ? 1 : 0]);
    case 'date':
      return i32(dateToDays(v));
    case 'time':
      return i32(timeToSecs(v));
    case 'date_time':
      return i64(dateTimeToSecs(v));
    default:
      throw new Error(`unknown column type ${type}`);
  }
}

export function datasetHashHex(ds) {
  const parts = [Buffer.from([0x01]), u32(ds.columns.length)];
  for (const c of ds.columns) {
    parts.push(str(c.name), Buffer.from([TAG[c.type]]), u64(c.values.length));
    for (const v of c.values) {
      if (v === null || v === undefined) parts.push(Buffer.from([0]));
      else parts.push(Buffer.from([1]), cellBytes(c.type, v));
    }
  }
  const needsLbls =
    (ds.label_sets && ds.label_sets.length > 0) ||
    ds.columns.some((c) => c.type === 'time' || c.type === 'date_time');
  if (needsLbls) {
    const sets = [...(ds.label_sets || [])].sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
    parts.push(Buffer.from('LBLS'), u32(sets.length));
    for (const set of sets) {
      parts.push(str(set.id), u32(set.entries.length));
      for (const [value, label] of set.entries) parts.push(writeValue(value), str(label));
    }
  }
  return createHash('sha256').update(Buffer.concat(parts)).digest('hex');
}

// ------------------------------------------------------------------
// JSON emitters

// dataset.json: the exact v2 document the corpus loader reads —
// label-set entries become [{type, value}, label] pairs where the
// typed value keeps its engine spelling.
export function datasetJson(ds) {
  const out = { id: ds.id, version: ds.version, columns: [] };
  for (const c of ds.columns) {
    const col = { name: c.name, type: c.type, values: c.values };
    if (c.meta && Object.keys(c.meta).length > 0) col.meta = c.meta;
    out.columns.push(col);
  }
  if (ds.label_sets && ds.label_sets.length > 0) {
    out.label_sets = ds.label_sets.map((s) => ({
      id: s.id,
      entries: s.entries.map(([value, label]) => [value, label]),
    }));
  }
  return out;
}

export function writeJson(obj) {
  return JSON.stringify(obj, null, 2) + '\n';
}

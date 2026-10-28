// Failing-first tests for the libmdb wasm module (v0.1 Task 4, plan step 1).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import * as mdb from '../mdb.mjs';

const data = (name) => join(dirname(fileURLToPath(import.meta.url)), 'data', name);

test('tables lists the sample project tables', async () => {
  const tables = await mdb.mdb_tables(data('Sample.mdb'));
  assert.ok(Array.isArray(tables), `expected array, got ${tables}`);
  // form table names recorded in Task 2/3: Oswego is the 4-column parent,
  // Oswego12 the 22-field data table (75 rows)
  for (const name of ['Oswego', 'Oswego12']) {
    assert.ok(tables.includes(name), `missing table ${name}`);
  }
});

test('export matches native mdb-export', async () => {
  // Reference produced by native mdbtools 1.0.1 (--disable-glib):
  //   mdb-export Sample.mdb Oswego12 > Oswego12.native.csv
  const native = readFileSync(data('Oswego12.native.csv'), 'utf8');
  const csv = await mdb.mdb_export_csv(data('Sample.mdb'), 'Oswego12');
  assert.ok(!csv.startsWith('ERROR:'), csv.slice(0, 200));
  const natLines = native.split('\n').filter((l) => l.length);
  const wasmLines = csv.split('\n').filter((l) => l.length);
  assert.equal(wasmLines.length, natLines.length, 'row count differs');
  assert.equal(wasmLines[1], natLines[1], 'first data row differs');
  assert.equal(csv, native, 'CSV output is not byte-identical to native mdb-export');
});

test('truncated file reports an error', async () => {
  const buf = readFileSync(data('Sample.mdb')).subarray(0, 4096);
  const dir = mkdtempSync(join(tmpdir(), 'mdb-wasm-'));
  const trunc = join(dir, 'truncated-4k.mdb');
  writeFileSync(trunc, buf);
  const v = await mdb.mdb_version(trunc);   // must not throw
  assert.match(v, /^ERROR:/, `expected ERROR:…, got ${v}`);
});

test('JET3 file reports its format', async () => {
  const v = await mdb.mdb_version(data('testV1997.mdb'));
  assert.equal(v, 'JET3');
  const tables = await mdb.mdb_tables(data('testV1997.mdb'));
  if (typeof tables === 'string') {
    assert.match(tables, /^ERROR:.*JET3/i,
      `ERROR must name JET3, got ${tables}`);
  } else {
    assert.ok(Array.isArray(tables));
    assert.ok(tables.length > 0, 'expected at least one table');
  }
});

// Unit tests for the golden imports (v1.1 Task 4). They drive
// golden.mjs against fixtures/legacy/samples/ei7/Sample/Sample.mdb
// with the native mdbtools CLIs (PATH, or MDB_TABLES/MDB_SCHEMA/
// MDB_EXPORT/MDB_VER/SQLITE3 overrides) — the same tools the
// legacy-golden workflow installs.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import { goldenFile } from './golden.mjs';

const SAMPLE = path.resolve('fixtures/legacy/samples/ei7/Sample/Sample.mdb');

function golden(file = SAMPLE) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'golden-'));
  goldenFile(file, dir, null);
  const project = JSON.parse(fs.readFileSync(path.join(dir, 'project.json'), 'utf8'));
  const dataset = (view) =>
    JSON.parse(
      fs.readFileSync(path.join(dir, project.views.find((v) => v.name === view).dataset), 'utf8')
    );
  return { project, dataset };
}

test('oswego_view_has_expected_columns', () => {
  const { project, dataset } = golden();
  const view = project.views.find((v) => v.name === 'Oswego');
  assert.ok(view, 'Oswego view present');
  assert.equal(view.records, 75);
  assert.equal(view.deleted_records, 0);

  const oswego = dataset('Oswego');
  const cols = new Map(oswego.columns.map((c) => [c.name, c]));
  for (const name of ['RecStatus', 'UniqueKey', 'GlobalRecordId', 'NAME', 'AGE', 'SEX', 'ILL', 'TIMESUPPER', 'DATEONSET', 'CODE_RW']) {
    assert.ok(cols.has(name), `column ${name}`);
  }
  assert.equal(cols.get('RecStatus').type, 'integer');
  assert.equal(cols.get('UniqueKey').type, 'integer');
  assert.equal(cols.get('GlobalRecordId').type, 'text');
  assert.equal(cols.get('AGE').type, 'integer');
  assert.equal(cols.get('ILL').type, 'boolean');
  assert.equal(cols.get('TIMESUPPER').type, 'date_time');
  assert.equal(cols.get('DATEONSET').type, 'date_time');
  assert.equal(cols.get('NAME').type, 'text');
  for (const c of oswego.columns) assert.equal(c.values.length, 75, c.name);
  // A known row: Patient45 (the first Oswego record by dump order).
  const i = cols.get('NAME').values.indexOf('Patient45');
  assert.ok(i >= 0, 'Patient45 row');
  assert.equal(cols.get('AGE').values[i], 20);
  assert.equal(cols.get('ILL').values[i], false);
  assert.equal(cols.get('TIMESUPPER').values[i], '1940-04-18 22:00:00');
  assert.equal(cols.get('DATEONSET').values[i], null);
  assert.match(view.hash, /^[0-9a-f]{64}$/);
});

test('code_table_becomes_label_set', () => {
  const { dataset } = golden();
  const oswego = dataset('Oswego');
  const sex = oswego.columns.find((c) => c.name === 'SEX');
  assert.equal(sex.meta.label_set, 'codeSex');
  const codeSex = oswego.label_sets.find((s) => s.id === 'codeSex');
  assert.ok(codeSex, 'codeSex label set emitted with the dataset');
  const values = codeSex.entries.map(([v]) => v.value);
  for (const v of ['Male', 'Female', 'Unknown']) assert.ok(values.includes(v), v);
  // Every stored SEX value is covered by the label set.
  for (const v of new Set(sex.values.filter(Boolean))) assert.ok(values.includes(v), v);
  // Codes fields (code column ≠ text column) keep the code→text pairs.
  const surveillance = dataset('Surveillance');
  const county = surveillance.columns.find((c) => c.name === 'CountyName');
  assert.ok(county?.meta?.label_set, 'CountyName carries a label set');
  const cnty = surveillance.label_sets.find((s) => s.id === county.meta.label_set);
  assert.ok(cnty.entries.some(([v, l]) => v.value === '44' && l === 'DEKALB'), 'code 44 → DEKALB');
});

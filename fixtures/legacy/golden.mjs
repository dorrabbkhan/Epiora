#!/usr/bin/env node
// Golden imports for the legacy Epi Info corpus (v1.1 Task 4).
//
//   node fixtures/legacy/golden.mjs --samples fixtures/legacy/samples --out fixtures/legacy/golden
//   node fixtures/legacy/golden.mjs --db <file> --out <dir>
//   node fixtures/legacy/golden.mjs --reimport --samples fixtures/legacy/samples --out fixtures/legacy/golden/reimport
//
// Reads every legacy project file with native mdbtools / sqlite3 dumps
// (never epiora-legacy), follows the Epi Info 7 layout from the
// Epi.Core source, and writes, per project, the project.json Task 7
// compares the mapper against plus one <view>.dataset.json per view —
// dataset JSON v2 with the engine's SHA-256 content hash recorded in
// project.json. `--reimport` writes the modified re-import copy of the
// Sample SQLite project (3 added, 2 changed, 1 deleted record, 1 added
// field) and its golden; expected-diff.json is authored, not produced.

import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { openSource } from './legacy-read.mjs';
import { unknownLayoutEntries } from './legacy-values.mjs';
import { buildEi7Project } from './ei7-project.mjs';
import { buildEi3Project } from './ei3-project.mjs';
import { datasetHashHex, datasetJson, writeJson } from './emit.mjs';

const args = process.argv.slice(2);
const opt = (name, dflt) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : dflt;
};
const has = (name) => args.includes(name);

function sha256File(file) {
  return createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}

// One legacy file → golden directory (project.json + dataset files).
export function goldenFile(file, outDir, sourceFormat) {
  const kind = /\.mdb$/i.test(file) ? 'mdb' : 'sqlite';
  const stem = path.basename(file).replace(/\.(mdb|db)$/i, '');
  const db = openSource(file, kind);
  const isEi7 = db.hasTable('metaViews');
  const isEi3 = kind === 'mdb' && !isEi7;
  // When the source lives inside the golden directory itself (the
  // reimport copy does), record the outDir-relative path so the golden
  // does not depend on where the directory was written.
  const outRel = path.relative(path.resolve(outDir), path.resolve(file));
  const relPath = (outRel.startsWith('..') ? path.relative(process.cwd(), file) : outRel)
    .split(path.sep).join('/');
  const project = isEi7
    ? buildEi7Project(db, { path: relPath, stem, jet: db.jet })
    : isEi3
      ? buildEi3Project(db, { path: relPath, stem, jet: db.jet })
      : { name: stem, legacy_id: null, datasets: [], views: [], links: [], programs: [], check_code: [], report: unknownLayoutEntries(db, stem), label_sets: [] };

  const format = sourceFormat || (kind === 'mdb' ? (isEi7 ? 'ei7_access' : 'ei3_access') : 'ei7_sqlite');
  fs.mkdirSync(outDir, { recursive: true });
  const labelSets = new Map((project.label_sets || []).map((s) => [s.id, s]));
  for (const ds of project.datasets) {
    const referenced = [...new Set(ds.columns.map((c) => c.meta?.label_set).filter(Boolean))];
    ds.label_sets = referenced.map((id) => labelSets.get(id)).filter(Boolean);
    const doc = datasetJson(ds);
    const hash = datasetHashHex(ds);
    const view = project.views.find((v) => v.dataset === ds.file);
    if (view) {
      view.hash = hash;
    } else {
      // Grid datasets are reached through a project link, not views[].
      const link = project.links.find((l) => l.child === ds.id);
      if (link) {
        link.dataset = ds.file;
        link.hash = hash;
      }
    }
    fs.writeFileSync(path.join(outDir, ds.file), writeJson(doc));
  }
  const projectJson = {
    name: project.name,
    legacy_id: project.legacy_id,
    source_format: format,
    source: {
      path: relPath,
      sha256: sha256File(file),
    },
    views: project.views,
    links: project.links,
    programs: project.programs.map((p) => ({
      name: p.name,
      text_sha256: createHash('sha256').update(p.content, 'utf8').digest('hex'),
    })),
    check_code: project.check_code,
    report: project.report,
  };
  fs.writeFileSync(path.join(outDir, 'project.json'), writeJson(projectJson));
  return { project: projectJson, datasets: project.datasets };
}

// ---- reimport copy -------------------------------------------------
// The modified copy of the Sample SQLite project, made entirely with
// fixed edits: the first three Oswego GlobalRecordIds (lexical order)
// are removed/changed, three new rows carry fixed GUIDs, and one text
// field is added. expected-diff.json pins these ids.
const REIMPORT = {
  added: [
    ['a0000000-0000-4000-8000-000000000001', 'Reimport Added One', 30, 'Male'],
    ['a0000000-0000-4000-8000-000000000002', 'Reimport Added Two', 41, 'Female'],
    ['a0000000-0000-4000-8000-000000000003', 'Reimport Added Three', 55, 'Male'],
  ],
  addedField: 'ReimportFlag',
};
function buildReimport(srcDb, outDir, sqlite3) {
  fs.mkdirSync(outDir, { recursive: true });
  const out = path.join(outDir, 'SampleDB.db');
  fs.copyFileSync(srcDb, out);
  const sql = (s) => execFileSync(sqlite3, [out, s], { encoding: 'utf8' });
  const gids = sql(
    "SELECT GlobalRecordId FROM Oswego12 ORDER BY GlobalRecordId LIMIT 3"
  ).trim().split('\n').filter(Boolean);
  if (gids.length < 3) throw new Error('reimport source has < 3 Oswego rows');
  const [rem, ch1, ch2] = gids;
  const newId = Number(sql('SELECT MAX(FieldId) FROM metaFields').trim()) + 1;
  sql(`
    INSERT INTO metaFields (FieldId, UniqueId, Name, ViewId, PageId, FieldTypeId, PromptText, DataTableName)
    VALUES (${newId}, '{${'0'.repeat(32 - String(newId).length)}${newId}}', '${REIMPORT.addedField}', 12, 12, 1, 'Added by the golden reimport fixture', 'Oswego');
    ALTER TABLE Oswego12 ADD COLUMN "${REIMPORT.addedField}" TEXT;
    DELETE FROM Oswego WHERE GlobalRecordId = '${rem}';
    DELETE FROM Oswego12 WHERE GlobalRecordId = '${rem}';
    UPDATE Oswego12 SET AGE = 99 WHERE GlobalRecordId = '${ch1}';
    UPDATE Oswego12 SET ILL = 1 WHERE GlobalRecordId = '${ch2}';
    UPDATE Oswego12 SET "${REIMPORT.addedField}" = 'edited' WHERE GlobalRecordId IN ('${ch1}', '${ch2}');
  `);
  const maxKey = Number(sql('SELECT MAX(UniqueKey) FROM Oswego').trim());
  REIMPORT.added.forEach(([guid, name, age, sex], i) => {
    sql(`
      INSERT INTO Oswego (GlobalRecordId, UniqueKey, RECSTATUS) VALUES ('${guid}', ${maxKey + i + 1}, 1);
      INSERT INTO Oswego12 (GlobalRecordId, NAME, AGE, SEX, "${REIMPORT.addedField}") VALUES ('${guid}', '${name}', ${age}, '${sex}', 'new');
    `);
  });
  return { removed: rem, changed: [ch1, ch2], added: REIMPORT.added.map((a) => a[0]) };
}

function* walk(dir) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) yield* walk(p);
    else if (/\.(mdb|db)$/i.test(e.name)) yield p;
  }
}

function main() {
  const samples = opt('--samples', 'fixtures/legacy/samples');
  const out = opt('--out', 'fixtures/legacy/golden');
  const single = opt('--db', null);
  if (single) {
    goldenFile(path.resolve(single), path.resolve(out), null);
    return;
  }
  if (has('--reimport')) {
    const src = path.join(samples, 'ei7', 'Sample', 'SampleDB.db');
    const gids = buildReimport(src, path.resolve(out), process.env.SQLITE3 || 'sqlite3');
    goldenFile(path.join(out, 'SampleDB.db'), path.resolve(out), 'ei7_sqlite');
    fs.writeFileSync(
      path.join(out, 'produced-ids.json'),
      writeJson(gids)
    );
    return;
  }
  for (const file of walk(path.resolve(samples))) {
    const rel = path.relative(samples, file).split(path.sep).join('/');
    const dest = path.join(path.resolve(out), path.dirname(rel), path.basename(file).replace(/\.(mdb|db)$/i, ''));
    goldenFile(file, dest, null);
  }
}

if (process.argv[1] && import.meta.url === `file://${process.argv[1]}`) main();

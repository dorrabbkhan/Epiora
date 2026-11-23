#!/usr/bin/env node
// Cross-backend golden comparison (v1.1 Task 4): two golden
// directories for the same legacy project — one produced from the
// Access file, one from the verbatim SQLite copy — must emit the same
// datasets and project metadata. `source` (path + sha256) and
// `source_format` are provenance and are excluded; everything else,
// including report order and label sets, compares byte-for-byte.
//
//   node fixtures/legacy/compare-golden.mjs --a <dir> --b <dir>

import * as fs from 'node:fs';
import * as path from 'node:path';

const args = process.argv.slice(2);
const opt = (name) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : null;
};
const a = opt('--a');
const b = opt('--b');
if (!a || !b) {
  console.error('usage: compare-golden.mjs --a <dir> --b <dir>');
  process.exit(2);
}

let failures = 0;
const fail = (msg) => {
  failures++;
  console.error(`FAIL ${msg}`);
};

const pa = JSON.parse(fs.readFileSync(path.join(a, 'project.json'), 'utf8'));
const pb = JSON.parse(fs.readFileSync(path.join(b, 'project.json'), 'utf8'));
for (const p of [pa, pb]) {
  delete p.source;
  p.source_format = 'compare';
}
if (JSON.stringify(pa) !== JSON.stringify(pb)) {
  fail('project.json differs beyond source provenance');
}

const dsFiles = fs.readdirSync(a).filter((f) => f.endsWith('.dataset.json'));
for (const f of dsFiles) {
  const fa = path.join(a, f);
  const fb = path.join(b, f);
  if (!fs.existsSync(fb)) {
    fail(`${f}: missing from ${b}`);
    continue;
  }
  if (fs.readFileSync(fa, 'utf8') !== fs.readFileSync(fb, 'utf8')) {
    fail(`${f}: dataset bytes differ`);
  }
}
for (const f of fs.readdirSync(b).filter((f) => f.endsWith('.dataset.json'))) {
  if (!fs.existsSync(path.join(a, f))) fail(`${f}: extra dataset in ${b}`);
}

if (failures > 0) {
  console.error(`${failures} comparison failure(s)`);
  process.exit(1);
}
console.log(`${path.basename(a)}: ${dsFiles.length} dataset(s) + project.json identical across backends`);

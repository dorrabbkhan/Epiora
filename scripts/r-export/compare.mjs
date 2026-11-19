#!/usr/bin/env node
// scripts/r-export/compare.mjs — checks every `out.txt` the R run
// produced against the case's `expected.json` (v1.0 Task 17,
// spec §8.3). `kind: "stats"` compares each `EPIORA <path> <value>`
// line to the fixture leaf under E7 (|a−e| ≤ tol·|e|, or |a| ≤ tol
// when e = 0; the leaf's class picks tol). `kind: "dataset"` matches
// `EPIORA value <col>[<row>] <json>` / `EPIORA missing <col>[<row>]`
// lines exactly — the hostile-text round trip.
//
// Usage: node scripts/r-export/compare.mjs <out-dir>

import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

const root = process.argv[2];
if (!root) {
  console.error('usage: compare.mjs <out-dir>');
  process.exit(2);
}

const TOL = { closed_form: 1e-9, iterative: 1e-6 };

function* caseDirs(dir) {
  for (const name of readdirSync(dir).sort()) {
    const p = join(dir, name);
    if (!statSync(p).isDirectory()) continue;
    if (statSync(join(p, 'expected.json'), { throwIfNoEntry: false })) {
      yield p;
    } else {
      yield* caseDirs(p);
    }
  }
}

const failures = [];
let checked = 0;

for (const dir of caseDirs(root)) {
  const rel = dir.slice(root.length + 1);
  const expected = JSON.parse(readFileSync(join(dir, 'expected.json'), 'utf8'));
  const lines = readFileSync(join(dir, 'out.txt'), 'utf8')
    .split('\n')
    .filter((l) => l.startsWith('EPIORA '))
    .map((l) => l.slice('EPIORA '.length));
  if (expected.kind === 'stats') {
    const got = new Map();
    for (const rest of lines) {
      const sp = rest.indexOf(' ');
      got.set(rest.slice(0, sp), rest.slice(sp + 1));
    }
    for (const [path, leaf] of Object.entries(expected.values)) {
      const line = got.get(path);
      if (line === undefined) {
        failures.push(`${rel}: missing EPIORA line for ${path}`);
        continue;
      }
      if (leaf.value === null) {
        if (line !== 'NULL') {
          failures.push(`${rel}: ${path}: expected NULL, got ${line}`);
        }
        continue;
      }
      const actual = Number(line);
      if (!Number.isFinite(actual)) {
        failures.push(`${rel}: ${path}: line is not a number: ${line}`);
        continue;
      }
      const tol = TOL[leaf.class];
      const e = leaf.value;
      const ok = e === 0 ? Math.abs(actual) <= tol
                         : Math.abs(actual - e) <= tol * Math.abs(e);
      if (!ok) {
        failures.push(
          `${rel}: ${path}: |${actual} - ${e}| > ${tol}*|${e}| (${leaf.class})`,
        );
      }
      checked += 1;
    }
  } else if (expected.kind === 'dataset') {
    const got = new Map();
    for (const rest of lines) {
      // "value <col>[<row>] <json>" or "missing <col>[<row>]".
      const m = rest.match(/^(value|missing) (\S+)(?: (.*))?$/s);
      if (m) got.set(m[2], { kind: m[1], value: m[3] });
    }
    for (const [key, cell] of Object.entries(expected.cells)) {
      const line = got.get(key);
      if (cell === null) {
        if (!line || line.kind !== 'missing') {
          failures.push(`${rel}: ${key}: expected missing, got ${JSON.stringify(line)}`);
        }
        continue;
      }
      if (!line || line.kind !== 'value') {
        failures.push(`${rel}: ${key}: expected ${JSON.stringify(cell)}, got ${JSON.stringify(line)}`);
        continue;
      }
      let parsed;
      try {
        parsed = JSON.parse(line.value);
      } catch {
        failures.push(`${rel}: ${key}: value is not JSON: ${line.value}`);
        continue;
      }
      if (JSON.stringify(parsed) !== JSON.stringify(cell)) {
        failures.push(`${rel}: ${key}: ${JSON.stringify(parsed)} != ${JSON.stringify(cell)}`);
      }
      checked += 1;
    }
  }
}
if (failures.length > 0) {
  console.error(`compare: ${failures.length} failures:`);
  for (const f of failures.slice(0, 50)) console.error(`  ${f}`);
  process.exit(1);
}
console.log(`compare: ${checked} statistics verified`);

#!/usr/bin/env node
// app/perf/gen-csv.mjs <rows> <out> — the deterministic perf CSV
// (v0.3 plan Task 16). Seeded xorshift32, seed 20261105; the same
// <rows> always produces byte-identical output, so a CI run and a
// local run measure the same file.
//
// Columns (10): id Integer 1..n · exposure Integer 0/1 · outcome
// Integer 0/1 · grp 3-level Text · flag Boolean · measure1 Real ·
// measure2 Real · event_date ISO Date · age Integer · note Text with
// 5% missing. exposure/outcome are the two binary columns the
// open_tables scenario cross-tabulates.

import { createWriteStream } from 'node:fs';
import { mkdirSync } from 'node:fs';
import { dirname } from 'node:path';
import { once } from 'node:events';

const SEED = 20261105;
const DAY_MS = 86_400_000;
const DATE_EPOCH = Date.UTC(2020, 0, 1); // 2020-01-01
const DATE_SPAN_DAYS = 2192; // ~6 years
const CHUNK = 8192;

/** xorshift32 → unsigned int; s must stay non-zero. */
function makeRng(seed) {
  let s = seed >>> 0;
  return () => {
    s ^= s << 13;
    s ^= s >>> 17;
    s ^= s << 5;
    return s >>> 0;
  };
}

const usage = 'usage: node gen-csv.mjs <rows> <out>';
const rows = Number(process.argv[2]);
const out = process.argv[3];
if (!Number.isSafeInteger(rows) || rows < 1 || out === undefined) {
  console.error(usage);
  process.exit(2);
}

const rng = makeRng(SEED);
/** Uniform in [0, 1). */
const u = () => rng() / 0x1_0000_0000;

mkdirSync(dirname(out), { recursive: true });
const stream = createWriteStream(out);
stream.write('id,exposure,outcome,grp,flag,measure1,measure2,event_date,age,note\n');

const groups = ['alpha', 'beta', 'gamma'];
let pending = '';
for (let id = 1; id <= rows; id++) {
  const exposure = rng() & 1;
  const outcome = rng() & 1;
  const grp = groups[Math.floor(u() * 3)];
  const flag = u() < 0.5 ? 'true' : 'false';
  const measure1 = (u() * 1000).toFixed(4);
  const measure2 = (u() * 50).toFixed(2);
  const day = Math.floor(u() * DATE_SPAN_DAYS);
  const eventDate = new Date(DATE_EPOCH + day * DAY_MS).toISOString().slice(0, 10);
  const age = Math.floor(u() * 100);
  const note = u() < 0.05 ? '' : `note-${id}`;
  pending += `${id},${exposure},${outcome},${grp},${flag},${measure1},${measure2},${eventDate},${age},${note}\n`;
  if (id % CHUNK === 0) {
    if (!stream.write(pending)) await once(stream, 'drain');
    pending = '';
  }
}
if (pending !== '') stream.write(pending);
stream.end();
await once(stream, 'finish');

// Drives the wasm parity build over the corpus and prints one
// parity_line per fixture — the same bytes `corpus-runner` prints
// natively (spec §5.2). Usage:
//
//   node crates/epiora-wasm/parity/run.mjs <fixtures-dir>
//
// Fixture order matches corpus-runner: families in FAMILIES order,
// paths sorted within each family.

import { createRequire } from "node:module";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const require = createRequire(import.meta.url);
const wasm = require("../pkg-parity/epiora_wasm.js");

const FAMILIES = ["special", "2x2", "stratified", "rxc", "freq"];

const root = process.argv[2];
if (!root) {
  console.error("usage: run.mjs <fixtures-dir>");
  process.exit(2);
}

// runCorpusCase takes the datasets as one JSON object keyed by id —
// the same *.dataset.json documents corpus-runner loads.
const datasets = {};
const dataDir = join(root, "data");
for (const file of readdirSync(dataDir).sort()) {
  if (file.endsWith(".dataset.json")) {
    const doc = JSON.parse(readFileSync(join(dataDir, file), "utf8"));
    datasets[doc.id] = doc;
  }
}
const datasetsJson = JSON.stringify(datasets);

const lines = [];
for (const family of FAMILIES) {
  const dir = join(root, family);
  if (!existsSync(dir)) continue;
  for (const file of readdirSync(dir).sort()) {
    if (!file.endsWith(".json")) continue;
    const fixtureJson = readFileSync(join(dir, file), "utf8");
    lines.push(wasm.runCorpusCase(fixtureJson, datasetsJson));
  }
}
process.stdout.write(lines.join("\n") + "\n");

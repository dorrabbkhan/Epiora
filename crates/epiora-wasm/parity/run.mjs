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

// Must match `corpus-runner`'s FAMILIES exactly — `calc` families are
// per calculator; `calc/random_numbers` has no fixture directory (its
// coverage is the golden file), so the missing-dir skip is load-bearing.
const FAMILIES = [
  "special",
  "2x2",
  "stratified",
  "rxc",
  "freq",
  "means",
  "calc/proportion",
  "calc/two_by_two",
  "calc/two_by_n",
  "calc/r_by_c",
  "calc/dose_response",
  "calc/matched_case_control",
  "calc/screening",
  "calc/smr",
  "calc/binomial",
  "calc/poisson",
  "calc/rate_one",
  "calc/rate_compare",
  "calc/mean_ci",
  "calc/median_ci",
  "calc/t_test_summary",
  "calc/anova_summary",
  "calc/ss_proportion",
  "calc/ss_unmatched_cc",
  "calc/ss_cohort",
  "calc/ss_mean_difference",
  "calc/power_unmatched_cc",
  "calc/power_cohort",
  "calc/power_mean_difference",
  "calc/random_numbers",
];

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
    // runCorpusCase returns the case line plus the Epi Info command
    // line of dataset cases (v1.0 Task 11), `\n`-separated.
    lines.push(...wasm.runCorpusCase(fixtureJson, datasetsJson).split("\n"));
  }
}

// Trailing explain section: one explain trace per registry
// statistic, in registry order — the same lines corpus-runner
// prints (v1.0 Task 11). Keys are `dir/id` and may root through `..`
// (the calc.random_numbers golden under crates/).
for (const [statistic, key] of wasm.explainCorpusPlan()) {
  const fixtureJson = readFileSync(join(root, `${key}.json`), "utf8");
  lines.push(wasm.runCorpusExplainLine(fixtureJson, datasetsJson, statistic));
}
process.stdout.write(lines.join("\n") + "\n");

// Chromium performance gate (spec §5.3): 1,000,000 rows, seed
// 20261031, `datasetFromColumns` + stratified `tables` (EXPOSED × ILL
// by REGION), 5 runs, median must be < 1000 ms. Row generation in the
// page is untimed; only datasetFromColumns + tables is inside the
// timer. Writes bench-results.json { seed, rows, runs_ms, median_ms,
// chromium_version, runner }.
//
//   npx playwright test            (from crates/epiora-wasm/bench)
//   scripts/bench.sh               (build pkg + install + run)

import { test, expect } from "@playwright/test";
import { writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { generate, SEED } from "./gen.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const ROWS = 1_000_000;
const BUDGET_MS = 1000;

test("generator is deterministic", () => {
  const a = generate(1000, SEED);
  const b = generate(1000, SEED);
  for (let i = 0; i < a.length; i++) {
    expect(a[i].name).toBe(b[i].name);
    expect(a[i].type).toBe(b[i].type);
    if (a[i].type === "text") {
      expect(a[i].dictionary).toEqual(b[i].dictionary);
      expect([...a[i].codes]).toEqual([...b[i].codes]);
    } else {
      expect([...a[i].values]).toEqual([...b[i].values]);
    }
  }
  // First 3 EXPOSED values, pinned from an actual run:
  //   node -e 'import("./gen.mjs").then(({generate}) =>
  //     console.log([...generate(1000, 20261031)[0].values.slice(0,3)]))'
  //   → [ 1, 1, 1 ]
  expect([...a[0].values.slice(0, 3)]).toEqual([1, 1, 1]);
});

test("1M rows: datasetFromColumns + stratified tables median < 1000 ms", async ({
  page,
  browserName,
}, testInfo) => {
  expect(browserName).toBe("chromium");
  const response = await page.goto("/bench/bench.html");
  expect(response.status(), "bench.html must be served").toBe(200);
  await page.waitForFunction(() => globalThis.__bench?.ready === true);

  // Untimed: generate the 1,000,000 rows inside the page.
  await page.evaluate(
    ([n, seed]) => {
      globalThis.__cols = globalThis.__bench.generate(n, seed);
    },
    [ROWS, SEED],
  );

  // Timed: datasetFromColumns + stratified tables, five runs.
  const runs = [];
  for (let i = 0; i < 5; i++) {
    const run = await page.evaluate(() =>
      globalThis.__bench.timedRun(globalThis.__cols),
    );
    expect(run.kind, `run ${i + 1}: tables must return TwoByTwo`).toBe(
      "two_by_two",
    );
    expect(run.strata, `run ${i + 1}: three strata`).toHaveLength(3);
    runs.push(run.ms);
  }

  const sorted = [...runs].sort((a, b) => a - b);
  const median = sorted[Math.floor(sorted.length / 2)];
  const results = {
    seed: SEED,
    rows: ROWS,
    runs_ms: runs,
    median_ms: median,
    chromium_version: page.context().browser().version(),
    runner: process.env.RUNNER_NAME ?? "local",
  };
  writeFileSync(
    join(here, "bench-results.json"),
    JSON.stringify(results, null, 2) + "\n",
  );
  console.log(
    `runs_ms=${runs.map((m) => m.toFixed(1)).join(", ")} ` +
      `median_ms=${median.toFixed(1)} chromium=${results.chromium_version} ` +
      `runner=${results.runner}`,
  );
  expect(median, `median ${median} ms must be under the ${BUDGET_MS} ms budget`).toBeLessThan(
    BUDGET_MS,
  );
});

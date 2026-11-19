// `means_1m` (v1.0 Task 18, spec §10) — time from the `runMeans`
// call to resolve on the same 1M-row perf dataset the TABLES
// scenario uses (`gen-csv.mjs`, seed 20261105), for
// `MEANS age grp` — the Integer `age` column by the 3-level Text
// `grp` column — after clearing the worker's handle LRU, so the
// sample pays a cold snapshot load + hash check + MEANS, the same
// shape `open_tables_1m` measures. Gated since v1.0 Task 19 at the
// owner-picked 3,173 ms (median of 5, gate-eval.ts's GATES — PR #94).
// The D7 throttled pass stays the two gated scenarios only.

import { label, runScenario, scenarioEnabled, skipScenario } from './shared';
import type { SampleCtx } from './shared';

export async function runMeans(ctx: SampleCtx): Promise<void> {
  if (ctx.gatedOnly || !scenarioEnabled(`means_${label}`)) return;
  if (ctx.datasetId === null) {
    skipScenario(ctx, `means_${label}`, `import_${label} failed`);
    return;
  }
  const did = ctx.datasetId;
  await runScenario(ctx, `means_${label}`, async () => {
    const ms = await ctx.page.evaluate(
      async ({ projectId: pid, datasetId: dataset }) => {
        const api = window.dataApi;
        const { currentVersion } = await api.getDataset(dataset);
        await api.__clearSnapshotCache();
        const t0 = performance.now();
        await api.runMeans(pid, dataset, currentVersion, {
          variable: 'age',
          group_by: 'grp',
          include_missing: false,
        });
        return performance.now() - t0;
      },
      { projectId: ctx.projectId, datasetId: did },
    );
    return { ms };
  });
}

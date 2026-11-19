// `open_tables_1m` — the plan's TABLES scenario: cold snapshot load
// (handle LRU cleared) + hash check + 2×2 TABLES of the two binary
// columns on the 1M-row perf dataset. Split out of perf.spec.ts in
// v1.0 Task 18; behaviour unchanged.

import { label, runScenario, scen, scenarioEnabled, skipScenario } from './shared';
import type { SampleCtx } from './shared';

export async function runTables(ctx: SampleCtx): Promise<void> {
  if (!scenarioEnabled(`open_tables_${label}`)) return;
  if (ctx.datasetId === null) {
    skipScenario(ctx, scen(`open_tables_${label}`), `import_${label} failed`);
    return;
  }
  const did = ctx.datasetId;
  await runScenario(ctx, scen(`open_tables_${label}`), async () => {
    const ms = await ctx.page.evaluate(
      async ({ projectId: pid, datasetId: dataset }) => {
        const api = window.dataApi;
        const { currentVersion } = await api.getDataset(dataset);
        await api.__clearSnapshotCache();
        const t0 = performance.now();
        await api.runTables(pid, dataset, currentVersion, {
          exposure: { name: 'exposure', levels: null },
          outcome: { name: 'outcome', levels: null },
          stratify_by: [],
          include_missing: false,
        });
        return performance.now() - t0;
      },
      { projectId: ctx.projectId, datasetId: did },
    );
    return { ms };
  });
}

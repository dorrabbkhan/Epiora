// Import scenarios — `import_1m` (time from commitImport call to
// resolve on the big generated CSV) and `reimport_1m` (carryover from
// v0.3 Task 9: re-import with one field edited — `records` is deleted
// and fully rewritten in the new file order). Split out of
// perf.spec.ts in v1.0 Task 18; behaviour unchanged.

import {
  importCsv,
  label,
  runScenario,
  scen,
  scenarioEnabled,
  skipScenario,
  startOpfsPoller,
} from './shared';
import type { SampleCtx } from './shared';

export async function runImport(ctx: SampleCtx): Promise<void> {
  if (scenarioEnabled(`import_${label}`)) {
    if (ctx.projectId === '') {
      skipScenario(ctx, scen(`import_${label}`), 'create_project failed');
    } else {
      await runScenario(ctx, scen(`import_${label}`), async () => {
        const poller = startOpfsPoller(ctx.page, ctx.growthLog);
        try {
          const imported = await importCsv(ctx.page, '/perf-csv/big', {
            projectId: ctx.projectId,
            datasetName: `perf-${label}.csv`,
          });
          ctx.datasetId = imported.datasetId;
          return { ms: imported.ms };
        } finally {
          poller.stop();
        }
      });
      ctx.growthLog.length = 0;
    }
  }

  if (ctx.gatedOnly || !scenarioEnabled(`reimport_${label}`)) return;
  if (ctx.datasetId === null) {
    // The 1M dataset is the fixture for the dependent scenarios; each
    // dependent records its own skip rather than vanishing.
    skipScenario(ctx, `reimport_${label}`, `import_${label} failed`);
    return;
  }
  const did = ctx.datasetId;
  await runScenario(ctx, `reimport_${label}`, async () => {
    const poller = startOpfsPoller(ctx.page, ctx.growthLog);
    try {
      const reimported = await importCsv(ctx.page, '/perf-csv/edited', {
        datasetId: did,
      });
      return { ms: reimported.ms };
    } finally {
      poller.stop();
    }
  });
  ctx.growthLog.length = 0;
}

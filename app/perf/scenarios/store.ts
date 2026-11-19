// Store-level scenarios — `empty_store_opfs` (carryover from v0.3
// Task 9: OPFS usage of an empty store; the store opens — migrations
// + sahpool reservation — on the first store RPC) and the
// `create_project` the import/table/mean scenarios depend on.
// Split out of perf.spec.ts in v1.0 Task 18; behaviour unchanged.

import { errorText, failureDiagnostics, record, runScenario, scenarioEnabled } from './shared';
import type { SampleCtx } from './shared';

export async function runStore(ctx: SampleCtx): Promise<void> {
  if (!ctx.gatedOnly && scenarioEnabled('empty_store_opfs')) {
    await runScenario(ctx, 'empty_store_opfs', async () => {
      const empty = await ctx.page.evaluate(async () => {
        const api = window.dataApi;
        await api.listProjects();
        return api.storageEstimate();
      });
      return { usageBytes: empty.usage, quotaBytes: empty.quota };
    });
  }

  // Not a measured scenario — recorded only on failure.
  try {
    ctx.projectId = await ctx.page.evaluate(
      async () => (await window.dataApi.createProject('Perf')).id,
    );
  } catch (e) {
    record({
      run: ctx.run,
      scenario: 'create_project',
      error: errorText(e),
      ...(await failureDiagnostics(ctx.page, ctx.consoleLines, ctx.growthLog)),
    });
  }
}

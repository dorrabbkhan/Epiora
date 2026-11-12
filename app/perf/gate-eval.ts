// Perf gate evaluation and batch id (v0.3 plan Task 17, spec §10.3) —
// extracted from perf.spec.ts so app/scripts/perf-gate.test.mjs (the
// `node --test scripts/*.test.mjs` lane of check-app.sh) exercises
// the real gate code; the spec imports these and only does the IO
// (results.jsonl in, gates.md out) and the Playwright expect.
//
// Erasable syntax only: the node:test lane imports this file
// directly and Node strips the types at load.

// Owner-picked thresholds (PR #52 comment, 2026-11-11:
// https://github.com/dorrabbkhan/Epiora/pull/52#issuecomment-6807191386)
// — basis is p90 × 1.25 of the persistent-profile run 43025510601. A
// scenario passes when the median `ms` of its successful samples is
// ≤ the threshold; a scenario with no successful sample cannot pass.
// The gates evaluate the `1m` scenario names only: a run at another
// PERF_ROWS records `*_<label>` names the gate does not apply.
export const GATES: Record<string, number> = {
  import_1m: 89_550, // p90 71,639.3 ms × 1.25
  open_tables_1m: 2_874, // p90 2,299.2 ms × 1.25
};

export type PerfLine = Record<string, unknown> & {
  scenario?: string;
  ms?: number;
  error?: string;
  skipped?: string;
};

/** The batch id stamped on every recorded line — the gate test
 *  evaluates this invocation's samples only, so a stale
 *  results.jsonl from an earlier local run can't pass or fail it.
 *  It must be shared by every worker of the invocation, including
 *  the replacement Playwright spawns when a sample times out:
 *  `pid-${process.pid}` scoped it to one worker, so a replacement
 *  worker's gate saw none of the dead worker's samples (main run
 *  43208388072 attempt 1). The Playwright runner is every worker's
 *  parent, so the parent's pid is the invocation id. PERF_BATCH
 *  overrides it (CI sets it from run id + attempt + pass; it also
 *  exercises the gate against a hand-built file). */
export function batchId(
  env: NodeJS.ProcessEnv,
  proc: { pid: number; ppid: number },
): string {
  return env.PERF_BATCH ?? `run-${proc.ppid}`;
}

export function medianOf(samples: number[]): number {
  const sorted = [...samples].sort((a, b) => a - b);
  const n = sorted.length;
  const mid = Math.floor(n / 2);
  return n % 2 === 1 ? sorted[mid]! : (sorted[mid - 1]! + sorted[mid]!) / 2;
}

/** Evaluates the owner gates against `lines` (a parsed
 *  results.jsonl) for the given batch/profile/label. Returns the
 *  gates.md markdown and the failure list — empty means pass. */
export function evaluateGates(
  lines: PerfLine[],
  opts: { batch: string; profile: string; label: string },
): { markdown: string; failures: string[] } {
  const mine = lines.filter(
    (l) => l.batch === opts.batch && l.profile === opts.profile,
  );

  const out = [
    '### Perf gates — owner thresholds ([PR #52](https://github.com/dorrabbkhan/Epiora/pull/52#issuecomment-6807191386))',
    '',
    'A scenario passes when the median `ms` of its successful samples is ≤ the threshold.',
    '',
  ];
  const failures: string[] = [];

  if (opts.label !== '1m') {
    out.push(
      `Not evaluated — scenarios are \`*_${opts.label}\`; the owner thresholds ` +
        'apply to `import_1m` / `open_tables_1m` only.',
    );
    return { markdown: `${out.join('\n')}\n`, failures };
  }

  out.push('| scenario | ok | failed | skipped | median ms | threshold ms | verdict |');
  out.push('|---|---|---|---|---|---|---|');
  for (const [name, limit] of Object.entries(GATES)) {
    const ok = mine
      .filter(
        (l) =>
          l.scenario === name &&
          l.error === undefined &&
          l.skipped === undefined &&
          typeof l.ms === 'number',
      )
      .map((l) => l.ms as number);
    const failed = mine.filter(
      (l) => l.scenario === name && l.error !== undefined,
    ).length;
    const skippedN = mine.filter(
      (l) => l.scenario === name && l.skipped !== undefined,
    ).length;
    if (ok.length === 0) {
      failures.push(`${name}: no successful sample`);
      out.push(`| ${name} | 0 | ${failed} | ${skippedN} | — | ${limit} | FAIL |`);
      continue;
    }
    const med = medianOf(ok);
    const pass = med <= limit;
    if (!pass) failures.push(`${name}: median ${med} ms > ${limit} ms`);
    out.push(
      `| ${name} | ${ok.length} | ${failed} | ${skippedN} | ${med} | ${limit} | ${pass ? 'PASS' : 'FAIL'} |`,
    );
  }
  return { markdown: `${out.join('\n')}\n`, failures };
}

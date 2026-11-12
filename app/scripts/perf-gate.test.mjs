// Tests for the perf gate's batch id and evaluation
// (app/perf/gate-eval.ts, used by app/perf/perf.spec.ts).
//
// The bug these pin down — main run
// https://github.com/dorrabbkhan/Epiora/actions/runs/43208388072
// attempt 1: samples 1–4 passed under the first Playwright worker,
// sample 5 hit the 1,800,000 ms test timeout, Playwright replaced
// the worker, and the gate test ran in the new process. With
// `pid-${process.pid}` batch ids the gate saw none of the dead
// worker's samples and reported "import_1m: no successful sample;
// open_tables_1m: no successful sample". The batch id must be one
// per Playwright invocation — every worker shares the runner as its
// parent process, so ppid is the invocation id.
//
// Run: `node --test app/scripts` (check-app.sh runs this lane).

import assert from 'node:assert/strict';
import test from 'node:test';
import { batchId, evaluateGates } from '../perf/gate-eval.ts';

test('gate counts samples recorded by an earlier worker of the same invocation', () => {
  // Worker 1 records its samples and dies on a timeout; Playwright's
  // replacement worker — a different pid, the same parent runner —
  // runs the gate. Its samples must still count.
  const worker1 = { pid: 11_111, ppid: 4_242 };
  const gateWorker = { pid: 22_222, ppid: 4_242 };
  const recorded = batchId({}, worker1);
  const gateBatch = batchId({}, gateWorker);
  assert.notEqual(
    worker1.pid,
    gateWorker.pid,
    'the scenario requires distinct worker pids',
  );

  const lines = [
    { batch: recorded, profile: 'persistent', run: 1, scenario: 'import_1m', ms: 60_000 },
    { batch: recorded, profile: 'persistent', run: 2, scenario: 'import_1m', ms: 62_000 },
    { batch: recorded, profile: 'persistent', run: 1, scenario: 'open_tables_1m', ms: 2_000 },
    { batch: recorded, profile: 'persistent', run: 2, scenario: 'open_tables_1m', ms: 2_100 },
  ];
  const { markdown, failures } = evaluateGates(lines, {
    batch: gateBatch,
    profile: 'persistent',
    label: '1m',
  });
  assert.deepEqual(failures, [], failures.join('; '));
  assert.match(markdown, /\| import_1m \| 2 \|/);
  assert.match(markdown, /\| open_tables_1m \| 2 \|/);
});

test('batch id is per-invocation; stale runs are still excluded', () => {
  const invocation = { pid: 10, ppid: 100 };
  const nextInvocation = { pid: 10, ppid: 101 }; // pid reuse, new runner
  assert.notEqual(batchId({}, invocation), batchId({}, nextInvocation));

  // A stale line from another invocation never counts.
  const batch = batchId({}, invocation);
  const lines = [
    { batch, profile: 'persistent', run: 1, scenario: 'import_1m', ms: 60_000 },
    { batch, profile: 'persistent', run: 1, scenario: 'open_tables_1m', ms: 2_000 },
    // If this stale line were counted, import_1m's median would
    // move above the 89,550 ms threshold.
    { batch: batchId({}, nextInvocation), profile: 'persistent', run: 9, scenario: 'import_1m', ms: 9_000_000 },
    { batch: 'pid-777', profile: 'persistent', run: 9, scenario: 'import_1m', ms: 9_000_000 },
  ];
  const { markdown, failures } = evaluateGates(lines, {
    batch,
    profile: 'persistent',
    label: '1m',
  });
  assert.deepEqual(failures, [], failures.join('; '));
  assert.match(markdown, /\| import_1m \| 1 \|/);
});

test('PERF_BATCH overrides the invocation id', () => {
  assert.equal(
    batchId({ PERF_BATCH: 'ci-1-x1' }, { pid: 10, ppid: 100 }),
    'ci-1-x1',
  );
});

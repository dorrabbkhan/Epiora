// Tests for the v0.3 validation report (v0.3 plan Task 18, spec
// §10.5): `validation-report.mjs <run-url>` renders
// docs/validation/v0.3.md from the source run's artifacts and job
// logs. `buildReport(input)` is the pure renderer — the test feeds it
// raw texts shaped like the real CI outputs and checks every gate,
// every section and the earlier-attempt facts.
//
// Run: `node --test app/scripts`.

import assert from 'node:assert/strict';
import test from 'node:test';

import { buildReport } from './validation-report.mjs';

const SOURCE_URL = 'https://github.com/dorrabbkhan/Epiora/actions/runs/43208388072';
const SHA = 'e19042cdde36954b40461e390ebcbae5abfbbd0b';
const GENERATED_BY =
  'https://github.com/dorrabbkhan/Epiora/actions/runs/99999999999';

const GATES_MD = [
  '### Perf gates — owner thresholds ([PR #52](https://github.com/dorrabbkhan/Epiora/pull/52#issuecomment-6807191386))',
  '',
  'A scenario passes when the median `ms` of its successful samples is ≤ the threshold.',
  '',
  '| scenario | ok | failed | skipped | median ms | threshold ms | verdict |',
  '|---|---|---|---|---|---|---|',
  '| import_1m | 5 | 0 | 0 | 71621.80000000005 | 89550 | PASS |',
  '| open_tables_1m | 5 | 0 | 0 | 2263.3000000000466 | 2874 | PASS |',
  '',
].join('\n');

const SUMMARY_MD = [
  'profile(s): persistent',
  '',
  '| scenario | ok | failed | skipped | field | median | p90 |',
  '|---|---|---|---|---|---|---|',
  '| empty_store_opfs | 5 | 0 | 0 | quotaBytes | 10808174480 | 10808174488 |',
  '| empty_store_opfs | 5 | 0 | 0 | usageBytes | 70756240 | 70756248 |',
  '| grid_scroll_10k | 5 | 0 | 0 | frameMedianMs | 5 | 5.5 |',
  '| grid_scroll_1m | 0 | 5 | 0 | — | — | — |',
  '| import_1m | 5 | 0 | 0 | ms | 71621.8 | 71937.9 |',
  '| import_1m_cpu4 | 3 | 0 | 0 | ms | 79425.5 | 79965.7 |',
  '| open_tables_1m | 5 | 0 | 0 | ms | 2263.3 | 2276.1 |',
  '| open_tables_1m_cpu4 | 3 | 0 | 0 | ms | 2980.3 | 3311.7 |',
  '| reimport_1m | 5 | 0 | 0 | ms | 48300.8 | 48444.4 |',
  '',
  '### Failures',
  '- run 1 · grid_scroll_1m · Error: page.waitForFunction: Target crashed  · cpu="x1" · batch="pid-4586"',
  '',
  '### Sizes (gzip -9, bytes)',
  '| file | dist-nohooks | bytes | dist | bytes |',
  '|---|---|---|---|---|',
  '| index.html | `index.html` | 340 | `index.html` | 344 |',
  '| entry JS | `assets/index-D9iosWQt.js` | 232477 | `assets/index-hS3VqISt.js` | 232436 |',
  '| entry CSS | `assets/index-DhWGPZ-b.css` | 2907 | `assets/index-DhWGPZ-b.css` | 2907 |',
  '| core .wasm | `assets/epiora_wasm_bg-d1kXcynH.wasm` | 138370 | `assets/epiora_wasm_bg-d1kXcynH.wasm` | 138370 |',
  '| sqlite3.wasm | `assets/sqlite3-k2cTanUO.wasm` | 407291 | `assets/sqlite3-k2cTanUO.wasm` | 407291 |',
  '| **first-load total** | | **781385** | | **781348** |',
  '',
  'First-load budget 5,000,000 bytes → headroom 4,218,615 bytes on `dist-nohooks` (reported, not gated).',
  '',
  'import-wasm gzip -9: **521469 bytes** (`crates/epiora-import-wasm/pkg/epiora_import_wasm_bg.wasm`)',
  '',
].join('\n');

const ROUTES = [
  '/',
  '/p/demo',
  '/p/demo/import',
  '/p/demo/d/demo',
  '/p/demo/analysis',
  '/settings',
];
const ENGINES = ['chromium', 'firefox', 'webkit'];

const axeLines = ENGINES.flatMap((engine) =>
  ROUTES.flatMap((route) =>
    ['light', 'dark'].map(
      (theme) =>
        `2026-11-12T13:01:04.5Z   ✓    1 [${engine}] › e2e/a11y.spec.ts:8:5 › ` +
        `${route} in ${theme} has no axe violations (1.4s)`,
    ),
  ),
);

const APP_JOB_LOG = [
  '2026-11-12T13:00:01.3Z  \x1b[32m✓\x1b[39m \x1b[30m\x1b[42m unit \x1b[49m\x1b[39m src/store/conformance/native.test.ts \x1b[2m(\x1b[22m\x1b[2m14 tests\x1b[22m\x1b[2m)\x1b[22m\x1b[33m 1966\x1b[22mms\x1b[22m',
  '2026-11-12T13:00:01.3Z    \x1b[32m✓\x1b[39m store conformance: native \x1b[2m(14)\x1b[22m',
  '2026-11-12T13:00:14.8Z  \x1b[32m✓\x1b[39m \x1b[30m\x1b[46m store-opfs (chromium) \x1b[49m\x1b[39m src/store/conformance/opfs.browser.test.ts \x1b[2m(\x1b[22m\x1b[2m15 tests\x1b[22m\x1b[2m)\x1b[22m\x1b[33m 4260\x1b[22mms\x1b[22m',
  '2026-11-12T13:00:28.5Z  \x1b[32m✓\x1b[39m \x1b[30m\x1b[43m store-opfs (firefox) \x1b[49m\x1b[39m src/store/conformance/opfs.browser.test.ts \x1b[2m(\x1b[22m\x1b[2m15 tests\x1b[22m\x1b[2m)\x1b[22m\x1b[33m 5165\x1b[22mms\x1b[22m',
  '2026-11-12T13:00:37.2Z  \x1b[32m✓\x1b[39m \x1b[30m\x1b[43m store-opfs (webkit) \x1b[49m\x1b[39m src/store/conformance/opfs.browser.test.ts \x1b[2m(\x1b[22m\x1b[2m15 tests\x1b[22m\x1b[2m)\x1b[22m\x1b[33m 3624\x1b[22mms\x1b[22m',
  '2026-11-12T13:05:10.3Z   171 passed (4.2m)',
  '2026-11-12T13:05:15.2Z first-load budget: 781385 bytes vs 5000000 limit → PASS (fails at or above the limit)',
  ...axeLines,
].join('\n');

const NATIVE_STORE_JOB_LOG = [
  '2026-11-12T11:24:08.8Z  \x1b[32m✓\x1b[39m \x1b[30m\x1b[42m unit \x1b[49m\x1b[39m src/store/conformance/native.test.ts \x1b[2m(\x1b[22m\x1b[2m14 tests\x1b[22m\x1b[2m)\x1b[22m\x1b[33m 1726\x1b[22mms\x1b[22m',
  '2026-11-12T11:24:08.8Z    \x1b[32m✓\x1b[39m store conformance: native \x1b[2m(14)\x1b[22m',
  '2026-11-12T11:24:08.8Z \x1b[2m Test Files \x1b[22m \x1b[1m\x1b[32m1 passed\x1b[39m\x1b[22m\x1b[90m (1)\x1b[39m',
  '2026-11-12T11:24:08.8Z \x1b[2m      Tests \x1b[22m \x1b[1m\x1b[32m14 passed\x1b[39m\x1b[22m\x1b[90m (14)\x1b[39m',
].join('\n');

const ATTEMPT1_PERF_LOG = [
  '2026-11-12T12:23:40.8Z   ✘  5 [chromium] › perf/perf.spec.ts:489:3 › sample 5/5 (1m, persistent profile) (30.2m)',
  '2026-11-12T12:23:41.4Z   ✘  6 [chromium] › perf/perf.spec.ts:705:1 › owner gates: medians meet the PR #52 thresholds (8ms)',
  '2026-11-12T12:23:41.4Z     Test timeout of 1800000ms exceeded.',
  '2026-11-12T12:23:41.4Z     Error: import_1m: no successful sample; open_tables_1m: no successful sample',
  '2026-11-12T12:28:39.8Z | import_1m | 5 | 0 | 0 | ms | 75461.2 | 76705.5 |',
  '2026-11-12T12:28:39.8Z | open_tables_1m | 5 | 0 | 0 | ms | 2376.7 | 2384.3 |',
].join('\n');

const ATTEMPT2_PERF_LOG = [
  '2026-11-12T13:39:10.2Z     Error: import_1m: median 101633.1000000001 ms > 89550 ms',
  '2026-11-12T13:45:54.5Z | import_1m | 5 | 0 | 0 | ms | 101633.1 | 102481 |',
  '2026-11-12T13:45:54.5Z | open_tables_1m | 5 | 0 | 0 | ms | 2769.6 | 2782.1 |',
].join('\n');

const DESKTOP_LINUX_LOG = [
  '2026-11-12T11:29:59.5Z [0-0] [smoke] app rendered',
  '2026-11-12T11:29:59.8Z [0-0] [smoke] project created',
  '2026-11-12T11:31:00.7Z [0-0] [smoke] import complete',
  '2026-11-12T11:31:02.0Z [0-0] [smoke] TABLES result card shown',
  '2026-11-12T11:31:02.0Z [0-0] [smoke] database file is encrypted',
  '2026-11-12T11:31:02.1Z [0-0] PASSED in undefined - file:///home/runner/work/Epiora/Epiora/app/desktop-e2e/smoke.spec.mjs',
  '2026-11-12T11:31:02.1Z Spec Files:\t 1 passed, 1 total (100% completed) in 00:01:09',
].join('\n');

const parityLine = '{"case":"utf8_comma","result":{}}\n';
const INPUT = {
  source: {
    url: SOURCE_URL,
    id: 43208388072,
    attempt: 3,
    sha: SHA,
    event: 'push',
    conclusion: 'success',
    workflow: 'app',
  },
  generatedByUrl: GENERATED_BY,
  gatesMd: GATES_MD,
  summaryMd: SUMMARY_MD,
  appJobLog: APP_JOB_LOG,
  nativeStoreJobLog: NATIVE_STORE_JOB_LOG,
  earlierAttempts: [
    { attempt: 1, conclusion: 'failure', perfJobLog: ATTEMPT1_PERF_LOG },
    { attempt: 2, conclusion: 'failure', perfJobLog: ATTEMPT2_PERF_LOG },
  ],
  importRun: {
    id: 43208388078,
    url: 'https://github.com/dorrabbkhan/Epiora/actions/runs/43208388078',
    conclusion: 'success',
    jobLog:
      '2026-11-12T11:22:00.9Z \x1b[36;1msize="$(scripts/wasm-size.sh "$f" 573616)"\x1b[0m\n' +
      '2026-11-12T11:22:00.9Z \x1b[36;1mexit $rc\x1b[0m',
    fixtureCount: 35,
    nativeJsonl: parityLine.repeat(35),
    wasmJsonl: parityLine.repeat(35),
  },
  desktopRun: {
    id: 43208388030,
    url: 'https://github.com/dorrabbkhan/Epiora/actions/runs/43208388030',
    conclusion: 'success',
    artifacts: [
      {
        name: 'epiora-desktop-ubuntu-24.04-e19042cdde36954b40461e390ebcbae5abfbbd0b',
        size_in_bytes: 206696638,
      },
      {
        name: 'epiora-desktop-windows-latest-e19042cdde36954b40461e390ebcbae5abfbbd0b',
        size_in_bytes: 8802234,
      },
      {
        name: 'epiora-desktop-macos-latest-e19042cdde36954b40461e390ebcbae5abfbbd0b',
        size_in_bytes: 8611246,
      },
    ],
    linuxBundleJobLog: DESKTOP_LINUX_LOG,
  },
};

const report = buildReport(INPUT);

test('report_lists_every_gate', () => {
  // Every §10.2 gate, with the owner's Task 17 thresholds and the
  // measured value read from the run — never typed in.
  const gates = [
    { name: 'first load', threshold: '5,000,000', measured: '781,385' },
    { name: 'import-wasm', threshold: '573,616', measured: '521,469' },
    { name: 'import_1m', threshold: '89,550', measured: '71,621.8' },
    { name: 'open_tables_1m', threshold: '2,874', measured: '2,263.3' },
  ];
  for (const g of gates) {
    const row = report
      .split('\n')
      .find((l) => l.startsWith('|') && l.includes(g.name));
    assert.ok(row !== undefined, `gate row for ${g.name} missing:\n${report}`);
    assert.ok(row.includes(g.threshold), `${g.name}: threshold ${g.threshold} missing from ${row}`);
    assert.ok(row.includes(g.measured), `${g.name}: measured ${g.measured} missing from ${row}`);
    assert.ok(row.includes('PASS'), `${g.name}: verdict missing from ${row}`);
  }

  // Provenance: the source run (URL, attempt, commit) and the
  // generating run are named separately.
  assert.match(report, /# Epiora v0\.3 validation report/);
  assert.ok(report.includes(SOURCE_URL));
  assert.ok(report.includes('attempt 3'));
  assert.ok(report.includes(`\`${SHA}\``));
  assert.ok(report.includes(GENERATED_BY));

  // Sizes: the first-load table total is rendered, never NaN.
  assert.ok(report.includes('**781,385**'), 'first-load total missing from sizes table');
  assert.ok(report.includes('**521,469 bytes**'), 'import-wasm size missing');

  // Perf medians and p90s, and the D7 throttled report.
  assert.ok(report.includes('| import_1m | 5 | 0 | 0 | ms | 71,621.8 | 71,937.9 |'));
  assert.ok(report.includes('| open_tables_1m | 5 | 0 | 0 | ms | 2,263.3 | 2,276.1 |'));
  assert.ok(report.includes('import_1m_cpu4'));
  assert.ok(report.includes('open_tables_1m_cpu4'));
  assert.ok(report.includes('79,425.5'));
  assert.ok(report.includes('2,980.3'));

  // axe per engine: 6 routes × 2 themes, 12 scans, 0 violations.
  for (const engine of ENGINES) {
    const row = report
      .split('\n')
      .find((l) => l.startsWith(`| ${engine} |`));
    assert.ok(row !== undefined, `axe row for ${engine} missing`);
    assert.ok(row.includes('| 6 | 2 | 12 | 0 |'), `axe counts wrong for ${engine}: ${row}`);
  }

  // Conformance per backend.
  assert.ok(/native.*\| 14 \|/.test(report), `native conformance row missing:\n${report}`);
  for (const engine of ENGINES) {
    assert.ok(
      new RegExp(`opfs.*${engine}.*\\| 15 \\|`).test(report),
      `opfs conformance row for ${engine} missing`,
    );
  }

  // Import fixtures and parity.
  assert.ok(report.includes('35'), 'fixture count missing');
  assert.ok(report.includes('parity PASS'));

  // Desktop artifacts and smoke test.
  for (const os of ['ubuntu-24.04', 'windows-latest', 'macos-latest']) {
    assert.ok(
      report.includes(`epiora-desktop-${os}-${SHA}`),
      `desktop artifact for ${os} missing`,
    );
  }
  assert.ok(report.includes('smoke') && report.includes('1 passed'));
});

test('report_states_earlier_attempt_facts', () => {
  // Attempt 1: the sample-5 timeout and the gate's empty batch —
  // read from the attempt's job log, with the pid mechanism named.
  assert.ok(report.includes('Attempt 1'));
  assert.ok(report.includes('1800000'));
  assert.ok(report.includes('sample 5/5'));
  assert.ok(report.includes('no successful sample'));
  assert.ok(report.includes('app/perf/perf.spec.ts:80'));

  // Attempt 2: the gate failure medians from the attempt's log.
  assert.ok(report.includes('Attempt 2'));
  assert.ok(report.includes('101,633.1'));
  assert.ok(report.includes('2,769.6'));
});

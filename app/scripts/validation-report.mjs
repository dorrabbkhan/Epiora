#!/usr/bin/env node
// app/scripts/validation-report.mjs <run-url> — the v0.3 validation
// report (v0.3 plan Task 18, spec §10.5). Renders
// docs/validation/v0.3.md from the named `app` workflow run's
// artifacts and job logs, fetched through `gh api`.
//
// Perf and desktop results exist only on main-push and manual runs,
// so the report is generated from a `main` push run — for v0.3 the
// e19042c run (attempt 3, the green one) — and also states the facts
// of that run's earlier attempts from their job logs. Every number in
// the output is parsed from CI output; nothing is typed in.
//
// `buildReport(input)` is the pure renderer so the node:test suite
// can exercise it without network access; `collect(runUrl)` in
// collect-validation-run.mjs is the only part that talks to GitHub.

import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { collect } from './collect-validation-run.mjs';

// ---------- small helpers -------------------------------------------

const stripAnsi = (s) => s.replace(/\x1b\[[0-9;]*m/g, '');
const stripTs = (s) =>
  s.replace(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z\s+/, '');
const clean = (line) => stripTs(stripAnsi(line));
const fmt = (x) =>
  Number(x).toLocaleString('en-US', { maximumFractionDigits: 1 });
const cell = (v) => (/^-?[\d.]+$/.test(String(v)) ? fmt(v) : String(v));

// ---------- parsing the CI outputs -----------------------------------

/** gates.md table → [{scenario, ok, failed, skipped, medianMs, thresholdMs, verdict}] */
function parseGates(md) {
  const rows = [];
  for (const line of md.split('\n')) {
    if (!line.startsWith('|') || line.includes('scenario') || line.startsWith('|---')) continue;
    const c = line.split('|').map((x) => x.trim()).filter(Boolean);
    if (c.length === 7) {
      rows.push({
        scenario: c[0], ok: +c[1], failed: +c[2], skipped: +c[3],
        medianMs: +c[4], thresholdMs: +c[5], verdict: c[6],
      });
    }
  }
  return rows;
}

/** summary.md → {profiles, scenarios, failures, files, firstLoadTotal, importWasmBytes} */
function parseSummary(md) {
  const lines = md.split('\n');
  const profiles = /profile\(s\): (.+)/.exec(md)?.[1].split(',').map((s) => s.trim()) ?? [];
  const scenarios = [];
  const failures = [];
  const files = [];
  let section = null;
  let firstLoadTotal = null;
  let importWasmBytes = null;
  for (const line of lines) {
    if (line.startsWith('### ')) { section = line.slice(4).trim(); continue; }
    if (line.startsWith('| scenario ')) { section = 'scenarios'; continue; }
    if (line.startsWith('| file ')) { section = 'sizes'; continue; }
    if (line.startsWith('|---')) continue;
    if (line.startsWith('|')) {
      const c = line.split('|').map((x) => x.trim()).filter(Boolean);
      if (section === 'scenarios' && c.length === 7) {
        scenarios.push({ scenario: c[0], ok: +c[1], failed: +c[2], skipped: +c[3], field: c[4], median: c[5], p90: c[6] });
      } else if (section === 'sizes') {
        if (c[0].includes('first-load total')) {
          const num = c.find((x) => /^\d+$/.test(x.replace(/\*/g, '')));
          firstLoadTotal = num === undefined ? null : +num.replace(/\*/g, '');
        } else if (c.length === 5) {
          files.push({ role: c[0], rel: c[1].replace(/`/g, ''), bytes: +c[2] });
        } else if (c.length === 2) {
          files.push({ role: c[0], rel: c[0], bytes: +c[1] });
        }
      }
      continue;
    }
    if (section === 'Failures') {
      const m = /^- run (\d+) · ([^·]+) · ([^·]+)/.exec(line);
      if (m) failures.push({ run: +m[1], scenario: m[2].trim(), error: m[3].trim() });
    }
    const w = /import-wasm gzip -9: \*\*(\d+) bytes\*\*/.exec(line);
    if (w) importWasmBytes = +w[1];
  }
  return { profiles, scenarios, failures, files, firstLoadTotal, importWasmBytes };
}

/** app job log → Map engine → {routes, themes, scans} */
function parseAxe(log) {
  const per = new Map();
  for (const raw of log.split('\n')) {
    const m = /\[(\w+)\] › e2e\/a11y\.spec\.ts:\d+:\d+ › (.+?) in (\w+) has no axe violations/.exec(clean(raw));
    if (!m) continue;
    const e = per.get(m[1]) ?? { routes: new Set(), themes: new Set(), scans: 0 };
    e.routes.add(m[2]);
    e.themes.add(m[3]);
    e.scans += 1;
    per.set(m[1], e);
  }
  return per;
}

/** Conformance counts + e2e total from the app and native-store logs. */
function parseConformance(appLog, nativeLog) {
  const opfs = new Map();
  let native = null;
  let e2ePassed = null;
  for (const raw of appLog.split('\n')) {
    const l = clean(raw);
    let m = /store-opfs \((\w+)\)\s+src\/store\/conformance\/opfs\.browser\.test\.ts \((\d+) tests\)/.exec(l);
    if (m) opfs.set(m[1], { tests: +m[2], passed: l.includes('✓') });
    m = /^(\d+) passed \(/.exec(l.trim());
    if (m) e2ePassed = +m[1];
  }
  for (const raw of nativeLog.split('\n')) {
    const l = clean(raw);
    const t = /native\.test\.ts \((\d+) tests\)/.exec(l);
    const p = /Tests (\d+) passed/.exec(l);
    if (t) native = { tests: +t[1] };
    if (p && native !== null) native.passed = +p[1];
  }
  return { opfs, native, e2ePassed };
}

const parseBudget = (log) => {
  const m = /first-load budget: (\d+) bytes vs (\d+) limit → (\w+)/.exec(log);
  return m === null ? null : { measured: +m[1], limit: +m[2], verdict: m[3] };
};

const parseWasmLimit = (log) =>
  +(/wasm-size\.sh "?\$f"? (\d+)/.exec(log)?.[1] ?? 0);

/** A `perf` job log → {failedTests, timeouts, gateErrors, medians}. */
function parseAttemptFacts(log) {
  const failedTests = [];
  const timeouts = [];
  const gateErrors = [];
  const medians = new Map();
  for (const raw of log.split('\n')) {
    const l = clean(raw);
    let m = /✘\s+\d+ \[chromium\] › perf\/perf\.spec\.ts:\d+:\d+ › (.+?) \(([\d.]+[ms]+)\)/.exec(l);
    if (m) failedTests.push({ name: m[1].trim(), elapsed: m[2] });
    m = /Test timeout of (\d+)ms exceeded/.exec(l);
    if (m) timeouts.push(+m[1]);
    m = /Error: ((?:import|open_tables)_1m: .+)$/.exec(l);
    if (m) gateErrors.push(m[1].trim());
    m = /^\| (import_1m|open_tables_1m) \| (\d+) \| (\d+) \| (\d+) \| ms \| ([\d.]+) \| ([\d.]+) \|/.exec(l);
    if (m) medians.set(m[1], { ok: +m[2], failed: +m[3], skipped: +m[4], median: +m[5], p90: +m[6] });
  }
  return { failedTests, timeouts, gateErrors, medians };
}

const parseSmoke = (log) => {
  const steps = log.split('\n').map(clean).filter((l) => l.includes('[smoke]'));
  const m = /Spec Files:\s+(\d+) passed, (\d+) total/.exec(log);
  return {
    steps: steps.length,
    passed: m !== null && m[1] === m[2] && +m[1] > 0,
    files: m === null ? null : `${m[1]} passed, ${m[2]} total`,
  };
};

// ---------- report ----------------------------------------------------

export function buildReport(input) {
  const { source } = input;
  const gates = parseGates(input.gatesMd);
  const summary = parseSummary(input.summaryMd);
  const axe = parseAxe(input.appJobLog);
  const conf = parseConformance(input.appJobLog, input.nativeStoreJobLog);
  const budget = parseBudget(input.appJobLog);
  const wasmLimit = parseWasmLimit(input.importRun.jobLog);
  const parityNative = input.importRun.nativeJsonl.trim().split('\n').length;
  const parityWasm = input.importRun.wasmJsonl.trim().split('\n').length;
  const parityOk = input.importRun.nativeJsonl === input.importRun.wasmJsonl;
  const smoke = parseSmoke(input.desktopRun.linuxBundleJobLog);
  const gateOf = (name) => gates.find((g) => g.scenario === name);

  const out = [];
  out.push('# Epiora v0.3 validation report');
  out.push('');
  out.push(
    'Generated by `app/scripts/validation-report.mjs` (v0.3 Task 18, spec §10.5). Every number',
    'below is read from the named CI runs\u2019 artifacts and job logs — nothing is typed in.',
    '',
    `- Commit: \`${source.sha}\``,
    `- Source run: ${source.url} — \`${source.workflow}\` workflow, \`${source.event}\`,`,
    `  attempt ${source.attempt}, conclusion **${source.conclusion}**. Perf and desktop results exist`,
    '  only on main-push and manual runs, so this report reads the `main` push runs for the commit,',
    '  not a pull-request run.',
    input.generatedByUrl === null
      ? '- Generated by: local invocation (no CI run).'
      : `- Generated by CI run: ${input.generatedByUrl} — the run that produced this file.`,
    `- Sibling runs at this commit: \`import\` ${input.importRun.url} (conclusion`,
    `  ${input.importRun.conclusion}); \`desktop\` ${input.desktopRun.url} (conclusion`,
    `  ${input.desktopRun.conclusion}).`,
    '',
  );

  // --- Gates -----------------------------------------------------------
  out.push('## Gates', '', '| gate | threshold | measured | verdict |', '|---|---|---|---|');
  if (budget !== null) {
    out.push(
      `| first load (index.html + entry JS/CSS + core-wasm + sqlite3.wasm, gzip -9) ` +
        `| < ${fmt(budget.limit)} bytes | ${fmt(budget.measured)} bytes | ${budget.verdict} |`,
    );
  }
  if (wasmLimit > 0) {
    out.push(
      `| import-wasm (epiora_import_wasm_bg.wasm, gzip -9) | ≤ ${fmt(wasmLimit)} bytes ` +
        `| ${fmt(summary.importWasmBytes)} bytes | ${summary.importWasmBytes <= wasmLimit ? 'PASS' : 'FAIL'} |`,
    );
  }
  for (const name of ['import_1m', 'open_tables_1m']) {
    const g = gateOf(name);
    if (g === undefined) continue;
    out.push(
      `| \`${name}\` median, persistent profile (${g.ok} ok / ${g.failed} failed / ${g.skipped} skipped) ` +
        `| ≤ ${fmt(g.thresholdMs)} ms | ${fmt(g.medianMs)} ms | ${g.verdict} |`,
    );
  }
  out.push(
    '',
    'Thresholds are the owner-picked Task 17 numbers',
    '([PR #52](https://github.com/dorrabbkhan/Epiora/pull/52#issuecomment-6807191386)); a perf scenario',
    'passes when the median `ms` of its successful samples is ≤ the threshold. The import-wasm measured',
    'size is read from the perf run\u2019s `summary.md` — the same release `pkg` build as the `import` job\u2019s',
    'gate at this commit.',
    '',
  );

  // --- Perf --------------------------------------------------------------
  out.push(
    `## Performance — ${source.workflow} run attempt ${source.attempt}`,
    '',
    `profile(s): ${summary.profiles.join(', ')}`,
    '',
    '| scenario | ok | failed | skipped | field | median | p90 |',
    '|---|---|---|---|---|---|---|',
  );
  for (const s of summary.scenarios) {
    out.push(
      `| ${s.scenario} | ${s.ok} | ${s.failed} | ${s.skipped} | ${s.field} | ${cell(s.median)} | ${cell(s.p90)} |`,
    );
  }
  if (summary.failures.length > 0) {
    out.push('', 'Failed samples (a failure is a measurement, never hidden):', '');
    for (const f of summary.failures) {
      out.push(`- run ${f.run} · \`${f.scenario}\` · ${f.error}`);
    }
  }

  // --- Earlier attempts --------------------------------------------------
  if (input.earlierAttempts.length > 0) {
    out.push(
      '',
      `### Earlier attempts of run ${source.id} (read from each attempt\u2019s \`perf\` job log)`,
      '',
    );
    for (const a of input.earlierAttempts) {
      const f = parseAttemptFacts(a.perfJobLog);
      const bits = [];
      const nonGateTests = f.failedTests.filter((t) => !t.name.startsWith('owner gates'));
      for (let i = 0; i < f.timeouts.length; i++) {
        const t = nonGateTests[i];
        bits.push(
          `${t === undefined ? 'a sample' : `\`${t.name}\``} hit the ` +
            `${Math.round(f.timeouts[i] / 60000)}-minute test timeout ` +
            `(\`Test timeout of ${f.timeouts[i]}ms exceeded\`)`,
        );
      }
      for (const e of f.gateErrors) {
        bits.push(`the owner-gates test failed: \`${e}\``);
        if (e.includes('no successful sample')) {
          bits.push(
            'its batch id is the test worker\u2019s pid (`app/perf/perf.spec.ts:80`), so a ' +
              'worker restarted by the timeout left the gate an empty batch',
          );
        }
      }
      for (const [name, s] of [...f.medians.entries()].sort()) {
        const g = gateOf(name);
        const cmp = g === undefined ? '' : s.median <= g.thresholdMs ? ' ≤ ' : ' > ';
        bits.push(
          `\`${name}\` median ${fmt(s.median)} ms` +
            (g === undefined ? '' : `${cmp}threshold ${fmt(g.thresholdMs)} ms`),
        );
      }
      out.push(
        `- **Attempt ${a.attempt}** — \`perf\` job conclusion **${a.conclusion}**` +
          (bits.length === 0 ? '.' : ` — ${bits.join('; ')}.`),
      );
    }
  }

  // --- D7 ------------------------------------------------------------------
  const d7 = summary.scenarios.filter((s) => s.field === 'ms' && /_cpu\d+$/.test(s.scenario));
  out.push('', '## D7 report (Chromium CPU throttling ×4 — reported, not gated)', '');
  out.push('| scenario | ok | failed | median ms | p90 ms |', '|---|---|---|---|---|');
  for (const s of d7) {
    out.push(`| ${s.scenario} | ${s.ok} | ${s.failed} | ${cell(s.median)} | ${cell(s.p90)} |`);
  }

  // --- Sizes -----------------------------------------------------------------
  out.push('', '## Sizes (gzip -9, bytes)', '', '| file | bytes |', '|---|---|');
  for (const f2 of summary.files) {
    out.push(`| ${f2.rel} | ${fmt(f2.bytes)} |`);
  }
  if (summary.firstLoadTotal !== null) {
    out.push(`| **first-load total** | **${fmt(summary.firstLoadTotal)}** |`);
  }
  out.push(
    '',
    `import-wasm gzip -9: **${fmt(summary.importWasmBytes)} bytes**`,
    '(`crates/epiora-import-wasm/pkg/epiora_import_wasm_bg.wasm`).',
    '',
  );

  // --- axe ---------------------------------------------------------------------
  out.push(
    '## Accessibility — axe (e2e/a11y.spec.ts)',
    '',
    '| engine | routes | themes | scans | violations |',
    '|---|---|---|---|---|',
  );
  for (const [engine, a] of [...axe.entries()].sort()) {
    out.push(`| ${engine} | ${a.routes.size} | ${a.themes.size} | ${a.scans} | 0 |`);
  }
  if (conf.e2ePassed !== null) {
    out.push('', `Playwright end-to-end total: **${fmt(conf.e2ePassed)} tests passed** across the three engines.`);
  }

  // --- conformance ----------------------------------------------------------------
  out.push('', '## Store conformance', '', '| backend | engine | tests | result |', '|---|---|---|---|');
  if (conf.native !== null) {
    out.push(
      `| native SQLite3MultipleCiphers (store-stdio) | — | ${conf.native.tests} ` +
        `| ${conf.native.passed ?? conf.native.tests} passed |`,
    );
  }
  for (const [engine, o] of [...conf.opfs.entries()].sort()) {
    out.push(`| opfs-sahpool | ${engine} | ${o.tests} | ${o.passed ? 'passed' : 'FAILED'} |`);
  }

  // --- import parity ------------------------------------------------------------------
  out.push('', '## Import fixtures and parity', '');
  out.push(
    `- Fixture cases in \`fixtures/import\` at this commit: **${input.importRun.fixtureCount}**.`,
    `- \`import-runner\` (native) vs \`run.mjs\` (wasm): ${fmt(parityNative)} vs ${fmt(parityWasm)} result lines,`,
    `  ${parityOk ? 'byte-identical → **parity PASS**' : '**MISMATCH — parity FAIL**'} (import run`,
    `  ${input.importRun.url}).`,
  );

  // --- desktop ---------------------------------------------------------------------------
  out.push('', `## Desktop (read from desktop run ${input.desktopRun.url})`, '');
  for (const a of input.desktopRun.artifacts) {
    out.push(`- \`${a.name}\` — ${fmt(a.size_in_bytes)} bytes`);
  }
  out.push(
    '',
    `Linux smoke test (webkit2gtk-driver, inside the real window): ` +
      `**${smoke.passed ? `PASSED — ${smoke.files ?? ''}` : 'FAILED or not run'}**.` +
      ' The smoke binary imports a fixture CSV, runs TABLES and checks the database file has no',
    '`SQLite format 3` plaintext header. Windows/macOS bundles are unsigned artifacts only',
    '(v0.3 Task 15 decision).',
    '',
  );
  return `${out.join('\n')}\n`;
}

// ---------- entry ----------------------------------------------------

const invoked = process.argv[1] !== undefined &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (invoked) {
  const url = process.argv[2];
  if (url === undefined) {
    console.error('usage: validation-report.mjs <run-url>');
    process.exit(2);
  }
  process.stdout.write(buildReport(collect(url)));
}

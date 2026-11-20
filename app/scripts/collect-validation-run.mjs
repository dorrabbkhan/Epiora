// app/scripts/collect-validation-run.mjs — the only part of the
// validation reports that talks to GitHub (v0.3 plan Task 18, spec
// §10.5; v1.0 plan Task 21). `collect(runUrl)` fetches the named
// `app` workflow run's perf artifacts, job logs and earlier attempts
// plus the sibling `import` and `desktop` runs at the same commit
// through `gh api`, and returns the input object `buildReport`
// renders. `collectV1(runUrl)` fetches the perf artifacts and `app`
// job log plus the sibling `engine` (`v0.2-validation-md` and
// `v1.0-validation-json` artifacts), `import` and `r-export` runs for
// `buildReportV1`.

import { execFileSync } from 'node:child_process';
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

// Newer `gh api` refuses to print responses containing terminal
// escape sequences (job logs are full of them) unless
// --allow-escape-sequences is passed; gh ≤ 2.46 predates both the
// guard and the flag, so probe support once.
const escapeFlag = execFileSync('gh', ['api', '--help'], { encoding: 'utf8' })
  .includes('escape-sequences')
  ? ['--allow-escape-sequences']
  : [];

const ghText = (endpoint) =>
  execFileSync('gh', ['api', endpoint, ...escapeFlag], {
    encoding: 'utf8',
    maxBuffer: 128 * 1024 * 1024,
  });
const ghJson = (endpoint) => JSON.parse(ghText(endpoint));

function ghUnzip(endpoint, destDir) {
  mkdirSync(destDir, { recursive: true });
  const zip = join(destDir, 'artifact.zip');
  writeFileSync(
    zip,
    execFileSync('gh', ['api', endpoint, ...escapeFlag], {
      encoding: 'buffer',
      maxBuffer: 128 * 1024 * 1024,
    }),
  );
  execFileSync('unzip', ['-o', '-q', zip, '-d', destDir]);
  rmSync(zip);
}

export function collect(runUrl) {
  const m = /github\.com\/([^/]+)\/([^/]+)\/actions\/runs\/(\d+)/.exec(runUrl);
  if (m === null) throw new Error(`not a GitHub Actions run URL: ${runUrl}`);
  const [, owner, repo, runId] = m;
  const api = (p) => `repos/${owner}/${repo}/${p}`;

  const run = ghJson(api(`actions/runs/${runId}`));
  const sha = run.head_sha;
  const attempt = run.run_attempt;
  const tmp = mkdtempSync(join(tmpdir(), 'epiora-validation-'));
  try {
    // Latest attempt's perf-results artifact (one is uploaded per
    // attempt; the newest by creation time is the latest attempt's).
    const artifacts = ghJson(api(`actions/runs/${runId}/artifacts?per_page=100`)).artifacts;
    const perf = artifacts
      .filter((a) => a.name === 'perf-results')
      .sort((a, b) => (a.created_at < b.created_at ? 1 : -1))[0];
    if (perf === undefined) throw new Error(`run ${runId} has no perf-results artifact`);
    ghUnzip(api(`actions/artifacts/${perf.id}/zip`), join(tmp, 'perf'));

    const jobLog = (j) => ghText(api(`actions/jobs/${j.id}/logs`));
    const jobs = ghJson(api(`actions/runs/${runId}/attempts/${attempt}/jobs?per_page=100`)).jobs;
    const byName = (n) => jobs.find((j) => j.name === n);

    const earlierAttempts = [];
    for (let n = 1; n < attempt; n++) {
      const js = ghJson(api(`actions/runs/${runId}/attempts/${n}/jobs?per_page=100`)).jobs;
      const p = js.find((j) => j.name === 'perf');
      earlierAttempts.push({
        attempt: n,
        conclusion: p?.conclusion ?? 'not run',
        perfJobLog: p === undefined ? '' : jobLog(p),
      });
    }

    // Sibling runs for the same commit: `import` (fixtures, parity,
    // wasm gate) and `desktop` (bundles, smoke test).
    const siblings = ghJson(api(`actions/runs?head_sha=${sha}&per_page=100`)).workflow_runs;
    const importMeta = siblings.find((r) => r.name === 'import');
    const desktopMeta = siblings.find((r) => r.name === 'desktop');
    if (importMeta === undefined || desktopMeta === undefined) {
      throw new Error(`no import/desktop run for ${sha}`);
    }
    const importJobs = ghJson(api(`actions/runs/${importMeta.id}/jobs?per_page=100`)).jobs;
    const importJob = importJobs.find((j) => j.name === 'import') ?? importJobs[0];
    const importArts = ghJson(api(`actions/runs/${importMeta.id}/artifacts?per_page=100`)).artifacts;
    const parityArt = importArts.find((a) => a.name === 'import-parity-lines');
    let nativeJsonl = '';
    let wasmJsonl = '';
    if (parityArt !== undefined) {
      ghUnzip(api(`actions/artifacts/${parityArt.id}/zip`), join(tmp, 'parity'));
      for (const f of readdirSync(join(tmp, 'parity'))) {
        const text = readFileSync(join(tmp, 'parity', f), 'utf8');
        if (f.includes('native')) nativeJsonl = text;
        else if (f.includes('wasm')) wasmJsonl = text;
      }
    }
    const fixtureCount = ghJson(api(`contents/fixtures/import?ref=${sha}`))
      .filter((e) => e.type === 'dir').length;

    const desktopArts = ghJson(api(`actions/runs/${desktopMeta.id}/artifacts?per_page=100`))
      .artifacts.map((a) => ({ name: a.name, size_in_bytes: a.size_in_bytes }));
    const desktopJobs = ghJson(api(`actions/runs/${desktopMeta.id}/jobs?per_page=100`)).jobs;
    const linuxJob = desktopJobs.find((j) => j.name.includes('ubuntu'));

    const { GITHUB_SERVER_URL, GITHUB_REPOSITORY, GITHUB_RUN_ID } = process.env;
    const generatedByUrl =
      GITHUB_SERVER_URL && GITHUB_REPOSITORY && GITHUB_RUN_ID
        ? `${GITHUB_SERVER_URL}/${GITHUB_REPOSITORY}/actions/runs/${GITHUB_RUN_ID}`
        : null;

    return {
      source: {
        url: run.html_url,
        id: run.id,
        attempt,
        sha,
        event: run.event,
        conclusion: run.conclusion,
        workflow: run.name,
      },
      generatedByUrl,
      gatesMd: readFileSync(join(tmp, 'perf', 'gates.md'), 'utf8'),
      summaryMd: readFileSync(join(tmp, 'perf', 'summary.md'), 'utf8'),
      appJobLog: jobLog(byName('app')),
      nativeStoreJobLog: jobLog(byName('native-store')),
      earlierAttempts,
      importRun: {
        id: importMeta.id,
        url: importMeta.html_url,
        conclusion: importMeta.conclusion,
        jobLog: jobLog(importJob),
        fixtureCount,
        nativeJsonl,
        wasmJsonl,
      },
      desktopRun: {
        id: desktopMeta.id,
        url: desktopMeta.html_url,
        conclusion: desktopMeta.conclusion,
        artifacts: desktopArts,
        linuxBundleJobLog: linuxJob === undefined ? '' : jobLog(linuxJob),
      },
    };
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
}

/** The v1.0 report's input (v1.0 plan Task 21): the source `app`
 *  run's perf artifact and `app` job log, the same commit's `engine`
 *  run's `v0.2-validation-md` + `v1.0-validation-json` artifacts, the
 *  `import` run's wasm-size gate line, the `r-export` run's job log
 *  and the Stata status recorded in docs/decisions.md. */
export function collectV1(runUrl) {
  const m = /github\.com\/([^/]+)\/([^/]+)\/actions\/runs\/(\d+)/.exec(runUrl);
  if (m === null) throw new Error(`not a GitHub Actions run URL: ${runUrl}`);
  const [, owner, repo, runId] = m;
  const api = (p) => `repos/${owner}/${repo}/${p}`;

  const run = ghJson(api(`actions/runs/${runId}`));
  const sha = run.head_sha;
  const attempt = run.run_attempt;
  const tmp = mkdtempSync(join(tmpdir(), 'epiora-validation-v1-'));
  try {
    const artifacts = ghJson(api(`actions/runs/${runId}/artifacts?per_page=100`)).artifacts;
    const perf = artifacts
      .filter((a) => a.name === 'perf-results')
      .sort((a, b) => (a.created_at < b.created_at ? 1 : -1))[0];
    if (perf === undefined) throw new Error(`run ${runId} has no perf-results artifact`);
    ghUnzip(api(`actions/artifacts/${perf.id}/zip`), join(tmp, 'perf'));

    const jobLog = (j) => ghText(api(`actions/jobs/${j.id}/logs`));
    const jobs = ghJson(api(`actions/runs/${runId}/attempts/${attempt}/jobs?per_page=100`)).jobs;
    const byName = (n) => jobs.find((j) => j.name === n);

    const earlierAttempts = [];
    for (let n = 1; n < attempt; n++) {
      const js = ghJson(api(`actions/runs/${runId}/attempts/${n}/jobs?per_page=100`)).jobs;
      const p = js.find((j) => j.name === 'perf');
      earlierAttempts.push({
        attempt: n,
        conclusion: p?.conclusion ?? 'not run',
        perfJobLog: p === undefined ? '' : jobLog(p),
      });
    }

    // Sibling runs for the same commit — newest first; prefer a green
    // one (the push fires several workflows).
    const siblings = ghJson(api(`actions/runs?head_sha=${sha}&per_page=100`)).workflow_runs;
    const sibling = (name) =>
      siblings.find((r) => r.name === name && r.conclusion === 'success') ??
      siblings.find((r) => r.name === name);
    const engineMeta = sibling('engine');
    const importMeta = sibling('import');
    const rExportMeta = sibling('r-export');
    for (const [name, meta] of [
      ['engine', engineMeta],
      ['import', importMeta],
      ['r-export', rExportMeta],
    ]) {
      if (meta === undefined) throw new Error(`no ${name} run for ${sha}`);
    }
    const jobsOf = (meta) =>
      ghJson(api(`actions/runs/${meta.id}/jobs?per_page=100`)).jobs;

    const engineArts = ghJson(
      api(`actions/runs/${engineMeta.id}/artifacts?per_page=100`),
    ).artifacts;
    const art = (name) => engineArts.find((a) => a.name === name);
    if (art('v0.2-validation-md') === undefined || art('v1.0-validation-json') === undefined) {
      throw new Error(`engine run ${engineMeta.id} has no validation artifacts`);
    }
    ghUnzip(api(`actions/artifacts/${art('v0.2-validation-md').id}/zip`), join(tmp, 'eng-md'));
    ghUnzip(api(`actions/artifacts/${art('v1.0-validation-json').id}/zip`), join(tmp, 'eng-json'));
    const validationMd = readdirSync(join(tmp, 'eng-md'))
      .map((f) => readFileSync(join(tmp, 'eng-md', f), 'utf8'))
      .find((t) => t.startsWith('# Epiora'));
    const validationJson = JSON.parse(
      readFileSync(join(tmp, 'eng-json', 'validation.json'), 'utf8'),
    );

    const importJob = jobsOf(importMeta).find((j) => j.name === 'import') ??
      jobsOf(importMeta)[0];
    const rExportJob = jobsOf(rExportMeta).find((j) => j.name === 'r-export') ??
      jobsOf(rExportMeta)[0];

    // Stata status is a repo fact (v1.0 Task 20's record), not a run
    // measurement: deferred when decisions.md carries the deferral,
    // verified when the human-step results file exists.
    const root = fileURLToPath(new URL('../../', import.meta.url));
    const decisions = readFileSync(join(root, 'docs/decisions.md'), 'utf8');
    const stata = /Stata export deferred/.test(decisions)
      ? 'deferred'
      : existsSync(join(root, 'fixtures/stata/results.json'))
        ? 'verified'
        : 'not recorded';

    const { GITHUB_SERVER_URL, GITHUB_REPOSITORY, GITHUB_RUN_ID } = process.env;
    const generatedByUrl =
      GITHUB_SERVER_URL && GITHUB_REPOSITORY && GITHUB_RUN_ID
        ? `${GITHUB_SERVER_URL}/${GITHUB_REPOSITORY}/actions/runs/${GITHUB_RUN_ID}`
        : null;

    return {
      source: {
        url: run.html_url,
        id: run.id,
        attempt,
        sha,
        event: run.event,
        conclusion: run.conclusion,
        workflow: run.name,
      },
      generatedByUrl,
      gatesMd: readFileSync(join(tmp, 'perf', 'gates.md'), 'utf8'),
      summaryMd: readFileSync(join(tmp, 'perf', 'summary.md'), 'utf8'),
      appJobLog: jobLog(byName('app')),
      earlierAttempts,
      importRun: {
        url: importMeta.html_url,
        conclusion: importMeta.conclusion,
        jobLog: jobLog(importJob),
      },
      engineRun: {
        url: engineMeta.html_url,
        conclusion: engineMeta.conclusion,
        validationMd,
        validationJson,
      },
      rExportRun: {
        url: rExportMeta.html_url,
        conclusion: rExportMeta.conclusion,
        jobLog: jobLog(rExportJob),
      },
      stata,
    };
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
}

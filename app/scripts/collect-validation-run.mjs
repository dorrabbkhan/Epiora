// app/scripts/collect-validation-run.mjs — the only part of the v0.3
// validation report that talks to GitHub (v0.3 plan Task 18, spec
// §10.5). `collect(runUrl)` fetches the named `app` workflow run's
// perf artifacts, job logs and earlier attempts plus the sibling
// `import` and `desktop` runs at the same commit through `gh api`,
// and returns the input object `buildReport` renders.

import { execFileSync } from 'node:child_process';
import {
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

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

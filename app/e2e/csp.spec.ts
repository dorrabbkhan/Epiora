// CSP e2e (v0.3 plan Task 13): the `_headers` file's security headers
// are applied to every response via a Playwright route (vite preview
// does not read `_headers`), and every visited state must record zero
// `securitypolicyviolation` events. The policy text comes from the
// `_headers` file itself so the spec stays in sync with what Netlify
// deploys.
import { readFileSync } from 'node:fs';
import { expect, test, type BrowserContext, type Page } from '@playwright/test';

const UTF8_COMMA = readFileSync(
  new URL('../../fixtures/import/utf8_comma/input.csv', import.meta.url),
  'utf8',
);

/** Parses the Netlify `_headers` file into path → header map. */
function parseHeaders(text: string): Map<string, Record<string, string>> {
  const map = new Map<string, Record<string, string>>();
  let path: string | null = null;
  for (const raw of text.split('\n')) {
    const line = raw.trimEnd();
    if (line === '' || line.trimStart().startsWith('#')) continue;
    if (!line.startsWith(' ') && !line.startsWith('\t')) {
      path = line.trim();
      map.set(path, {});
      continue;
    }
    const idx = line.indexOf(':');
    if (path !== null && idx > 0) {
      map.get(path)![line.slice(0, idx).trim()] = line.slice(idx + 1).trim();
    }
  }
  return map;
}

const headersFile = parseHeaders(
  readFileSync(new URL('../public/_headers', import.meta.url), 'utf8'),
);
const SECURITY = headersFile.get('/*') ?? {};

/** Merges every `/*` `_headers` entry plus the matching path-specific
 *  headers into each response, like Netlify does. */
async function applyHeaders(context: BrowserContext): Promise<void> {
  const paths = [...headersFile.keys()];
  await context.route('**/*', async (route) => {
    const response = await route.fetch();
    const url = new URL(route.request().url());
    const headers = { ...response.headers(), ...SECURITY };
    for (const p of paths) {
      if (p === '/*') continue;
      const prefix = p.endsWith('/*') ? p.slice(0, -1) : p;
      if (url.pathname === p || url.pathname.startsWith(prefix)) {
        Object.assign(headers, headersFile.get(p));
      }
    }
    await route.fulfill({ response, headers });
  });
}

declare global {
  interface Window {
    __cspViolations: string[];
  }
}

async function violations(page: Page): Promise<string[]> {
  return page.evaluate(() => window.__cspViolations);
}

test.beforeEach(async ({ context, page }) => {
  await applyHeaders(context);
  await page.addInitScript(() => {
    window.__cspViolations = [];
    document.addEventListener('securitypolicyviolation', (e) => {
      window.__cspViolations.push(
        `${e.violatedDirective} blocked ${e.blockedURI} (${e.sourceFile ?? 'inline'}:${e.lineNumber})`,
      );
    });
  });
});

test('no_csp_violations_on_any_route', async ({ page }) => {
  test.setTimeout(120_000);
  const expectClean = async (where: string) => {
    expect(await violations(page), `CSP violations on ${where}`).toEqual([]);
  };

  for (const path of ['/', '/settings', '/p/demo', '/p/demo/d/demo', '/p/demo/analysis']) {
    await page.goto(path);
    await expect(page.locator('main')).toBeVisible();
    await expectClean(path);
  }

  // A real import and a real run through the RPC: worker, wasm and
  // OPFS all run under the policy.
  const seed = await page.evaluate(async (csv) => {
    const api = window.dataApi;
    const project = await api.createProject('CSP');
    const bytes = new TextEncoder().encode(csv);
    const { sessionId, inspection } = await api.inspectFile(
      'input.csv',
      bytes.buffer as ArrayBuffer,
    );
    const commit = await api.commitImport(
      sessionId,
      { projectId: project.id, datasetName: 'input.csv' },
      inspection.settings,
      false,
      () => {},
    );
    const run = await api.runFreq(project.id, commit.datasetId, 1, {
      variables: ['AGE', 'NAME'],
      stratify_by: null,
      include_missing: false,
      ci_method: 'clopper_pearson',
    });
    return { projectId: project.id, runId: run.id };
  }, UTF8_COMMA);
  await page.goto(`/p/${seed.projectId}`);
  await expect(
    page.getByRole('heading', { name: 'FREQ AGE, NAME — input.csv v1' }),
  ).toBeVisible();
  await expectClean('import + FREQ');
});

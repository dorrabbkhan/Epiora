// CSP e2e (v0.3 plan Task 13): the `_headers` file's security headers
// are applied to every response via a Playwright route (vite preview
// does not read `_headers`), and every visited state must record zero
// `securitypolicyviolation` events. The policy text comes from the
// `_headers` file itself so the spec stays in sync with what Netlify
// deploys.
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { expect, test, type BrowserContext, type Page } from '@playwright/test';
import { ROUTES } from './routes';

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

/** Every `<style>` element's text in the live page, hashed the way the
 *  CSP hashes it. Text is hashed in the test process — `crypto.subtle`
 *  is not guaranteed in every e2e engine. */
async function styleHashes(page: Page): Promise<Set<string>> {
  const texts = await page.evaluate(() =>
    [...document.querySelectorAll('style')].map((s) => s.textContent ?? ''),
  );
  return new Set(
    texts.map(
      (text) =>
        'sha256-' + createHash('sha256').update(text, 'utf8').digest('base64'),
    ),
  );
}

/** The source list `_headers` grants `style-src`. */
function styleSrcSources(): string[] {
  const directive = (SECURITY['Content-Security-Policy'] ?? '')
    .split(';')
    .map((d) => d.trim())
    .find((d) => d.startsWith('style-src '));
  if (directive === undefined) {
    throw new Error('no style-src directive in public/_headers');
  }
  return directive.split(/\s+/).slice(1);
}

/** Clicks the wizard's Choose file button and hands it `data`. */
async function chooseFile(page: Page, name: string, data: string): Promise<void> {
  const chooser = page.waitForEvent('filechooser');
  await page.getByRole('button', { name: 'Choose file' }).click();
  await (await chooser).setFiles({
    name,
    mimeType: 'text/csv',
    buffer: Buffer.from(data, 'utf8'),
  });
}

/** The plan's "run an import" below goes through `window.dataApi`, so
 *  the wizard UI itself is driven to the settings step here: its
 *  preview and settings components run under the policy. `atStep` is
 *  called once the preview and once the settings step are visible. */
async function driveWizard(
  page: Page,
  atStep: (step: string) => Promise<void>,
): Promise<void> {
  await page.goto('/p/demo/import');
  await chooseFile(page, 'input.csv', UTF8_COMMA);
  await expect(page.getByRole('columnheader', { name: /AGE/ })).toBeVisible();
  await atStep('wizard preview');
  await page.getByRole('button', { name: 'Next' }).click();
  await expect(
    page.getByRole('button', { name: 'Record key' }),
  ).toBeVisible();
  await atStep('wizard settings');
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

  for (const path of ROUTES) {
    await page.goto(path);
    await expect(page.locator('main')).toBeVisible();
    await expectClean(path);
  }

  await driveWizard(page, expectClean);

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

// Owner's CSP option A (PR #49): `style-src` is 'self' plus exactly the
// two sha256 sources below — the texts React Aria injects at runtime
// (usePress's touch-action rule, mounted with every pressable; and
// usePreventScroll's overscroll-behavior rule, present only while a
// modal overlay is open). This spec hashes every <style> element the
// app actually produces and asserts the set is exactly what _headers
// grants: a React Aria upgrade that changes either text changes its
// hash and fails here, and so does any new injected stylesheet.
test('runtime_styles_are_exactly_the_csp_hashes', async ({
  browser,
  page,
}) => {
  const sources = styleSrcSources();
  const expected = new Set(
    sources.filter((s) => s.startsWith("'sha256-")).map((s) => s.slice(1, -1)),
  );
  // 'self' + exactly two hashes; no 'unsafe-inline', nonces or other
  // sources (the owner's option A wording).
  expect(sources.filter((s) => s === "'self'")).toHaveLength(1);
  expect(expected.size).toBe(2);
  expect(sources).toHaveLength(3);

  const seen = new Set<string>();
  const collect = async () => {
    for (const h of await styleHashes(page)) seen.add(h);
  };

  for (const path of ROUTES) {
    await page.goto(path);
    await expect(page.locator('main')).toBeVisible();
    await collect();
  }

  await driveWizard(page, collect);

  // A real press that also opens a modal dialog: usePress's sheet is
  // injected with the first pressable, usePreventScroll's only while
  // the overlay is mounted.
  await page.goto('/');
  await page
    .getByRole('button', { name: 'New project', exact: true })
    .click();
  const dialog = page.getByRole('dialog', { name: 'New project' });
  await expect(dialog).toBeVisible();
  await collect();

  // Back out with a second press and collect once more after the
  // dialog — and its stylesheet — is gone.
  await dialog.getByRole('button', { name: 'Cancel' }).click();
  await expect(dialog).not.toBeVisible();
  await collect();

  // usePreventScroll's overscroll stylesheet only ever mounts on iOS
  // WebKit — react-aria gates it on `isIOS() && isWebKit()` — so the
  // second hash needs an iPhone-realm page: a Mobile-Safari UA plus
  // navigator.platform/maxTouchPoints spoofed before app code runs.
  const ios = await browser.newContext({
    userAgent:
      'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) ' +
      'AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 ' +
      'Mobile/15E148 Safari/604.1',
    hasTouch: true,
  });
  try {
    await applyHeaders(ios);
    await ios.addInitScript(() => {
      // `userAgentData` would win the platform check over
      // navigator.platform, so hide it too.
      Object.defineProperty(navigator, 'userAgentData', {
        get: () => undefined,
      });
      Object.defineProperty(navigator, 'platform', { get: () => 'iPhone' });
      Object.defineProperty(navigator, 'maxTouchPoints', { get: () => 5 });
      window.__cspViolations = [];
      document.addEventListener('securitypolicyviolation', (e) => {
        window.__cspViolations.push(
          `${e.violatedDirective} blocked ${e.blockedURI} (${e.sourceFile ?? 'inline'}:${e.lineNumber})`,
        );
      });
    });
    const iosPage = await ios.newPage();
    await iosPage.goto('/');
    await iosPage
      .getByRole('button', { name: 'New project', exact: true })
      .click();
    const iosDialog = iosPage.getByRole('dialog', { name: 'New project' });
    await expect(iosDialog).toBeVisible();
    for (const h of await styleHashes(iosPage)) seen.add(h);
    expect(
      await violations(iosPage),
      'CSP violations in the iOS realm',
    ).toEqual([]);
  } finally {
    await ios.close();
  }

  expect([...seen].sort()).toEqual([...expected].sort());
  expect(await violations(page), 'CSP violations in style sweep').toEqual([]);
});

// Desktop smoke test (v0.3 plan Task 15, spec §10.1): inside the real
// desktop window — create a project, import the utf8_comma fixture
// through the import wizard, run TABLES, and prove the on-disk database
// is not plaintext SQLite.
//
// The build under test is the test-hooks dist (VITE_TEST_HOOKS=1): the
// wizard's file dialog is driven by handing a *path* to the file input
// (WebDriver element send keys). WebKitWebDriver versions that cannot
// send paths to a file input fall back to injecting the same bytes into
// the same input — either way the wizard's own onSelect/onFile path
// runs. Node-side helpers read the fixture and the database file.
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';

const FIXTURE =
  process.env.EPIORA_FIXTURE ??
  fileURLToPath(
    new URL('../../fixtures/import/utf8_comma/input.csv', import.meta.url),
  );
const DB_FILE = join(
  process.env.EPIORA_APP_DATA_DIR ??
    join(process.env.HOME ?? '', '.local', 'share', 'org.epiora.app'),
  'epiora.db',
);

const TIMEOUT = 60_000;
const say = (msg) => console.log(`[smoke] ${msg}`);

const byText = (tag, text) =>
  $(`//${tag}[normalize-space()='${text}']`);

async function waitFor(el, what = 'element') {
  await el.waitForExist({ timeout: TIMEOUT, timeoutMsg: `no ${what}` });
  return el;
}

async function clickText(tag, text) {
  const el = await waitFor(byText(tag, text), `${tag} "${text}"`);
  await el.click();
}

/** Waits for the app shell — or the full-screen startup error. */
async function waitForApp() {
  await browser.waitUntil(
    async () =>
      (await byText('a', 'Epiora').isExisting()) ||
      (await $('//p[@role="alert"]').isExisting()),
    { timeout: 90_000, timeoutMsg: 'app did not render' },
  );
  const fatal = await $('//p[@role="alert"]');
  if (await fatal.isExisting()) {
    assert.fail(`startup error screen: ${await fatal.getText()}`);
  }
}

/** React Aria Select: click the trigger next to its label, then the
 *  named option. */
async function pick(label, option) {
  let trigger = await $(
    `//*[normalize-space()='${label}']/following-sibling::button[1]`,
  );
  if (!(await trigger.isExisting())) {
    trigger = await $(
      `//button[@aria-labelledby=//*[normalize-space()='${label}']/@id]`,
    );
  }
  await waitFor(trigger, `select "${label}"`);
  await trigger.click();
  await clickText('*[@role="option"]', option);
}

/** Chooses `path` in the wizard's file input. Prefers the WebDriver
 *  file-dialog path (element send keys); falls back to injecting the
 *  bytes when the driver cannot set files. */
async function chooseFile(path) {
  const input = await waitFor($('input[type=file]'), 'file input');
  let viaDriver = false;
  try {
    await input.setValue(path);
    viaDriver =
      (await browser.execute((el) => el.files.length, input)) === 1;
  } catch {
    viaDriver = false;
  }
  if (viaDriver) {
    say('file input set via webdriver path');
    return;
  }
  const base64 = readFileSync(path).toString('base64');
  const count = await browser.execute((payload, name) => {
    const bytes = Uint8Array.from(atob(payload), (c) => c.charCodeAt(0));
    const file = new File([bytes], name, { type: 'text/csv' });
    const transfer = new DataTransfer();
    transfer.items.add(file);
    const el = document.querySelector('input[type=file]');
    if (el === null) return 0;
    el.files = transfer.files;
    el.dispatchEvent(new Event('change', { bubbles: true }));
    return el.files.length;
  }, base64, 'input.csv');
  assert.equal(count, 1, 'could not set the wizard file input');
  say('file input set via in-page injection');
}

describe('Epiora desktop', () => {
  it('imports the fixture CSV, runs TABLES and encrypts the store', async () => {
    await waitForApp();
    say('app rendered');

    // New project through the real dialog: the submit is labelled
    // "Create" (t.common.create), not the trigger's "New project", so
    // it is addressed dialog-scoped.
    await clickText('button', 'New project');
    const dialog = await waitFor($('//*[@role="dialog"]'), 'project dialog');
    await (await dialog.$('.//input')).setValue('Smoke');
    await clickText('*[@role="dialog"]//button', 'Create');
    say('project created');

    await clickText('a', 'Smoke');
    await clickText('a', 'Import data');
    await waitFor(byText('button', 'Choose file'), 'wizard choose step');

    await chooseFile(FIXTURE);
    say('fixture chosen');

    // preview → settings → confirm → import
    await waitFor(
      $('//*[@role="columnheader" and normalize-space()="AGE"]'),
      'preview columns',
    );
    await clickText('button', 'Next');
    // "Record key" is the select's accessible name, not its text: the
    // Label span sits next to the trigger button (same pattern pick()
    // uses for the TABLES selects).
    await waitFor(
      $(`//*[normalize-space()='Record key']/following-sibling::button[1]`),
      'wizard settings step',
    );
    await clickText('button', 'Next');
    await waitFor(
      $('//h2[normalize-space()="Confirm import"]'),
      'wizard confirm step',
    );
    await clickText('button', 'Import');
    await waitFor(
      $('//h2[normalize-space()="Import complete"]'),
      'import done',
    );
    await waitFor(byText('p', 'Dataset version 1 created.'), 'version note');
    say('import complete');

    await clickText('a', 'Back to project');
    await clickText('a', 'Analysis workspace');

    // TABLES AGE × NAME; NAME is a two-level text variable so its
    // levels are picked explicitly (the engine never guesses).
    await clickText('button', 'Run procedure');
    await clickText('*[@role="menuitem"]', 'TABLES');
    await waitFor($('//*[@role="dialog"]'), 'TABLES dialog');
    await pick('Exposure variable', 'AGE');
    await pick('Outcome variable', 'NAME');
    await pick('NAME first level', 'alice');
    await pick('NAME second level', 'bob');
    await clickText('*[@role="dialog"]//button', 'Run');
    await waitFor(
      $('//*[normalize-space()="TABLES AGE × NAME — input.csv v1"]'),
      'TABLES result card',
    );
    say('TABLES result card shown');

    // The store file must be SQLite3MultipleCiphers ciphertext: an
    // unencrypted SQLite file starts with "SQLite format 3\0".
    assert.ok(existsSync(DB_FILE), `no database file at ${DB_FILE}`);
    const head = readFileSync(DB_FILE).subarray(0, 16);
    assert.notEqual(
      head.toString('latin1'),
      'SQLite format 3\0',
      'database file has a plaintext SQLite header',
    );
    say('database file is encrypted');
  });
});

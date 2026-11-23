// v1.1 Task 5: the committed wasm module must produce EPTB batches
// byte-identical to the native epiora-mdb build's batches, committed
// under crates/epiora-mdb/tests/data/batches/sample/ (regenerate with
// `cargo run -p epiora-mdb --bin dump-batches --
//   fixtures/legacy/samples/ei7/Sample/Sample.mdb
//   crates/epiora-mdb/tests/data/batches/sample`).
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { describe, expect, test } from 'vitest';
import { openMdb } from './mdb';

// Vitest runs with cwd = app/ (import.meta.url is rewritten to a dead
// dev-server URL here, so resolve against the repo root via cwd).
const repoRoot = resolve(process.cwd(), '..');
const sampleMdb = `${repoRoot}/fixtures/legacy/samples/ei7/Sample/Sample.mdb`;
const moduleUrl = pathToFileURL(resolve(process.cwd(), 'public/mdb/mdb.js')).href;

describe('epiora-mdb wasm module', () => {
  test('wasm_and_native_batches_identical', async () => {
    const bytes = new Uint8Array(readFileSync(sampleMdb));
    const mdb = await openMdb(bytes, undefined, moduleUrl);
    try {
      expect(mdb.format()).toBe('JET4');
      const tables = mdb.tables();
      expect(tables.length).toBe(93);
      for (const table of tables) {
        const expected = new Uint8Array(
          readFileSync(
            `${repoRoot}/crates/epiora-mdb/tests/data/batches/sample/${encodeURIComponent(table)}.eptb`,
          ),
        );
        const chunks: Uint8Array[] = [];
        for await (const chunk of mdb.batches(table)) chunks.push(chunk);
        expect(chunks.length, table).toBe(1);
        expect(chunks[0], table).toEqual(expected);
      }
    } finally {
      mdb.close();
    }
  });
});

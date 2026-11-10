// The shared conformance suite against native SQLite3MultipleCiphers
// (v0.3 plan Task 14 Step 2): `openStdioStore` spawns the test-only
// `store-stdio` binary — one database per suite run, every open() a new
// connection to it (the Task 8 decisions entry's contract).

import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { storeConformance } from './suite';
import { openStdioStore } from '../stdio';

const dir = mkdtempSync(join(tmpdir(), 'epiora-native-'));
const DB = join(dir, 'conformance.db');
const KEY = Buffer.from(crypto.getRandomValues(new Uint8Array(32))).toString(
  'hex',
);

storeConformance('native', () => openStdioStore(DB, KEY));

// gen-linelist.mjs — deterministic synthetic line list for the D3 query-bench
// spike (plan v0.1 Task 6). Seed 42, ChaCha20 keystream RNG (spec §7.4:
// seeded ChaCha; Math.random is never used).
//
// Browser:  import { generateLinelistCSV } from './gen-linelist.mjs'
// CLI:      node gen-linelist.mjs [rows] > linelist.csv
//
// generateLinelistCSV materializes the whole dataset as one CSV string —
// that is also how both engines ingest it (sqlite: parsed inserts; duckdb:
// registerFileText + COPY), so requesting ~50M rows overflows V8's maximum
// string length and throws a catchable RangeError instead of stalling.

export const SEED = 42;
export const DEFAULT_ROWS = 1_000_000;
export const HEADER = 'onset_date,age,sex,district,exposed,ill';

const DAY_MS = 86_400_000;
const ONSET_DAYS = 26 * 7; // onset_date uniform over 26 weeks
const ONSET_START = Date.UTC(2026, 0, 5); // 2026-01-05, a Monday
const DISTRICTS = Array.from({ length: 50 }, (_, i) => `district-${String(i + 1).padStart(2, '0')}`);

const rotl = (x, n) => (x << n) | (x >>> (32 - n));
function quarterRound(x, a, b, c, d) {
  x[a] = (x[a] + x[b]) >>> 0; x[d] = rotl(x[d] ^ x[a], 16);
  x[c] = (x[c] + x[d]) >>> 0; x[b] = rotl(x[b] ^ x[c], 12);
  x[a] = (x[a] + x[b]) >>> 0; x[d] = rotl(x[d] ^ x[a], 8);
  x[c] = (x[c] + x[d]) >>> 0; x[b] = rotl(x[b] ^ x[c], 7);
}

// ChaCha20 keystream: constants "expand 32-byte k", key = seed as u64 little
// endian followed by zeros, 64-bit counter, zero nonce. Only determinism
// matters here — the same seed reproduces the same stream.
export class ChaCha20 {
  #state = new Uint32Array(16);
  #block = new Uint32Array(16);
  #pos = 16;
  constructor(seed) {
    const s = this.#state;
    s.set([0x61707865, 0x3320646e, 0x79622d32, 0x6b206574]);
    s[4] = seed >>> 0;
    s[5] = Math.floor(seed / 0x1_0000_0000) >>> 0;
  }
  #refill() {
    const s = this.#state, b = this.#block;
    b.set(s);
    for (let i = 0; i < 10; i++) {
      quarterRound(b, 0, 4, 8, 12); quarterRound(b, 1, 5, 9, 13);
      quarterRound(b, 2, 6, 10, 14); quarterRound(b, 3, 7, 11, 15);
      quarterRound(b, 0, 5, 10, 15); quarterRound(b, 1, 6, 11, 12);
      quarterRound(b, 2, 7, 8, 13); quarterRound(b, 3, 4, 9, 14);
    }
    for (let i = 0; i < 16; i++) b[i] = (b[i] + s[i]) >>> 0;
    s[12] = (s[12] + 1) >>> 0;
    if (s[12] === 0) s[13] = (s[13] + 1) >>> 0;
    this.#pos = 0;
  }
  u32() {
    if (this.#pos >= 16) this.#refill();
    return this.#block[this.#pos++];
  }
  intBelow(n) { return Math.floor((this.u32() / 0x1_0000_0000) * n); }
}

const ISO_DATES = Array.from({ length: ONSET_DAYS }, (_, i) =>
  new Date(ONSET_START + i * DAY_MS).toISOString().slice(0, 10));

export function* generateRows(rowCount = DEFAULT_ROWS, seed = SEED) {
  const rng = new ChaCha20(seed);
  for (let i = 0; i < rowCount; i++) {
    yield {
      onset_date: ISO_DATES[rng.intBelow(ONSET_DAYS)],
      age: rng.intBelow(90),
      sex: rng.intBelow(2) === 0 ? 'M' : 'F',
      district: DISTRICTS[rng.intBelow(DISTRICTS.length)],
      exposed: rng.intBelow(2),
      ill: rng.u32() < 0.2 * 0x1_0000_0000 ? 1 : 0,
    };
  }
}

const toLine = (r) =>
  `${r.onset_date},${r.age},${r.sex},${r.district},${r.exposed},${r.ill}`;

// Whole dataset as one CSV string. For rowCount ~5e7 the accumulated text
// exceeds the maximum JS string length and throws RangeError — recorded by
// the bench harness as the "allocation failure" measurement. Rows are joined
// in CHUNK-sized slices so the cons-string tree carries few nodes: the live
// allocation when the length limit trips is ~the serialized bytes, not
// ~50M small nodes (which OOM-crashes the renderer before it can throw).
const CHUNK = 250_000;
export function generateLinelistCSV(rowCount = DEFAULT_ROWS, seed = SEED) {
  let csv = HEADER;
  let parts = [];
  for (const row of generateRows(rowCount, seed)) {
    parts.push(toLine(row));
    if (parts.length === CHUNK) { csv += '\n' + parts.join('\n'); parts = []; }
  }
  if (parts.length) csv += '\n' + parts.join('\n');
  return csv;
}

// CLI entry point: node gen-linelist.mjs [rows] [seed]
if (typeof process !== 'undefined' && process.argv[1]?.endsWith('gen-linelist.mjs')) {
  const rows = Number(process.argv[2] ?? DEFAULT_ROWS);
  const seed = Number(process.argv[3] ?? SEED);
  process.stdout.write(generateLinelistCSV(rows, seed) + '\n');
}

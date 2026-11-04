// Seeded row generator for the Chromium performance gate (spec §5.3).
// SplitMix32 — the same algorithm, seed handling and draw order as
// crates/epiora-core/benches/tables_1m.rs, so the native and wasm
// benches run on identical data. Pure ESM: imported by bench.html in
// the page and by bench.spec.mjs under Node.
//
// Draw order is column-major: every EXPOSED row first, then ILL, then
// REGION. One u32 per cell.

export const SEED = 20261031;
export const REGION_LEVELS = ["north", "south", "east"];

function splitmix32(seed) {
  let state = seed | 0;
  return () => {
    state = (state + 0x9e3779b9) | 0;
    let z = state;
    z = Math.imul(z ^ (z >>> 16), 0x21f0aaad);
    z = Math.imul(z ^ (z >>> 15), 0x735a2d97);
    return (z ^ (z >>> 15)) >>> 0;
  };
}

// Boolean column in the spec §5.1 wire encoding: 0/1 values, 255 =
// missing. u % 100 === 0 marks ~1% of rows missing; bit 8 of u is the
// boolean value.
function booleanColumn(name, n, next) {
  const values = new Uint8Array(n);
  for (let i = 0; i < n; i++) {
    const u = next();
    values[i] = u % 100 === 0 ? 255 : (u >>> 8) & 1;
  }
  return { name, type: "boolean", values };
}

// Text stratum (spec §5.1 wire encoding): dictionary + Uint32Array
// codes; no missing values.
function regionColumn(n, next) {
  const codes = new Uint32Array(n);
  for (let i = 0; i < n; i++) {
    codes[i] = next() % 3;
  }
  return { name: "REGION", type: "text", dictionary: REGION_LEVELS, codes };
}

// `generate(n, seed) → columns` in the spec §5.1 ColumnInput encoding,
// ready for `datasetFromColumns`.
export function generate(n, seed = SEED) {
  const next = splitmix32(seed);
  return [
    booleanColumn("EXPOSED", n, next),
    booleanColumn("ILL", n, next),
    regionColumn(n, next),
  ];
}

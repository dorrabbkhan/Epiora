// Replays the parity cases the native `native_and_wasm_bit_identical`
// test recorded (target/parity/epiora-legacy-wasm-cases.jsonl) through
// the real wasm32 `legacyCase` (the nodejs `pkg-parity` build) and
// prints the same {input,ok|err} lines — byte-identical when the wasm
// module and the native pipeline agree (v1.1 plan Task 13). Usage:
//
//   node crates/epiora-legacy-wasm/parity/run.mjs <cases.jsonl>
//
// CI builds the package with `scripts/wasm-build.sh parity
// epiora-legacy-wasm` and diffs this output against the JSONL.

import { createRequire } from "node:module";
import { readFileSync } from "node:fs";

const require = createRequire(import.meta.url);
const wasm = require("../pkg-parity/epiora_legacy_wasm.js");

const src = process.argv[2];
if (!src) {
  console.error("usage: run.mjs <cases.jsonl>");
  process.exit(2);
}

const lines = readFileSync(src, "utf8")
  .split("\n")
  .filter((l) => l.length > 0);

const out = lines.map((line) => {
  const rec = JSON.parse(line);
  try {
    return JSON.stringify({ input: rec.input, ok: wasm.legacyCase(rec.input) });
  } catch (e) {
    // js_error sets `code`; the message is the error's Display text —
    // the same pair the native side writes into its "err" object.
    return JSON.stringify({
      input: rec.input,
      err: { code: e?.code ?? "", message: String(e?.message ?? e) },
    });
  }
});
process.stdout.write(out.join("\n") + "\n");

// Drives the wasm parity build over the import fixtures and prints one
// parity_line per case — the same bytes `import-runner` prints natively
// (v0.3 spec §6.5). Usage:
//
//   node crates/epiora-import-wasm/parity/run.mjs <fixtures-import-dir>
//
// Case order matches import-runner: directory names sorted. Each case's
// expected.json supplies the input file name and the settings
// overrides, exactly like the native runner.

import { createRequire } from "node:module";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const require = createRequire(import.meta.url);
const wasm = require("../pkg-parity/epiora_import_wasm.js");

const root = process.argv[2];
if (!root) {
  console.error("usage: run.mjs <fixtures-import-dir>");
  process.exit(2);
}

const lines = [];
for (const caseId of readdirSync(root, { withFileTypes: true })) {
  if (!caseId.isDirectory()) continue;
  const dir = join(root, caseId.name);
  const expected = JSON.parse(readFileSync(join(dir, "expected.json"), "utf8"));
  const bytes = readFileSync(join(dir, expected.file));
  const settingsJson = JSON.stringify(expected.settings ?? {});
  lines.push(wasm.runImportCase(bytes, expected.file, settingsJson, caseId.name));
}
process.stdout.write(lines.join("\n") + "\n");

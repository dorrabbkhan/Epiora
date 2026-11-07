// Binding tests against the Node parity build (crates/epiora-import-wasm/
// pkg-parity, produced by `scripts/wasm-build.sh parity epiora-import-wasm`).
// Every check asserts the wasm import behaves identically to the native
// `import-runner` output recorded in target/import-native.jsonl (the
// check-engine/import.yml steps pass its utf8_comma line as
// NATIVE_IMPORT_UTF8_COMMA).

import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import test from "node:test";

const require = createRequire(import.meta.url);
const wasm = require("../../pkg-parity/epiora_import_wasm.js");

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..", "..", "..", "..");
const importFixtures = join(repo, "fixtures", "import");

function caseDir(caseId) {
  return join(importFixtures, caseId);
}

function inputBytes(caseId) {
  const expected = JSON.parse(readFileSync(join(caseDir(caseId), "expected.json"), "utf8"));
  return {
    expected,
    fileName: expected.file,
    bytes: readFileSync(join(caseDir(caseId), expected.file)),
  };
}

function nativeUtf8Comma() {
  return JSON.parse(process.env.NATIVE_IMPORT_UTF8_COMMA);
}

test("inspect utf8_comma matches expected detection", () => {
  const { expected, fileName, bytes } = inputBytes("utf8_comma");
  const inspection = wasm.inspect(bytes, fileName);
  assert.deepEqual(inspection.detection, expected.expect.detection);
});

test("importFile toSnapshot hash equals native", () => {
  const { fileName, bytes } = inputBytes("utf8_comma");
  const handle = wasm.importFile(bytes, fileName, {});
  const native = nativeUtf8Comma();
  const snapshot = handle.toSnapshot("utf8_comma", 1);
  const sha256 = createHash("sha256").update(snapshot).digest("hex");
  assert.equal(sha256, native.snapshot_sha256, "snapshot bytes identical to native");
  assert.equal(handle.contentHash(), native.snapshot_hash, "content hash equals native");
});

test("recordBatch pages cover every row once", () => {
  const { fileName, bytes } = inputBytes("utf8_comma");
  const handle = wasm.importFile(bytes, fileName, {});
  const rowCount = handle.summary().rowCount;
  const ids = [];
  const json = [];
  for (let start = 0; start < rowCount; start += 1) {
    const page = handle.recordBatch(start, 1);
    assert.equal(page.ids.length, 1);
    assert.equal(page.json.length, 1);
    ids.push(...page.ids);
    json.push(...page.json);
  }
  assert.equal(new Set(ids).size, rowCount, "every id exactly once");
  assert.equal(json.length, rowCount);
  for (const row of json) {
    assert.equal(typeof JSON.parse(row), "object");
  }
  assert.deepEqual(handle.recordBatch(rowCount, 10), { ids: [], json: [] });
});

test("diffAgainst identical is unchanged", () => {
  const { fileName, bytes } = inputBytes("utf8_comma");
  const handle = wasm.importFile(bytes, fileName, {});
  const diff = wasm.diffAgainst(handle.toSnapshot("utf8_comma", 1), handle);
  const summary = diff.summary();
  assert.equal(summary.unchanged, true);
  assert.equal(summary.added, 0);
  assert.equal(summary.removed, 0);
  assert.equal(summary.changed, 0);
  assert.deepEqual(diff.removed(), []);
  assert.deepEqual(diff.addedBatch(0, 10), { ids: [], json: [] });
  assert.deepEqual(diff.changedBatch(0, 10), []);
});

test("ragged row throws code ragged_row with line", () => {
  const { expected, fileName, bytes } = inputBytes("ragged_row_error");
  assert.throws(
    () => wasm.importFile(bytes, fileName, {}),
    (err) => {
      assert.ok(err instanceof Error);
      assert.equal(err.code, expected.expect_error.code);
      assert.equal(err.line, expected.expect_error.line);
      return true;
    },
  );
});

test("datasetFromRecords round-trips hash", () => {
  const { fileName, bytes } = inputBytes("utf8_comma");
  const handle = wasm.importFile(bytes, fileName, {});
  const { schema, rowCount } = handle.summary();
  const rows = [];
  for (let start = 0; start < rowCount; start += 64) {
    rows.push(...handle.recordBatch(start, 64).json);
  }
  const snapshot = wasm.datasetFromRecords("utf8_comma", 1, schema, rows);
  assert.deepEqual(
    [...snapshot],
    [...handle.toSnapshot("utf8_comma", 1)],
    "records rebuilt to the same snapshot bytes",
  );
});

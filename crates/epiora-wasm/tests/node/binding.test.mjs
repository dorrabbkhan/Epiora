// Binding tests against the Node parity build (crates/epiora-wasm/
// pkg-parity, produced by `scripts/wasm-build.sh parity`). Column
// transfer is the spec §5.1 encoding; every check asserts the decoded
// dataset behaves identically to the native engine.

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import test from "node:test";

const require = createRequire(import.meta.url);
const wasm = require("../../pkg-parity/epiora_wasm.js");

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..", "..", "..", "..");
const fixturesDir = join(repo, "fixtures");
const CTX = { timestamp: "2026-10-30T12:00:00Z" };

// spec §5.1 encoding: Integer → BigInt64Array + presence mask,
// Real → Float64Array + mask, Date → Int32Array + mask,
// Text → dictionary + Uint32Array codes (0xFFFFFFFF = missing),
// Boolean → Uint8Array (0, 1, 255 = missing).
function columnInput(column) {
  const values = column.values;
  const presence = new Uint8Array(values.map((v) => (v === null ? 0 : 1)));
  switch (column.type) {
    case "integer":
      return {
        name: column.name,
        type: "integer",
        values: new BigInt64Array(values.map((v) => (v === null ? 0n : BigInt(v)))),
        presence,
      };
    case "real":
      return {
        name: column.name,
        type: "real",
        values: new Float64Array(values.map((v) => (v === null ? 0 : v))),
        presence,
      };
    case "date":
      return {
        name: column.name,
        type: "date",
        values: new Int32Array(
          values.map((v) => (v === null ? 0 : Math.floor(Date.parse(`${v}T00:00:00Z`) / 86400000))),
        ),
        presence,
      };
    case "text": {
      const dictionary = [...new Set(values.filter((v) => v !== null))];
      const index = new Map(dictionary.map((s, i) => [s, i]));
      return {
        name: column.name,
        type: "text",
        dictionary,
        codes: new Uint32Array(values.map((v) => (v === null ? 0xffffffff : index.get(v)))),
      };
    }
    case "boolean":
      return {
        name: column.name,
        type: "boolean",
        values: new Uint8Array(values.map((v) => (v === null ? 255 : v ? 1 : 0))),
      };
    default:
      throw new Error(`unknown column type ${column.type}`);
  }
}

function datasetHandle(path) {
  const doc = JSON.parse(readFileSync(path, "utf8"));
  return wasm.datasetFromColumns(doc.id, doc.version, doc.columns.map(columnInput));
}

const oswegoJson = readFileSync(join(fixturesDir, "data", "oswego.dataset.json"), "utf8");
const oswegoDoc = JSON.parse(oswegoJson);

function freqNames(handle, name) {
  return wasm.freq(
    handle,
    { variables: [name], stratify_by: null, include_missing: true, ci_method: "clopper_pearson" },
    CTX,
  );
}

test("oswego hash matches native", () => {
  const handle = datasetHandle(join(fixturesDir, "data", "oswego.dataset.json"));
  assert.equal(wasm.datasetHash(handle), process.env.NATIVE_OSWEGO_HASH);
});

test("typed-array transfer round-trips every type and missing", () => {
  const handle = wasm.datasetFromColumns("roundtrip", 1, [
    {
      name: "I",
      type: "integer",
      values: new BigInt64Array([10n, 0n, 20n]),
      presence: new Uint8Array([1, 0, 1]),
    },
    {
      name: "R",
      type: "real",
      values: new Float64Array([1.5, 0, -2.25]),
      presence: new Uint8Array([1, 0, 1]),
    },
    {
      name: "T",
      type: "text",
      dictionary: ["a", "b"],
      codes: new Uint32Array([0, 0xffffffff, 1]),
    },
    { name: "B", type: "boolean", values: new Uint8Array([0, 1, 255]) },
    {
      name: "D",
      type: "date",
      values: new Int32Array([19723, 0, 19000]), // 2024-01-01, missing, 2022-01-08
      presence: new Uint8Array([1, 0, 1]),
    },
  ]);
  for (const name of ["I", "R", "T", "B", "D"]) {
    const out = freqNames(handle, name);
    assert.equal(out.result.tables[0].missing, 1, `${name}: missing count`);
    assert.equal(out.result.tables[0].total, 3, `${name}: total`);
    assert.deepEqual(
      out.result.tables[0].labels.at(-1),
      "(missing)",
      `${name}: (missing) label is last`,
    );
  }
  const dates = freqNames(handle, "D").result.tables[0];
  assert.deepEqual(dates.labels, ["2022-01-08", "2024-01-01", "(missing)"]);
});

test("text missing code 0xFFFFFFFF", () => {
  const handle = wasm.datasetFromColumns("t", 1, [
    {
      name: "T",
      type: "text",
      dictionary: ["a", "b"],
      codes: new Uint32Array([0, 0xffffffff, 0xffffffff, 1]),
    },
  ]);
  const table = freqNames(handle, "T").result.tables[0];
  assert.equal(table.missing, 2);
  assert.equal(table.categories.at(-1).count, 2);
});

test("boolean 255 is missing", () => {
  const handle = wasm.datasetFromColumns("b", 1, [
    { name: "B", type: "boolean", values: new Uint8Array([0, 1, 255]) },
  ]);
  const table = freqNames(handle, "B").result.tables[0];
  assert.equal(table.missing, 1);
  assert.deepEqual(table.labels, ["Yes", "No", "(missing)"]);
});

test("tables via handle equals runCorpusCase receipt", () => {
  const handle = wasm.datasetFromColumns(
    oswegoDoc.id,
    oswegoDoc.version,
    oswegoDoc.columns.map(columnInput),
  );
  const out = wasm.tables(
    handle,
    {
      exposure: { name: "VANILLA", levels: null },
      outcome: { name: "ILL", levels: null },
      stratify_by: [],
      include_missing: false,
    },
    CTX,
  );

  const datasets = {};
  for (const file of readdirSync(join(fixturesDir, "data"))) {
    if (file.endsWith(".dataset.json")) {
      const doc = JSON.parse(readFileSync(join(fixturesDir, "data", file), "utf8"));
      datasets[doc.id] = doc;
    }
  }
  const fixtureJson = readFileSync(join(fixturesDir, "2x2", "oswego_vanilla.json"), "utf8");
  const line = JSON.parse(wasm.runCorpusCase(fixtureJson, JSON.stringify(datasets)));
  const corpusReceipt = JSON.parse(line.receipt);
  assert.equal(out.receipt.receipt_sha256, corpusReceipt.receipt_sha256);
});

test("unknown variable throws with code unknown_variable", () => {
  const handle = wasm.datasetFromColumns("b", 1, [
    { name: "B", type: "boolean", values: new Uint8Array([0, 1, 1]) },
  ]);
  assert.throws(
    () =>
      wasm.freq(
        handle,
        { variables: ["NOPE"], stratify_by: null, include_missing: false, ci_method: "clopper_pearson" },
        CTX,
      ),
    (err) => {
      assert.ok(err instanceof Error);
      assert.equal(err.code, "unknown_variable");
      return true;
    },
  );
  assert.throws(
    () =>
      wasm.tables(
        handle,
        {
          exposure: { name: "NOPE", levels: null },
          outcome: { name: "B", levels: null },
          stratify_by: [],
          include_missing: false,
        },
        CTX,
      ),
    (err) => err.code === "unknown_variable",
  );
});

test("engineVersion reports the engine version", () => {
  assert.equal(typeof wasm.engineVersion(), "string");
});

test("snapshot round-trips oswego and hash matches native", () => {
  const handle = datasetHandle(join(fixturesDir, "data", "oswego.dataset.json"));
  const bytes = wasm.datasetToSnapshot(handle);
  const restored = wasm.datasetFromSnapshot(oswegoDoc.id, oswegoDoc.version, bytes);
  assert.equal(wasm.datasetHash(restored), process.env.NATIVE_OSWEGO_HASH);
});

test("snapshot bytes equal native", () => {
  const handle = datasetHandle(join(fixturesDir, "data", "oswego.dataset.json"));
  const hex = Buffer.from(wasm.datasetToSnapshot(handle)).toString("hex");
  assert.equal(hex, process.env.NATIVE_OSWEGO_SNAPSHOT_HEX.trim());
});

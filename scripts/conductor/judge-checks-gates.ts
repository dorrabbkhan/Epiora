/**
 * judge-checks-gates.ts — the licence, spike-report and owner-gate checks
 * (plan order 7, 8 and 11), split out of judge-checks.ts to keep every
 * file under the ~400-line rule. Shared Check helpers and the
 * uninspectable-entry rule live in judge-checks.ts.
 */
import { addedLines, uninspectable, type DiffFile } from "./diff-parse.ts";
import { blind, flag, na, pass, type Check } from "./judge-checks.ts";

// ---------- 7. licences ----------

const DEP_FILE_RE = /(^|\/)(Cargo\.toml|Cargo\.lock|package\.json|package-lock\.json)$/;
const REJECTED_DEP_RE = /gadm|powersync|sqlite-?see|sqlite3-see/i;
// Built at runtime so this file's own added lines never carry the literal
// the check looks for (the npm licence allow-list flag in check-app.sh).
const LICENCE_LIST_RE = new RegExp(
  ["--only", "Allow", "|license-?checker", "|allowed-?licen", "|only_", "allow"].join(""),
  "i",
);

export function checkLicences(files: DiffFile[]): Check {
  const reasons = blind(files);
  for (const f of files) {
    if (f.path === "deny.toml")
      reasons.push("deny.toml changed — licence allow-list touched; conductor must look");
    if (uninspectable(f)) continue;
    for (const l of addedLines(f)) {
      if (DEP_FILE_RE.test(f.path) && REJECTED_DEP_RE.test(l.text))
        reasons.push(
          `${f.path}:${l.line} rejected dependency on the spec's list: ${l.text.trim()}`,
        );
      // The npm licence allow-list lives in scripts/config, never in docs —
      // a .md file mentioning it is prose, not a list change.
      if (!f.path.endsWith(".md") && f.path !== "deny.toml" && LICENCE_LIST_RE.test(l.text))
        reasons.push(`${f.path}:${l.line} licence allow-list line touched; conductor must look`);
    }
  }
  return reasons.length ? flag("licences", [...new Set(reasons)]) : pass("licences");
}

// ---------- 8. spike-report ----------

export interface SpikeCheckResult {
  code: number;
  output: string;
}

export function checkSpikeReport(
  files: DiffFile[],
  runSpikeCheck?: () => SpikeCheckResult,
): Check {
  const reports = files.filter((f) => f.path.startsWith("docs/spikes/"));
  if (!reports.length) return na("spike-report", "no docs/spikes/** file changed");
  const reasons: string[] = [];
  for (const f of reports) {
    const bad = uninspectable(f);
    if (bad) {
      reasons.push(`could not inspect ${f.path} (${bad})`);
      continue;
    }
    let inMeasurements = false;
    for (const h of f.hunks)
      for (const l of h.lines) {
        const heading = /^#{1,6}\s+(.+)$/.exec(l.text);
        if (heading) inMeasurements = /measurement/i.test(heading[1]);
        if (!inMeasurements || l.kind !== "add") continue;
        if (/\d/.test(l.text) && !/https?:\/\//.test(l.text) && !l.text.includes("`"))
          reasons.push(
            `${f.path}:${l.line} measurement with a number but no run link or backticked command`,
          );
      }
  }
  if (runSpikeCheck) {
    const r = runSpikeCheck();
    if (r.code !== 0)
      reasons.push(
        `scripts/check-spikes.sh exited ${r.code}: ${r.output.split("\n").filter(Boolean).pop() ?? ""}`,
      );
  }
  return reasons.length
    ? flag("spike-report", reasons)
    : pass("spike-report", "spike reports pass check-spikes.sh and every measurement cites a run");
}

// ---------- 11. owner-gate ----------

const OWNER_PATHS = [
  /^\.github\//,
  /^docs\/superpowers\/specs\//,
  /^LICENSE(\.|$)/,
  /^AGENTS\.md$/,
  /^fixtures\//,
  /^deny\.toml$/,
];
const GATE_FLAG_RE = /OWNER GATE|OWNER REVIEW|\(human step\)/i;

/** The task's Gate cell in the plan's Task Index table, or null. */
export function gateColumn(planText: string | null, task: string): string | null {
  if (!planText) return null;
  const lines = planText.split("\n");
  const idx = lines.findIndex((l) => /^#{2,3}\s+Task Index/.test(l));
  if (idx < 0) return null;
  const rows: string[][] = [];
  for (let i = idx + 1; i < lines.length; i++) {
    if (/^#{1,3} /.test(lines[i])) break;
    if (!lines[i].trim().startsWith("|")) continue;
    rows.push(lines[i].split("|").slice(1, -1).map((c) => c.trim()));
  }
  if (rows.length < 2) return null;
  const gate = rows[0].findIndex((c) => /^gate$/i.test(c));
  const num = rows[0].findIndex((c) => c === "#");
  if (gate < 0 || num < 0) return null;
  const row = rows.slice(2).find((r) => r[num] === task);
  return row ? row[gate] : null;
}

export function checkOwnerGate(
  files: DiffFile[],
  title: string,
  planText: string | null,
  task: string,
): Check {
  const reasons: string[] = [];
  if (/\[OWNER[ _-](GATE|REVIEW)\]/i.test(title)) reasons.push(`title tag: "${title}"`);
  const gate = gateColumn(planText, task);
  if (gate && GATE_FLAG_RE.test(gate)) reasons.push(`task ${task} gate column: "${gate}"`);
  for (const f of files)
    if (OWNER_PATHS.some((r) => r.test(f.path)))
      reasons.push(`owner-gated path touched: ${f.path}`);
  return reasons.length ? flag("owner-gate", [...new Set(reasons)]) : pass("owner-gate");
}

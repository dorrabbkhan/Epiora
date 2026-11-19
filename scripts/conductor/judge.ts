/**
 * judge.ts — the mechanical half of CLAUDE.md "Judgment rules" (conductor
 * efficiency plan Task 0a). Ported from babelgate's judge.ts.
 *
 *   scripts/conductor/judge.sh <PR> <version> <task>
 *   node scripts/conductor/judge.ts --pr <PR> --version <v> --task <N>
 *   node scripts/conductor/judge.ts --task <N> [--version <v>]
 *       --diff-file <patch> --plan <plan.md> [--claimed out.md …] [--title t]
 *       [--report-dir <dir>]
 *
 * Stdout (≤ 60 lines): one `PASS|FLAG|N/A <check-id> <reason> [file:line]`
 * per check — at most five items per line, each whole, then "(+k more in
 * the report)" — then `JUDGE: PASS` or `JUDGE: FLAG (<n>)`. Full detail
 * goes to <report-dir>/judge-<stamp>.md (default ~/epiora-conductor/<v>-task<N>).
 * Exit 0 = all pass, 1 = any FLAG, 2 = usage error.
 *
 * A check that cannot run on part of the diff — binary file, rename-only
 * entry, truncated hunk — FLAGs naming the file. Never a silent PASS.
 */
import { execFileSync, spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  realpathSync,
  writeFileSync,
} from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { parseDiff } from "./diff-parse.ts";
import {
  checkAssertions,
  checkDiff,
  checkDiffStat,
  checkFilesBlock,
  checkFixtures,
  checkNumbers,
  checkSecrets,
  checkSkips,
  checkSpikesImport,
  CONTENT_CHECK_IDS,
  type Check,
} from "./judge-checks.ts";
import {
  checkLicences,
  checkLineCount,
  checkOwnerGate,
  checkSpikeReport,
  type SpikeCheckResult,
} from "./judge-checks-gates.ts";

export interface JudgeOpts {
  pr?: string;
  version?: string;
  task: string;
  diffFile?: string;
  plan?: string;
  /** One path, or several (--claimed may repeat); each stat list unions. */
  claimed?: string | string[];
  title?: string;
  reportDir?: string;
}

export interface JudgeResult {
  exitCode: number;
  stdout: string;
  report: string;
  reportPath?: string;
}

/** Fetch/run seams so tests can exercise the failure paths offline. */
export interface JudgeDeps {
  fetchDiff?: (pr: string) => string;
  fetchTitle?: (pr: string) => string;
  runSpikeCheck?: () => SpikeCheckResult;
}

// One stdout line per check: at most five items, each whole — stdout is for
// the conductor's context; the report file lists every reason.
const summarize = (reasons: string[], max = 5) =>
  reasons.length <= max
    ? reasons.join("; ")
    : `${reasons.slice(0, max).join("; ")}; (+${reasons.length - max} more in the report)`;
const firstLine = (e: unknown) =>
  String(e instanceof Error ? e.message : e).split("\n")[0];

const gh = (args: string[]) =>
  execFileSync("gh", args, { encoding: "utf8", maxBuffer: 64 << 20 });

// `gh pr diff` fails on huge PRs (HTTP 406 "diff too large"): fall back to
// fetching the head oid and diffing locally against the base branch.
function ghPrDiff(pr: string): string {
  try {
    return gh(["pr", "diff", pr]);
  } catch (e) {
    try {
      const meta = JSON.parse(
        gh(["pr", "view", pr, "--json", "baseRefName,headRefOid"]),
      ) as { baseRefName: string; headRefOid: string };
      execFileSync("git", ["fetch", "origin", meta.baseRefName, meta.headRefOid]);
      return execFileSync("git", ["diff", `origin/${meta.baseRefName}...${meta.headRefOid}`], {
        encoding: "utf8",
        maxBuffer: 64 << 20,
      });
    } catch (e2) {
      throw new Error(`${firstLine(e)}; local-diff fallback failed: ${firstLine(e2)}`);
    }
  }
}

const ghPrTitle = (pr: string) =>
  gh(["pr", "view", pr, "--json", "title", "--jq", ".title"]);

const couldNotRun = (why: string): Check[] =>
  CONTENT_CHECK_IDS.map((id) => ({
    id,
    status: "FLAG" as const,
    reasons: [`could not run: ${why}`],
  }));

/** scripts/check-spikes.sh, run from the repo root. */
function defaultSpikeCheck(): SpikeCheckResult {
  const root = execFileSync("git", ["rev-parse", "--show-toplevel"], {
    encoding: "utf8",
  }).trim();
  const r = spawnSync("bash", ["scripts/check-spikes.sh"], {
    encoding: "utf8",
    cwd: root,
  });
  return { code: r.status ?? 1, output: `${r.stdout ?? ""}\n${r.stderr ?? ""}` };
}

/** Newest docs/superpowers/plans/*epiora-<version>-*.md containing
 * `### Task <task>:` — a version may have several plan files (v1.0 does). */
function findPlan(version: string, task: string): string | undefined {
  const planDir = "docs/superpowers/plans";
  if (!existsSync(planDir)) return undefined;
  const needle = `### Task ${task}:`;
  for (const f of readdirSync(planDir)
    .filter((x) => x.includes(`epiora-${version}-`) && x.endsWith(".md"))
    .sort()
    .reverse())
    if (readFileSync(join(planDir, f), "utf8").includes(needle)) return join(planDir, f);
  return undefined;
}

export function runJudge(opts: JudgeOpts, deps: JudgeDeps = {}): JudgeResult {
  if (!opts.task || (!opts.pr && !opts.diffFile))
    return { exitCode: 2, stdout: "", report: "usage error: need --task and --pr or --diff-file" };

  let diffText: string | null = null;
  let diffErr: string | null = null;
  if (opts.diffFile) {
    diffText = readFileSync(opts.diffFile, "utf8");
  } else {
    try {
      diffText = (deps.fetchDiff ?? ghPrDiff)(opts.pr!);
    } catch (e) {
      diffErr = firstLine(e);
    }
  }
  const files = diffText === null ? [] : parseDiff(diffText);

  const dir =
    opts.reportDir ??
    join(homedir(), "epiora-conductor", `${opts.version ?? "dev"}-task${opts.task}`);

  let checks: Check[];
  if (diffErr !== null) {
    checks = [
      { id: "diff", status: "FLAG", reasons: [`could not fetch PR diff: ${diffErr}`] },
      ...couldNotRun("could not fetch PR diff"),
    ];
  } else if (files.length === 0) {
    checks = [checkDiff(files), ...couldNotRun("empty or unparseable diff")];
  } else {
    let title = opts.title ?? "";
    let titleErr = false;
    if (opts.pr && !opts.title)
      try {
        title = (deps.fetchTitle ?? ghPrTitle)(opts.pr).trim();
      } catch {
        titleErr = true;
      }

    const planPath = opts.plan ?? (opts.version ? findPlan(opts.version, opts.task) : undefined);
    const planText = planPath && existsSync(planPath) ? readFileSync(planPath, "utf8") : null;

    // The claimed stat defaults to every task-* and fix-* session output
    // under ~/epiora-conductor/<v>-task<N>/ — a PR's diff is the task
    // session plus its fix rounds, so the claimed file set is the union
    // across them. Never a review's .out.md: it may belong to a session
    // still running and has no diff-stat block yet. An explicit --claimed
    // (repeatable) overrides the scan and unions the same way.
    const explicit = opts.claimed
      ? Array.isArray(opts.claimed)
        ? opts.claimed
        : [opts.claimed]
      : [];
    const claimedPaths = explicit.length
      ? explicit
      : existsSync(dir)
        ? readdirSync(dir)
            .filter((f) => /^(?:task|fix)-.+\.out\.md$/.test(f))
            .sort()
            .map((f) => join(dir, f))
        : [];
    const claimedTexts = claimedPaths
      .filter((p) => existsSync(p))
      .map((p) => readFileSync(p, "utf8"));
    const claimedText = claimedTexts.length ? claimedTexts.join("\n") : null;

    checks = [
      checkDiff(files),
      checkFixtures(files),
      checkNumbers(files),
      checkAssertions(files),
      checkSkips(files),
      checkFilesBlock(files, planText, opts.task),
      checkLineCount(files),
      checkSpikesImport(files),
      checkLicences(files),
      checkSpikeReport(files, deps.runSpikeCheck ?? defaultSpikeCheck),
      checkSecrets(files),
      checkDiffStat(files, claimedText),
      checkOwnerGate(files, title, planText, opts.task),
    ];
    if (titleErr) {
      const og = checks.find((c) => c.id === "owner-gate")!;
      og.status = "FLAG";
      og.reasons.unshift("could not read title");
    }
  }

  const nFlags = checks.filter((c) => c.status === "FLAG").length;
  const lines = checks.map(
    (c) => `${c.status} ${c.id}${c.reasons.length ? " " + summarize(c.reasons) : ""}`,
  );
  lines.push(nFlags ? `JUDGE: FLAG (${nFlags})` : "JUDGE: PASS");
  const stdout = lines.slice(0, 60).join("\n");

  const stamp = new Date().toISOString().replace(/[-:T]/g, "").replace(/\..+$/, "");
  const report = [
    `# judge — ${opts.pr ? `PR #${opts.pr}` : (opts.diffFile ?? "diff")} task ${opts.task} ${stamp}`,
    "",
    `files: ${files.length} · ${files.map((f) => f.path).join(", ")}`,
    "",
    ...checks.flatMap((c) => [
      `## ${c.status} ${c.id}`,
      ...(c.reasons.length ? c.reasons.map((r) => `- ${r}`) : ["- (no detail)"]),
      "",
    ]),
    nFlags ? `JUDGE: FLAG (${nFlags})` : "JUDGE: PASS",
  ].join("\n");

  let reportPath: string | undefined;
  try {
    mkdirSync(dir, { recursive: true });
    reportPath = join(dir, `judge-${stamp}.md`);
    writeFileSync(reportPath, report);
  } catch (e) {
    process.stderr.write(`judge: could not write report to ${dir}: ${e}\n`);
  }

  return { exitCode: nFlags ? 1 : 0, stdout, report, reportPath };
}

const KNOWN_OPTS = new Set([
  "pr",
  "version",
  "task",
  "diffFile",
  "plan",
  "claimed",
  "title",
  "reportDir",
]);
const USAGE =
  "usage: judge.ts (--pr <PR> | --diff-file <patch>) --task <N> " +
  "[--version <v>] [--plan <plan.md>] [--claimed <out.md> …] [--title <t>] [--report-dir <d>]\n";

export function main(argv: string[]): number {
  const opts = { task: "" } as JudgeOpts & Record<string, string | string[]>;
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const key = a.startsWith("--")
      ? a.slice(2).replace(/-([a-z])/g, (_, c) => c.toUpperCase())
      : "";
    if (!KNOWN_OPTS.has(key) || i + 1 >= argv.length) {
      process.stderr.write(USAGE);
      return 2;
    }
    if (key === "claimed") {
      const prev = opts.claimed;
      opts.claimed = [...(Array.isArray(prev) ? prev : prev ? [prev] : []), argv[++i]];
    } else {
      opts[key] = argv[++i];
    }
  }
  if (!opts.task || (!opts.pr && !opts.diffFile)) {
    process.stderr.write(USAGE);
    return 2;
  }
  const r = runJudge(opts);
  if (r.stdout) process.stdout.write(r.stdout + "\n");
  return r.exitCode;
}

const invoked = process.argv[1] ? realpathSync(process.argv[1]) : "";
if (invoked && import.meta.url === pathToFileURL(invoked).href)
  process.exit(main(process.argv.slice(2)));

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { main, runJudge, type JudgeResult } from "./judge.ts";

/**
 * judge.ts — the mechanical half of CLAUDE.md "Judgment rules" (plan Task 0a).
 * One seeded diff per case under fixtures/judge/; --diff-file/--plan/--claimed
 * run fully offline (no gh calls, no network).
 */
const ROOT = join(import.meta.dirname, "..", "..");
const PLAN = join(import.meta.dirname, "fixtures", "judge", "plan.md");
const FIX = join(import.meta.dirname, "fixtures", "judge");

const spikeOk = () => ({ code: 0, output: "all spike reports ok" });

function judge(diff: string, extra: Partial<Parameters<typeof runJudge>[0]> = {}): JudgeResult {
  return runJudge(
    {
      version: "v9.9",
      task: "1",
      diffFile: join(FIX, diff),
      plan: PLAN,
      title: "chore(conductor): judge script, waits, usage report",
      reportDir: mkdtempSync(join(tmpdir(), "judge-test-")),
      ...extra,
    },
    { runSpikeCheck: spikeOk },
  );
}

const line = (r: JudgeResult, id: string) =>
  r.stdout.split("\n").find((l) => l.includes(` ${id} `) || l.includes(` ${id}`));

// Stub gh: counts "--json name --jq length" calls in $GH_STUB_COUNT and
// answers from the $GH_STUB_ANSWERS comma list (repeating the last); any
// other call (the --watch, the final table) succeeds.
const GH_STUB = `#!/usr/bin/env bash
if [[ " $* " == *" --json name "* && " $* " == *" --jq length"* ]]; then
  n=$(cat "$GH_STUB_COUNT" 2>/dev/null || echo 0); n=$((n+1)); echo "$n" > "$GH_STUB_COUNT"
  IFS=, read -ra a <<< "$GH_STUB_ANSWERS"
  i=$((n-1)); [ "$i" -ge "\${#a[@]}" ] && i=$((\${#a[@]}-1))
  echo "\${a[$i]}"
  exit 0
fi
echo "checks table"
exit 0
`;

// Stub gh as 2.46 behaves: every `pr checks --json` fails with
// "unknown flag: --json"; `pr view --json statusCheckRollup` still answers;
// `--watch` exits 1 (a red CI); a bare `pr checks` prints one fail row and
// one pass row; `run view` prints a 50-line log. All calls are recorded to
// $GH_STUB_CALLS.
const GH_STUB_FAILING = `#!/usr/bin/env bash
echo "$*" >> "$GH_STUB_CALLS"
if [[ " $* " == *" statusCheckRollup "* ]]; then
  echo 2
  exit 0
fi
if [[ " $* " == *" --json "* ]]; then
  echo "unknown flag: --json" >&2
  exit 1
fi
if [[ " $* " == *" --watch "* ]]; then
  exit 1
fi
if [[ " $1 $2 " == " pr checks " ]]; then
  printf 'lint\tfail\t12s\thttps://github.com/o/r/actions/runs/1/job/111\n'
  printf 'check\tpass\t1m31s\thttps://github.com/o/r/actions/runs/2/job/222\n'
  exit 0
fi
if [[ " $1 $2 " == " run view " ]]; then
  seq 1 50
  exit 0
fi
exit 0
`;

function runWaitCi(env: Record<string, string>, stub = GH_STUB) {
  const dir = mkdtempSync(join(tmpdir(), "wait-ci-"));
  writeFileSync(join(dir, "gh"), stub, { mode: 0o755 });
  return {
    dir,
    r: spawnSync("bash", ["scripts/conductor/wait-ci.sh", "123"], {
      encoding: "utf8",
      cwd: ROOT,
      env: {
        ...process.env,
        PATH: `${dir}:${process.env.PATH}`,
        GH_STUB_COUNT: join(dir, "count"),
        GH_STUB_CALLS: join(dir, "calls"),
        ...env,
      },
    }),
  };
}

describe("judge.ts (conductor efficiency plan Task 0a)", () => {
  it("fixture change flags", () => {
    const r = judge("fixture-change.diff");
    assert.equal(r.exitCode, 1);
    assert.match(line(r, "fixtures")!, /^FLAG fixtures .*fixtures\/ei7\/two-proportion\.json/);
  });

  it("tolerance number change flags", () => {
    const r = judge("tolerance.diff");
    assert.equal(r.exitCode, 1);
    assert.match(line(r, "numbers")!, /^FLAG numbers .*stats\.rs.*1e-9.*→.*1e-8/);
  });

  it("removed assert flags", () => {
    const r = judge("removed-assert.diff");
    assert.equal(r.exitCode, 1);
    assert.match(line(r, "assertions")!, /^FLAG assertions .*removed.*added/);
  });

  it("assert_eq weakened to assert flags", () => {
    const r = judge("weakened-assert.diff");
    assert.equal(r.exitCode, 1);
    assert.match(line(r, "assertions")!, /^FLAG assertions .*assert_eq.*assert/);
  });

  it("exact matcher weakened flags", () => {
    const r = judge("weakened-matcher.diff");
    assert.equal(r.exitCode, 1);
    assert.match(line(r, "assertions")!, /^FLAG assertions .*toMatchObject/);
  });

  it("added ignore attribute flags", () => {
    const r = judge("added-ignore.diff");
    assert.equal(r.exitCode, 1);
    assert.match(line(r, "skips")!, /^FLAG skips .*#\[ignore/);
  });

  it("added .only flags", () => {
    const r = judge("added-only.diff");
    assert.equal(r.exitCode, 1);
    assert.match(line(r, "skips")!, /^FLAG skips .*only/);
  });

  it("file outside Files block without decisions entry flags", () => {
    const r = judge("extra-file.diff");
    assert.equal(r.exitCode, 1);
    assert.match(line(r, "files-block")!, /^FLAG files-block .*src\/extra\.ts/);
  });

  it("file outside Files block with decisions entry passes", () => {
    const r = judge("extra-file-decisions.diff");
    assert.match(line(r, "files-block")!, /^PASS files-block/);
  });

  it("bare file name in Files block resolves by basename", () => {
    const r = judge("bare-basename.diff");
    assert.match(line(r, "files-block")!, /^PASS files-block/);
  });

  it("glob question mark matches one char, dots are literal", () => {
    const one = judge("glob-question.diff");
    assert.match(line(one, "files-block")!, /^PASS files-block/);
    const two = judge("glob-question-two.diff");
    assert.match(line(two, "files-block")!, /^FLAG files-block .*src\/glob-ab\.ts/);
    const dots = judge("glob-dots.diff");
    assert.match(line(dots, "files-block")!, /^FLAG files-block .*src\/foo1test2ts/);
  });

  it("long reason is never cut mid-item", () => {
    const r = judge("long-reason.diff");
    assert.equal(r.exitCode, 1);
    const l = line(r, "numbers")!;
    // the reason ends with the whole added line — a sliced reason would end in "…"
    assert.ok(l.includes("0.77777] };"), l);
    assert.ok(!l.includes("…"), l);
  });

  it("default claimed ignores review output", () => {
    const dir = mkdtempSync(join(tmpdir(), "judge-claimed-"));
    for (const f of ["task-20261101-000000.out.md", "review-20261231-235959.out.md"])
      writeFileSync(join(dir, f), "session output\nno stat block here\n");
    writeFileSync(
      join(dir, "fix-20261102-000000.out.md"),
      readFileSync(join(FIX, "claimed-clean.out.md"), "utf8"),
    );
    const r = judge("clean.diff", { reportDir: dir });
    assert.match(line(r, "diff-stat")!, /^PASS diff-stat/);
  });

  it("claimed files are the union of task and fix sessions", () => {
    const dir = mkdtempSync(join(tmpdir(), "judge-union-"));
    writeFileSync(
      join(dir, "task-20261101-000000.out.md"),
      "task output\n\n src/foo.ts | 3 ++-\n 1 file changed, 2 insertions(+), 1 deletion(-)\n",
    );
    writeFileSync(
      join(dir, "fix-20261102-000000.out.md"),
      "fix output\n\n src/foo.test.ts | 3 ++-\n docs/decisions.md | 3 ++-\n" +
        " 2 files changed, 4 insertions(+), 2 deletions(-)\n",
    );
    // a review's stat block never joins the union
    writeFileSync(
      join(dir, "review-20261103-000000.out.md"),
      "review output\n\n src/review-only.ts | 1 +\n 1 file changed, 1 insertion(+)\n",
    );
    const r = judge("clean.diff", { reportDir: dir });
    assert.match(line(r, "diff-stat")!, /^PASS diff-stat/);
    assert.ok(!r.report.includes("review-only"), r.report);
    // repeated --claimed unions the same way
    const r2 = judge("clean.diff", {
      claimed: [
        join(dir, "task-20261101-000000.out.md"),
        join(dir, "fix-20261102-000000.out.md"),
      ],
    });
    assert.match(line(r2, "diff-stat")!, /^PASS diff-stat/);
  });

  it("stdout lists at most five items per check", () => {
    const r = judge("many-files.diff", { claimed: join(FIX, "claimed-clean.out.md") });
    const fb = line(r, "files-block")!;
    assert.match(fb, /^FLAG files-block /);
    const items = fb.replace(/^FLAG files-block /, "").split("; ");
    assert.equal(items.filter((i) => !i.startsWith("(+")).length, 5, fb);
    assert.match(items[items.length - 1], /^\(\+2 more in the report\)$/);
    const ds = line(r, "diff-stat")!;
    assert.match(ds, /\(\+5 more in the report\)/, ds);
    // the report file lists every item
    for (const p of ["src/extra6.ts", "src/extra7.ts", "docs/decisions.md"])
      assert.ok(r.report.includes(p), p);
  });

  it("import of spikes from crates flags", () => {
    const r = judge("spikes-import.diff");
    assert.equal(r.exitCode, 1);
    assert.match(line(r, "spikes-import")!, /^FLAG spikes-import .*spikes/);
  });

  it("docs/spikes path string is not a spikes import", () => {
    const r = judge("spikes-docs-path.diff");
    assert.equal(r.exitCode, 0, r.stdout);
    assert.match(line(r, "spikes-import")!, /^PASS spikes-import/);
  });

  it("rejected licence dependency flags", () => {
    const r = judge("licence.diff");
    assert.equal(r.exitCode, 1);
    assert.match(line(r, "licences")!, /^FLAG licences .*powersync/);
  });

  it("spike report number without run link flags", () => {
    const r = judge("spike-report.diff");
    assert.equal(r.exitCode, 1);
    assert.match(line(r, "spike-report")!, /^FLAG spike-report .*wasm-mdb\.md/);
  });

  it("api key literal flags", () => {
    const r = judge("api-key.diff");
    assert.equal(r.exitCode, 1);
    // the flag names file:line but never echoes the key itself
    assert.match(line(r, "secrets")!, /^FLAG secrets .*key\/token.*x\.sh:2/);
    assert.ok(!line(r, "secrets")!.includes("sk-ant"));
  });

  it("diff stat mismatch with claimed files flags", () => {
    const r = judge("clean.diff", { claimed: join(FIX, "claimed-mismatch.out.md") });
    assert.equal(r.exitCode, 1);
    assert.match(line(r, "diff-stat")!, /^FLAG diff-stat .*other\.ts/);
  });

  it("owner gate title, gate column or workflow file flags owner-gate", () => {
    const byTitle = judge("clean.diff", {
      claimed: join(FIX, "claimed-clean.out.md"),
      title: "[OWNER REVIEW] chore(conductor): x",
    });
    assert.match(line(byTitle, "owner-gate")!, /^FLAG owner-gate .*title/);
    const byColumn = judge("clean.diff", {
      claimed: join(FIX, "claimed-clean.out.md"),
      task: "2",
    });
    assert.match(line(byColumn, "owner-gate")!, /^FLAG owner-gate .*gate column/);
    const byFile = judge("owner-gate-workflow.diff");
    assert.match(line(byFile, "owner-gate")!, /^FLAG owner-gate .*workflows\/new\.yml/);
    assert.equal(byFile.exitCode, 1);
  });

  it("clean task diff passes all", () => {
    const r = judge("clean.diff", { claimed: join(FIX, "claimed-clean.out.md") });
    assert.equal(r.exitCode, 0);
    assert.match(r.stdout, /JUDGE: PASS/);
    assert.ok(r.stdout.split("\n").filter(Boolean).length <= 60);
  });

  it("unparseable diff flags, never passes", () => {
    const r = judge("unparseable.diff");
    assert.equal(r.exitCode, 1);
    // binary + rename-only + truncated hunk: every check that could not run
    // must say so — never a silent PASS.
    assert.match(line(r, "secrets")!, /^FLAG secrets .*could not inspect/);
    assert.match(line(r, "assertions")!, /^FLAG assertions .*could not inspect/);
    assert.match(r.stdout, /^JUDGE: FLAG \(\d+\)$/m);
  });

  it("diff fetch failure flags", () => {
    const r = runJudge(
      {
        pr: "999999",
        version: "v9.9",
        task: "1",
        plan: PLAN,
        title: "chore(conductor): x",
        reportDir: mkdtempSync(join(tmpdir(), "judge-test-")),
      },
      {
        fetchDiff: () => {
          throw new Error("HTTP 406: diff too large\nresponse body");
        },
        runSpikeCheck: spikeOk,
      },
    );
    assert.equal(r.exitCode, 1);
    assert.match(r.stdout, /FLAG diff could not fetch PR diff: HTTP 406: diff too large/);
    assert.match(r.stdout, /^JUDGE: FLAG \(\d+\)$/m);
  });

  it("missing args exit 2", () => {
    assert.equal(main(["--pr", "1"]), 2); // no --task
    assert.equal(main(["--bogus", "x", "--task", "1"]), 2); // unknown flag
    assert.equal(main(["--task", "1", "--diff-file"]), 2); // flag without value
  });
});

describe("conductor shell scripts", () => {
  it("wait-ci waits for checks to appear", () => {
    const { dir, r } = runWaitCi({ WAIT_CI_POLL_S: "0", GH_STUB_ANSWERS: "0,0,2" });
    assert.equal(r.status, 0);
    assert.equal(readFileSync(join(dir, "count"), "utf8").trim(), "3");
  });

  it("wait-ci times out with no checks", () => {
    const { r } = runWaitCi({ WAIT_CI_POLL_S: "0", WAIT_CI_MAX_S: "0", GH_STUB_ANSWERS: "0" });
    assert.equal(r.status, 3);
    assert.match(r.stderr, /wait-ci: no checks registered after/);
  });

  it("wait-ci prints failing jobs from plain gh output", () => {
    const { dir, r } = runWaitCi({ WAIT_CI_POLL_S: "0" }, GH_STUB_FAILING);
    assert.equal(r.status, 1);
    assert.match(r.stdout, /--- failing checks:\nlint\n/);
    // the job-111 block is the last output: the header plus exactly lines
    // 11–50 of the 50-line stub log (tail -40).
    const tail40 = Array.from({ length: 40 }, (_, i) => `${i + 11}`).join("\n");
    assert.ok(r.stdout.includes("--- job 111: last 40 failed-log lines\n" + tail40 + "\n"));
    const calls = readFileSync(join(dir, "calls"), "utf8");
    assert.match(calls, /run view --job 111 --log-failed/);
    assert.doesNotMatch(calls, /--job 222/);
  });

  it("argc guards exit 2", () => {
    for (const s of ["judge.sh", "wait-ci.sh", "build-prompt.sh"]) {
      const r = spawnSync("bash", [`scripts/conductor/${s}`], { encoding: "utf8", cwd: ROOT });
      assert.equal(r.status, 2, s);
      assert.match(r.stderr, /usage:/);
    }
  });

  it("build-prompt.sh unknown task exits 2", () => {
    const r = spawnSync("bash", ["scripts/conductor/build-prompt.sh", "task", "v1.0", "99"], {
      encoding: "utf8",
      cwd: ROOT,
    });
    assert.equal(r.status, 2);
    assert.match(r.stderr, /build-prompt\.sh:/);
  });

  it("devin.sh empty prompt exits 2 without calling devin", () => {
    const dir = mkdtempSync(join(tmpdir(), "devin-stub-"));
    const marker = join(dir, "devin-called");
    writeFileSync(
      join(dir, "devin"),
      `#!/usr/bin/env bash\ntouch "${marker}"\nexit 0\n`,
      { mode: 0o755 },
    );
    const r = spawnSync("bash", ["scripts/conductor/devin.sh", "task", "v1.0", "0a"], {
      encoding: "utf8",
      cwd: ROOT,
      input: "",
      env: { ...process.env, PATH: `${dir}:${process.env.PATH}`, HOME: dir },
    });
    assert.equal(r.status, 2);
    assert.ok(!existsSync(marker), "devin must not be called on an empty prompt");
    assert.match(r.stderr, /empty prompt/);
  });
});

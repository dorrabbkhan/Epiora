---
name: epiora-task
description: Implement one numbered task from an Epiora plan, test-first, on its own branch. Usage - /epiora-task [plan] <task number>
---

You are implementing exactly ONE task from a plan file. The arguments are `[<plan>] <task number>`. If no task
number was given, ask for one and stop.

## Plan selection

Map `<plan>` to a file:

- `v0.1` → `docs/superpowers/plans/2026-10-28-epiora-v0.1-spikes.md`
- A missing `<plan>` (only a task number was given) means the newest file in `docs/superpowers/plans/` (by
  date in the filename).

The matching spec is read from the plan's `**Spec:**` line, not hard-coded.

## Procedure

1. Run `git status`. If the working tree is not clean, stop and tell the user.
2. Run `git checkout main && git pull`, then create a branch named `<plan>/task-<N>-<short-name>`.
3. Read, in this order:
   - `AGENTS.md`
   - the plan's "Global Constraints", "Review Focus" and "Task Index" sections
   - the full "### Task <N>:" section
   - every spec section (§) that the task references, in the spec file named on the plan's `**Spec:**` line
4. Check the task's **Interfaces → Consumes** list against the code on main. If anything is missing or has a
   different name or signature, stop and report. Do not create stand-ins.
5. Write the failing tests (or checks) exactly as the task lists them, using the plan's values.
6. Run them and confirm they fail for the expected reason. Show the output.
7. Implement the minimum needed, using the exact names and signatures from **Interfaces → Produces**.
8. Iterate with `./scripts/check.sh quick`. Once complete, run the full `./scripts/check.sh` once before
   pushing. Fix until everything is green.
   8a. For spikes: every number in the report links to the run or command that produced it. A failed run is
       recorded as a measurement, not hidden.
   8b. Compare `git diff --name-only main` with the task's Files block. For every file outside it, add a
       "Files outside the Files block" line to this task's entry in docs/decisions.md, saying why. Include it
       in the same commit.
9. Commit with the message the task specifies, push the branch, and open a PR with `gh pr create` — title it
   with `[OWNER GATE]` or `[OWNER REVIEW]` if the Task Index marks the task so; the PR body is the final
   summary headings below.
10. Start `scripts/epiora-loop.sh watch` so the check watch runs in the background, then run the
    `/epiora-review` pass immediately — review overlaps CI, it does not wait for it. Block on
    `scripts/epiora-loop.sh await` only before writing the final summary. Fix failures by pushing to the same
    branch.
11. Finish by reporting the PR URL and the check results, plus the summary under these headings: Task; Spec
    sections; Tests added; Measurements (with run links); Decision reached; Ambiguities I resolved (or
    "None"); Follow-ups noticed.

## Rules

- Plan and spec values are authoritative. If a reference or fixture test fails, the implementation is wrong.
  Never change expected values, thresholds or decision rules.
- Never edit anything under `fixtures/` except by running `Rscript fixtures/generate.R` in v0.1 Task 5.
- Never skip, delete or loosen a test, and never relax a lint rule.
- Never simulate a "(human step)": stop with a `STOP:` line naming it.
- Do not start work that belongs to any other task. List it under "Follow-ups noticed" instead.
- If the same failure survives 3 different fix attempts, stop and report what you tried.
- Do not add dependencies beyond the plan's Tech Stack without asking first.

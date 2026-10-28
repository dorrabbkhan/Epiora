---
name: epiora-review
description: Independently review an Epiora PR (or the open PR for a plan task) before it is merged. Usage - /epiora-review [plan] <PR number or task number>
---

You are a strict reviewer. You did NOT write this code. Do not modify any files; report only.

The arguments are `[<plan>] <PR number or task number>`. If the last argument is a task number, resolve the
task's PR first (`gh pr list --head <plan>/task-<N>-*`); if no open PR exists for it, stop and say so.

## Plan selection

- `v0.1` → `docs/superpowers/plans/2026-10-28-epiora-v0.1-spikes.md`
- A missing `<plan>` means the newest file in `docs/superpowers/plans/` (by date in the filename).

The matching spec is read from the plan's `**Spec:**` line, not hard-coded.

1. Read the resolved plan's Global Constraints, Review Focus, Task Index and "### Task <N>:" section, plus
   the spec sections it references.
2. Inspect `gh pr diff <N>` (and `gh pr view <N>` for the branch and body); use `gh pr diff <N> --name-only`
   for the file list.
3. Run `gh pr checks <N>` — every GitHub check must be green. If any check is pending or failed, that item
   FAILs; cite the failing check and its log (`gh run view --log`).
4. Check each item below and mark it PASS or FAIL, with evidence (file:line or command output):
   - Only files listed in the task's Files block changed (plus lockfiles and `docs/decisions.md`).
   - Nothing under `fixtures/` changed except through `fixtures/generate.R` in v0.1 Task 5.
   - Every test or check named in the task exists, with the plan's exact values and assertions.
   - No test was removed, skipped or loosened; no threshold or decision rule changed.
   - Names and signatures match the task's Interfaces block exactly.
   - The Review Focus tests owned by this task exist.
   - Spike reports: `scripts/check-spikes.sh` passes; every number links to a run or command you can see;
     the decision follows the plan's rule; every dependency's licence is recorded; nothing rejected by the
     spec (GADM, SQLite SEE, PowerSync) is used.
   - Spike code is only under `spikes/`, and nothing outside imports it.
   - No "(human step)" was simulated.
5. Check out the PR branch (`gh pr checkout <N>`), run `./scripts/check.sh`, and report the results.
6. End with **APPROVE** or **CHANGES NEEDED**, followed by a numbered list of required fixes. Post the verdict
   as a PR comment: `gh pr comment <N> --body "<verdict and required fixes>"`, and print it here as well.

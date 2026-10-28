# AGENTS.md — Epiora

Epiora is a local-first, open-core successor to CDC's Epi Info: a Rust statistics engine compiled to wasm and
native, a React app, and R (WebR) on demand. The current version, v0.1, is a set of throwaway **spikes** that
test risky technical bets and record decisions.

**The source of truth is `docs/superpowers/specs/2026-10-28-epiora-platform-design.md`, and for each task, the
plan named by the conductor.** Read the spec sections your task references before writing code. If the spec
or plan and this file disagree, the spec and plan win. If they are ambiguous or seem wrong, stop and ask
(`STOP:` line). Do not guess.

## Commands

- **`./scripts/check.sh quick` — the iteration gate.** Spike report checker, shell tests, and the tests of
  any spike whose folder changed (`cargo test` for Rust spikes, `node --test` for Node spikes).
- **`./scripts/check.sh` — the full local gate; run it once before every push.** All of the above for every
  spike, plus Playwright spikes when `E2E=1`. CI-only checks (Windows, macOS, Android emulator) run on GitHub.
- Spike report checker: `scripts/check-spikes.sh` (created by v0.1 Task 1).
- Regenerate R fixtures (only when the task says so): `Rscript fixtures/generate.R`.

## Hard rules

1. **Fixtures are the oracle.** Never edit anything in `fixtures/` by hand. If a reference test fails,
   assume the implementation is wrong. Never set ALLOW_FIXTURE_CHANGE; only the owner does.
2. Tests ship in the same PR as the code they test. Write the test first.
3. **Measurements come from runs.** Every number in a spike report links to the workflow run or states the
   exact command that produced it. Never estimate, round up or invent a measurement; if a run fails, the
   failure is the measurement.
4. **Decision rules are fixed by the plan.** Apply the rule written in your task; do not change thresholds.
5. **Spike code stays in `spikes/<name>/`** and is never imported from outside it.
6. **Licences:** record the licence of every dependency you add. Never use GADM data, the SQLite Encryption
   Extension (SEE) or PowerSync (spec decisions A4, A6, A7).
7. Any randomness uses a seeded RNG, and the seed is recorded in the report. No `Math.random` in measured code.
8. Keep files single-purpose and under ~400 lines.
9. Do not add dependencies beyond the plan's Tech Stack without saying why in the PR description.
10. After editing, verify every changed file with `git diff` before committing. If a write doesn't land,
    rewrite the whole file and re-verify.
11. **Human steps:** a step marked "(human step)" is never simulated. Stop with a `STOP:` line naming it.

## Workflow

- One task per branch, named `v0.1/task-<N>-<short-name>`.
- Iterate with `./scripts/check.sh quick`; run the full `./scripts/check.sh` once before pushing. Do not
  disable, skip or loosen tests to make it pass.
- Push the branch and open a PR with `gh pr create`; the PR body is the final summary.
- Start the review immediately after `gh pr create` — it runs in parallel with CI
  (`scripts/epiora-loop.sh watch` backgrounds the check watch; `scripts/epiora-loop.sh await` blocks for green
  only before the final summary).
- The PR is done when all GitHub checks are green. Never merge; the owner merges.
- Final summary: task, spec sections, tests added, measurements with run links, decision reached, anything
  ambiguous you resolved.
- Record any deviation from the task's Files block, interfaces or plan in `docs/decisions.md` in the same PR.
- Stay inside the task's scope. Note follow-ups in the PR description instead of doing them.

## Layout

- `spikes/<name>/`: throwaway probe code, one folder per spike
- `docs/spikes/`: one report per spike; `docs/decisions.md`: decision log
- `fixtures/`: R-generated reference values and the script that generates them
- `docs/superpowers/`: spec and plans · `docs/conductor/`: conductor notes and rulesets

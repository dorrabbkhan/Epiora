# v1.0 Task 19 — conductor notes

## Precondition

The `means_1m` threshold is the number the owner posts on PR #94 (Task 18). If no owner comment on #94
names a number, STOP: `STOP: no owner means_1m threshold on PR #94`. Copy it exactly; never use the
"proposal" in docs/perf/v1.0-measurement.md.

## Add: faster perf measurement runs (owner, 2026-11-19, "much faster from now on")

One 10-sample `workflow_dispatch` perf run took 91 minutes in Task 18, and each rerun took as long again.
Measurement runs must only measure what the task needs. Add to `.github/workflows/app.yml` and `app/perf/`:

- Two `workflow_dispatch` inputs: `scenarios` (string, comma-separated scenario names, default empty = all)
  and `cpu4` (boolean, default `false`; whether the D7 CPU-throttled ×4 report pass runs on a dispatch).
- Push to `main` is unchanged: every scenario runs, the throttled pass runs, the gate evaluates every gated
  scenario. The filter and `cpu4=false` apply only to `workflow_dispatch`.
- `PERF_SCENARIOS` (env from the input) filters which scenarios run in `perf.spec.ts`. A scenario that
  another needs as its fixture (`import_1m` for `reimport_1m`, `open_tables_1m`, `means_1m`, `grid_scroll_1m`)
  still runs when a dependent is listed. A scenario not run is not recorded at all (no skip line).
- `gate-eval.ts`: on a filtered run, evaluate only the gated scenarios that ran and print
  `<name>: not run (filtered)` for the others; on an unfiltered run any missing gated scenario stays a FAIL,
  exactly as today. Add unit tests for both cases next to the existing gate-eval tests.
- Thresholds, sample counts on push (5), batch ids and the summary format do not change.
- Record these files and the owner instruction under "Files outside the Files block" in this task's
  docs/decisions.md entry.

Use the filter for this task's own measurement runs: for example `runs=5`,
`scenarios=import_1m,open_tables_1m,means_1m,grid_scroll_1m` and `cpu4=false` for Step 2 and Step 3.
Paste the run links. The push-to-main gate on merge still runs everything.

## Runner variance (context, PR #94)

GitHub perf runners fall into two speed classes. On slow ones `import_1m` is about 102 s and `open_tables_1m`
about 2.90 s, so both fail their gates whatever the code. Investigation:
https://github.com/dorrabbkhan/Epiora/pull/94#issuecomment-7010429729. If a gate fails, report the run's
"Build wasm packages" step time next to the medians (about 21 s on a fast runner, 24–26 s on a slow one). Do
not rerun more than once yourself; the conductor applies the PR #55 policy.

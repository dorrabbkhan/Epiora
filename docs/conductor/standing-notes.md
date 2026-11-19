Verify every write with git diff before committing. Use ./scripts/check.sh quick while iterating and the
full check before pushing. If a parity test, premise, budget or decision rule fails, stop and report (STOP:
line); don't change tolerances, thresholds, fixtures or decision rules on your own. Every number in a spike
report links to the run or command that produced it; a failed run is a measurement. Never simulate a
"(human step)". Record any file outside the task's Files block in docs/decisions.md. Spike code stays in
spikes/<name>/.
A spike report's Decision names one of its task's plan options verbatim; never invent a label. Any binary or
archive a workflow downloads is pinned by version and SHA-256, and its licence is recorded from what the
download itself states (D2 covers the Epi Info source at 4cd353c only, not CDC release binaries).
Conductor process (context only, your contract is unchanged): since v1.0 Task 0b the conductor runs one headless `/conduct` session per task via `scripts/conductor/run-plan.sh` with the explicit allow-list in `scripts/conductor/headless-tools.txt`, judges diffs with `scripts/conductor/judge.sh` and reads long logs through the `log-reader` Haiku subagent. Keep your `git diff --stat`, Files-block deviations and decisions.md entries exact: judge.sh checks them mechanically and a mismatch is a FLAG, not a pass. Paste `git diff --stat`/`git show --stat` verbatim — never abbreviated (`*.diff | 21 files`) or summarised; judge.sh's diff-stat check compares it file by file.
Speed (owner, 2026-11-19): a task session ends at the push + PR — no CI wait, no self-review, no local full
`./scripts/check.sh` (targeted tests + `./scripts/check.sh quick` only; CI is the gate and the conductor waits
for it). devin.sh already exports `PLAYWRIGHT_BROWSERS_PATH` (`.pw-browsers/`, all three engines installed),
`npm_config_cache` (`.npm-cache/`), `CARGO_HOME` (`.cargo-home/`, seeded from ~/.cargo) and puts `.local-bin/`
(cargo-deny) on PATH — use them, never reinstall. If npm reports ECONNREFUSED on a 127.0.0.1 proxy port, set
`npm_config_proxy=http://localhost:3128 npm_config_https_proxy=http://localhost:3128` for that command and move on.

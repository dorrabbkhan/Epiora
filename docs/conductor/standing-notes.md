Verify every write with git diff before committing. Use ./scripts/check.sh quick while iterating and the
full check before pushing. If a parity test, premise, budget or decision rule fails, stop and report (STOP:
line); don't change tolerances, thresholds, fixtures or decision rules on your own. Every number in a spike
report links to the run or command that produced it; a failed run is a measurement. Never simulate a
"(human step)". Record any file outside the task's Files block in docs/decisions.md. Spike code stays in
spikes/<name>/.
A spike report's Decision names one of its task's plan options verbatim; never invent a label. Any binary or
archive a workflow downloads is pinned by version and SHA-256, and its licence is recorded from what the
download itself states (D2 covers the Epi Info source at 4cd353c only, not CDC release binaries).

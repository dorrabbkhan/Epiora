# Conductor usage baseline — 2026-11-17

Aggregates only — counts, tokens, $, tool names. No prompt text, no tool inputs or outputs.

## Provenance

The numbers below were produced by the **conductor** with babelgate's `usage-report.ts`
(the Task 0a reference implementation) against `~/.claude/projects/-home-dorrabb-Epiora`
with `--since 2026-10-28`, before this task's `scripts/conductor/usage-report.ts` existed.
The session-log directory is not part of the repository, so the raw output was handed to the
task session as `.conductor-ref/usage-baseline.txt` and `.conductor-ref/usage-baseline.json`
(untracked, never committed). The totals differ slightly from plan E1 (3,943 turns / $235.66
here vs 3,932 / $234.60 in the plan) because this output is a later run of the same command;
both are measurements of the real logs.

## Summary (all sessions since 2026-10-28, all models)

| version | sessions | tasks | turns | cache-read | cache-write | output | $ | $opus | $/task median | ctx-first median | ctx-mean median |
|---|---|---|---|---|---|---|---|---|---|---|---|
| unattributed | 57 | 1 | 2,323 | 232.12M | 6.92M | 1.44M | 127.215 | 127.097 | 127.215 | 61.2k | 87.3k |
| v0.1 | 1 | 1 | 38 | 3.18M | 62.9k | 21.2k | 1.564 | 1.564 | 1.564 | 63.5k | 85.3k |
| v0.2 | 4 | 3 | 561 | 64.46M | 2.20M | 380.2k | 38.140 | 38.140 | 4.546 | 64.8k | 117.0k |
| v0.3 | 2 | 2 | 129 | 11.62M | 435.0k | 71.3k | 7.231 | 7.231 | 3.616 | 48.0k | 91.0k |
| v1.0 | 7 | 7 | 892 | 114.02M | 3.61M | 490.8k | 61.510 | 61.510 | 7.405 | 45.1k | 130.1k |
| **total** | 71 | 14 | 3,943 | 425.40M | 13.23M | 2.40M | 235.661 | 235.543 | 5.925 | 50.4k | 91.8k |

## v1.0 attributed tasks (per task, all models)

| task | sessions | turns | input | cache-read | cache-write | output | $ | ctx1~ | ctxμ~ |
|---|---|---|---|---|---|---|---|---|---|
| v1.0-task2 | 1 | 73 | 170 | 7.08M | 399.9k | 39.8k | 5.414 | 44.9k | 102.5k |
| v1.0-task3 | 1 | 111 | 252 | 16.41M | 553.2k | 82.1k | 9.351 | 45.6k | 152.8k |
| v1.0-task4 | 1 | 285 | 674 | 38.28M | 665.6k | 134.9k | 15.682 | 45.0k | 136.7k |
| v1.0-task5 | 1 | 101 | 240 | 12.23M | 253.8k | 49.7k | 5.471 | 45.0k | 123.6k |
| v1.0-task7 | 1 | 115 | 262 | 14.70M | 262.7k | 66.8k | 6.378 | 45.1k | 130.1k |
| v1.0-task8 | 1 | 103 | 242 | 13.56M | 441.9k | 57.8k | 7.405 | 45.7k | 135.9k |
| v1.0-task10 | 1 | 104 | 240 | 11.76M | 1.03M | 59.6k | 11.809 | 45.2k | 123.0k |

v1.0 median $/task = **$7.41** (median of the seven task totals; the plan's target for Tasks
11+ is ≤ $4.00/task).

## E1–E2 cost split (priced at Opus 4 / 20 / 0.20 / 8 per 1M tokens)

| scope | cache-write $ (tok × $8/M) | cache-read $ (tok × $0.20/M) | output $ (tok × $20/M) | input $ | total $ |
|---|---|---|---|---|---|
| all sessions | 105.85 (13.23M) | 85.08 (425.40M) | 48.06 (2.40M) | 0.04 | 235.66* |
| v1.0 | 28.88 (3.61M) | 22.80 (114.02M) | 9.82 (490.8k) | 0.01 | 61.51 |

\* Components sum to $238.99; the $3.33 remainder over $235.66 is non-Opus turns priced at
Sonnet/Haiku rates (the component costs above price every turn at the Opus table for
comparison with E2).

New context entering the cache (cache-write) is the largest cost, then re-reading it
(cache-read); thinking/output is the smallest — the split the plan's E2 describes.

## Re-run with this task's script

`scripts/conductor/usage-report.ts` was run against the same logs:

```
node scripts/conductor/usage-report.ts --since 2026-10-28
```

Its summary table (same day, so it also counts a few turns of the session that ran it):

```
summary
version        sess tasks  turns  cache-rd  cache-wr   output        $    $opus  $/task~    ctx1~    ctxμ~
unattributed     11     1    278    30.27M     1.16M   331.2k   21.809   21.805   21.809    47.4k    69.0k
v0.1             12    11    475    44.76M     1.35M   243.2k   24.616   24.616    1.623    63.3k    86.3k
v0.2             19    14   1106   113.53M     3.46M   658.1k   63.545   63.545    2.338    64.8k    94.0k
v0.3             19    18   1048   108.71M     3.23M   585.9k   56.138   56.023    2.297    48.0k    82.8k
v1.0             10    10   1047   129.64M     4.04M   589.6k   70.041   70.041    5.925    45.2k   123.3k
total            71    54   3954   426.90M    13.24M    2.41M  236.149  236.031    2.399    50.4k    91.9k
```

The new attribution rules (E6) re-attribute most of the 57 `unattributed` sessions to real
tasks — 11 remain unattributed — which is why per-version medians shift. The baseline for the
before/after comparison after three product tasks is this table's v1.0 section.

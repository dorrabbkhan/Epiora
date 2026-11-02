# v0.2 Task 9 — conductor notes

## Owner decision (2026-11-02, PR #18): z-based 95% intervals use z = 1.96 in Epi Info 7

Epi Info 7 builds its z-based 95% confidence intervals with z = 1.96 exactly; R (and the corpus, and the
engine) use qnorm(0.975) = 1.959963984540054. Evidence (conductor, from the committed fixtures):
recomputing the corpus interval with z = 1.96 reproduces every printed Epi Info digit —

| Interval | Corpus (qnorm) | Corpus rescaled to 1.96 | Epi Info 7 |
|---|---|---|---|
| OR Taylor (`two_by_two.odds_ratio.ci_95_wald`) | 5.841191–94.178692 | 5.8410–94.1811 | 5.8410–94.1811 |
| RR Taylor (`two_by_two.risk_ratio.ci_95_wald`) | 1.938340–16.029337 | 1.9383–16.0296 | 1.9383–16.0296 |
| MH RR (`stratified_two_by_two.mantel_haenszel.risk_ratio.ci_95`) | 1.986792–16.512399 | 1.9868–16.5127 | 1.9868–16.5127 |
| MH OR RGB (`stratified_two_by_two.mantel_haenszel.odds_ratio.ci_95`) | 7.191633–197.000244 | 7.1914–197.0062 | 7.1914–197.0062 |

The owner chose option (a): **the engine keeps qnorm(0.975); the Epi Info comparison converts z-based
interval bounds to z = 1.96 before applying the unchanged display rule.** Concretely:

- The conversion lives in the comparison code (`corpus/ei7.rs` or the `ei7` test), never in `stats` or
  `procedures`. Do not add a z / confidence-level option to the engine.
- For a ratio interval (log scale) with engine bounds `lo`, `hi`: `c = sqrt(lo·hi)`,
  `se = (ln hi − ln lo) / (2·qnorm(0.975))`, converted bounds `c·exp(∓1.96·se)`. For an additive interval
  (e.g. risk difference): `m = (lo+hi)/2`, `se = (hi−lo)/(2·qnorm(0.975))`, bounds `m ∓ 1.96·se`. Use the
  qnorm constant above as a literal with a comment, and `libm` for exp/ln/sqrt.
- Apply it ONLY to bounds of z-based intervals: keys ending `ci_95_wald.lower|upper`, the
  `mantel_haenszel.*.ci_95.lower|upper` keys, and (if Step 5 shows Wilson) a Wilson proportion CI.
  Point estimates, p-values, chi-squares, and Fisher / mid-p exact limits are compared unconverted.
- The display rule `|engine − value| ≤ 0.5 × 10^−decimals` is unchanged. If a z-based bound still fails
  after conversion, or any other value fails, the plan's Step 4 STOP applies as written.
- Record this in the `### v0.2 Task 9 — Epi Info 7 method choices` entry in docs/decisions.md with the table
  above and the list of labels it applies to. Do not write an "Owner sign-off" line; the owner adds it.

## Units, not tolerances

- Epi Info prints `Risk Difference (RD%)` and its limits on the percent scale; the corpus `risk.difference`
  leaves are 0–1. Compare `engine × 100` against Epi Info's value with the unchanged display rule at Epi
  Info's decimals. Apply this only to labels whose Epi Info text is on the percent scale; list them in the
  decisions entry.
- `Fisher exact 1-tailed p` is mapped to `fisher_exact.p_upper` because Epi Info prints the tail in the
  direction of the association (positive for Oswego). Keep it; note in the decisions entry that this is
  unverified for a negative association.

## Don'ts

- Never edit `fixtures/ei7/*` (including `labels.json`) in this task. A wrong mapping is a STOP, not an edit.
- Never change a tolerance class, the display rule, or `checked ≥ 20`.

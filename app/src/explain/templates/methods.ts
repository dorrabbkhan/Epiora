// Method templates (v1.0 plan Task 16, spec §6): exact and
// iterative calculations have no single closed-form step, so the
// explain trace carries a `method.*` formula id. The panel renders
// the words and the inputs/result table only — no substituted
// mathematics, and the formula guard skips them.
import type { FormulaTemplate } from '../index';

const EXACT = 'Exact methods; StatXact / Epitools reference implementations.';
const MARTIN = 'Martin & Austin, exact method for matched pairs.';

interface MethodInfo {
  words: string;
  source: string;
}

const INFO: Record<string, MethodInfo> = {
  'method.binom_eq': {
    words:
      'The exact binomial probability of the observed count is ' +
      'summed directly from the binomial probability function.',
    source: EXACT,
  },
  'method.binom_tail_ge': {
    words:
      'The probability of the observed count or more is the sum ' +
      'of the binomial probabilities in the upper tail.',
    source: EXACT,
  },
  'method.binom_tail_gt': {
    words:
      'The probability of more than the observed count is the ' +
      'sum of the binomial probabilities above it.',
    source: EXACT,
  },
  'method.binom_tail_le': {
    words:
      'The probability of the observed count or fewer is the ' +
      'sum of the binomial probabilities in the lower tail.',
    source: EXACT,
  },
  'method.binom_tail_lt': {
    words:
      'The probability of fewer than the observed count is the ' +
      'sum of the binomial probabilities below it.',
    source: EXACT,
  },
  'method.clogit': {
    words:
      'The odds ratio is estimated by conditional logistic ' +
      'regression on the discordant pairs.',
    source: 'Conditional logistic regression; ' + EXACT,
  },
  'method.clopper_pearson': {
    words:
      'The Clopper–Pearson confidence bounds are found by ' +
      'inverting two one-sided exact binomial tests.',
    source: 'Clopper & Pearson, Biometrika.',
  },
  'method.cmle': {
    words:
      'The estimate is the conditional maximum likelihood ' +
      'estimate of the odds ratio, found iteratively.',
    source: 'Conditional MLE; ' + EXACT,
  },
  'method.fisher_ci': {
    words:
      "Fisher's exact confidence bounds are found iteratively by " +
      'inverting the hypergeometric tail probabilities.',
    source: "Fisher's exact interval; " + EXACT,
  },
  'method.fisher_exact_p': {
    words:
      "Fisher's exact p-value sums the hypergeometric " +
      'probabilities of all tables as extreme as the observed.',
    source: "Fisher's exact test; " + EXACT,
  },
  'method.matched_fisher': {
    words:
      'The exact matched-pairs interval inverts the binomial ' +
      'tests on the discordant pairs.',
    source: MARTIN,
  },
  'method.matched_fisher_1t': {
    words:
      'The one-sided exact p-value is the binomial tail on the ' +
      'discordant pairs.',
    source: MARTIN,
  },
  'method.matched_fisher_2t': {
    words:
      'The two-sided exact p-value doubles the smaller binomial ' +
      'tail on the discordant pairs, truncated at one.',
    source: MARTIN,
  },
  'method.matched_midp': {
    words:
      'The mid-p interval for the matched odds ratio inverts ' +
      'two one-sided mid-p binomial tests iteratively.',
    source: 'Mid-p method; ' + EXACT,
  },
  'method.matched_midp_p': {
    words:
      'The mid-p p-value halves the probability of the observed ' +
      'discordant split and adds the more extreme tail.',
    source: 'Mid-p method; ' + EXACT,
  },
  'method.matched_or': {
    words:
      'The matched odds ratio is the ratio of the discordant ' +
      'pairs.',
    source: 'McNemar / matched pairs; ' + EXACT,
  },
  'method.midp_ci': {
    words:
      'The mid-p confidence bounds are found iteratively by ' +
      'inverting two one-sided mid-p tests.',
    source: 'Mid-p method; ' + EXACT,
  },
  'method.midp_p': {
    words:
      'The mid-p p-value halves the probability of the observed ' +
      'table and adds the more extreme tail probabilities.',
    source: 'Mid-p method; ' + EXACT,
  },
  'method.pois_eq': {
    words:
      'The exact Poisson probability of the observed count is ' +
      'evaluated directly from the Poisson probability function.',
    source: EXACT,
  },
  'method.pois_fisher': {
    words:
      "Fisher's exact bound is found iteratively by inverting " +
      'the Poisson tail probability.',
    source: "Fisher's exact Poisson interval; " + EXACT,
  },
  'method.pois_fisher_ci': {
    words:
      "Fisher's exact confidence interval inverts two one-sided " +
      'Poisson tail probabilities.',
    source: "Fisher's exact Poisson interval; " + EXACT,
  },
  'method.pois_midp': {
    words:
      'The mid-p p-value halves the probability of the observed ' +
      'count and adds the more extreme Poisson tail.',
    source: 'Mid-p method; ' + EXACT,
  },
  'method.pois_midp_ci': {
    words:
      'The mid-p confidence bounds are found iteratively by ' +
      'inverting two one-sided mid-p Poisson tests.',
    source: 'Mid-p method; ' + EXACT,
  },
  'method.pois_rate_fisher': {
    words:
      "Fisher's exact p-value for the rate inverts the Poisson " +
      'tail probability.',
    source: "Fisher's exact Poisson test; " + EXACT,
  },
  'method.pois_rate_midp': {
    words:
      'The mid-p p-value for the rate halves the probability of ' +
      'the observed count and adds the more extreme Poisson tail.',
    source: 'Mid-p method; ' + EXACT,
  },
  'method.pois_tail_ge': {
    words:
      'The probability of the observed count or more is the ' +
      'Poisson upper tail sum.',
    source: EXACT,
  },
  'method.pois_tail_gt': {
    words:
      'The probability of more than the observed count is the ' +
      'Poisson tail above it.',
    source: EXACT,
  },
  'method.pois_tail_le': {
    words:
      'The probability of the observed count or fewer is the ' +
      'Poisson lower tail sum.',
    source: EXACT,
  },
  'method.pois_tail_lt': {
    words:
      'The probability of fewer than the observed count is the ' +
      'Poisson tail below it.',
    source: EXACT,
  },
  'method.random_numbers': {
    words:
      'The random numbers come from a seeded pseudo-random ' +
      'generator; the seed makes the output reproducible.',
    source: 'Seeded PRNG (ChaCha); OpenEpi Random Numbers specification.',
  },
};

const noMath = (): string => '';
const noCompute = (): number => NaN;

export const METHODS: Record<string, FormulaTemplate> = Object.fromEntries(
  Object.entries(INFO).map(([id, { words, source }]) => [
    id,
    { id, words, mathml: noMath, compute: noCompute, source },
  ]),
);

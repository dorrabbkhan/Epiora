// Distribution tails and test statistics (v1.0 plan Task 16, spec
// §6): normal-tail probabilities, the t statistic and its tail,
// confidence bounds around a difference, degrees-of-freedom
// formulas (pooled, Welch, between/within) and the F statistic and
// its tails.
import type { FormulaTemplate } from '../index';
import { bin, math, mfrac, mi, mn, mo, mrow, msub, msup } from './ml';
import { ibeta, phi } from './special';

type V = Record<string, number>;
type S = Record<string, string>;

const eq = (lhs: string, rhs: string): string => mrow(lhs, mo('='), rhs);

const ROSNER = 'Rosner, Fundamentals of Biostatistics.';
const NR = 'Regularized incomplete beta and gamma functions (Numerical Recipes).';
const PHI2T = 'two-sided normal tail probability';

export const TAILS: Record<string, FormulaTemplate> = {
  normal_cdf_2t: {
    id: 'normal_cdf_2t',
    words:
      'The two-sided p-value is twice the standard normal ' +
      'probability below minus the absolute value of the z score.',
    mathml: (v: S) =>
      math(
        PHI2T,
        eq(
          mi('p'),
          mrow(mn('2'), mo('×'), mi('Φ'), mrow(mo('('), mo('−'), mo('|'), mn(v.z ?? ''), mo('|'), mo(')'))),
        ),
      ),
    compute: (v: V) => 2 * phi(-Math.abs(v.z ?? NaN)),
    source: NR,
  },
  normal_cdf_neg: {
    id: 'normal_cdf_neg',
    words:
      'The one-sided p-value is the standard normal probability ' +
      'below minus the absolute value of the z score.',
    mathml: (v: S) =>
      math(
        'one-sided normal tail probability',
        eq(mi('p'), mrow(mi('Φ'), mrow(mo('('), mo('−'), mo('|'), mn(v.z ?? ''), mo('|'), mo(')')))),
      ),
    compute: (v: V) => phi(-Math.abs(v.z ?? NaN)),
    source: NR,
  },
  normal_cdf_neg_2: {
    id: 'normal_cdf_neg_2',
    words:
      'The two-sided p-value is twice the standard normal ' +
      'probability below minus the absolute value of the z score.',
    mathml: (v: S) =>
      math(
        PHI2T,
        eq(
          mi('p'),
          mrow(mn('2'), mo('×'), mi('Φ'), mrow(mo('('), mo('−'), mo('|'), mn(v.z ?? ''), mo('|'), mo(')'))),
        ),
      ),
    compute: (v: V) => 2 * phi(-Math.abs(v.z ?? NaN)),
    source: NR,
  },
  p_2t: {
    id: 'p_2t',
    words: 'The two-sided p-value doubles the one-sided p-value.',
    mathml: (v: S) =>
      math('two-sided p', eq(msub(mi('p'), mi('two-sided')), mrow(mn('2'), mn(v.p ?? '')))),
    compute: (v: V) => 2 * (v.p ?? NaN),
    source: ROSNER,
  },
  t_stat: {
    id: 't_stat',
    words:
      'The t statistic is the difference between the means (or ' +
      'between the mean and the reference value) divided by its ' +
      'standard error.',
    mathml: (v: S) =>
      math(
        't = difference over standard error',
        eq(mi('t'), mfrac(mn(v.diff ?? ''), mn(v.se ?? ''))),
      ),
    compute: (v: V) => (v.diff ?? NaN) / (v.se ?? NaN),
    source: ROSNER,
  },
  t_sf_2t: {
    id: 't_sf_2t',
    words:
      'The two-sided p-value is the probability of a t statistic ' +
      'at least as far from zero as the one observed, computed from ' +
      'the t distribution with the stated degrees of freedom.',
    mathml: (v: S) =>
      math(
        'two-sided t tail probability',
        eq(
          mi('p'),
          mrow(
            mi('P'),
            mo('('),
            mo('|'),
            msub(mi('t'), mn(v.df ?? '')),
            mo('|'),
            mo('≥'),
            mo('|'),
            mn(v.t ?? ''),
            mo('|'),
            mo(')'),
          ),
        ),
      ),
    compute: (v: V) =>
      ibeta((v.df ?? NaN) / ((v.df ?? NaN) + (v.t ?? NaN) ** 2), (v.df ?? NaN) / 2, 0.5),
    source: NR,
  },
  t_ci_lower: {
    id: 't_ci_lower',
    words:
      'The lower confidence bound is the difference minus the ' +
      't critical value times its standard error.',
    mathml: (v: S) =>
      math(
        'lower t confidence bound',
        eq(mi('L'), mrow(mn(v.diff ?? ''), mo('−'), mn(v.crit ?? ''), mo('×'), mn(v.se ?? ''))),
      ),
    compute: (v: V) => (v.diff ?? NaN) - (v.crit ?? NaN) * (v.se ?? NaN),
    source: ROSNER,
  },
  t_ci_upper: {
    id: 't_ci_upper',
    words:
      'The upper confidence bound is the difference plus the ' +
      't critical value times its standard error.',
    mathml: (v: S) =>
      math(
        'upper t confidence bound',
        eq(mi('U'), mrow(mn(v.diff ?? ''), mo('+'), mn(v.crit ?? ''), mo('×'), mn(v.se ?? ''))),
      ),
    compute: (v: V) => (v.diff ?? NaN) + (v.crit ?? NaN) * (v.se ?? NaN),
    source: ROSNER,
  },
  df_pooled: {
    id: 'df_pooled',
    words:
      'The pooled t test has the two group sizes minus two degrees ' +
      'of freedom.',
    mathml: (v: S) =>
      math(
        'df = n₁ + n₂ − 2',
        eq(mi('df'), mrow(mn(v.n1 ?? ''), mo('+'), mn(v.n2 ?? ''), mo('−'), mn('2'))),
      ),
    compute: (v: V) => (v.n1 ?? NaN) + (v.n2 ?? NaN) - 2,
    source: ROSNER,
  },
  df_welch: {
    id: 'df_welch',
    words:
      'The Welch–Satterthwaite degrees of freedom combine the two ' +
      'group variances and sizes; it need not be a whole number.',
    mathml: (v: S) =>
      math(
        'Welch–Satterthwaite df',
        eq(
          mi('df'),
          mfrac(
            msup(mn(v.se_welch ?? ''), mn('4')),
            mrow(
              mfrac(
                msup(mrow(mo('('), mfrac(mn(v.v1 ?? ''), mn(v.n1 ?? '')), mo(')')), mn('2')),
                mrow(mn(v.n1 ?? ''), mo('−'), mn('1')),
              ),
              mo('+'),
              mfrac(
                msup(mrow(mo('('), mfrac(mn(v.v2 ?? ''), mn(v.n2 ?? '')), mo(')')), mn('2')),
                mrow(mn(v.n2 ?? ''), mo('−'), mn('1')),
              ),
            ),
          ),
        ),
      ),
    compute: (v: V) =>
      (v.se_welch ?? NaN) ** 4 /
      (((v.v1 ?? NaN) / (v.n1 ?? NaN)) ** 2 / ((v.n1 ?? NaN) - 1) +
        ((v.v2 ?? NaN) / (v.n2 ?? NaN)) ** 2 / ((v.n2 ?? NaN) - 1)),
    source: 'Welch, Biometrika; Satterthwaite.',
  },
  df_n: {
    id: 'df_n',
    words: 'The test has the sample size minus one degrees of freedom.',
    mathml: (v: S) =>
      math('df = n − 1', eq(mi('df'), bin(mn(v.n ?? ''), '−', mn('1')))),
    compute: (v: V) => (v.n ?? NaN) - 1,
    source: ROSNER,
  },
  df_n_minus1: {
    id: 'df_n_minus1',
    words: 'The test has the sample size minus one degrees of freedom.',
    mathml: (v: S) =>
      math('df = n − 1', eq(mi('df'), bin(mn(v.n ?? ''), '−', mn('1')))),
    compute: (v: V) => (v.n ?? NaN) - 1,
    source: ROSNER,
  },
  df_between: {
    id: 'df_between',
    words:
      'The between-groups degrees of freedom is the number of ' +
      'groups minus one.',
    mathml: (v: S) =>
      math('df between = k − 1', eq(msub(mi('df'), mi('between')), bin(mn(v.k ?? ''), '−', mn('1')))),
    compute: (v: V) => (v.k ?? NaN) - 1,
    source: ROSNER,
  },
  df_within: {
    id: 'df_within',
    words:
      'The within-groups degrees of freedom is the total sample ' +
      'size minus the number of groups.',
    mathml: (v: S) =>
      math('df within = n − k', eq(msub(mi('df'), mi('within')), bin(mn(v.n ?? ''), '−', mn(v.k ?? '')))),
    compute: (v: V) => (v.n ?? NaN) - (v.k ?? NaN),
    source: ROSNER,
  },
  f_sf: {
    id: 'f_sf',
    words:
      'The p-value is the upper-tail probability of the F ' +
      'distribution with the stated numerator and denominator ' +
      'degrees of freedom beyond the F statistic.',
    mathml: (v: S) =>
      math(
        'upper-tail F probability',
        eq(
          mi('p'),
          mrow(
            mi('P'),
            mo('('),
            msub(mi('F'), mrow(mn(v.df1 ?? ''), mo(','), mn(v.df2 ?? ''))),
            mo('≥'),
            mn(v.f ?? ''),
            mo(')'),
          ),
        ),
      ),
    compute: (v: V) =>
      ibeta(
        (v.df2 ?? NaN) / ((v.df2 ?? NaN) + (v.df1 ?? NaN) * (v.f ?? NaN)),
        (v.df2 ?? NaN) / 2,
        (v.df1 ?? NaN) / 2,
      ),
    source: NR,
  },
  f_stat: {
    id: 'f_stat',
    words:
      'The F statistic is the ratio of the between-groups mean ' +
      'square to the within-groups mean square.',
    mathml: (v: S) =>
      math('F = MS between over MS within', eq(mi('F'), mfrac(mn(v.v1 ?? ''), mn(v.v2 ?? '')))),
    compute: (v: V) => (v.v1 ?? NaN) / (v.v2 ?? NaN),
    source: ROSNER,
  },
  f_p_2t: {
    id: 'f_p_2t',
    words:
      'The two-sided F p-value doubles the smaller tail of the F ' +
      'distribution at the observed statistic.',
    mathml: () =>
      math(
        'two-sided F probability',
        eq(
          mi('p'),
          mrow(
            mn('2'),
            mo('×'),
            mi('min'),
            mo('('),
            msub(mi('P'), mi('upper')),
            mo(','),
            msub(mi('P'), mi('lower')),
            mo(')'),
          ),
        ),
      ),
    compute: (v: V) => {
      const sf = ibeta(
        (v.df_den ?? NaN) / ((v.df_den ?? NaN) + (v.df_num ?? NaN) * (v.f ?? NaN)),
        (v.df_den ?? NaN) / 2,
        (v.df_num ?? NaN) / 2,
      );
      return 2 * Math.min(sf, 1 - sf);
    },
    source: NR,
  },
};

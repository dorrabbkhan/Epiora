// Continuous-summary templates (v1.0 plan Task 16, spec §6): the
// median/percentile rank bounds, mean confidence bounds (z and t),
// the ANOVA group confidence bounds, the sums of squares, and the
// Bartlett and Kruskal–Wallis statistics.
import type { FormulaTemplate } from '../index';
import { math, mfrac, mi, mn, mo, mrow, msqrt, msub, msubsup, msup } from './ml';

type V = Record<string, number>;
type S = Record<string, string>;

const eq = (lhs: string, rhs: string): string => mrow(lhs, mo('='), rhs);

const ROSNER = 'Rosner, Fundamentals of Biostatistics.';
const SNEDECOR = 'Snedecor & Cochran, Statistical Methods.';

const rankBound = (v: S, sign: string): string =>
  mrow(
    mn(v.n ?? ''),
    mo('×'),
    mn(v.p ?? ''),
    mo(sign),
    mn(v.z ?? ''),
    mo('×'),
    msqrt(
      mrow(
        mn(v.n ?? ''),
        mo('×'),
        mn(v.p ?? ''),
        mo('×'),
        mrow(mo('('), mn('1'), mo('−'), mn(v.p ?? ''), mo(')')),
      ),
    ),
  );

const ciBound = (mean: string, v: S, sign: string, crit: string): string =>
  mrow(mean, mo(sign), mn(v[crit] ?? ''), mo('×'), mn(v.se ?? ''));

export const CONTINUOUS: Record<string, FormulaTemplate> = {
  median_rank_lower: {
    id: 'median_rank_lower',
    words:
      'The lower rank bound of the percentile interval is the ' +
      'expected rank minus z times its binomial standard ' +
      'deviation.',
    mathml: (v: S) =>
      math('lower rank bound', eq(mi('r'), rankBound(v, '−'))),
    compute: (v: V) =>
      (v.n ?? NaN) * (v.p ?? NaN) -
      (v.z ?? NaN) * Math.sqrt((v.n ?? NaN) * (v.p ?? NaN) * (1 - (v.p ?? NaN))),
    source: 'Binomial rank interval; ' + ROSNER,
  },
  median_rank_upper: {
    id: 'median_rank_upper',
    words:
      'The upper rank bound of the percentile interval is the ' +
      'expected rank plus z times its binomial standard ' +
      'deviation.',
    mathml: (v: S) =>
      math('upper rank bound', eq(mi('r'), rankBound(v, '+'))),
    compute: (v: V) =>
      (v.n ?? NaN) * (v.p ?? NaN) +
      (v.z ?? NaN) * Math.sqrt((v.n ?? NaN) * (v.p ?? NaN) * (1 - (v.p ?? NaN))),
    source: 'Binomial rank interval; ' + ROSNER,
  },
  mean_ci_z_lower: {
    id: 'mean_ci_z_lower',
    words:
      'The lower confidence bound of the mean is the mean minus ' +
      'z times its standard error.',
    mathml: (v: S) =>
      math(
        'lower bound of the mean (z)',
        eq(mi('L'), ciBound(mn(v.mean ?? ''), v, '−', 'z')),
      ),
    compute: (v: V) => (v.mean ?? NaN) - (v.z ?? NaN) * (v.se ?? NaN),
    source: ROSNER,
  },
  mean_ci_z_upper: {
    id: 'mean_ci_z_upper',
    words:
      'The upper confidence bound of the mean is the mean plus ' +
      'z times its standard error.',
    mathml: (v: S) =>
      math(
        'upper bound of the mean (z)',
        eq(mi('U'), ciBound(mn(v.mean ?? ''), v, '+', 'z')),
      ),
    compute: (v: V) => (v.mean ?? NaN) + (v.z ?? NaN) * (v.se ?? NaN),
    source: ROSNER,
  },
  mean_ci_t_lower: {
    id: 'mean_ci_t_lower',
    words:
      'The lower confidence bound of the mean is the mean minus ' +
      'the t critical value times its standard error.',
    mathml: (v: S) =>
      math(
        'lower bound of the mean (t)',
        eq(mi('L'), ciBound(mn(v.mean ?? ''), v, '−', 't')),
      ),
    compute: (v: V) => (v.mean ?? NaN) - (v.t ?? NaN) * (v.se ?? NaN),
    source: ROSNER,
  },
  mean_ci_t_upper: {
    id: 'mean_ci_t_upper',
    words:
      'The upper confidence bound of the mean is the mean plus ' +
      'the t critical value times its standard error.',
    mathml: (v: S) =>
      math(
        'upper bound of the mean (t)',
        eq(mi('U'), ciBound(mn(v.mean ?? ''), v, '+', 't')),
      ),
    compute: (v: V) => (v.mean ?? NaN) + (v.t ?? NaN) * (v.se ?? NaN),
    source: ROSNER,
  },
  group_ci_lower: {
    id: 'group_ci_lower',
    words:
      'The lower bound of the group mean uses the critical value ' +
      'times the pooled-within standard error term.',
    mathml: (v: S) =>
      math(
        'group mean lower bound',
        eq(mi('L'), mrow(mn(v.mean ?? ''), mo('−'), mn(v.crit ?? ''), mo('×'), mn(v.h_base ?? ''))),
      ),
    compute: (v: V) => (v.mean ?? NaN) - (v.crit ?? NaN) * (v.h_base ?? NaN),
    source: 'ANOVA group intervals; ' + SNEDECOR,
  },
  group_ci_upper: {
    id: 'group_ci_upper',
    words:
      'The upper bound of the group mean uses the critical value ' +
      'times the pooled-within standard error term.',
    mathml: (v: S) =>
      math(
        'group mean upper bound',
        eq(mi('U'), mrow(mn(v.mean ?? ''), mo('+'), mn(v.crit ?? ''), mo('×'), mn(v.h_base ?? ''))),
      ),
    compute: (v: V) => (v.mean ?? NaN) + (v.crit ?? NaN) * (v.h_base ?? NaN),
    source: 'ANOVA group intervals; ' + SNEDECOR,
  },
  ss_between: {
    id: 'ss_between',
    words:
      'The between-groups sum of squares weights each squared ' +
      'difference of a group mean from the grand mean by the ' +
      'group size.',
    mathml: (v: S) =>
      math(
        'between-groups sum of squares',
        eq(
          msub(mi('SS'), mi('between')),
          mrow(
            mi('Σ'),
            msub(mi('n'), mi('i')),
            msup(
              mrow(mo('('), msub(mi('x̄'), mi('i')), mo('−'), mn(v.grand ?? ''), mo(')')),
              mn('2'),
            ),
          ),
        ),
      ),
    compute: (v: V) => {
      let s = 0;
      for (const k of Object.keys(v)) {
        if (!/^n_/.test(k)) continue;
        const m = v[`mean_${k.slice(2)}`];
        if (m === undefined) return NaN;
        s += (v[k] ?? 0) * (m - (v.grand ?? NaN)) ** 2;
      }
      return s;
    },
    source: SNEDECOR,
  },
  ss_within: {
    id: 'ss_within',
    words:
      'The within-groups sum of squares pools each group variance ' +
      'weighted by its degrees of freedom.',
    mathml: () =>
      math(
        'within-groups sum of squares',
        eq(
          msub(mi('SS'), mi('within')),
          mrow(mi('Σ'), msub(mi('df'), mi('i')), msubsup(mi('s'), mi('i'), mn('2'))),
        ),
      ),
    compute: (v: V) => {
      let s = 0;
      for (const k of Object.keys(v)) {
        if (!/^df_/.test(k)) continue;
        const w = v[`var_${k.slice(3)}`];
        if (w === undefined) return NaN;
        s += (v[k] ?? 0) * w;
      }
      return s;
    },
    source: SNEDECOR,
  },
  bartlett: {
    id: 'bartlett',
    words:
      "Bartlett's statistic is the pooled log-variance numerator " +
      'divided by the small-sample correction.',
    mathml: (v: S) =>
      math(
        "Bartlett's chi-square",
        eq(msup(mi('χ'), mn('2')), mfrac(mn(v.numerator ?? ''), mn(v.correction ?? ''))),
      ),
    compute: (v: V) => (v.numerator ?? NaN) / (v.correction ?? NaN),
    source: "Bartlett's test of homogeneity of variances; " + SNEDECOR,
  },
  kruskal_wallis: {
    id: 'kruskal_wallis',
    words:
      'The Kruskal–Wallis H is the rank sum-of-squares statistic ' +
      'divided by the ties correction.',
    mathml: (v: S) =>
      math(
        'Kruskal–Wallis H',
        eq(mi('H'), mfrac(mn(v.h_raw ?? ''), mn(v.correction ?? ''))),
      ),
    compute: (v: V) => (v.h_raw ?? NaN) / (v.correction ?? NaN),
    source: 'Kruskal & Wallis, Journal of the American Statistical Association.',
  },
};

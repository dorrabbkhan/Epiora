// Shared/scalar formula templates (v1.0 plan Task 16, spec §6) —
// the small arithmetic ids every family reuses: identity reads,
// sums, ratios, means and percentage scalings.
import type { FormulaTemplate } from '../index';
import { bin, math, mfrac, mi, mn, mo, mrow, msqrt, msup } from './ml';

type V = Record<string, number>;
type S = Record<string, string>;

const SRC = 'OpenEpi and Epi Info StatCalc conventions; standard definitions.';

const eq = (lhs: string, rhs: string): string => mrow(lhs, mo('='), rhs);

export const SHARED: Record<string, FormulaTemplate> = {
  data: {
    id: 'data',
    words:
      'The value is read directly from the data — a count, a total or an ' +
      'order statistic — with no further calculation applied to it.',
    mathml: (v: S) =>
      math('the recorded value', eq(mi('x'), mn(v.value ?? ''))),
    compute: (v: V) => v.value ?? NaN,
    source: SRC,
  },
  sum_two: {
    id: 'sum_two',
    words: 'Two counts or values are added together.',
    mathml: (v: S) =>
      math('a plus b', eq(mi('s'), bin(mn(v.a ?? ''), '+', mn(v.b ?? '')))),
    compute: (v: V) => (v.a ?? 0) + (v.b ?? 0),
    source: SRC,
  },
  ceil: {
    id: 'ceil',
    words:
      'The value is rounded up to the next whole number, since a count ' +
      'of subjects or units cannot be fractional.',
    mathml: (v: S) =>
      math(
        'x rounded up',
        eq(mi('n'), mrow(mo('⌈'), mn(v.x ?? ''), mo('⌉'))),
      ),
    compute: (v: V) => Math.ceil(v.x ?? NaN),
    source: SRC,
  },
  sqrt: {
    id: 'sqrt',
    words: 'The square root of the value is taken.',
    mathml: (v: S) =>
      math('square root of x', eq(mi('r'), msqrt(mn(v.x ?? '')))),
    compute: (v: V) => Math.sqrt(v.x ?? NaN),
    source: SRC,
  },
  ratio: {
    id: 'ratio',
    words: 'The numerator is divided by the denominator.',
    mathml: (v: S) =>
      math(
        'numerator over denominator',
        eq(mi('r'), mfrac(mn(v.num ?? ''), mn(v.den ?? ''))),
      ),
    compute: (v: V) => (v.num ?? NaN) / (v.den ?? NaN),
    source: SRC,
  },
  mean: {
    id: 'mean',
    words: 'The total is divided by the number of observations.',
    mathml: (v: S) =>
      math(
        'mean = total over n',
        eq(mi('x̄'), mfrac(mn(v.total ?? ''), mn(v.n ?? ''))),
      ),
    compute: (v: V) => (v.total ?? NaN) / (v.n ?? NaN),
    source: SRC,
  },
  mean_diff: {
    id: 'mean_diff',
    words: 'The second group mean is subtracted from the first.',
    mathml: (v: S) =>
      math(
        'difference of means',
        eq(mi('d'), bin(mn(v.m1 ?? ''), '−', mn(v.m2 ?? ''))),
      ),
    compute: (v: V) => (v.m1 ?? NaN) - (v.m2 ?? NaN),
    source: SRC,
  },
  sample_variance: {
    id: 'sample_variance',
    words:
      'The sum of squared deviations from the mean is divided by its ' +
      'degrees of freedom, giving the sample variance.',
    mathml: (v: S) =>
      math(
        'variance = sum of squares over df',
        eq(msup(mi('s'), mn('2')), mfrac(mn(v.ssq ?? ''), mn(v.df ?? ''))),
      ),
    compute: (v: V) => (v.ssq ?? NaN) / (v.df ?? NaN),
    source: SRC,
  },
  proportion: {
    id: 'proportion',
    words:
      'The number with the characteristic is divided by the total ' +
      'number, giving the proportion.',
    mathml: (v: S) =>
      math('proportion = x over n', eq(mi('p'), mfrac(mn(v.x ?? ''), mn(v.n ?? '')))),
    compute: (v: V) => (v.x ?? NaN) / (v.n ?? NaN),
    source: SRC,
  },
  proportion_pct: {
    id: 'proportion_pct',
    words:
      'The number with the characteristic is divided by the total ' +
      'number and multiplied by one hundred, giving the percentage.',
    mathml: (v: S) =>
      math(
        'percentage = x over n times one hundred',
        eq(mi('%'), mrow(mfrac(mn(v.x ?? ''), mn(v.n ?? '')), mo('×'), mn('100'))),
      ),
    compute: (v: V) => ((v.x ?? NaN) / (v.n ?? NaN)) * 100,
    source: SRC,
  },
  cumulative_percent_pct: {
    id: 'cumulative_percent_pct',
    words:
      'The running total up to and including this row is divided by ' +
      'the overall total and multiplied by one hundred.',
    mathml: (v: S) =>
      math(
        'cumulative percent',
        eq(
          mi('%'),
          mrow(
            mfrac(mn(v.cumulative ?? ''), mn(v.total ?? '')),
            mo('×'),
            mn('100'),
          ),
        ),
      ),
    compute: (v: V) => ((v.cumulative ?? NaN) / (v.total ?? NaN)) * 100,
    source: SRC,
  },
  cell_percent: {
    id: 'cell_percent',
    words:
      'The count in this cell is divided by the total of all cells ' +
      'and multiplied by one hundred.',
    mathml: (v: S) =>
      math(
        'cell percent',
        eq(
          mi('%'),
          mrow(
            mfrac(mn(v.count ?? ''), mn(v.total_cells ?? '')),
            mo('×'),
            mn('100'),
          ),
        ),
      ),
    compute: (v: V) => ((v.count ?? NaN) / (v.total_cells ?? NaN)) * 100,
    source: SRC,
  },
};

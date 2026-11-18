// Binomial test templates (v1.0 plan Task 16, spec §6): the
// one-sample z test of a proportion, the binomial two-sided p-value
// and the Clopper–Pearson bound converted back to a count. The
// proportion interval bounds are in `proportions.ts`.
import type { FormulaTemplate } from '../index';
import { math, mfrac, mi, mn, mo, mrow, msqrt } from './ml';

type V = Record<string, number>;
type S = Record<string, string>;

const eq = (lhs: string, rhs: string): string => mrow(lhs, mo('='), rhs);

export const BINOMIAL: Record<string, FormulaTemplate> = {
  prop_ztest_z: {
    id: 'prop_ztest_z',
    words:
      'The z statistic compares the observed proportion with the ' +
      'reference proportion under its binomial standard error.',
    mathml: (v: S) =>
      math(
        'z for a proportion vs a standard',
        eq(
          mi('z'),
          mfrac(
            mrow(mn(v.p ?? ''), mo('−'), mn(v.p0 ?? '')),
            msqrt(mfrac(mrow(mn(v.p0 ?? ''), mrow(mo('('), mn('1'), mo('−'), mn(v.p0 ?? ''), mo(')'))), mn(v.n ?? ''))),
          ),
        ),
      ),
    compute: (v: V) =>
      ((v.p ?? NaN) - (v.p0 ?? NaN)) /
      Math.sqrt(((v.p0 ?? NaN) * (1 - (v.p0 ?? NaN))) / (v.n ?? NaN)),
    source: 'One-sample z test of a proportion; Fleiss, Levin & Paik.',
  },
  binom_p_2t: {
    id: 'binom_p_2t',
    words:
      'The two-sided binomial p-value doubles the smaller of the ' +
      'two tails, capped at one.',
    mathml: (v: S) =>
      math(
        'two-sided binomial p',
        eq(
          mi('p'),
          mrow(
            mi('min'),
            mo('('),
            mrow(mn('2'), mo('×'), mi('min'), mo('('), mn(v.tail_le ?? ''), mo(','), mn(v.tail_ge ?? ''), mo(')')),
            mo(','),
            mn('1'),
            mo(')'),
          ),
        ),
      ),
    compute: (v: V) =>
      Math.min(2 * Math.min(v.tail_le ?? NaN, v.tail_ge ?? NaN), 1),
    source: 'Binomial two-sided tail; Epi Info StatCalc documentation.',
  },
  binom_cp_count: {
    id: 'binom_cp_count',
    words:
      'The Clopper–Pearson bound converted back to a count: the ' +
      'bound times the sample size, rounded to the nearest whole ' +
      'number.',
    mathml: (v: S) =>
      math(
        'count = bound × n, rounded',
        eq(mi('x'), mrow(mo('⌊'), mn(v.bound ?? ''), mo('×'), mn(v.n ?? ''), mo('⌉'))),
      ),
    compute: (v: V) => Math.round((v.bound ?? NaN) * (v.n ?? NaN)),
    source: 'Clopper & Pearson, Biometrika.',
  },
};

// Pooled and homogeneity chi-square templates (v1.0 plan Task 16,
// spec §6): the pooled Mantel–Haenszel χ² (plain and
// continuity-corrected), Woolf homogeneity, the extended MH trend
// test, Byar's χ² for Poisson counts and the Breslow–Day–Tarone
// homogeneity statistic. The single-table forms are in
// `chiSquare.ts`, which owns the shared `CHI`, `SQ` and `MH` pieces.
import type { FormulaTemplate } from '../index';
import { CHI, MH, SQ } from './chiSquare';
import { bin, math, mfrac, mi, mn, mo, mrow, msqrt, msub, msup } from './ml';

type V = Record<string, number>;
type S = Record<string, string>;

const eq = (lhs: string, rhs: string): string => mrow(lhs, mo('='), rhs);

const BND = 'Breslow & Day, Statistical Methods in Cancer Research.';

export const CHI_SQUARE_POOLED: Record<string, FormulaTemplate> = {
  mh_chi_square: {
    id: 'mh_chi_square',
    words:
      'The pooled Mantel–Haenszel chi-square squares the summed ' +
      'observed-minus-expected difference and divides by its ' +
      'variance.',
    mathml: () =>
      math(
        'pooled MH chi-square',
        eq(CHI, mfrac(msup(mi('δ'), mn('2')), mi('Var'))),
      ),
    compute: (v: V) => (v.delta ?? NaN) ** 2 / (v.var ?? NaN),
    source: MH,
  },
  mh_chi_square_cc: {
    id: 'mh_chi_square_cc',
    words:
      'The continuity-corrected pooled Mantel–Haenszel chi-square ' +
      'shrinks the absolute difference by the correction — never ' +
      'below zero — before squaring.',
    mathml: (v: S) =>
      math(
        'pooled MH chi-square, continuity corrected',
        eq(
          CHI,
          mfrac(
            SQ(mrow(mi('max'), mo('('), mo('|'), mi('δ'), mo('|'), mo('−'), mn(v.yates ?? '0.5'), mo(')'))),
            mi('Var'),
          ),
        ),
      ),
    compute: (v: V) => {
      const s =
        v.yates !== undefined
          ? Math.max(Math.abs(v.delta ?? NaN) - v.yates, 0)
          : Math.max(Math.abs(v.delta ?? NaN) - 0.5, 0);
      return (s * s) / (v.var ?? NaN);
    },
    source: MH,
  },
  woolf_chisq: {
    id: 'woolf_chisq',
    words:
      "Woolf's homogeneity test sums the weighted squared deviations " +
      'of each stratum log odds ratio from the pooled adjusted value.',
    mathml: () =>
      math(
        'Woolf homogeneity chi-square',
        eq(CHI, mrow(mi('Σ'), mrow(msub(mi('w'), mi('i')), SQ(mrow(msub(mi('θ'), mi('i')), mo('−'), mi('θ̄')))))),
      ),
    compute: (v: V) => {
      let s = 0;
      for (const k of Object.keys(v)) {
        if (!/^e_/.test(k)) continue;
        const w = v[`w_${k.slice(2)}`];
        if (w === undefined) return NaN;
        s += w * ((v[k] ?? 0) - (v.adj ?? NaN)) ** 2;
      }
      return s;
    },
    source: 'Woolf, Annals of Human Genetics.',
  },
  xmh: {
    id: 'xmh',
    words:
      'The extended Mantel–Haenszel trend statistic squares the ' +
      'summed row scores and divides by their variance.',
    mathml: () =>
      math(
        'extended MH trend chi-square',
        eq(CHI, mfrac(msup(msub(mi('V'), mn('1')), mn('2')), mi('V'))),
      ),
    compute: (v: V) => (v.v1sum ?? NaN) ** 2 / (v.vsum ?? NaN),
    source: 'Mantel, Journal of the American Statistical Association.',
  },
  xmh_cc: {
    id: 'xmh_cc',
    words:
      'The continuity-corrected trend statistic shrinks the summed ' +
      'score by a half — never below zero — before squaring.',
    mathml: () =>
      math(
        'extended MH trend chi-square, corrected',
        eq(
          CHI,
          mfrac(
            SQ(mrow(mi('max'), mo('('), mo('|'), msub(mi('V'), mn('1')), mo('|'), mo('−'), mi('0.5'), mo(')'))),
            mi('V'),
          ),
        ),
      ),
    compute: (v: V) =>
      Math.max(Math.abs(v.v1sum ?? NaN) - 0.5, 0) ** 2 / (v.vsum ?? NaN),
    source: 'Mantel, Journal of the American Statistical Association.',
  },
  byar_chisq: {
    id: 'byar_chisq',
    words:
      "Byar's chi-square approximation for a Poisson count uses the " +
      'higher of the observed count and observed plus one in the ' +
      'deviance transform.',
    mathml: () =>
      math(
        "Byar's chi-square approximation",
        eq(
          CHI,
          msup(
            mrow(
              msqrt(mrow(mn('9'), mi('x'))),
              mo('×'),
              mrow(
                mo('('),
                mn('1'),
                mo('−'),
                mfrac(mn('1'), mrow(mn('9'), mi('x'))),
                mo('−'),
                msup(mrow(mo('('), mfrac(mi('e'), mi('x')), mo(')')), mfrac(mn('1'), mn('3'))),
                mo(')'),
              ),
            ),
            mn('2'),
          ),
        ),
      ),
    compute: (v: V) => {
      const o = v.o ?? NaN;
      const e = v.e ?? NaN;
      const x = o > e ? o : o + 1;
      const dev = Math.sqrt(9 * x) * (1 - 1 / (9 * x) - Math.cbrt(e / x));
      return dev * dev;
    },
    source: BND,
  },
  bdt: {
    id: 'bdt',
    words:
      'The Breslow–Day–Tarone homogeneity statistic subtracts the ' +
      'summed-score term from the total chi-square across strata.',
    mathml: () =>
      math(
        'Breslow–Day–Tarone chi-square',
        eq(
          CHI,
          bin(
            msub(mi('χ'), mn('2')),
            '−',
            mfrac(
              SQ(mrow(msub(mi('Σ'), mi('a')), mo('−'), msub(mi('Σ'), mi('e')))),
              msub(mi('Σ'), mi('Var')),
            ),
          ),
        ),
      ),
    compute: (v: V) =>
      (v.x2 ?? NaN) - ((v.sum_a ?? NaN) - (v.sum_e ?? NaN)) ** 2 / (v.sum_var ?? NaN),
    source: 'Breslow, Day & Tarone; ' + BND,
  },
  bdt_df: {
    id: 'bdt_df',
    words:
      'The homogeneity test has one degree of freedom fewer than ' +
      'the number of strata.',
    mathml: (v: S) =>
      math('df = strata − 1', eq(mi('df'), bin(mn(v.k ?? ''), '−', mn('1')))),
    compute: (v: V) => (v.k ?? NaN) - 1,
    source: BND,
  },
};

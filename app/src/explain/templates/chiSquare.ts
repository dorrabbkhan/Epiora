// Chi-square and related test-statistic templates (v1.0 plan Task
// 16, spec §6): degrees of freedom, distribution tails, the cell-sum
// χ² forms (uncorrected, Yates and per-cell continuity correction),
// Mantel–Haenszel, McNemar, Woolf homogeneity, the extended MH
// trend test, Byar's χ² for Poisson counts and the
// Breslow–Day–Tarone homogeneity statistic.
import type { FormulaTemplate } from '../index';
import { bin, math, mfrac, mi, mn, mo, mrow, msqrt, msub, msup } from './ml';
import { chiCells, gamQ } from './special';

type V = Record<string, number>;
type S = Record<string, string>;

const eq = (lhs: string, rhs: string): string => mrow(lhs, mo('='), rhs);
const CHI = msup(mi('χ'), mn('2'));
const SQ = (inner: string): string =>
  msup(mrow(mo('('), inner, mo(')')), mn('2'));
const O_MINUS_E = SQ(mrow(mi('o'), mo('−'), mi('e')));

const FLEISS = 'Fleiss, Levin & Paik, Statistical Methods for Rates and Proportions.';
const MH = 'Mantel & Haenszel, Journal of the National Cancer Institute.';
const BND = 'Breslow & Day, Statistical Methods in Cancer Research.';

export const CHI_SQUARE: Record<string, FormulaTemplate> = {
  chi_df: {
    id: 'chi_df',
    words:
      'The degrees of freedom of the chi-square test on an r-by-c ' +
      'table are the number of rows minus one times the number of ' +
      'columns minus one.',
    mathml: (v: S) =>
      math(
        'df = (rows − 1)(cols − 1)',
        eq(
          mi('df'),
          mrow(
            mrow(mo('('), mn(v.rows ?? ''), mo('−'), mn('1'), mo(')')),
            mo('×'),
            mrow(mo('('), mn(v.cols ?? ''), mo('−'), mn('1'), mo(')')),
          ),
        ),
      ),
    compute: (v: V) => ((v.rows ?? NaN) - 1) * ((v.cols ?? NaN) - 1),
    source: FLEISS,
  },
  chi_sf: {
    id: 'chi_sf',
    words:
      'The p-value is the upper-tail probability of the chi-square ' +
      'distribution with the stated degrees of freedom beyond the ' +
      'test statistic.',
    mathml: (v: S) =>
      math(
        'upper-tail chi-square probability',
        eq(
          mi('p'),
          mrow(
            mi('P'),
            mo('('),
            msub(CHI, mn(v.df ?? '')),
            mo('≥'),
            mn(v.x ?? ''),
            mo(')'),
          ),
        ),
      ),
    compute: (v: V) => gamQ((v.df ?? NaN) / 2, (v.x ?? NaN) / 2),
    source: 'The regularized incomplete gamma function; ' + FLEISS,
  },
  chi_sf_half: {
    id: 'chi_sf_half',
    words:
      'The one-sided p-value is half the upper-tail probability of ' +
      'the chi-square distribution beyond the test statistic.',
    mathml: (v: S) =>
      math(
        'half the upper-tail chi-square probability',
        eq(
          mi('p'),
          mfrac(
            mrow(
              mi('P'),
              mo('('),
              msub(CHI, mn(v.df ?? '')),
              mo('≥'),
              mn(v.x ?? ''),
              mo(')'),
            ),
            mn('2'),
          ),
        ),
      ),
    compute: (v: V) => gamQ((v.df ?? NaN) / 2, (v.x ?? NaN) / 2) / 2,
    source: 'The regularized incomplete gamma function; ' + FLEISS,
  },
  chi_square_cells: {
    id: 'chi_square_cells',
    words:
      'For every cell, the observed minus expected count is squared ' +
      'and divided by the expected count; the chi-square statistic ' +
      'is the sum over cells.',
    mathml: () =>
      math(
        'sum over cells of (o − e)²/e',
        eq(CHI, mrow(mi('Σ'), mfrac(O_MINUS_E, mi('e')))),
      ),
    compute: (v: V) => chiCells(v, 0),
    source: FLEISS,
  },
  chi_square_yates: {
    id: 'chi_square_yates',
    words:
      "Yates' continuity correction subtracts the stated adjustment " +
      'from each absolute cell difference before squaring; the ' +
      'statistic is again the sum over cells.',
    mathml: (v: S) =>
      math(
        "Yates' corrected cell sum",
        eq(
          CHI,
          mrow(
            mi('Σ'),
            mfrac(
              SQ(mrow(mo('|'), mi('o'), mo('−'), mi('e'), mo('|'), mo('−'), mn(v.yates_adj ?? ''))),
              mi('e'),
            ),
          ),
        ),
      ),
    compute: (v: V) => chiCells(v, v.yates_adj ?? NaN),
    source: 'Yates, Journal of the Royal Statistical Society; ' + FLEISS,
  },
  chi_square_yates_cc: {
    id: 'chi_square_yates_cc',
    words:
      'Each absolute cell difference is reduced by a half — never ' +
      'below zero — then squared and divided by the expected count; ' +
      'the statistic is the sum over cells.',
    mathml: () =>
      math(
        'continuity-corrected cell sum',
        eq(
          CHI,
          mrow(
            mi('Σ'),
            mfrac(
              SQ(
                mrow(
                  mi('max'),
                  mo('('),
                  mo('|'),
                  mi('o'),
                  mo('−'),
                  mi('e'),
                  mo('|'),
                  mo('−'),
                  mi('0.5'),
                  mo(')'),
                ),
              ),
              mi('e'),
            ),
          ),
        ),
      ),
    compute: (v: V) => {
      let s = 0;
      for (const k of Object.keys(v)) {
        if (!/^o_/.test(k)) continue;
        const e = v[`e_${k.slice(2)}`];
        if (e === undefined) return NaN;
        s += Math.max(Math.abs((v[k] ?? 0) - e) - 0.5, 0) ** 2 / e;
      }
      return s;
    },
    source: 'Yates continuity correction; ' + FLEISS,
  },
  chi_square_mh: {
    id: 'chi_square_mh',
    words:
      'The Mantel–Haenszel chi-square compares the observed ' +
      'exposed-ill count with its expectation under the margins, ' +
      'scaled by the table margins.',
    mathml: (v: S) =>
      math(
        'Mantel–Haenszel chi-square',
        eq(
          CHI,
          mfrac(
            SQ(mrow(mn(v.a ?? ''), mo('×'), mn(v.d ?? ''), mo('−'), mn(v.b ?? ''), mo('×'), mn(v.c ?? ''))),
            mrow(
              msub(mi('n'), mn('1')),
              msub(mi('n'), mn('0')),
              msub(mi('m'), mn('1')),
              msub(mi('m'), mn('0')),
            ),
          ),
        ),
      ),
    compute: (v: V) => {
      const { a, b, c, d } = v as { a: number; b: number; c: number; d: number };
      const n1 = a + b;
      const n0 = c + d;
      const m1 = a + c;
      const m0 = b + d;
      const n = n1 + n0;
      const delta = a - (n1 * m1) / n;
      return (delta * delta * n * n * (n - 1)) / (n1 * n0 * m1 * m0);
    },
    source: MH,
  },
  chi_square_mh_delta: {
    id: 'chi_square_mh_delta',
    words:
      'The Mantel–Haenszel chi-square computed from the observed ' +
      'minus expected difference and the table margins.',
    mathml: () =>
      math(
        'Mantel–Haenszel chi-square from delta',
        eq(
          CHI,
          mfrac(
            mrow(
              msup(mi('δ'), mn('2')),
              msup(mi('n'), mn('2')),
              mrow(mo('('), mi('n'), mo('−'), mn('1'), mo(')')),
            ),
            mrow(
              msub(mi('n'), mn('1')),
              msub(mi('n'), mn('0')),
              msub(mi('m'), mn('1')),
              msub(mi('m'), mn('0')),
            ),
          ),
        ),
      ),
    compute: (v: V) =>
      ((v.delta ?? NaN) ** 2 * (v.n ?? NaN) ** 2 * ((v.n ?? NaN) - 1)) /
      ((v.n1 ?? NaN) * (v.n0 ?? NaN) * (v.m1 ?? NaN) * (v.m0 ?? NaN)),
    source: MH,
  },
  mcnemar: {
    id: 'mcnemar',
    words:
      "McNemar's statistic squares the difference between the two " +
      'discordant-pair counts and divides by their sum.',
    mathml: (v: S) =>
      math(
        "McNemar's chi-square",
        eq(
          CHI,
          mfrac(
            SQ(mrow(mn(v.db ?? ''), mo('−'), mn(v.dc ?? ''))),
            mrow(mn(v.db ?? ''), mo('+'), mn(v.dc ?? '')),
          ),
        ),
      ),
    compute: (v: V) => ((v.db ?? NaN) - (v.dc ?? NaN)) ** 2 / ((v.db ?? NaN) + (v.dc ?? NaN)),
    source: 'McNemar, Psychometrika.',
  },
  mcnemar_cc: {
    id: 'mcnemar_cc',
    words:
      "McNemar's statistic with the continuity correction subtracts " +
      'one from the absolute discordant-pair difference before ' +
      'squaring.',
    mathml: (v: S) =>
      math(
        "McNemar's chi-square, continuity corrected",
        eq(
          CHI,
          mfrac(
            SQ(mrow(mo('|'), mn(v.db ?? ''), mo('−'), mn(v.dc ?? ''), mo('|'), mo('−'), mn('1'))),
            mrow(mn(v.db ?? ''), mo('+'), mn(v.dc ?? '')),
          ),
        ),
      ),
    compute: (v: V) => {
      const n = (v.db ?? NaN) + (v.dc ?? NaN);
      if (n === 0) return NaN;
      if (v.db === v.dc) return 0;
      return (Math.abs((v.db ?? 0) - (v.dc ?? 0)) - 1) ** 2 / n;
    },
    source: 'McNemar, Psychometrika; Edwards correction.',
  },
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
  chisq_oe: {
    id: 'chisq_oe',
    words:
      'A single observed-minus-expected cell contribution: the ' +
      'squared difference divided by the expected count.',
    mathml: () =>
      math(
        'one chi-square cell contribution',
        eq(CHI, mfrac(O_MINUS_E, mi('e'))),
      ),
    compute: (v: V) => ((v.o ?? NaN) - (v.e ?? NaN)) ** 2 / (v.e ?? NaN),
    source: FLEISS,
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

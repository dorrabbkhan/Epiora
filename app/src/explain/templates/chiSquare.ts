// Single-table chi-square templates (v1.0 plan Task 16, spec §6):
// degrees of freedom, distribution tails, the cell-sum χ² forms
// (uncorrected, Yates and per-cell continuity correction), the
// single-table Mantel–Haenszel forms, McNemar and the single-cell
// contribution. The pooled and homogeneity statistics are in
// `chiSquarePooled.ts`; the shared χ² pieces (`CHI`, `SQ`, `MH`) are
// exported for it.
import type { FormulaTemplate } from '../index';
import { math, mfrac, mi, mn, mo, mrow, msub, msup } from './ml';
import { chiCells, gamQ } from './special';

type V = Record<string, number>;
type S = Record<string, string>;

const eq = (lhs: string, rhs: string): string => mrow(lhs, mo('='), rhs);
export const CHI = msup(mi('χ'), mn('2'));
export const SQ = (inner: string): string =>
  msup(mrow(mo('('), inner, mo(')')), mn('2'));
const O_MINUS_E = SQ(mrow(mi('o'), mo('−'), mi('e')));

const FLEISS = 'Fleiss, Levin & Paik, Statistical Methods for Rates and Proportions.';
export const MH = 'Mantel & Haenszel, Journal of the National Cancer Institute.';

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
};

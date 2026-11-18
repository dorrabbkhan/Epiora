// Proportion and binomial interval templates (v1.0 plan Task 16,
// spec §6): Wald, Wilson (score), Agresti–Coull, the rounded score
// bounds, Wilson with the finite-population correction and the score
// bounds with continuity correction. The one-sample z test and the
// binomial tail formulas are in `binomial.ts`.
import type { FormulaTemplate } from '../index';
import { bin, math, mfrac, mi, mn, mo, mrow, msqrt, msup } from './ml';
import { wilson } from './special';

type V = Record<string, number>;
type S = Record<string, string>;

const eq = (lhs: string, rhs: string): string => mrow(lhs, mo('='), rhs);
const PHAT = mi('p̂');
const SQRT_PQ = msqrt(
  mfrac(mrow(PHAT, mo('('), mn('1'), mo('−'), PHAT, mo(')')), mi('n')),
);

const WILSON =
  'Wilson score interval; Wilson, Journal of the American Statistical Association.';

const waldInner = (v: S, sign: string): string =>
  mrow(PHAT, mo(sign), mn(v.z ?? ''), mo('×'), SQRT_PQ);

const Z2 = (v: S): string => msup(mn(v.z ?? ''), mn('2'));
/** Wilson score bound: (p̂ + z²/2n ∓ z·√(p̂q̂/n + z²/4n²)) / (1 + z²/n). */
const wilsonBound = (v: S, sign: string): string =>
  mfrac(
    mrow(
      mrow(
        mo('('),
        PHAT,
        mo('+'),
        mfrac(Z2(v), mrow(mn('2'), mn(v.n ?? ''))),
        mo(')'),
      ),
      mo(sign),
      mn(v.z ?? ''),
      mo('×'),
      msqrt(
        mrow(
          mfrac(mrow(PHAT, mrow(mo('('), mn('1'), mo('−'), PHAT, mo(')'))), mn(v.n ?? '')),
          mo('+'),
          mfrac(Z2(v), mrow(mn('4'), msup(mn(v.n ?? ''), mn('2')))),
        ),
      ),
    ),
    mrow(mn('1'), mo('+'), mfrac(Z2(v), mn(v.n ?? ''))),
  );

export const PROPORTIONS: Record<string, FormulaTemplate> = {
  prop_wald_lower: {
    id: 'prop_wald_lower',
    words:
      'The lower Wald bound is the sample proportion minus z ' +
      'times its binomial standard error.',
    mathml: (v: S) =>
      math('lower Wald bound of a proportion', eq(mi('L'), waldInner(v, '−'))),
    compute: (v: V) => {
      const p = (v.x ?? NaN) / (v.n ?? NaN);
      return p - (v.z ?? NaN) * Math.sqrt((p * (1 - p)) / (v.n ?? NaN));
    },
    source: 'Wald interval; Fleiss, Levin & Paik.',
  },
  prop_wald_upper: {
    id: 'prop_wald_upper',
    words:
      'The upper Wald bound is the sample proportion plus z ' +
      'times its binomial standard error.',
    mathml: (v: S) =>
      math('upper Wald bound of a proportion', eq(mi('U'), waldInner(v, '+'))),
    compute: (v: V) => {
      const p = (v.x ?? NaN) / (v.n ?? NaN);
      return p + (v.z ?? NaN) * Math.sqrt((p * (1 - p)) / (v.n ?? NaN));
    },
    source: 'Wald interval; Fleiss, Levin & Paik.',
  },
  prop_wald_lower_pct: {
    id: 'prop_wald_lower_pct',
    words:
      'The lower Wald bound is the sample proportion minus z ' +
      'times its binomial standard error, as a percentage.',
    mathml: (v: S) =>
      math(
        'lower Wald bound of a proportion, percent',
        eq(mi('L'), mrow(mrow(mo('('), waldInner(v, '−'), mo(')')), mo('×'), mn('100'))),
      ),
    compute: (v: V) => {
      const p = (v.x ?? NaN) / (v.n ?? NaN);
      return (p - (v.z ?? NaN) * Math.sqrt((p * (1 - p)) / (v.n ?? NaN))) * 100;
    },
    source: 'Wald interval; Fleiss, Levin & Paik.',
  },
  prop_wald_upper_pct: {
    id: 'prop_wald_upper_pct',
    words:
      'The upper Wald bound is the sample proportion plus z ' +
      'times its binomial standard error, as a percentage.',
    mathml: (v: S) =>
      math(
        'upper Wald bound of a proportion, percent',
        eq(mi('U'), mrow(mrow(mo('('), waldInner(v, '+'), mo(')')), mo('×'), mn('100'))),
      ),
    compute: (v: V) => {
      const p = (v.x ?? NaN) / (v.n ?? NaN);
      return (p + (v.z ?? NaN) * Math.sqrt((p * (1 - p)) / (v.n ?? NaN))) * 100;
    },
    source: 'Wald interval; Fleiss, Levin & Paik.',
  },
  wilson_lower_pct: {
    id: 'wilson_lower_pct',
    words:
      'The lower Wilson (score) bound solves the score equation ' +
      'for the proportion — it centres on a shrunken estimate and ' +
      'is expressed as a percentage.',
    mathml: (v: S) =>
      math(
        'lower Wilson score bound, percent',
        eq(mi('L'), wilsonBound(v, '−')),
      ),
    compute: (v: V) => wilson(v.x ?? NaN, v.n ?? NaN, v.z ?? NaN)[0] * 100,
    source: WILSON,
  },
  wilson_upper_pct: {
    id: 'wilson_upper_pct',
    words:
      'The upper Wilson (score) bound solves the score equation ' +
      'for the proportion — it centres on a shrunken estimate and ' +
      'is expressed as a percentage.',
    mathml: (v: S) =>
      math(
        'upper Wilson score bound, percent',
        eq(mi('U'), wilsonBound(v, '+')),
      ),
    compute: (v: V) => wilson(v.x ?? NaN, v.n ?? NaN, v.z ?? NaN)[1] * 100,
    source: WILSON,
  },
  prop_ac_lower: {
    id: 'prop_ac_lower',
    words:
      'The Agresti–Coull lower bound shifts the proportion toward ' +
      'a half by adding half of z² pseudo-observations before the ' +
      'Wald interval is taken; the bound is clipped at zero.',
    mathml: (v: S) =>
      math(
        'Agresti–Coull lower bound',
        eq(
          mi('L'),
          mrow(
            mi('max'),
            mo('('),
            bin(mi('p̃'), '−', mrow(mn(v.z ?? ''), mo('×'), msqrt(mfrac(mrow(mi('p̃'), mo('('), mn('1'), mo('−'), mi('p̃'), mo(')')), mi('ñ'))))),
            mo(')'),
          ),
        ),
      ),
    compute: (v: V) => {
      const zz = (v.z ?? NaN) ** 2;
      const tn = (v.n ?? NaN) + zz;
      const tp = ((v.x ?? NaN) + zz / 2) / tn;
      const h = (v.z ?? NaN) * Math.sqrt((tp * (1 - tp)) / tn);
      return Math.max(tp - h, 0);
    },
    source: 'Agresti & Coull, The American Statistician.',
  },
  prop_ac_upper: {
    id: 'prop_ac_upper',
    words:
      'The Agresti–Coull upper bound shifts the proportion toward ' +
      'a half by adding half of z² pseudo-observations before the ' +
      'Wald interval is taken; the bound is clipped at one.',
    mathml: (v: S) =>
      math(
        'Agresti–Coull upper bound',
        eq(
          mi('U'),
          mrow(
            mi('min'),
            mo('('),
            bin(mi('p̃'), '+', mrow(mn(v.z ?? ''), mo('×'), msqrt(mfrac(mrow(mi('p̃'), mo('('), mn('1'), mo('−'), mi('p̃'), mo(')')), mi('ñ'))))),
            mo(')'),
          ),
        ),
      ),
    compute: (v: V) => {
      const zz = (v.z ?? NaN) ** 2;
      const tn = (v.n ?? NaN) + zz;
      const tp = ((v.x ?? NaN) + zz / 2) / tn;
      const h = (v.z ?? NaN) * Math.sqrt((tp * (1 - tp)) / tn);
      return Math.min(tp + h, 1);
    },
    source: 'Agresti & Coull, The American Statistician.',
  },
  prop_scoreci_lower: {
    id: 'prop_scoreci_lower',
    words:
      'The lower score bound of the proportion, rounded to four ' +
      'decimal places as the reference software displays it.',
    mathml: (v: S) =>
      math(
        'lower score bound',
        eq(mi('L'), mfrac(mn(v.x ?? ''), mn(v.n ?? ''))),
      ),
    compute: (v: V) =>
      Math.round(wilson(v.x ?? NaN, v.n ?? NaN, v.z ?? NaN)[0] * 1e4) / 1e4,
    source: WILSON,
  },
  prop_scoreci_upper: {
    id: 'prop_scoreci_upper',
    words:
      'The upper score bound of the proportion, rounded to four ' +
      'decimal places as the reference software displays it.',
    mathml: (v: S) =>
      math(
        'upper score bound',
        eq(mi('U'), mfrac(mn(v.x ?? ''), mn(v.n ?? ''))),
      ),
    compute: (v: V) =>
      Math.round(wilson(v.x ?? NaN, v.n ?? NaN, v.z ?? NaN)[1] * 1e4) / 1e4,
    source: WILSON,
  },
  prop_wilson_fpc_lower: {
    id: 'prop_wilson_fpc_lower',
    words:
      'The lower Wilson bound scaled by the finite-population ' +
      'correction, which shrinks the interval when the sample is a ' +
      'large fraction of the population.',
    mathml: (v: S) =>
      math(
        'lower Wilson bound with FPC',
        eq(
          mi('L'),
          mfrac(
            mrow(
              mrow(mo('('), PHAT, mo('+'), mfrac(Z2(v), mrow(mn('2'), mn(v.n ?? ''))), mo(')')),
              mo('−'),
              mi('fpc'),
              mo('×'),
              mn(v.z ?? ''),
              mo('×'),
              mi('r'),
            ),
            mrow(mn('1'), mo('+'), mfrac(Z2(v), mn(v.n ?? ''))),
          ),
        ),
      ),
    compute: (v: V) => {
      const zz = (v.z ?? NaN) ** 2;
      const p = (v.x ?? NaN) / (v.n ?? NaN);
      if (p === 0) return 0;
      const fpc = Math.sqrt(((v.pop ?? NaN) - (v.n ?? NaN)) / ((v.pop ?? NaN) - 1));
      const r = Math.sqrt(
        ((v.x ?? NaN) * ((v.n ?? NaN) - (v.x ?? NaN))) / (v.n ?? NaN) ** 3 +
          zz / (4 * (v.n ?? NaN) ** 2),
      );
      return (
        ((v.n ?? NaN) / ((v.n ?? NaN) + zz)) *
        (p + zz / (2 * (v.n ?? NaN)) - fpc * (v.z ?? NaN) * r)
      );
    },
    source: 'Wilson score interval with the finite-population correction; ' + WILSON,
  },
  prop_wilson_fpc_upper: {
    id: 'prop_wilson_fpc_upper',
    words:
      'The upper Wilson bound scaled by the finite-population ' +
      'correction, which shrinks the interval when the sample is a ' +
      'large fraction of the population.',
    mathml: (v: S) =>
      math(
        'upper Wilson bound with FPC',
        eq(
          mi('U'),
          mfrac(
            mrow(
              mrow(mo('('), PHAT, mo('+'), mfrac(Z2(v), mrow(mn('2'), mn(v.n ?? ''))), mo(')')),
              mo('+'),
              mi('fpc'),
              mo('×'),
              mn(v.z ?? ''),
              mo('×'),
              mi('r'),
            ),
            mrow(mn('1'), mo('+'), mfrac(Z2(v), mn(v.n ?? ''))),
          ),
        ),
      ),
    compute: (v: V) => {
      const zz = (v.z ?? NaN) ** 2;
      const p = (v.x ?? NaN) / (v.n ?? NaN);
      if (p === 1) return 1;
      const fpc = Math.sqrt(((v.pop ?? NaN) - (v.n ?? NaN)) / ((v.pop ?? NaN) - 1));
      const r = Math.sqrt(
        ((v.x ?? NaN) * ((v.n ?? NaN) - (v.x ?? NaN))) / (v.n ?? NaN) ** 3 +
          zz / (4 * (v.n ?? NaN) ** 2),
      );
      return (
        ((v.n ?? NaN) / ((v.n ?? NaN) + zz)) *
        (p + zz / (2 * (v.n ?? NaN)) + fpc * (v.z ?? NaN) * r)
      );
    },
    source: 'Wilson score interval with the finite-population correction; ' + WILSON,
  },
  prop_score_cc_lower: {
    id: 'prop_score_cc_lower',
    words:
      'The lower score bound with the continuity correction: the ' +
      'quadratic is shifted by one before the square-root term is ' +
      'taken.',
    mathml: (v: S) =>
      math('lower score bound, continuity corrected', eq(mi('L'), mfrac(mn(v.x ?? ''), mn(v.n ?? '')))),
    compute: (v: V) => {
      const zz = (v.z ?? NaN) ** 2;
      const p = (v.x ?? NaN) / (v.n ?? NaN);
      if (p === 0) return 0;
      return (
        (2 * (v.n ?? NaN) * p + zz - 1 -
          (v.z ?? NaN) *
            Math.sqrt(zz - (2 + 1 / (v.n ?? NaN)) + 4 * p * ((v.n ?? NaN) * (1 - p) + 1))) /
        (2 * ((v.n ?? NaN) + zz))
      );
    },
    source: 'Score interval with continuity correction; Fleiss, Levin & Paik.',
  },
  prop_score_cc_upper: {
    id: 'prop_score_cc_upper',
    words:
      'The upper score bound with the continuity correction: the ' +
      'quadratic is shifted by one before the square-root term is ' +
      'taken.',
    mathml: (v: S) =>
      math('upper score bound, continuity corrected', eq(mi('U'), mfrac(mn(v.x ?? ''), mn(v.n ?? '')))),
    compute: (v: V) => {
      const zz = (v.z ?? NaN) ** 2;
      const p = (v.x ?? NaN) / (v.n ?? NaN);
      if (p === 1) return 1;
      return (
        (2 * (v.n ?? NaN) * p + zz + 1 +
          (v.z ?? NaN) *
            Math.sqrt(zz + (2 - 1 / (v.n ?? NaN)) + 4 * p * ((v.n ?? NaN) * (1 - p) - 1))) /
        (2 * ((v.n ?? NaN) + zz))
      );
    },
    source: 'Score interval with continuity correction; Fleiss, Levin & Paik.',
  },
};

// Two-by-two table measures (v1.0 plan Task 16, spec §6): the odds
// ratio, risk ratio and risk difference, the small-sample risk
// ratio, the matched-pairs odds ratio, discordant-pair counts, and
// the dose-response odds-ratio terms.
import type { FormulaTemplate } from '../index';
import { bin, math, mfrac, mi, mn, mo, mrow } from './ml';

type V = Record<string, number>;
type S = Record<string, string>;

const eq = (lhs: string, rhs: string): string => mrow(lhs, mo('='), rhs);

const ROTHMAN = 'Rothman, Greenland & Lash, Modern Epidemiology.';
const EI = 'Epi Info StatCalc documentation.';

export const TWO_BY_TWO: Record<string, FormulaTemplate> = {
  odds_ratio: {
    id: 'odds_ratio',
    words:
      'The odds ratio is the odds of the outcome among the exposed ' +
      'divided by the odds among the unexposed — the cross product ' +
      'a·d over b·c of the two-by-two table.',
    mathml: (v: S) =>
      math(
        'odds ratio = (a·d)/(b·c)',
        mrow(
          eq(
            mi('OR'),
            mfrac(
              mrow(mn(v.a ?? ''), mo('×'), mn(v.d ?? '')),
              mrow(mn(v.b ?? ''), mo('×'), mn(v.c ?? '')),
            ),
          ),
          mo('='),
          mn(v.result ?? ''),
        ),
      ),
    compute: (v: V) => ((v.a ?? NaN) * (v.d ?? NaN)) / ((v.b ?? NaN) * (v.c ?? NaN)),
    source: ROTHMAN,
  },
  risk_ratio: {
    id: 'risk_ratio',
    words:
      'The risk ratio is the proportion with the outcome among the ' +
      'exposed divided by the proportion among the unexposed.',
    mathml: (v: S) =>
      math(
        'risk ratio = p₁/p₀',
        eq(mi('RR'), mfrac(mn(v.p1 ?? ''), mn(v.p0 ?? ''))),
      ),
    compute: (v: V) => (v.p1 ?? NaN) / (v.p0 ?? NaN),
    source: ROTHMAN,
  },
  risk_difference: {
    id: 'risk_difference',
    words:
      'The risk difference is the proportion with the outcome ' +
      'among the exposed minus the proportion among the unexposed.',
    mathml: (v: S) =>
      math(
        'risk difference = p₁ − p₀',
        eq(mi('RD'), bin(mn(v.p1 ?? ''), '−', mn(v.p0 ?? ''))),
      ),
    compute: (v: V) => (v.p1 ?? NaN) - (v.p0 ?? NaN),
    source: ROTHMAN,
  },
  risk_difference_pct: {
    id: 'risk_difference_pct',
    words:
      'The risk difference is the proportion with the outcome ' +
      'among the exposed minus the proportion among the unexposed, ' +
      'expressed as a percentage.',
    mathml: (v: S) =>
      math(
        'risk difference percent',
        eq(
          mi('RD%'),
          mrow(
            mrow(mo('('), mn(v.p1 ?? ''), mo('−'), mn(v.p0 ?? ''), mo(')')),
            mo('×'),
            mn('100'),
          ),
        ),
      ),
    compute: (v: V) => ((v.p1 ?? NaN) - (v.p0 ?? NaN)) * 100,
    source: ROTHMAN,
  },
  rr_small: {
    id: 'rr_small',
    words:
      'The small-sample risk ratio compares the exposed proportion ' +
      'with the unexposed proportion computed as a rate — the ' +
      'unexposed ill plus one over the unexposed total plus one.',
    mathml: (v: S) =>
      math(
        'small-sample risk ratio',
        eq(
          mi('RR'),
          mfrac(
            mn(v.pe ?? ''),
            mfrac(
              mrow(mn(v.c ?? ''), mo('+'), mn('1')),
              mrow(mn(v.n0 ?? ''), mo('+'), mn('1')),
            ),
          ),
        ),
      ),
    compute: (v: V) => (v.pe ?? NaN) / (((v.c ?? NaN) + 1) / ((v.n0 ?? NaN) + 1)),
    source: 'Jewell correction for small samples; ' + EI,
  },
  or_pairs: {
    id: 'or_pairs',
    words:
      'The matched-pairs odds ratio is the count of pairs ' +
      'discordant in one direction divided by the count discordant ' +
      'in the other.',
    mathml: (v: S) =>
      math(
        'matched odds ratio = b/c',
        eq(mi('OR'), mfrac(mn(v.db ?? ''), mn(v.dc ?? ''))),
      ),
    compute: (v: V) => (v.db ?? NaN) / (v.dc ?? NaN),
    source: ROTHMAN,
  },
  discordant_pairs: {
    id: 'discordant_pairs',
    words:
      'The number of discordant pairs is the sum of the two ' +
      'off-diagonal cell counts of the matched table.',
    mathml: (v: S) =>
      math(
        'discordant pairs = b + c',
        eq(mi('d'), bin(mn(v.db ?? ''), '+', mn(v.dc ?? ''))),
      ),
    compute: (v: V) => (v.db ?? NaN) + (v.dc ?? NaN),
    source: ROTHMAN,
  },
  dr_or: {
    id: 'dr_or',
    words:
      'Each dose-response level contributes its own odds ratio ' +
      'against the baseline level.',
    mathml: (v: S) =>
      math(
        'level odds ratio',
        eq(
          mi('OR'),
          mfrac(
            mrow(mn(v.a ?? ''), mo('×'), mn(v.b_base ?? '')),
            mrow(mn(v.b ?? ''), mo('×'), mn(v.a_base ?? '')),
          ),
        ),
      ),
    compute: (v: V) =>
      ((v.a ?? NaN) * (v.b_base ?? NaN)) / ((v.b ?? NaN) * (v.a_base ?? NaN)),
    source: 'Dose-response chi-square for trend; ' + EI,
  },
  dr_mh_or: {
    id: 'dr_mh_or',
    words:
      'The Mantel–Haenszel summary odds ratio across the dose ' +
      'levels is the sum of the per-level numerator terms divided ' +
      'by the sum of the denominator terms.',
    mathml: (v: S) =>
      math(
        'MH pooled odds ratio = Σ num / Σ den',
        eq(mi('OR'), mfrac(mn(v.num ?? ''), mn(v.den ?? ''))),
      ),
    compute: (v: V) => (v.num ?? NaN) / (v.den ?? NaN),
    source: 'Mantel & Haenszel; ' + ROTHMAN,
  },
};

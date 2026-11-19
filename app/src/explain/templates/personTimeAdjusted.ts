// Adjusted and pooled person-time measure templates (v1.0 plan
// Task 16, spec §6): the directly adjusted rate and its bounds, the
// adjusted rate ratio, and the Mantel–Haenszel rate ratio and odds
// ratio. The rate, its bounds and the z scores are in
// `personTime.ts`, which owns the shared `RB` citation.
import type { FormulaTemplate } from '../index';
import { bin, math, mfrac, mi, mn, mo, mrow, msqrt, msub, msup } from './ml';
import { RB } from './personTime';

type V = Record<string, number>;
type S = Record<string, string>;

const eq = (lhs: string, rhs: string): string => mrow(lhs, mo('='), rhs);

const directLn = (v: S, sign?: string): string =>
  msup(
    mi('e'),
    mrow(
      mo('('),
      mfrac(msub(mi('Σ'), mi('we')), msub(mi('Σ'), mi('w'))),
      ...(sign === undefined
        ? []
        : [mo(sign), mfrac(mn(v.z ?? ''), msqrt(msub(mi('Σ'), mi('w'))))]),
      mo(')'),
    ),
  );

const rrLn = (v: S, sign?: string): string =>
  msup(
    mi('e'),
    mrow(
      mo('('),
      msub(mi('θ'), mi('')),
      ...(sign === undefined
        ? []
        : [mo(sign), mfrac(mn(v.z ?? ''), msqrt(msub(mi('Σ'), mi('w'))))]),
      mo(')'),
    ),
  );

export const PERSON_TIME_ADJUSTED: Record<string, FormulaTemplate> = {
  direct_adjust: {
    id: 'direct_adjust',
    words:
      'The directly adjusted log ratio is the sum of the weighted ' +
      'stratum log ratios over the sum of the weights.',
    mathml: (v: S) =>
      math(
        'directly adjusted value',
        eq(mi('θ̄'), mfrac(mn(v.sum_we ?? ''), mn(v.sum_w ?? ''))),
      ),
    compute: (v: V) => v.adj ?? (v.sum_we ?? NaN) / (v.sum_w ?? NaN),
    source: 'Direct standardization; ' + RB,
  },
  direct_adjust_lower: {
    id: 'direct_adjust_lower',
    words:
      'The lower bound of the directly adjusted log rate is the ' +
      'adjusted value minus z over the square root of the summed ' +
      'weights.',
    mathml: (v: S) =>
      math(
        'directly adjusted lower bound',
        eq(
          mi('L'),
          bin(
            mfrac(mn(v.sum_we ?? ''), mn(v.sum_w ?? '')),
            '−',
            mfrac(mn(v.z ?? ''), msqrt(mn(v.sum_w ?? ''))),
          ),
        ),
      ),
    compute: (v: V) =>
      (v.sum_we ?? NaN) / (v.sum_w ?? NaN) - (v.z ?? NaN) / Math.sqrt(v.sum_w ?? NaN),
    source: 'Direct standardization; ' + RB,
  },
  direct_adjust_upper: {
    id: 'direct_adjust_upper',
    words:
      'The upper bound of the directly adjusted log rate is the ' +
      'adjusted value plus z over the square root of the summed ' +
      'weights.',
    mathml: (v: S) =>
      math(
        'directly adjusted upper bound',
        eq(
          mi('U'),
          bin(
            mfrac(mn(v.sum_we ?? ''), mn(v.sum_w ?? '')),
            '+',
            mfrac(mn(v.z ?? ''), msqrt(mn(v.sum_w ?? ''))),
          ),
        ),
      ),
    compute: (v: V) =>
      (v.sum_we ?? NaN) / (v.sum_w ?? NaN) + (v.z ?? NaN) / Math.sqrt(v.sum_w ?? NaN),
    source: 'Direct standardization; ' + RB,
  },
  direct_adjust_ln: {
    id: 'direct_adjust_ln',
    words:
      'The directly adjusted rate is the exponential of the ' +
      'weighted average log rate.',
    mathml: (v: S) =>
      math('adjusted rate = exp(Σwe/Σw)', eq(mi('r'), directLn(v))),
    compute: (v: V) => Math.exp((v.sum_we ?? NaN) / (v.sum_w ?? NaN)),
    source: 'Direct standardization; ' + RB,
  },
  direct_adjust_ln_lower: {
    id: 'direct_adjust_ln_lower',
    words:
      'The lower bound of the adjusted rate is the exponential of ' +
      'the adjusted log rate minus z over the root summed weight.',
    mathml: (v: S) =>
      math('adjusted rate, lower bound', eq(mi('L'), directLn(v, '−'))),
    compute: (v: V) =>
      Math.exp(
        (v.sum_we ?? NaN) / (v.sum_w ?? NaN) -
          (v.z ?? NaN) / Math.sqrt(v.sum_w ?? NaN),
      ),
    source: 'Direct standardization; ' + RB,
  },
  direct_adjust_ln_upper: {
    id: 'direct_adjust_ln_upper',
    words:
      'The upper bound of the adjusted rate is the exponential of ' +
      'the adjusted log rate plus z over the root summed weight.',
    mathml: (v: S) =>
      math('adjusted rate, upper bound', eq(mi('U'), directLn(v, '+'))),
    compute: (v: V) =>
      Math.exp(
        (v.sum_we ?? NaN) / (v.sum_w ?? NaN) +
          (v.z ?? NaN) / Math.sqrt(v.sum_w ?? NaN),
      ),
    source: 'Direct standardization; ' + RB,
  },
  direct_rr: {
    id: 'direct_rr',
    words:
      'The adjusted rate ratio is the exponential of the adjusted ' +
      'log ratio.',
    mathml: (v: S) =>
      math('adjusted rate ratio', eq(mi('RR'), msup(mi('e'), mrow(mo('('), mn(v.adj_ln ?? ''), mo(')'))))),
    compute: (v: V) => Math.exp(v.adj_ln ?? NaN),
    source: 'Direct standardization; ' + RB,
  },
  direct_rr_ln_lower: {
    id: 'direct_rr_ln_lower',
    words:
      'The lower bound of the adjusted ratio is the exponential of ' +
      'the adjusted log ratio minus z over the root summed weight.',
    mathml: (v: S) =>
      math('adjusted ratio, lower bound', eq(mi('L'), rrLn(v, '−'))),
    compute: (v: V) =>
      Math.exp((v.adj ?? NaN) - (v.z ?? NaN) / Math.sqrt(v.sw ?? NaN)),
    source: 'Direct standardization; ' + RB,
  },
  direct_rr_ln_upper: {
    id: 'direct_rr_ln_upper',
    words:
      'The upper bound of the adjusted ratio is the exponential of ' +
      'the adjusted log ratio plus z over the root summed weight.',
    mathml: (v: S) =>
      math('adjusted ratio, upper bound', eq(mi('U'), rrLn(v, '+'))),
    compute: (v: V) =>
      Math.exp((v.adj ?? NaN) + (v.z ?? NaN) / Math.sqrt(v.sw ?? NaN)),
    source: 'Direct standardization; ' + RB,
  },
  mh_or: {
    id: 'mh_or',
    words:
      'The Mantel–Haenszel pooled odds ratio is the sum of the ' +
      'stratum numerator terms over the sum of the denominator ' +
      'terms.',
    mathml: (v: S) =>
      math('MH odds ratio', eq(mi('OR'), mfrac(mn(v.num ?? ''), mn(v.den ?? '')))),
    compute: (v: V) => (v.num ?? NaN) / (v.den ?? NaN),
    source: 'Mantel & Haenszel.',
  },
  mh_rr: {
    id: 'mh_rr',
    words:
      'The Mantel–Haenszel pooled risk ratio is the sum of the ' +
      'stratum numerator terms over the sum of the denominator ' +
      'terms.',
    mathml: (v: S) =>
      math('MH risk ratio', eq(mi('RR'), mfrac(mn(v.num ?? ''), mn(v.den ?? '')))),
    compute: (v: V) => (v.num ?? NaN) / (v.den ?? NaN),
    source: 'Mantel & Haenszel.',
  },
};

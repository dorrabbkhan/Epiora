// Wald confidence bounds (v1.0 plan Task 16, spec §6): estimate ±
// z·SE in natural, percentage and per-unit scalings, and the
// log-scale Wald bound exponentiated back.
import type { FormulaTemplate } from '../index';
import { math, mi, mn, mo, mrow, msup } from './ml';

type V = Record<string, number>;
type S = Record<string, string>;

const eq = (lhs: string, rhs: string): string => mrow(lhs, mo('='), rhs);

const SRC =
  'Wald large-sample interval; Rothman, Greenland & Lash, Modern Epidemiology.';

const bound = (v: S, sign: string, scale?: string): string =>
  mrow(
    mn(v.est ?? ''),
    mo(sign),
    mn(v.z ?? ''),
    mo('×'),
    mn(v.se ?? ''),
    ...(scale !== undefined ? [mo('×'), mn(scale)] : []),
  );

const expBound = (v: S, sign: string): string =>
  mrow(
    mn(v.est ?? ''),
    mo('×'),
    msup(
      mi('e'),
      mrow(mo('('), mo(sign), mn(v.z ?? ''), mo('×'), mn(v.se ?? ''), mo(')')),
    ),
  );

export const WALD: Record<string, FormulaTemplate> = {
  wald_lower: {
    id: 'wald_lower',
    words:
      'The lower bound of the Wald interval is the estimate minus ' +
      'z times its standard error.',
    mathml: (v: S) => math('estimate − z·SE', eq(mi('L'), bound(v, '−'))),
    compute: (v: V) => (v.est ?? NaN) - (v.z ?? NaN) * (v.se ?? NaN),
    source: SRC,
  },
  wald_upper: {
    id: 'wald_upper',
    words:
      'The upper bound of the Wald interval is the estimate plus ' +
      'z times its standard error.',
    mathml: (v: S) => math('estimate + z·SE', eq(mi('U'), bound(v, '+'))),
    compute: (v: V) => (v.est ?? NaN) + (v.z ?? NaN) * (v.se ?? NaN),
    source: SRC,
  },
  wald_lower_pct: {
    id: 'wald_lower_pct',
    words:
      'The lower bound of the Wald interval is the estimate minus ' +
      'z times its standard error, expressed as a percentage.',
    mathml: (v: S) => math('(estimate − z·SE) × 100', eq(mi('L'), bound(v, '−', '100'))),
    compute: (v: V) => ((v.est ?? NaN) - (v.z ?? NaN) * (v.se ?? NaN)) * 100,
    source: SRC,
  },
  wald_upper_pct: {
    id: 'wald_upper_pct',
    words:
      'The upper bound of the Wald interval is the estimate plus ' +
      'z times its standard error, expressed as a percentage.',
    mathml: (v: S) => math('(estimate + z·SE) × 100', eq(mi('U'), bound(v, '+', '100'))),
    compute: (v: V) => ((v.est ?? NaN) + (v.z ?? NaN) * (v.se ?? NaN)) * 100,
    source: SRC,
  },
  wald_lower_scaled: {
    id: 'wald_lower_scaled',
    words:
      'The lower bound of the Wald interval is the estimate minus ' +
      'z times its standard error, rescaled to the reporting unit.',
    mathml: (v: S) =>
      math('(estimate − z·SE) × per unit', eq(mi('L'), bound(v, '−', v.per_unit ?? ''))),
    compute: (v: V) =>
      ((v.est ?? NaN) - (v.z ?? NaN) * (v.se ?? NaN)) * (v.per_unit ?? NaN),
    source: SRC,
  },
  wald_upper_scaled: {
    id: 'wald_upper_scaled',
    words:
      'The upper bound of the Wald interval is the estimate plus ' +
      'z times its standard error, rescaled to the reporting unit.',
    mathml: (v: S) =>
      math('(estimate + z·SE) × per unit', eq(mi('U'), bound(v, '+', v.per_unit ?? ''))),
    compute: (v: V) =>
      ((v.est ?? NaN) + (v.z ?? NaN) * (v.se ?? NaN)) * (v.per_unit ?? NaN),
    source: SRC,
  },
  wald_exp_lower: {
    id: 'wald_exp_lower',
    words:
      'The interval is formed on the log scale: the lower bound is ' +
      'the estimate times the exponential of minus z times the ' +
      'log standard error.',
    mathml: (v: S) => math('estimate × exp(−z·SE)', eq(mi('L'), expBound(v, '−'))),
    compute: (v: V) => (v.est ?? NaN) * Math.exp(-(v.z ?? NaN) * (v.se ?? NaN)),
    source: SRC,
  },
  wald_exp_upper: {
    id: 'wald_exp_upper',
    words:
      'The interval is formed on the log scale: the upper bound is ' +
      'the estimate times the exponential of z times the ' +
      'log standard error.',
    mathml: (v: S) => math('estimate × exp(+z·SE)', eq(mi('U'), expBound(v, '+'))),
    compute: (v: V) => (v.est ?? NaN) * Math.exp((v.z ?? NaN) * (v.se ?? NaN)),
    source: SRC,
  },
};

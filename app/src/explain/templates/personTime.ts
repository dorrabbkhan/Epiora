// Person-time measure templates (v1.0 plan Task 16, spec §6): the
// rate and its Taylor, Byar-scaled and Rothman–Greenland bounds,
// the person-time z score (single table and pooled), rate ratio
// and difference, the directly adjusted rate and its bounds, the
// adjusted rate ratio, and the Mantel–Haenszel rate ratio and
// odds ratio.
import type { FormulaTemplate } from '../index';
import { bin, math, mfrac, mi, mn, mo, mrow, msqrt, msub, msup } from './ml';
import { byarBase } from './special';

type V = Record<string, number>;
type S = Record<string, string>;

const eq = (lhs: string, rhs: string): string => mrow(lhs, mo('='), rhs);

const RB = 'Rothman & Boice, Epidemiologic Analysis with a Programmable Calculator.';
const MIETTINEN = "Miettinen's formula; " + RB;

const scaled = (inner: string, v: S): string =>
  mrow(mrow(mo('('), inner, mo(')')), mo('×'), mn(v.per_unit ?? ''));

const rg = (v: S, sign: string): string =>
  mrow(
    msup(
      mi('e'),
      mrow(
        mo('('),
        mi('ln'),
        mfrac(mn(v.o ?? ''), mn(v.pt ?? '')),
        mo(sign),
        mfrac(mn(v.z ?? ''), msqrt(mn(v.o ?? ''))),
        mo(')'),
      ),
    ),
    mo('×'),
    mn(v.per_unit ?? ''),
  );

const byarScaled = (v: S, sign: string): string =>
  mrow(
    mrow(mo('('), mn(v.x ?? ''), mo('+'), mfrac(mn('1'), mn('2')), mo(')')),
    mo('×'),
    msup(
      mrow(
        mo('('),
        mn('1'),
        mo('−'),
        mfrac(mn('1'), mrow(mn('9'), mo('('), mn(v.x ?? ''), mo('+'), mfrac(mn('1'), mn('2')), mo(')'))),
        mo(sign),
        mfrac(mn(v.z ?? ''), msqrt(mrow(mn('9'), mo('('), mn(v.x ?? ''), mo('+'), mfrac(mn('1'), mn('2')), mo(')')))),
        mo(')'),
      ),
      mn('3'),
    ),
    mo('×'),
    mn(v.scale ?? ''),
  );

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

export const PERSON_TIME: Record<string, FormulaTemplate> = {
  rate: {
    id: 'rate',
    words:
      'The rate is the number of cases divided by the person-time ' +
      'at risk, scaled to the reporting unit.',
    mathml: (v: S) =>
      math(
        'rate = cases/person-time × unit',
        eq(
          mi('r'),
          scaled(mfrac(mn(v.x ?? v.cases ?? ''), mn(v.pt ?? '')), v),
        ),
      ),
    compute: (v: V) => (((v.x ?? v.cases) ?? NaN) / (v.pt ?? NaN)) * (v.per_unit ?? NaN),
    source: RB,
  },
  rate_taylor_lower: {
    id: 'rate_taylor_lower',
    words:
      "Taylor's lower rate bound subtracts z times the square root " +
      'of the case count before dividing by the person-time, ' +
      'scaled to the reporting unit.',
    mathml: (v: S) =>
      math(
        'Taylor lower rate bound',
        eq(
          mi('L'),
          scaled(
            mfrac(mrow(mn(v.x ?? ''), mo('−'), mn(v.z ?? ''), msqrt(mn(v.x ?? ''))), mn(v.pt ?? '')),
            v,
          ),
        ),
      ),
    compute: (v: V) =>
      (((v.x ?? NaN) - (v.z ?? NaN) * Math.sqrt(v.x ?? NaN)) / (v.pt ?? NaN)) *
      (v.per_unit ?? NaN),
    source: 'Taylor series approximation; ' + RB,
  },
  rate_taylor_upper: {
    id: 'rate_taylor_upper',
    words:
      "Taylor's upper rate bound adds z times the square root of " +
      'the case count before dividing by the person-time, scaled ' +
      'to the reporting unit.',
    mathml: (v: S) =>
      math(
        'Taylor upper rate bound',
        eq(
          mi('U'),
          scaled(
            mfrac(mrow(mn(v.x ?? ''), mo('+'), mn(v.z ?? ''), msqrt(mn(v.x ?? ''))), mn(v.pt ?? '')),
            v,
          ),
        ),
      ),
    compute: (v: V) =>
      (((v.x ?? NaN) + (v.z ?? NaN) * Math.sqrt(v.x ?? NaN)) / (v.pt ?? NaN)) *
      (v.per_unit ?? NaN),
    source: 'Taylor series approximation; ' + RB,
  },
  pois_byar_scaled_lower: {
    id: 'pois_byar_scaled_lower',
    words:
      "Byar's lower bound on the case count, scaled to the " +
      'reporting unit of the rate.',
    mathml: (v: S) =>
      math("Byar lower bound, scaled", eq(mi('L'), byarScaled(v, '−'))),
    compute: (v: V) => {
      const [lo, , ap] = byarBase(v.x ?? NaN, v.z ?? NaN);
      return ap * lo ** 3 * (v.scale ?? NaN);
    },
    source: 'Byar approximation; ' + RB,
  },
  pois_byar_scaled_upper: {
    id: 'pois_byar_scaled_upper',
    words:
      "Byar's upper bound on the case count, scaled to the " +
      'reporting unit of the rate.',
    mathml: (v: S) =>
      math("Byar upper bound, scaled", eq(mi('U'), byarScaled(v, '+'))),
    compute: (v: V) => {
      const [, hi, ap] = byarBase(v.x ?? NaN, v.z ?? NaN);
      return ap * hi ** 3 * (v.scale ?? NaN);
    },
    source: 'Byar approximation; ' + RB,
  },
  rate_rg_lower: {
    id: 'rate_rg_lower',
    words:
      'The Rothman–Greenland lower rate bound is taken on the log ' +
      'scale, then scaled to the reporting unit.',
    mathml: (v: S) =>
      math('Rothman–Greenland lower rate bound', eq(mi('L'), rg(v, '−'))),
    compute: (v: V) =>
      Math.exp(Math.log((v.o ?? NaN) / (v.pt ?? NaN)) - (v.z ?? NaN) / Math.sqrt(v.o ?? NaN)) *
      (v.per_unit ?? NaN),
    source: RB,
  },
  rate_rg_upper: {
    id: 'rate_rg_upper',
    words:
      'The Rothman–Greenland upper rate bound is taken on the log ' +
      'scale, then scaled to the reporting unit.',
    mathml: (v: S) =>
      math('Rothman–Greenland upper rate bound', eq(mi('U'), rg(v, '+'))),
    compute: (v: V) =>
      Math.exp(Math.log((v.o ?? NaN) / (v.pt ?? NaN)) + (v.z ?? NaN) / Math.sqrt(v.o ?? NaN)) *
      (v.per_unit ?? NaN),
    source: RB,
  },
  pt_zscore: {
    id: 'pt_zscore',
    words:
      'The z score compares the observed exposed cases with their ' +
      'conditional expectation under the person-time split.',
    mathml: (v: S) =>
      math(
        'person-time z score',
        eq(
          mi('z'),
          mfrac(
            mrow(
              mn(v.a ?? ''),
              mo('−'),
              mfrac(
                mrow(mn(v.n1 ?? ''), mo('×'), mrow(mn(v.a ?? ''), mo('+'), mn(v.b ?? ''))),
                mrow(mn(v.n1 ?? ''), mo('+'), mn(v.n0 ?? '')),
              ),
            ),
            msqrt(
              mfrac(
                mrow(mrow(mn(v.a ?? ''), mo('+'), mn(v.b ?? '')), mn(v.n1 ?? ''), mn(v.n0 ?? '')),
                msup(mrow(mo('('), mn(v.n1 ?? ''), mo('+'), mn(v.n0 ?? ''), mo(')')), mn('2')),
              ),
            ),
          ),
        ),
      ),
    compute: (v: V) => {
      const m1 = (v.a ?? NaN) + (v.b ?? NaN);
      const n = (v.n1 ?? NaN) + (v.n0 ?? NaN);
      return (
        ((v.a ?? NaN) - ((v.n1 ?? NaN) * m1) / n) /
        Math.sqrt((m1 * (v.n1 ?? NaN) * (v.n0 ?? NaN)) / (n * n))
      );
    },
    source: MIETTINEN,
  },
  pt_zscore_pooled: {
    id: 'pt_zscore_pooled',
    words:
      'The pooled person-time z score divides the summed ' +
      'observed-minus-expected cases by the square root of the ' +
      'summed variance.',
    mathml: (v: S) =>
      math(
        'pooled person-time z score',
        eq(
          mi('z'),
          mfrac(
            mrow(mn(v.sum_a ?? ''), mo('−'), mn(v.sum_e ?? '')),
            msqrt(mn(v.var ?? '')),
          ),
        ),
      ),
    compute: (v: V) =>
      ((v.sum_a ?? NaN) - (v.sum_e ?? NaN)) / Math.sqrt(v.var ?? NaN),
    source: MIETTINEN,
  },
  rate_ratio: {
    id: 'rate_ratio',
    words:
      'The rate ratio is the second rate divided by the first ' +
      '(the unexposed divided by the exposed, as the reference ' +
      'implementations report it).',
    mathml: (v: S) =>
      math('rate ratio', eq(mi('RR'), mfrac(mn(v.ru ?? ''), mn(v.re ?? '')))),
    compute: (v: V) => (v.ru ?? NaN) / (v.re ?? NaN),
    source: RB,
  },
  rate_difference: {
    id: 'rate_difference',
    words:
      'The rate difference is the exposed rate minus the ' +
      'unexposed rate, scaled to the reporting unit.',
    mathml: (v: S) =>
      math(
        'rate difference',
        eq(
          mi('RD'),
          scaled(bin(mn(v.re ?? ''), '−', mn(v.ru ?? '')), v),
        ),
      ),
    compute: (v: V) => ((v.re ?? NaN) - (v.ru ?? NaN)) * (v.per_unit ?? NaN),
    source: RB,
  },
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

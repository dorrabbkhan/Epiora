// Poisson and SMR templates (v1.0 plan Task 16, spec §6): the
// standardized morbidity/mortality ratio, Byar's Poisson interval,
// the Rothman–Greenland log interval, the Ury–Wiggins approximations
// and the Vandenbroucke shortcut bounds.
import type { FormulaTemplate } from '../index';
import { math, mfrac, mi, mn, mo, mrow, msqrt, msup } from './ml';
import { byarBase } from './special';

type V = Record<string, number>;
type S = Record<string, string>;

const eq = (lhs: string, rhs: string): string => mrow(lhs, mo('='), rhs);

const BND = 'Breslow & Day, Statistical Methods in Cancer Research.';
const EI = 'Epi Info StatCalc documentation.';
const VB_Z = 1.959963984540054;

export const POISSON: Record<string, FormulaTemplate> = {
  smr: {
    id: 'smr',
    words:
      'The standardized morbidity or mortality ratio is the ' +
      'observed count divided by the expected count.',
    mathml: (v: S) =>
      math('SMR = observed/expected', eq(mi('SMR'), mfrac(mn(v.o ?? ''), mn(v.e ?? '')))),
    compute: (v: V) => (v.o ?? NaN) / (v.e ?? NaN),
    source: BND,
  },
  pois_byar_lower: {
    id: 'pois_byar_lower',
    words:
      "Byar's lower bound applies a normal approximation to the " +
      'cube-root transform of the Poisson count, then divides by ' +
      'the expected count.',
    mathml: (v: S) =>
      math(
        "Byar lower bound",
        eq(
          mi('L'),
          mfrac(
            mrow(
              mrow(mo('('), mn(v.x ?? ''), mo('+'), mfrac(mn('1'), mn('2')), mo(')')),
              msup(
                mrow(
                  mo('('),
                  mn('1'),
                  mo('−'),
                  mfrac(mn('1'), mrow(mn('9'), mo('('), mn(v.x ?? ''), mo('+'), mfrac(mn('1'), mn('2')), mo(')'))),
                  mo('−'),
                  mfrac(mn(v.z ?? ''), msqrt(mrow(mn('9'), mo('('), mn(v.x ?? ''), mo('+'), mfrac(mn('1'), mn('2')), mo(')')))),
                  mo(')'),
                ),
                mn('3'),
              ),
            ),
            mn(v.e ?? ''),
          ),
        ),
      ),
    compute: (v: V) => {
      const [lo, , ap] = byarBase(v.x ?? NaN, v.z ?? NaN);
      return (ap * lo ** 3) / (v.e ?? NaN);
    },
    source: 'Byar approximation; ' + BND,
  },
  pois_byar_upper: {
    id: 'pois_byar_upper',
    words:
      "Byar's upper bound applies a normal approximation to the " +
      'cube-root transform of the Poisson count, then divides by ' +
      'the expected count.',
    mathml: (v: S) =>
      math(
        "Byar upper bound",
        eq(
          mi('U'),
          mfrac(
            mrow(
              mrow(mo('('), mn(v.x ?? ''), mo('+'), mfrac(mn('1'), mn('2')), mo(')')),
              msup(
                mrow(
                  mo('('),
                  mn('1'),
                  mo('−'),
                  mfrac(mn('1'), mrow(mn('9'), mo('('), mn(v.x ?? ''), mo('+'), mfrac(mn('1'), mn('2')), mo(')'))),
                  mo('+'),
                  mfrac(mn(v.z ?? ''), msqrt(mrow(mn('9'), mo('('), mn(v.x ?? ''), mo('+'), mfrac(mn('1'), mn('2')), mo(')')))),
                  mo(')'),
                ),
                mn('3'),
              ),
            ),
            mn(v.e ?? ''),
          ),
        ),
      ),
    compute: (v: V) => {
      const [, hi, ap] = byarBase(v.x ?? NaN, v.z ?? NaN);
      return (ap * hi ** 3) / (v.e ?? NaN);
    },
    source: 'Byar approximation; ' + BND,
  },
  smr_rg_lower: {
    id: 'smr_rg_lower',
    words:
      'The Rothman–Greenland lower SMR bound is taken on the log ' +
      'scale: the log ratio minus z over the square root of the ' +
      'observed count, exponentiated.',
    mathml: (v: S) =>
      math(
        'Rothman–Greenland lower SMR bound',
        eq(
          mi('L'),
          msup(
            mi('e'),
            mrow(
              mo('('),
              mi('ln'),
              mfrac(mn(v.o ?? ''), mn(v.e ?? '')),
              mo('−'),
              mfrac(mn(v.z ?? ''), msqrt(mn(v.o ?? ''))),
              mo(')'),
            ),
          ),
        ),
      ),
    compute: (v: V) =>
      Math.exp(Math.log((v.o ?? NaN) / (v.e ?? NaN)) - (v.z ?? NaN) / Math.sqrt(v.o ?? NaN)),
    source: 'Rothman & Greenland; ' + BND,
  },
  smr_rg_upper: {
    id: 'smr_rg_upper',
    words:
      'The Rothman–Greenland upper SMR bound is taken on the log ' +
      'scale: the log ratio plus z over the square root of the ' +
      'observed count, exponentiated.',
    mathml: (v: S) =>
      math(
        'Rothman–Greenland upper SMR bound',
        eq(
          mi('U'),
          msup(
            mi('e'),
            mrow(
              mo('('),
              mi('ln'),
              mfrac(mn(v.o ?? ''), mn(v.e ?? '')),
              mo('+'),
              mfrac(mn(v.z ?? ''), msqrt(mn(v.o ?? ''))),
              mo(')'),
            ),
          ),
        ),
      ),
    compute: (v: V) =>
      Math.exp(Math.log((v.o ?? NaN) / (v.e ?? NaN)) + (v.z ?? NaN) / Math.sqrt(v.o ?? NaN)),
    source: 'Rothman & Greenland; ' + BND,
  },
  smr_ury_lower: {
    id: 'smr_ury_lower',
    words:
      'The Ury–Wiggins lower SMR bound subtracts z times the ' +
      'square root of the observed count plus a small ' +
      'level-dependent constant, then divides by the expected count.',
    mathml: (v: S) =>
      math(
        'Ury–Wiggins lower SMR bound',
        eq(
          mi('L'),
          mfrac(
            mrow(mn(v.o ?? ''), mo('−'), mn(v.z ?? ''), msqrt(mn(v.o ?? '')), mo('+'), mi('k')),
            mn(v.e ?? ''),
          ),
        ),
      ),
    compute: (v: V) => {
      const s = Math.sqrt(v.o ?? NaN);
      const k =
        v.conf === 99 ? 2 : v.conf === 95 ? 1 : v.conf === 90 ? 0.65 : NaN;
      return ((v.o ?? NaN) - (v.z ?? NaN) * s + k) / (v.e ?? NaN);
    },
    source: 'Ury & Wiggins; ' + EI,
  },
  smr_ury_upper: {
    id: 'smr_ury_upper',
    words:
      'The Ury–Wiggins upper SMR bound adds z times the square root ' +
      'of the observed count plus a level-dependent constant — and ' +
      'a small correction when the count is fifty or fewer — then ' +
      'divides by the expected count.',
    mathml: (v: S) =>
      math(
        'Ury–Wiggins upper SMR bound',
        eq(
          mi('U'),
          mfrac(
            mrow(mn(v.o ?? ''), mo('+'), mn(v.z ?? ''), msqrt(mn(v.o ?? '')), mo('+'), mi('k')),
            mn(v.e ?? ''),
          ),
        ),
      ),
    compute: (v: V) => {
      const s = Math.sqrt(v.o ?? NaN);
      const add = (v.o ?? NaN) > 50 ? 0 : 0.1;
      const k =
        v.conf === 99 ? 3 : v.conf === 95 ? 2 : v.conf === 90 ? 1.65 : NaN;
      return ((v.o ?? NaN) + (v.z ?? NaN) * s + k + add) / (v.e ?? NaN);
    },
    source: 'Ury & Wiggins; ' + EI,
  },
  smr_vb_lower: {
    id: 'smr_vb_lower',
    words:
      "Vandenbroucke's shortcut lower SMR bound squares the square " +
      'root of the observed count minus half the normal deviate, ' +
      'then divides by the expected count.',
    mathml: (v: S) =>
      math(
        'Vandenbroucke lower SMR bound',
        eq(
          mi('L'),
          mfrac(
            msup(mrow(mo('('), msqrt(mn(v.o ?? '')), mo('−'), mfrac(mn('1.9600'), mn('2')), mo(')')), mn('2')),
            mn(v.e ?? ''),
          ),
        ),
      ),
    compute: (v: V) => (Math.sqrt(v.o ?? NaN) - 0.5 * VB_Z) ** 2 / (v.e ?? NaN),
    source: 'Vandenbroucke shortcut; ' + EI,
  },
  smr_vb_upper: {
    id: 'smr_vb_upper',
    words:
      "Vandenbroucke's shortcut upper SMR bound squares the square " +
      'root of the observed count plus half the normal deviate, ' +
      'then divides by the expected count.',
    mathml: (v: S) =>
      math(
        'Vandenbroucke upper SMR bound',
        eq(
          mi('U'),
          mfrac(
            msup(mrow(mo('('), msqrt(mn(v.o ?? '')), mo('+'), mfrac(mn('1.9600'), mn('2')), mo(')')), mn('2')),
            mn(v.e ?? ''),
          ),
        ),
      ),
    compute: (v: V) => (Math.sqrt(v.o ?? NaN) + 0.5 * VB_Z) ** 2 / (v.e ?? NaN),
    source: 'Vandenbroucke shortcut; ' + EI,
  },
};

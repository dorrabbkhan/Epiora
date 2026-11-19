// Sample-size and power templates (v1.0 plan Task 16, spec §6):
// the OpenEpi proportion formula, cluster allocation, Kelsey and
// Fleiss unmatched designs, the matched-design group size, the
// mean-difference sample size, the power percentage, and the
// implied measures from proportions.
import type { FormulaTemplate } from '../index';
import { bin, math, mfrac, mi, mn, mo, mrow, msqrt, msub, msup } from './ml';
import { phi } from './special';

type V = Record<string, number>;
type S = Record<string, string>;

const eq = (lhs: string, rhs: string): string => mrow(lhs, mo('='), rhs);
const OPENEPI = 'OpenEpi documentation (sample size and power).';
const FLEISS = 'Fleiss, Statistical Methods for Rates and Proportions.';
const KELSEY = 'Kelsey et al., Methods in Observational Epidemiology.';

const f1 = (id: string, label: string, rhs: string): string =>
  math(label, eq(mi(id), rhs));

export const SAMPLE_SIZE: Record<string, FormulaTemplate> = {
  ss_prop_openepi: {
    id: 'ss_prop_openepi',
    words:
      'The OpenEpi sample size for a proportion applies the ' +
      'design effect and the finite-population correction to the ' +
      'simple random sample size.',
    mathml: (v: S) =>
      f1('n',
        'OpenEpi sample size for a proportion',
        mfrac(
          mrow(
            mn(v.deff ?? ''),
            mo('×'),
            mn(v.n_pop ?? ''),
            mo('×'),
            mn(v.p ?? ''),
            mo('×'),
            mrow(mo('('), mn('1'), mo('−'), mn(v.p ?? ''), mo(')')),
          ),
          mrow(
            mfrac(msup(mn(v.d ?? ''), mn('2')), msup(mn(v.za ?? ''), mn('2'))),
            mo('×'),
            mrow(mo('('), mn(v.n_pop ?? ''), mo('−'), mn('1'), mo(')')),
            mo('+'),
            mn(v.p ?? ''),
            mo('×'),
            mrow(mo('('), mn('1'), mo('−'), mn(v.p ?? ''), mo(')')),
          ),
        ),
      ),
    compute: (v: V) =>
      ((v.deff ?? NaN) * (v.n_pop ?? NaN) * (v.p ?? NaN) * (1 - (v.p ?? NaN))) /
      (((v.d ?? NaN) ** 2 / (v.za ?? NaN) ** 2) * ((v.n_pop ?? NaN) - 1) +
        (v.p ?? NaN) * (1 - (v.p ?? NaN))),
    source: OPENEPI,
  },
  ss_prop_cluster_real: {
    id: 'ss_prop_cluster_real',
    words:
      'The per-cluster sample size is the design-effect-adjusted ' +
      'sample size divided by the number of clusters.',
    mathml: (v: S) =>
      f1('n',
        'per-cluster sample size',
        mfrac(mrow(mn(v.deff ?? ''), mo('×'), mn(v.n_raw ?? '')), mn(v.clusters ?? '')),
      ),
    compute: (v: V) => ((v.deff ?? NaN) * (v.n_raw ?? NaN)) / (v.clusters ?? NaN),
    source: OPENEPI,
  },
  ss_prop_total: {
    id: 'ss_prop_total',
    words: 'The total sample is the cluster size times the number of clusters.',
    mathml: (v: S) =>
      f1('n',
        'total sample',
        mrow(mn(v.cluster_size ?? ''), mo('×'), mn(v.clusters ?? '')),
      ),
    compute: (v: V) => (v.cluster_size ?? NaN) * (v.clusters ?? NaN),
    source: OPENEPI,
  },
  ss_kelsey: {
    id: 'ss_kelsey',
    words:
      "Kelsey's unmatched case-control sample size uses the pooled " +
      'proportion and the design ratio.',
    mathml: (v: S) =>
      f1('n',
        'Kelsey sample size',
        mfrac(
          mrow(
            msup(
              mrow(mo('('), mn(v.za ?? ''), mo('+'), mn(v.zb ?? ''), mo(')')),
              mn('2'),
            ),
            msub(mi('p̄'), mi('')),
            msub(mi('q̄'), mi('')),
            mo('×'),
            mrow(mo('('), mn(v.r ?? ''), mo('+'), mn('1'), mo(')')),
          ),
          mrow(msup(mn(v.d ?? ''), mn('2')), mo('×'), mn(v.r ?? '')),
        ),
      ),
    compute: (v: V) => {
      const { p1, p2, r, za, zb } = v;
      if (p1 !== undefined && p2 !== undefined && r !== undefined &&
          za !== undefined && zb !== undefined) {
        if (p1 >= 0 && p1 <= 1 && p1 === p2) return Infinity;
        const p = 0 <= p1 && p1 <= 1 ? p1 : NaN;
        const pb = (p + r * p2) / (1 + r);
        const qb = 1 - pb;
        const d = p - p2;
        return ((za + zb) ** 2 * pb * qb * (r + 1)) / (d * d * r);
      }
      return NaN;
    },
    source: KELSEY,
  },
  ss_fleiss: {
    id: 'ss_fleiss',
    words:
      "Fleiss's unmatched case-control sample size adds a term for " +
      'the separate group variances under the alternative.',
    mathml: (v: S) =>
      f1('n',
        'Fleiss sample size',
        mfrac(
          msup(
            mrow(
              mo('('),
              mn(v.za ?? ''),
              mo('×'),
              msqrt(mrow(mo('('), mn(v.r ?? ''), mo('+'), mn('1'), mo(')'), msub(mi('p̄'), mi('')), msub(mi('q̄'), mi('')))),
              mo('+'),
              mn(v.zb ?? ''),
              mo('×'),
              msqrt(mrow(mn(v.r ?? ''), mn(v.p1 ?? ''), msub(mi('q'), mi('1')), mo('+'), mn(v.p2 ?? ''), msub(mi('q'), mi('2')))),
              mo(')'),
            ),
            mn('2'),
          ),
          mrow(mn(v.r ?? ''), mo('×'), msup(mn(v.d ?? ''), mn('2'))),
        ),
      ),
    compute: (v: V) => {
      const { p1, p2, r, za, zb } = v;
      if (p1 !== undefined && p2 !== undefined && r !== undefined &&
          za !== undefined && zb !== undefined) {
        if (p1 >= 0 && p1 <= 1 && p1 === p2) return Infinity;
        const p = 0 <= p1 && p1 <= 1 ? p1 : NaN;
        const pb = (p + r * p2) / (1 + r);
        const qb = 1 - pb;
        const d = p - p2;
        const f =
          za * Math.sqrt((r + 1) * pb * qb) +
          zb * Math.sqrt(r * p * (1 - p) + p2 * (1 - p2));
        return (f * f) / (r * d * d);
      }
      return NaN;
    },
    source: FLEISS,
  },
  ss_fleiss_cc: {
    id: 'ss_fleiss_cc',
    words:
      "Fleiss's continuity-corrected sample size inflates the " +
      'uncorrected value by the correction factor.',
    mathml: (v: S) =>
      f1('n',
        'Fleiss sample size (continuity corrected)',
        mfrac(
          mrow(
            msub(mi('n'), mi('F')),
            msup(
              mrow(
                mo('('),
                mn('1'),
                mo('+'),
                msqrt(
                  mrow(
                    mn('1'),
                    mo('+'),
                    mfrac(
                      mrow(mn('2'), mo('('), mn(v.r ?? ''), mo('+'), mn('1'), mo(')')),
                      mrow(msub(mi('n'), mi('F')), mn(v.r ?? ''), mo('|'), mn(v.d ?? ''), mo('|')),
                    ),
                  ),
                ),
                mo(')'),
              ),
              mn('2'),
            ),
          ),
          mn('4'),
        ),
      ),
    compute: (v: V) => {
      const { p1, p2, r, za, zb } = v;
      if (p1 !== undefined && p2 !== undefined && r !== undefined &&
          za !== undefined && zb !== undefined) {
        if (p1 >= 0 && p1 <= 1 && p1 === p2) return Infinity;
        const p = 0 <= p1 && p1 <= 1 ? p1 : NaN;
        const pb = (p + r * p2) / (1 + r);
        const qb = 1 - pb;
        const d = p - p2;
        const f =
          za * Math.sqrt((r + 1) * pb * qb) +
          zb * Math.sqrt(r * p * (1 - p) + p2 * (1 - p2));
        const vf = (f * f) / (r * d * d);
        return (vf * (1 + Math.sqrt(1 + (2 * (r + 1)) / (vf * r * Math.abs(d)))) ** 2) / 4;
      }
      return NaN;
    },
    source: FLEISS,
  },
  ss_group2_real: {
    id: 'ss_group2_real',
    words: 'The second group size is the first times the allocation ratio.',
    mathml: (v: S) =>
      f1('n', 'second group size', mrow(mn(v.vn ?? ''), mo('×'), mn(v.r ?? ''))),
    compute: (v: V) => (v.vn ?? NaN) * (v.r ?? NaN),
    source: KELSEY,
  },
  ss_mean_diff: {
    id: 'ss_mean_diff',
    words:
      'The sample size for a mean difference combines the two ' +
      'variances and the critical values over the squared ' +
      'detectable difference.',
    mathml: (v: S) =>
      f1('n',
        'sample size for a mean difference',
        mfrac(
          mrow(
            mrow(
              mo('('),
              msup(mn(v.s1 ?? ''), mn('2')),
              mo('+'),
              mfrac(msup(mn(v.s2 ?? ''), mn('2')), mn(v.ratio ?? '')),
              mo(')'),
            ),
            mo('×'),
            msup(mrow(mo('('), mn(v.za ?? ''), mo('+'), mn(v.zb ?? ''), mo(')')), mn('2')),
          ),
          msup(mn(v.diff ?? ''), mn('2')),
        ),
      ),
    compute: (v: V) =>
      (((v.s1 ?? NaN) ** 2 + (v.s2 ?? NaN) ** 2 / (v.ratio ?? NaN)) *
        ((v.za ?? NaN) + (v.zb ?? NaN)) ** 2) /
      (v.diff ?? NaN) ** 2,
    source: OPENEPI,
  },
  ss_mean_diff_n2: {
    id: 'ss_mean_diff_n2',
    words: 'The second group size is the first times the allocation ratio.',
    mathml: (v: S) =>
      f1('n', 'second group size', mrow(mn(v.n1 ?? ''), mo('×'), mn(v.ratio ?? ''))),
    compute: (v: V) => (v.n1 ?? NaN) * (v.ratio ?? NaN),
    source: OPENEPI,
  },
  power_pct: {
    id: 'power_pct',
    words:
      'The power is the standard normal cumulative probability at ' +
      'the computed z, reported as a percentage.',
    mathml: (v: S) =>
      f1('P',
        'power percentage',
        mrow(mrow(mo('('), mi('Φ'), mo('('), mn(v.powerz ?? ''), mo(')'), mo(')')), mo('×'), mn('100')),
      ),
    compute: (v: V) => phi(v.powerz ?? NaN) * 100,
    source: OPENEPI,
  },
  or_from_props: {
    id: 'or_from_props',
    words: 'The odds ratio implied by the two proportions.',
    mathml: (v: S) =>
      f1('OR',
        'odds ratio from proportions',
        mfrac(
          mrow(mn(v.p1 ?? ''), mo('('), mn('1'), mo('−'), mn(v.p2 ?? ''), mo(')')),
          mrow(mn(v.p2 ?? ''), mo('('), mn('1'), mo('−'), mn(v.p1 ?? ''), mo(')')),
        ),
      ),
    compute: (v: V) => ((v.p1 ?? NaN) * (1 - (v.p2 ?? NaN))) / ((v.p2 ?? NaN) * (1 - (v.p1 ?? NaN))),
    source: 'Definition of the odds ratio.',
  },
  risk_ratio_props: {
    id: 'risk_ratio_props',
    words: 'The risk ratio implied by the two proportions.',
    mathml: (v: S) =>
      f1('RR', 'risk ratio from proportions', mfrac(mn(v.p1 ?? ''), mn(v.p2 ?? ''))),
    compute: (v: V) => (v.p1 ?? NaN) / (v.p2 ?? NaN),
    source: 'Definition of the risk ratio.',
  },
  risk_diff_props: {
    id: 'risk_diff_props',
    words: 'The risk difference implied by the two proportions.',
    mathml: (v: S) =>
      f1('RD', 'risk difference from proportions', bin(mn(v.p1 ?? ''), '−', mn(v.p2 ?? ''))),
    compute: (v: V) => (v.p1 ?? NaN) - (v.p2 ?? NaN),
    source: 'Definition of the risk difference.',
  },
};

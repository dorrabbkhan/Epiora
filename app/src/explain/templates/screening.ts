// Screening-test measure templates (v1.0 plan Task 16, spec §6):
// likelihood ratios, the diagnostic odds ratio, Cohen's kappa, the
// entropy reduction from the test, the bias index, and the Simel
// level likelihood-ratio bounds.
import type { FormulaTemplate } from '../index';
import { bin, math, mfrac, mi, mn, mo, mrow, msup } from './ml';
import { entropy } from './special';

type V = Record<string, number>;
type S = Record<string, string>;

const eq = (lhs: string, rhs: string): string => mrow(lhs, mo('='), rhs);

const ALTMAN = 'Altman, Diagnostic Tests in Statistics and Epidemiology.';
const SIMEL = 'Simel, Samsa & Matchar, Journal of Clinical Epidemiology.';

export const SCREENING: Record<string, FormulaTemplate> = {
  lr_ratio: {
    id: 'lr_ratio',
    words:
      'The likelihood ratio is the probability of the test result ' +
      'among the diseased divided by its probability among the ' +
      'non-diseased.',
    mathml: (v: S) =>
      math(
        'likelihood ratio = p₁/p₂',
        eq(mi('LR'), mfrac(mn(v.p1 ?? ''), mn(v.p2 ?? ''))),
      ),
    compute: (v: V) => (v.p1 ?? NaN) / (v.p2 ?? NaN),
    source: ALTMAN,
  },
  diagnostic_odds: {
    id: 'diagnostic_odds',
    words:
      'The diagnostic odds ratio is the product of the true ' +
      'positives and true negatives over the product of the false ' +
      'positives and false negatives.',
    mathml: (v: S) =>
      math(
        'diagnostic odds ratio',
        eq(
          mi('DOR'),
          mfrac(
            mrow(mn(v.tp ?? ''), mo('×'), mn(v.tn ?? '')),
            mrow(mn(v.fp ?? ''), mo('×'), mn(v.fn_ ?? '')),
          ),
        ),
      ),
    compute: (v: V) =>
      ((v.tp ?? NaN) * (v.tn ?? NaN)) / ((v.fp ?? NaN) * (v.fn_ ?? NaN)),
    source: 'Glas et al., Journal of Clinical Epidemiology.',
  },
  kappa: {
    id: 'kappa',
    words:
      "Cohen's kappa is the observed agreement beyond chance " +
      'agreement, scaled by the maximum possible beyond-chance ' +
      'agreement.',
    mathml: (v: S) =>
      math(
        "Cohen's kappa",
        eq(
          mi('κ'),
          mfrac(
            mrow(mn(v.po ?? ''), mo('−'), mn(v.pe ?? '')),
            mrow(mn('1'), mo('−'), mn(v.pe ?? '')),
          ),
        ),
      ),
    compute: (v: V) => ((v.po ?? NaN) - (v.pe ?? NaN)) / (1 - (v.pe ?? NaN)),
    source: 'Cohen, Educational and Psychological Measurement.',
  },
  entropy_reduction: {
    id: 'entropy_reduction',
    words:
      'The entropy reduction compares the uncertainty about the ' +
      'disease state before the test with the uncertainty after a ' +
      'positive result — the difference of the two binary entropies.',
    mathml: (v: S) =>
      math(
        'entropy reduction = H(prev) − H(post)',
        eq(mi('ΔH'), bin(mrow(mi('H'), mrow(mo('('), mn(v.prev ?? ''), mo(')'))), '−', mrow(mi('H'), mrow(mo('('), mn(v.post ?? ''), mo(')'))))),
      ),
    compute: (v: V) => entropy(v.prev ?? NaN) - entropy(v.post ?? NaN),
    source: 'Information gain of a screening test; Epi Info StatCalc documentation.',
  },
  bias_index: {
    id: 'bias_index',
    words:
      'The bias index is the net bias toward false positives or ' +
      'false negatives, scaled by the total number tested.',
    mathml: (v: S) =>
      math(
        'bias index = (fp − fn)/n',
        eq(
          mi('BI'),
          mfrac(
            mrow(mn(v.fp ?? ''), mo('−'), mn(v.fn_ ?? '')),
            mn(v.n ?? ''),
          ),
        ),
      ),
    compute: (v: V) => ((v.fp ?? NaN) - (v.fn_ ?? NaN)) / (v.n ?? NaN),
    source: ALTMAN,
  },
  level_lr_lower: {
    id: 'level_lr_lower',
    words:
      'The lower bound of the level likelihood ratio interval is ' +
      'the estimate times the exponential of minus its half-width.',
    mathml: (v: S) =>
      math(
        'lower level LR bound',
        eq(
          mi('L'),
          mrow(
            mn(v.est ?? ''),
            mo('×'),
            msup(mi('e'), mrow(mo('('), mo('−'), mn(v.half ?? ''), mo(')'))),
          ),
        ),
      ),
    compute: (v: V) => (v.est ?? NaN) * Math.exp(-(v.half ?? NaN)),
    source: SIMEL,
  },
  level_lr_upper: {
    id: 'level_lr_upper',
    words:
      'The upper bound of the level likelihood ratio interval is ' +
      'the estimate times the exponential of its half-width.',
    mathml: (v: S) =>
      math(
        'upper level LR bound',
        eq(
          mi('U'),
          mrow(
            mn(v.est ?? ''),
            mo('×'),
            msup(mi('e'), mrow(mo('('), mn(v.half ?? ''), mo(')'))),
          ),
        ),
      ),
    compute: (v: V) => (v.est ?? NaN) * Math.exp(v.half ?? NaN),
    source: SIMEL,
  },
};

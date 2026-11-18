// Etiologic and prevented fraction templates (v1.0 plan Task 16,
// spec §6): the exposed/population fractions from risks, odds
// ratios and person-time rates, with their confidence bounds, all
// reported as percentages (Epi Info StatCalc formulas).
import type { FormulaTemplate } from '../index';
import { math, mfrac, mi, mn, mo, mrow, msqrt } from './ml';
import { clamp1 } from './special';

type V = Record<string, number>;
type S = Record<string, string>;

const eq = (lhs: string, rhs: string): string => mrow(lhs, mo('='), rhs);
const pct = (inner: string): string =>
  mrow(mrow(mo('('), inner, mo(')')), mo('×'), mn('100'));
const EI = 'Epi Info StatCalc documentation (etiologic fractions).';

const FRACTION_WORDS: Record<string, string> = {
  efp: 'The etiologic fraction in the population is the share of all cases attributable to the exposure.',
  efe: 'The etiologic fraction among the exposed is the share of exposed cases attributable to the exposure.',
  pfp: 'The prevented fraction in the population is the share of potential cases the exposure prevents.',
  pfe: 'The prevented fraction among the exposed is the share of expected cases the exposure prevents.',
};

const bound = (v: S, sign: string, base: string): string =>
  mrow(mn(v[base] ?? ''), mo(sign), mn(v.z ?? ''), mo('×'), msqrt(mn(v.var ?? '')));

const shareExpr = (v: S, rr: string): string =>
  mfrac(
    mrow(
      mn(v.share ?? ''),
      mo('('),
      mn(v[rr] ?? ''),
      mo('−'),
      mn('1'),
      mo(')'),
    ),
    mrow(
      mn(v.share ?? ''),
      mo('('),
      mn(v[rr] ?? ''),
      mo('−'),
      mn('1'),
      mo(')'),
      mo('+'),
      mn('1'),
    ),
  );

const invFrac = (v: S, efp: string): string =>
  mrow(mn('1'), mo('−'), mfrac(mn('1'), mrow(mn('1'), mo('−'), mn(v[efp] ?? ''))));

export const ETIOLOGIC: Record<string, FormulaTemplate> = {
  ef_risk_efp: {
    id: 'ef_risk_efp',
    words: FRACTION_WORDS.efp ?? '',
    mathml: (v: S) =>
      math(
        'EF percent = (r₀ − rᵤ)/r₀ × 100',
        eq(mi('EF%'), pct(mfrac(mrow(mn(v.ro ?? ''), mo('−'), mn(v.ru ?? '')), mn(v.ro ?? '')))),
      ),
    compute: (v: V) => clamp1(((v.ro ?? NaN) - (v.ru ?? NaN)) / (v.ro ?? NaN)) * 100,
    source: EI,
  },
  ef_risk_efp_lower: {
    id: 'ef_risk_efp_lower',
    words: 'The lower bound of the etiologic fraction in the population.',
    mathml: (v: S) =>
      math('EF lower bound', eq(mi('L'), pct(mrow(mi('max'), mo('('), bound(v, '−', 'efp'), mo(')'))))),
    compute: (v: V) => Math.max((v.efp ?? NaN) - (v.z ?? NaN) * Math.sqrt(v.var ?? NaN), -1) * 100,
    source: EI,
  },
  ef_risk_efp_upper: {
    id: 'ef_risk_efp_upper',
    words: 'The upper bound of the etiologic fraction in the population.',
    mathml: (v: S) =>
      math('EF upper bound', eq(mi('U'), pct(mrow(mi('min'), mo('('), bound(v, '+', 'efp'), mo(')'))))),
    compute: (v: V) => Math.min((v.efp ?? NaN) + (v.z ?? NaN) * Math.sqrt(v.var ?? NaN), 1) * 100,
    source: EI,
  },
  ef_risk_efe: {
    id: 'ef_risk_efe',
    words: FRACTION_WORDS.efe ?? '',
    mathml: (v: S) =>
      math(
        'EF among exposed = (RR − 1)/RR × 100',
        eq(
          mi('EF%'),
          pct(mfrac(mrow(mfrac(mn(v.re ?? ''), mn(v.ru ?? '')), mo('−'), mn('1')), mfrac(mn(v.re ?? ''), mn(v.ru ?? '')))),
        ),
      ),
    compute: (v: V) =>
      clamp1(((v.re ?? NaN) / (v.ru ?? NaN) - 1) / ((v.re ?? NaN) / (v.ru ?? NaN))) * 100,
    source: EI,
  },
  ef_risk_efe_lower: {
    id: 'ef_risk_efe_lower',
    words: 'The lower bound of the etiologic fraction among the exposed.',
    mathml: (v: S) =>
      math(
        'EFe lower bound',
        eq(mi('L'), pct(mfrac(mrow(mn(v.rr_lo ?? ''), mo('−'), mn('1')), mn(v.rr_lo ?? '')))),
      ),
    compute: (v: V) => clamp1(((v.rr_lo ?? NaN) - 1) / (v.rr_lo ?? NaN)) * 100,
    source: EI,
  },
  ef_risk_efe_upper: {
    id: 'ef_risk_efe_upper',
    words: 'The upper bound of the etiologic fraction among the exposed.',
    mathml: (v: S) =>
      math(
        'EFe upper bound',
        eq(mi('U'), pct(mfrac(mrow(mn(v.rr_hi ?? ''), mo('−'), mn('1')), mn(v.rr_hi ?? '')))),
      ),
    compute: (v: V) => clamp1(((v.rr_hi ?? NaN) - 1) / (v.rr_hi ?? NaN)) * 100,
    source: EI,
  },
  ef_risk_pfp: {
    id: 'ef_risk_pfp',
    words: FRACTION_WORDS.pfp ?? '',
    mathml: (v: S) =>
      math(
        'PF percent = (rᵤ − r₀)/rᵤ × 100',
        eq(mi('PF%'), pct(mfrac(mrow(mn(v.ru ?? ''), mo('−'), mn(v.ro ?? '')), mn(v.ru ?? '')))),
      ),
    compute: (v: V) => (((v.ru ?? NaN) - (v.ro ?? NaN)) / (v.ru ?? NaN)) * 100,
    source: EI,
  },
  ef_risk_pfp_lower: {
    id: 'ef_risk_pfp_lower',
    words: 'The lower bound of the prevented fraction in the population.',
    mathml: (v: S) =>
      math('PF lower bound', eq(mi('L'), pct(invFrac(v, 'efp_hi')))),
    compute: (v: V) => (1 - 1 / (1 - (v.efp_hi ?? NaN))) * 100,
    source: EI,
  },
  ef_risk_pfp_upper: {
    id: 'ef_risk_pfp_upper',
    words: 'The upper bound of the prevented fraction in the population.',
    mathml: (v: S) =>
      math('PF upper bound', eq(mi('U'), pct(invFrac(v, 'efp_lo')))),
    compute: (v: V) => (1 - 1 / (1 - (v.efp_lo ?? NaN))) * 100,
    source: EI,
  },
  ef_risk_pfe: {
    id: 'ef_risk_pfe',
    words: FRACTION_WORDS.pfe ?? '',
    mathml: (v: S) =>
      math(
        'PF among exposed = (rᵤ − rₑ)/rᵤ × 100',
        eq(mi('PF%'), pct(mfrac(mrow(mn(v.ru ?? ''), mo('−'), mn(v.re ?? '')), mn(v.ru ?? '')))),
      ),
    compute: (v: V) => (((v.ru ?? NaN) - (v.re ?? NaN)) / (v.ru ?? NaN)) * 100,
    source: EI,
  },
  ef_risk_pfe_lower: {
    id: 'ef_risk_pfe_lower',
    words: 'The lower bound of the prevented fraction among the exposed.',
    mathml: (v: S) =>
      math('PFe lower bound', eq(mi('L'), pct(mrow(mn('1'), mo('−'), mn(v.rr_hi ?? ''))))),
    compute: (v: V) => (1 - (v.rr_hi ?? NaN)) * 100,
    source: EI,
  },
  ef_risk_pfe_upper: {
    id: 'ef_risk_pfe_upper',
    words: 'The upper bound of the prevented fraction among the exposed.',
    mathml: (v: S) =>
      math('PFe upper bound', eq(mi('U'), pct(mrow(mn('1'), mo('−'), mn(v.rr_lo ?? ''))))),
    compute: (v: V) => (1 - (v.rr_lo ?? NaN)) * 100,
    source: EI,
  },
  ef_or_efpor: {
    id: 'ef_or_efpor',
    words: FRACTION_WORDS.efp ?? '',
    mathml: (v: S) =>
      math('EF from OR', eq(mi('EF%'), pct(shareExpr(v, 'od')))),
    compute: (v: V) =>
      clamp1(
        ((v.share ?? NaN) * ((v.od ?? NaN) - 1)) /
          ((v.share ?? NaN) * ((v.od ?? NaN) - 1) + 1),
      ) * 100,
    source: EI,
  },
  ef_or_efpor_lower: {
    id: 'ef_or_efpor_lower',
    words: 'The lower bound of the etiologic fraction in the population from the odds ratio.',
    mathml: (v: S) =>
      math('EF lower bound', eq(mi('L'), pct(mrow(mi('max'), mo('('), bound(v, '−', 'efpor'), mo(')'))))),
    compute: (v: V) => Math.max((v.efpor ?? NaN) - (v.z ?? NaN) * Math.sqrt(v.var ?? NaN), -1) * 100,
    source: EI,
  },
  ef_or_efpor_upper: {
    id: 'ef_or_efpor_upper',
    words: 'The upper bound of the etiologic fraction in the population from the odds ratio.',
    mathml: (v: S) =>
      math('EF upper bound', eq(mi('U'), pct(mrow(mi('min'), mo('('), bound(v, '+', 'efpor'), mo(')'))))),
    compute: (v: V) => Math.min((v.efpor ?? NaN) + (v.z ?? NaN) * Math.sqrt(v.var ?? NaN), 1) * 100,
    source: EI,
  },
  ef_or_efeor: {
    id: 'ef_or_efeor',
    words: FRACTION_WORDS.efe ?? '',
    mathml: (v: S) =>
      math(
        'EFe from OR = (OR − 1)/OR × 100',
        eq(mi('EF%'), pct(mfrac(mrow(mn(v.od ?? ''), mo('−'), mn('1')), mn(v.od ?? '')))),
      ),
    compute: (v: V) => clamp1(((v.od ?? NaN) - 1) / (v.od ?? NaN)) * 100,
    source: EI,
  },
  ef_or_efeor_lower: {
    id: 'ef_or_efeor_lower',
    words: 'The lower bound of the etiologic fraction among the exposed from the odds ratio.',
    mathml: (v: S) =>
      math('EFe lower bound', eq(mi('L'), pct(mfrac(mrow(mn(v.od_lo ?? ''), mo('−'), mn('1')), mn(v.od_lo ?? ''))))),
    compute: (v: V) => clamp1(((v.od_lo ?? NaN) - 1) / (v.od_lo ?? NaN)) * 100,
    source: EI,
  },
  ef_or_efeor_upper: {
    id: 'ef_or_efeor_upper',
    words: 'The upper bound of the etiologic fraction among the exposed from the odds ratio.',
    mathml: (v: S) =>
      math('EFe upper bound', eq(mi('U'), pct(mfrac(mrow(mn(v.od_hi ?? ''), mo('−'), mn('1')), mn(v.od_hi ?? ''))))),
    compute: (v: V) => clamp1(((v.od_hi ?? NaN) - 1) / (v.od_hi ?? NaN)) * 100,
    source: EI,
  },
  ef_or_pfpor: {
    id: 'ef_or_pfpor',
    words: FRACTION_WORDS.pfp ?? '',
    mathml: (v: S) =>
      math(
        'PF from OR',
        eq(mi('PF%'), pct(mrow(mn(v.share ?? ''), mo('('), mn('1'), mo('−'), mn(v.od ?? ''), mo(')')))),
      ),
    compute: (v: V) => (v.share ?? NaN) * (1 - (v.od ?? NaN)) * 100,
    source: EI,
  },
  ef_or_pfpor_lower: {
    id: 'ef_or_pfpor_lower',
    words: 'The lower bound of the prevented fraction in the population from the odds ratio.',
    mathml: (v: S) => math('PF lower bound', eq(mi('L'), pct(invFrac(v, 'hi')))),
    compute: (v: V) => (1 - 1 / (1 - (v.hi ?? NaN))) * 100,
    source: EI,
  },
  ef_or_pfpor_upper: {
    id: 'ef_or_pfpor_upper',
    words: 'The upper bound of the prevented fraction in the population from the odds ratio.',
    mathml: (v: S) => math('PF upper bound', eq(mi('U'), pct(invFrac(v, 'lo')))),
    compute: (v: V) => (1 - 1 / (1 - (v.lo ?? NaN))) * 100,
    source: EI,
  },
  ef_or_pfeor: {
    id: 'ef_or_pfeor',
    words: FRACTION_WORDS.pfe ?? '',
    mathml: (v: S) =>
      math('PFe from OR', eq(mi('PF%'), pct(mrow(mn('1'), mo('−'), mn(v.od ?? ''))))),
    compute: (v: V) => (1 - (v.od ?? NaN)) * 100,
    source: EI,
  },
  ef_or_pfeor_lower: {
    id: 'ef_or_pfeor_lower',
    words: 'The lower bound of the prevented fraction among the exposed from the odds ratio.',
    mathml: (v: S) => math('PFe lower bound', eq(mi('L'), pct(mrow(mn('1'), mo('−'), mn(v.od_hi ?? ''))))),
    compute: (v: V) => (1 - (v.od_hi ?? NaN)) * 100,
    source: EI,
  },
  ef_or_pfeor_upper: {
    id: 'ef_or_pfeor_upper',
    words: 'The upper bound of the prevented fraction among the exposed from the odds ratio.',
    mathml: (v: S) => math('PFe upper bound', eq(mi('U'), pct(mrow(mn('1'), mo('−'), mn(v.od_lo ?? ''))))),
    compute: (v: V) => (1 - (v.od_lo ?? NaN)) * 100,
    source: EI,
  },
  ef_pt_efp: {
    id: 'ef_pt_efp',
    words: FRACTION_WORDS.efp ?? '',
    mathml: (v: S) =>
      math('EF percent (person-time)', eq(mi('EF%'), pct(mfrac(mrow(mn(v.ro ?? ''), mo('−'), mn(v.ru ?? '')), mn(v.ro ?? ''))))),
    compute: (v: V) => clamp1(((v.ro ?? NaN) - (v.ru ?? NaN)) / (v.ro ?? NaN)) * 100,
    source: EI,
  },
  ef_pt_efp_lower: {
    id: 'ef_pt_efp_lower',
    words: 'The lower bound of the person-time etiologic fraction in the population.',
    mathml: (v: S) => math('EF lower bound', eq(mi('L'), pct(shareExpr(v, 'rr_lo')))),
    compute: (v: V) =>
      ((v.share ?? NaN) * ((v.rr_lo ?? NaN) - 1)) /
        ((v.share ?? NaN) * ((v.rr_lo ?? NaN) - 1) + 1) * 100,
    source: EI,
  },
  ef_pt_efp_upper: {
    id: 'ef_pt_efp_upper',
    words: 'The upper bound of the person-time etiologic fraction in the population.',
    mathml: (v: S) => math('EF upper bound', eq(mi('U'), pct(shareExpr(v, 'rr_hi')))),
    compute: (v: V) =>
      ((v.share ?? NaN) * ((v.rr_hi ?? NaN) - 1)) /
        ((v.share ?? NaN) * ((v.rr_hi ?? NaN) - 1) + 1) * 100,
    source: EI,
  },
  ef_pt_efe: {
    id: 'ef_pt_efe',
    words: FRACTION_WORDS.efe ?? '',
    mathml: (v: S) =>
      math('EFe (person-time)', eq(mi('EF%'), pct(mfrac(mrow(mn(v.rr ?? ''), mo('−'), mn('1')), mn(v.rr ?? ''))))),
    compute: (v: V) => clamp1(((v.rr ?? NaN) - 1) / (v.rr ?? NaN)) * 100,
    source: EI,
  },
  ef_pt_efe_lower: {
    id: 'ef_pt_efe_lower',
    words: 'The lower bound of the person-time etiologic fraction among the exposed.',
    mathml: (v: S) =>
      math('EFe lower bound', eq(mi('L'), pct(mfrac(mrow(mn(v.rr_lo ?? ''), mo('−'), mn('1')), mn(v.rr_lo ?? ''))))),
    compute: (v: V) => (((v.rr_lo ?? NaN) - 1) / (v.rr_lo ?? NaN)) * 100,
    source: EI,
  },
  ef_pt_efe_upper: {
    id: 'ef_pt_efe_upper',
    words: 'The upper bound of the person-time etiologic fraction among the exposed.',
    mathml: (v: S) =>
      math('EFe upper bound', eq(mi('U'), pct(mfrac(mrow(mn(v.rr_hi ?? ''), mo('−'), mn('1')), mn(v.rr_hi ?? ''))))),
    compute: (v: V) => (((v.rr_hi ?? NaN) - 1) / (v.rr_hi ?? NaN)) * 100,
    source: EI,
  },
  ef_pt_pfp: {
    id: 'ef_pt_pfp',
    words: FRACTION_WORDS.pfp ?? '',
    mathml: (v: S) =>
      math('PF percent (person-time)', eq(mi('PF%'), pct(mfrac(mrow(mn(v.ru ?? ''), mo('−'), mn(v.ro ?? '')), mn(v.ru ?? ''))))),
    compute: (v: V) => (((v.ru ?? NaN) - (v.ro ?? NaN)) / (v.ru ?? NaN)) * 100,
    source: EI,
  },
  ef_pt_pfp_lower: {
    id: 'ef_pt_pfp_lower',
    words: 'The lower bound of the person-time prevented fraction in the population.',
    mathml: (v: S) => math('PF lower bound', eq(mi('L'), pct(invFrac(v, 'efp_hi')))),
    compute: (v: V) => (1 - 1 / (1 - (v.efp_hi ?? NaN))) * 100,
    source: EI,
  },
  ef_pt_pfp_upper: {
    id: 'ef_pt_pfp_upper',
    words: 'The upper bound of the person-time prevented fraction in the population.',
    mathml: (v: S) => math('PF upper bound', eq(mi('U'), pct(invFrac(v, 'efp_lo')))),
    compute: (v: V) => (1 - 1 / (1 - (v.efp_lo ?? NaN))) * 100,
    source: EI,
  },
  ef_pt_pfe: {
    id: 'ef_pt_pfe',
    words: FRACTION_WORDS.pfe ?? '',
    mathml: (v: S) =>
      math('PFe (person-time)', eq(mi('PF%'), pct(mfrac(mrow(mn(v.ru ?? ''), mo('−'), mn(v.re ?? '')), mn(v.ru ?? ''))))),
    compute: (v: V) => (((v.ru ?? NaN) - (v.re ?? NaN)) / (v.ru ?? NaN)) * 100,
    source: EI,
  },
  ef_pt_pfe_lower: {
    id: 'ef_pt_pfe_lower',
    words: 'The lower bound of the person-time prevented fraction among the exposed.',
    mathml: (v: S) => math('PFe lower bound', eq(mi('L'), pct(mrow(mn('1'), mo('−'), mn(v.rr_hi ?? ''))))),
    compute: (v: V) => (1 - (v.rr_hi ?? NaN)) * 100,
    source: EI,
  },
  ef_pt_pfe_upper: {
    id: 'ef_pt_pfe_upper',
    words: 'The upper bound of the person-time prevented fraction among the exposed.',
    mathml: (v: S) => math('PFe upper bound', eq(mi('U'), pct(mrow(mn('1'), mo('−'), mn(v.rr_lo ?? ''))))),
    compute: (v: V) => (1 - (v.rr_lo ?? NaN)) * 100,
    source: EI,
  },
};

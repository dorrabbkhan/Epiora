// Explain template registry (v1.0 plan Task 16, spec §6): one
// `FormulaTemplate` per engine formula id in
// `crates/epiora-core/src/explain/formulas.rs`. `TEMPLATES` merges
// the family modules under `templates/`; the coverage test proves
// the merge matches `FORMULAS` exactly, and the guard test proves
// every closed-form `compute` reproduces the engine result within
// the statistic's tolerance class.
import { BINOMIAL } from './templates/binomial';
import { CHI_SQUARE } from './templates/chiSquare';
import { CHI_SQUARE_POOLED } from './templates/chiSquarePooled';
import { CONTINUOUS } from './templates/continuous';
import { ETIOLOGIC } from './templates/etiologic';
import { METHODS } from './templates/methods';
import { PERSON_TIME } from './templates/personTime';
import { PERSON_TIME_ADJUSTED } from './templates/personTimeAdjusted';
import { POISSON } from './templates/poisson';
import { PROPORTIONS } from './templates/proportions';
import { SAMPLE_SIZE } from './templates/sampleSize';
import { SCREENING } from './templates/screening';
import { SHARED } from './templates/shared';
import { TAILS } from './templates/tails';
import { TWO_BY_TWO } from './templates/twoByTwo';
import { WALD } from './templates/wald';

/** One statistic a result row can explain — the path is concrete
 *  (`two_by_two.odds_ratio.ci_95_wald.lower`); the label is the
 *  picker's digit-free display name. */
export interface ExplainStat {
  path: string;
  label: string;
}

export interface FormulaTemplate {
  /** Engine formula id (`FORMULAS`). */
  id: string;
  /** Plain-language description of the formula. */
  words: string;
  /** Substituted MathML (`<math>` element string); '' for method ids. */
  mathml: (v: Record<string, string>) => string;
  /** Re-evaluate the formula from the trace's numeric values. */
  compute: (v: Record<string, number>) => number;
  /** Textbook or reference-documentation citation. */
  source: string;
}

export const TEMPLATES: Record<string, FormulaTemplate> = {
  ...SHARED,
  ...CHI_SQUARE,
  ...CHI_SQUARE_POOLED,
  ...TAILS,
  ...WALD,
  ...TWO_BY_TWO,
  ...PROPORTIONS,
  ...BINOMIAL,
  ...SCREENING,
  ...POISSON,
  ...PERSON_TIME,
  ...PERSON_TIME_ADJUSTED,
  ...CONTINUOUS,
  ...ETIOLOGIC,
  ...SAMPLE_SIZE,
  ...METHODS,
};

/** The template for an engine formula id, or `undefined`. */
export function templateFor(id: string): FormulaTemplate | undefined {
  return TEMPLATES[id];
}

/** Panel strings — shared by the calculator and Analysis explain
 *  dialogs so wording stays identical across surfaces (spec §6). */
export const et = {
  explain: 'Explain',
  explainTitle: (label: string): string => `Explain — ${label}`,
  loading: 'Loading the trace…',
  pendingReview: 'Explain text is pending adviser review.',
  howComputed: 'How it is computed',
  showMaths: 'Show the maths',
  inputs: 'Inputs and result',
  inputCol: 'Input',
  valueCol: 'Value',
  resultCol: 'Result',
  source: 'Source',
  statistic: 'Statistic',
  error: 'The trace could not be computed.',
} as const;

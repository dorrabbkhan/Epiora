// "Which calculator?" guide (v1.0 plan Task 23, spec §6.1): the
// question flow as data — a tree with no cycles whose leaves are
// registry calculator ids, each optionally carrying `prefill` inputs
// the calculator's `decodeInputs` accepts with no errors. The unit
// test (`guide.test.ts`) proves reachability, acyclicity and clean
// prefills against the wasm registry itself.
//
// Each answered option becomes one letter of the `?path=a.b.c` link
// state (a = first option), so a guide state can be linked.

export interface GuideQuestion {
  id: string;
  question: string;
  /** One line on why the distinction matters. */
  why: string;
  options: { label: string; next: string }[];
}

export interface GuideLeaf {
  id: string;
  /** Registry calculator id. */
  calculator: string;
  /** Inputs written into the calculator link (`study_type`, `design`). */
  prefill?: Record<string, string>;
}

export type GuideNode = GuideQuestion | GuideLeaf;

export function isLeaf(node: GuideNode): node is GuideLeaf {
  return 'calculator' in node;
}

export const GUIDE_ROOT = 'what_you_have';

export const GUIDE: Record<string, GuideNode> = {
  what_you_have: {
    id: 'what_you_have',
    question: 'What do you want to work out?',
    why: 'The right calculator depends on the data you have — counts, person-time, summary statistics, or a study you are still planning.',
    options: [
      { label: 'I have counts or a table', next: 'counts_shape' },
      { label: 'I have person-time (rates)', next: 'person_time' },
      { label: 'I have means or summary statistics', next: 'summaries' },
      { label: 'I am planning a study', next: 'planning' },
      { label: 'I need random numbers', next: 'leaf_random' },
    ],
  },
  counts_shape: {
    id: 'counts_shape',
    question: 'How are the counts arranged?',
    why: 'A single proportion, a 2×2 table and a larger table each use a different method.',
    options: [
      { label: 'One proportion — x out of n', next: 'one_proportion' },
      {
        label: 'A 2×2 table — exposure by outcome',
        next: 'two_by_two_design',
      },
      { label: 'A larger table — rows by columns', next: 'bigger_table' },
      { label: 'Matched or paired data', next: 'leaf_matched' },
      { label: 'A screening or diagnostic test', next: 'leaf_screening' },
    ],
  },
  one_proportion: {
    id: 'one_proportion',
    question: 'What do you want to do with the count?',
    why: 'Estimating a proportion, comparing it to a fixed standard, and comparing a count to an expected number use different methods.',
    options: [
      {
        label: 'Estimate the proportion and its confidence interval',
        next: 'leaf_proportion',
      },
      {
        label: 'Compare it to a fixed standard (binomial test)',
        next: 'leaf_binomial',
      },
      {
        label: 'Compare observed to expected — an SMR',
        next: 'leaf_smr',
      },
      {
        label: 'A rare event count against a Poisson standard',
        next: 'leaf_poisson',
      },
    ],
  },
  two_by_two_design: {
    id: 'two_by_two_design',
    question: 'What is the study design?',
    why: 'The calculator computes risk ratios for cohort designs and odds ratios for case-control designs; Epi Info stratifies several 2×2 tables in its own form.',
    options: [
      {
        label: 'Cohort, cross-sectional or trial',
        next: 'leaf_two_by_two_cohort',
      },
      { label: 'Case-control', next: 'leaf_two_by_two_cc' },
      {
        label: 'Several 2×2 tables stratified together (Epi Info 2×N)',
        next: 'leaf_two_by_n',
      },
    ],
  },
  bigger_table: {
    id: 'bigger_table',
    question: 'Is the exposure ordered — a dose-response?',
    why: 'An ordered exposure supports the chi-square test for trend; an unordered table uses the plain chi-square test.',
    options: [
      { label: 'No — an unordered R×C table', next: 'leaf_r_by_c' },
      { label: 'Yes — exposure rises by dose', next: 'leaf_dose_response' },
    ],
  },
  person_time: {
    id: 'person_time',
    question: 'How many rates are you comparing?',
    why: 'Person-time data answer rate questions; an observed count against expected is a standardized ratio.',
    options: [
      { label: 'One rate and its confidence interval', next: 'leaf_rate_one' },
      { label: 'Two rates — compare them', next: 'leaf_rate_compare' },
      {
        label: 'Observed events against expected (SMR)',
        next: 'leaf_smr',
      },
    ],
  },
  summaries: {
    id: 'summaries',
    question: 'What do you want to estimate or compare?',
    why: 'Confidence intervals and group comparisons from summary statistics use different calculators.',
    options: [
      { label: 'A confidence interval for one mean', next: 'leaf_mean_ci' },
      {
        label: 'A confidence interval for a median or percentile',
        next: 'leaf_median_ci',
      },
      { label: 'Compare two means — a t test', next: 'leaf_t_test' },
      { label: 'Compare three or more means — ANOVA', next: 'leaf_anova' },
    ],
  },
  planning: {
    id: 'planning',
    question: 'Sample size or power?',
    why: 'Sample size tells you how many subjects to enrol; power tells you whether a planned study can detect the effect.',
    options: [
      { label: 'Sample size', next: 'ss_design' },
      { label: 'Power', next: 'power_design' },
    ],
  },
  ss_design: {
    id: 'ss_design',
    question: 'What is the study design?',
    why: 'The formula depends on the design — survey, case-control, cohort or a comparison of means.',
    options: [
      {
        label: 'A population survey or single proportion',
        next: 'leaf_ss_proportion',
      },
      {
        label: 'An unmatched case-control study',
        next: 'leaf_ss_unmatched_cc',
      },
      {
        label: 'A cohort, cross-sectional or randomized trial',
        next: 'leaf_ss_cohort',
      },
      { label: 'A comparison of two means', next: 'leaf_ss_mean_diff' },
    ],
  },
  power_design: {
    id: 'power_design',
    question: 'What is the study design?',
    why: 'Power is computed differently for case-control, cohort and mean-difference studies.',
    options: [
      {
        label: 'An unmatched case-control study',
        next: 'leaf_power_unmatched_cc',
      },
      {
        label: 'A cohort study or trial',
        next: 'power_cohort_design',
      },
      { label: 'A comparison of two means', next: 'leaf_power_mean_diff' },
    ],
  },
  power_cohort_design: {
    id: 'power_cohort_design',
    question: 'Which cohort design is it?',
    why: 'The power calculation is the same formula, but the design goes into the link so the result is labelled correctly.',
    options: [
      { label: 'Cohort', next: 'leaf_power_cohort' },
      { label: 'Randomized trial', next: 'leaf_power_cohort_rct' },
      { label: 'Cross-sectional', next: 'leaf_power_cohort_xs' },
    ],
  },
  leaf_proportion: { id: 'leaf_proportion', calculator: 'proportion' },
  leaf_binomial: { id: 'leaf_binomial', calculator: 'binomial' },
  leaf_poisson: { id: 'leaf_poisson', calculator: 'poisson' },
  leaf_smr: { id: 'leaf_smr', calculator: 'smr' },
  leaf_two_by_two_cohort: {
    id: 'leaf_two_by_two_cohort',
    calculator: 'two_by_two',
    prefill: { study_type: 'cohort' },
  },
  leaf_two_by_two_cc: {
    id: 'leaf_two_by_two_cc',
    calculator: 'two_by_two',
    prefill: { study_type: 'case_control' },
  },
  leaf_two_by_n: { id: 'leaf_two_by_n', calculator: 'two_by_n' },
  leaf_matched: {
    id: 'leaf_matched',
    calculator: 'matched_case_control',
  },
  leaf_screening: { id: 'leaf_screening', calculator: 'screening' },
  leaf_r_by_c: { id: 'leaf_r_by_c', calculator: 'r_by_c' },
  leaf_dose_response: {
    id: 'leaf_dose_response',
    calculator: 'dose_response',
  },
  leaf_rate_one: { id: 'leaf_rate_one', calculator: 'rate_one' },
  leaf_rate_compare: { id: 'leaf_rate_compare', calculator: 'rate_compare' },
  leaf_mean_ci: { id: 'leaf_mean_ci', calculator: 'mean_ci' },
  leaf_median_ci: { id: 'leaf_median_ci', calculator: 'median_ci' },
  leaf_t_test: { id: 'leaf_t_test', calculator: 't_test_summary' },
  leaf_anova: { id: 'leaf_anova', calculator: 'anova_summary' },
  leaf_ss_proportion: {
    id: 'leaf_ss_proportion',
    calculator: 'ss_proportion',
  },
  leaf_ss_unmatched_cc: {
    id: 'leaf_ss_unmatched_cc',
    calculator: 'ss_unmatched_cc',
  },
  leaf_ss_cohort: { id: 'leaf_ss_cohort', calculator: 'ss_cohort' },
  leaf_ss_mean_diff: {
    id: 'leaf_ss_mean_diff',
    calculator: 'ss_mean_difference',
  },
  leaf_power_unmatched_cc: {
    id: 'leaf_power_unmatched_cc',
    calculator: 'power_unmatched_cc',
  },
  leaf_power_cohort: {
    id: 'leaf_power_cohort',
    calculator: 'power_cohort',
    prefill: { design: 'cohort' },
  },
  leaf_power_cohort_rct: {
    id: 'leaf_power_cohort_rct',
    calculator: 'power_cohort',
    prefill: { design: 'rct' },
  },
  leaf_power_cohort_xs: {
    id: 'leaf_power_cohort_xs',
    calculator: 'power_cohort',
    prefill: { design: 'cross_sectional' },
  },
  leaf_power_mean_diff: {
    id: 'leaf_power_mean_diff',
    calculator: 'power_mean_difference',
  },
  leaf_random: { id: 'leaf_random', calculator: 'random_numbers' },
};

/** One step already answered: the question node and the option index
 *  the path letter chose. */
export interface GuideCrumb {
  node: GuideQuestion;
  option: number;
}

export interface GuideState {
  /** The node the path lands on — a question to show or a leaf. */
  node: GuideNode;
  /** Every answered step, root first — drives the back link. */
  trail: GuideCrumb[];
}

/** Letter → option index: 'a' is the first option, 'b' the second. */
function optionIndex(letter: string): number {
  return letter.length === 1 ? letter.charCodeAt(0) - 97 : -1;
}

/** Option index → path letter. */
export function optionLetter(index: number): string {
  return String.fromCharCode(97 + index);
}

/** Walks `?path=a.b.c` from the root, stopping at the last valid
 *  node — a bad letter or a step past a leaf just ends the walk there
 *  (the path is link state, never trusted). */
export function resolveGuide(path: string): GuideState {
  const trail: GuideCrumb[] = [];
  let node: GuideNode = GUIDE[GUIDE_ROOT] as GuideNode;
  const steps = path.split('.').filter((s) => s !== '');
  for (const step of steps) {
    if (isLeaf(node)) break;
    const index = optionIndex(step);
    const option = node.options[index];
    if (option === undefined) break;
    const next = GUIDE[option.next];
    if (next === undefined) break;
    trail.push({ node, option: index });
    node = next;
  }
  return { node, trail };
}

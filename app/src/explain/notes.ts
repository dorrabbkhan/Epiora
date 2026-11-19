// Assumption-note wording (v1.0 plan Task 23, spec §6.1): the
// engine emits notes as `{code, statistics, values, suggests}`; the
// UI must never show the raw code, so every code the engine can emit
// (its `NOTE_CODES` const, parsed by `notes.test.ts`) maps to a
// plain-language text here — `words` for the line under the result
// table, `rule` for explain's "why this note fired" and `source` for
// the literature behind the threshold. `NOTE_VALUE` names the
// engine-computed trigger keys; unknown keys humanize so a new rule
// still renders. All wording is pending adviser review.

export interface NoteText {
  /** The plain-language sentence shown beneath the table. */
  words: string;
  /** The rule in words, shown in explain. */
  rule: string;
  /** The literature source for the threshold, shown in explain. */
  source: string;
}

export const NOTE_TEXT: Record<string, NoteText> = {
  expected_count_below_5: {
    words:
      'An expected cell count is below 5, so the chi-square p-values may be unreliable.',
    rule:
      'Fires when at least one expected cell count in the table is below 5 — the chi-square approximation then needs care.',
    source: 'Cochran (1954), Biometrics 10(2): 417–451.',
  },
  zero_cell: {
    words:
      'A table cell is zero, so the Wald confidence intervals for the odds and risk ratios are unreliable or undefined.',
    rule:
      'Fires when an observed cell of the table is zero — the 1/cell terms of the Wald standard errors then fail.',
    source: 'Agresti (2003), Categorical Data Analysis, 2nd ed., §2.3.3.',
  },
  normal_approx_small: {
    words:
      'n·p or n·(1−p) is below 5, so the Wald interval may have poor coverage.',
    rule:
      'Fires when the smaller of n·p and n·(1−p) is below 5 — the classical rule of thumb for the normal-approximation interval.',
    source: 'Brown, Cai & DasGupta (2001), Statistical Science 16(2): 101–133.',
  },
  unequal_variances: {
    words:
      'The group variances differ (Bartlett p < 0.05), so the equal-variance statistics are suspect.',
    rule:
      'Fires when Bartlett’s test of equal variances rejects at p < 0.05 — the pooled t test and ANOVA then assume something the data dispute.',
    source: 'Bartlett (1937), Proc. R. Soc. London A 160: 268–282.',
  },
  heterogeneous_strata: {
    words:
      'The stratum odds ratios differ (homogeneity p < 0.05), so the pooled Mantel–Haenszel estimates are suspect — report the stratum-specific estimates.',
    rule:
      'Fires when the Breslow–Day homogeneity test rejects at p < 0.05 — a single common odds ratio then misdescribes the strata.',
    source:
      'Breslow & Day (1980), Statistical Methods in Cancer Research I, IARC, §4.4; Tarone (1985).',
  },
};

/** The plain-language name of one engine-computed trigger key. */
const NOTE_VALUE: Record<string, string> = {
  bartlett_p: 'Bartlett p',
  breslow_day_or_p: 'Breslow–Day OR p',
  breslow_day_tarone_p: 'Breslow–Day–Tarone p',
  expected_lt5: 'Expected cells below 5',
  expected_min: 'Smallest expected cell',
  np: 'n·p',
  n_one_minus_p: 'n·(1−p)',
  stratum: 'Stratum',
  zero_cells: 'Zero cells',
};

function humanize(id: string): string {
  const s = id.replace(/_/g, ' ');
  return s.charAt(0).toUpperCase() + s.slice(1);
}

/** Trigger key → display name (explain's "values that fired it"). */
export function noteValueLabel(key: string): string {
  return NOTE_VALUE[key] ?? humanize(key);
}

/** The note's `suggests` is a statistic path (or, in principle, a
 *  calculator id). A statistic id anchors to the row that displays
 *  it; `noteAnchor` is the id both the row and the link use. */
export function noteAnchor(statistic: string): string {
  return `note-${statistic}`;
}

/** `suggests` → a readable link label: the last two path segments
 *  humanized (`means.t_test.welch.t` → "Welch t"). Callers with a
 *  richer dictionary (the calculator's `ct.statistic`) may prefer
 *  their own — this is the shared fallback. */
export function suggestLabel(statistic: string): string {
  const parts = statistic.split('.').filter((p) => p !== '');
  const tail = parts.slice(-2).join(' ');
  return humanize(tail === '' ? statistic : tail);
}

/** The analysis-side `suggests` link: the anchor of the row that
 *  displays the suggested statistic (`two_by_two.fisher_exact.
 *  p_two_sided` → `#note-…`), or the calculator route when the
 *  suggestion names a calculator id. */
export function statLink(suggests: string): { href: string; label: string } {
  if (!suggests.includes('.')) {
    return { href: `/calc/${suggests}`, label: humanize(suggests) };
  }
  return { href: `#${noteAnchor(suggests)}`, label: suggestLabel(suggests) };
}

/** The block of a result a note belongs under: the first path
 *  segment after the run-surface prefix (`calc.<id>.` for calculator
 *  notes, none for procedure notes), with `strata[]` refined by the
 *  note's `stratum` value and `means.*` refined to its test block
 *  (`means.bartlett`). Every statistic of a note shares the surface,
 *  so the first one names the block. */
export function noteBlock(note: { statistics: string[]; values: Record<string, number> }): string {
  let path = note.statistics[0] ?? '';
  if (path.startsWith('calc.')) {
    path = path.split('.').slice(2).join('.');
  }
  const first = path.split('.')[0] ?? '';
  if (first === 'strata[]' && Number.isFinite(note.values.stratum)) {
    return `strata[${note.values.stratum}]`;
  }
  if (first === 'means') {
    return `means.${path.split('.')[1] ?? ''}`;
  }
  return first;
}

/** Groups notes by `noteBlock`, preserving the engine's order. */
export function groupNotes<T extends { statistics: string[]; values: Record<string, number> }>(
  notes: T[],
): Map<string, T[]> {
  const groups = new Map<string, T[]>();
  for (const note of notes) {
    const key = noteBlock(note);
    const list = groups.get(key) ?? [];
    list.push(note);
    groups.set(key, list);
  }
  return groups;
}

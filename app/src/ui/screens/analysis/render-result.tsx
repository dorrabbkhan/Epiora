// TABLES result rendering (v0.3 plan Task 12, spec §8.2): the
// `TablesResult` becomes semantic `<table>` markup — captions and
// scoped headers — through the shared `StatsTable` in
// `stats-table.tsx` (an undefined `Estimate` renders "—" with the
// reason in a visually hidden span; every statistic goes through the
// one `formatNumber` helper). The FREQ renderers live in
// `render-freq.tsx` and MEANS in `render-means.tsx`; they and the
// `stats-table` pieces are re-exported here so callers keep a single
// entry point. Visual styling comes from the caller's
// `.result` scope in analysis.module.css so this module stays free of
// stylesheet imports.
import type { ReactNode } from 'react';
import type { MantelHaenszel } from '@engine-types/MantelHaenszel';
import type { Note } from '@engine-types/Note';
import type { StratifiedStats } from '@engine-types/StratifiedStats';
import type { TablesResult } from '@engine-types/TablesResult';
import type { TwoByTwoStats } from '@engine-types/TwoByTwoStats';
import type { ExplainStat } from '../../../explain';
import { groupNotes, statLink } from '../../../explain/notes';
import { NotesList, type NoteLink } from '../NotesList';
import { t } from '../../strings';
import {
  ciStats,
  stat,
  StatsTable,
  type ExplainHandler,
  type StatRow,
} from './stats-table';

// `stats-table` holds the row model and generic table; `renderFreq`
// lives in ./render-freq and `renderMeans` in ./render-means —
// re-exported here so callers keep a single entry point.
export { estimate, StatsTable } from './stats-table';
export type { ExplainHandler, StatRow } from './stats-table';
export { renderFreq } from './render-freq';
export { renderMeans } from './render-means';

/** Every statistic v0.2 returns for a 2×2 (spec §8.2). `prefix` is
 *  `two_by_two` for the crude table and
 *  `stratified_two_by_two.crude` for the pooled-across-strata one. */
function crudeRows(s: TwoByTwoStats, prefix = 'two_by_two'): StatRow[] {
  const or = s.odds_ratio;
  const orCi = (base: string): ExplainStat[] => ciStats(prefix, `odds_ratio.${base}`);
  return [
    {
      label: t.analysis.attackExposed,
      estimate: s.risk.exposed,
      stats: [stat(`${prefix}.risk.exposed`, 'Risk')],
    },
    {
      label: t.analysis.attackUnexposed,
      estimate: s.risk.unexposed,
      stats: [stat(`${prefix}.risk.unexposed`, 'Risk')],
    },
    {
      label: t.analysis.riskDifference,
      estimate: s.risk.difference,
      ci: s.risk.difference_ci_95_wald,
      stats: [
        stat(`${prefix}.risk.difference`, 'Estimate'),
        ...ciStats(prefix, 'risk.difference_ci_95_wald'),
      ],
    },
    {
      label: t.analysis.riskRatio,
      estimate: s.risk_ratio.estimate,
      ci: s.risk_ratio.ci_95_wald,
      stats: [
        stat(`${prefix}.risk_ratio.estimate`, 'Estimate'),
        ...ciStats(prefix, 'risk_ratio.ci_95_wald'),
      ],
    },
    {
      label: t.analysis.riskRatioSmall,
      estimate: s.risk_ratio.small_sample.estimate,
      ci: {
        lower: s.risk_ratio.small_sample.lower,
        upper: s.risk_ratio.small_sample.upper,
      },
      stats: [
        stat(`${prefix}.risk_ratio.small_sample.estimate`, 'Estimate'),
        ...ciStats(prefix, 'risk_ratio.small_sample'),
      ],
    },
    {
      label: t.analysis.oddsRatio,
      estimate: or.estimate,
      ci: or.ci_95_wald,
      stats: [
        stat(`${prefix}.odds_ratio.estimate`, 'Estimate'),
        ...orCi('ci_95_wald'),
      ],
    },
    {
      label: t.analysis.oddsRatioCmle,
      estimate: or.cmle,
      stats: [stat(`${prefix}.odds_ratio.cmle`, 'Estimate')],
    },
    {
      label: t.analysis.oddsRatioFisherCi,
      ci: or.ci_95_fisher_exact,
      stats: orCi('ci_95_fisher_exact'),
    },
    {
      label: t.analysis.oddsRatioMidpCi,
      ci: or.ci_95_midp_exact,
      stats: orCi('ci_95_midp_exact'),
    },
    {
      label: t.analysis.chiUncorrected,
      estimate: s.chi_square.uncorrected.statistic,
      p: s.chi_square.uncorrected.p_value,
      stats: [
        stat(`${prefix}.chi_square.uncorrected.statistic`, 'Statistic'),
        stat(`${prefix}.chi_square.uncorrected.p_value`, 'p-value'),
      ],
    },
    {
      label: t.analysis.chiYates,
      estimate: s.chi_square.yates.statistic,
      p: s.chi_square.yates.p_value,
      stats: [
        stat(`${prefix}.chi_square.yates.statistic`, 'Statistic'),
        stat(`${prefix}.chi_square.yates.p_value`, 'p-value'),
      ],
    },
    {
      label: t.analysis.chiMantelHaenszel,
      estimate: s.chi_square.mantel_haenszel.statistic,
      p: s.chi_square.mantel_haenszel.p_value,
      stats: [
        stat(`${prefix}.chi_square.mantel_haenszel.statistic`, 'Statistic'),
        stat(`${prefix}.chi_square.mantel_haenszel.p_value`, 'p-value'),
      ],
    },
    {
      label: t.analysis.chiDf,
      estimate: s.chi_square.df,
      stats: [stat(`${prefix}.chi_square.df`, 'df')],
    },
    {
      label: t.analysis.fisherLower,
      p: s.fisher_exact.p_lower,
      stats: [stat(`${prefix}.fisher_exact.p_lower`, 'p-value')],
    },
    {
      label: t.analysis.fisherUpper,
      p: s.fisher_exact.p_upper,
      stats: [stat(`${prefix}.fisher_exact.p_upper`, 'p-value')],
    },
    {
      label: t.analysis.fisherTwoSided,
      p: s.fisher_exact.p_two_sided,
      stats: [stat(`${prefix}.fisher_exact.p_two_sided`, 'p-value')],
    },
    {
      label: t.analysis.midpTwoSided,
      p: s.midp_exact.p_two_sided,
      stats: [stat(`${prefix}.midp_exact.p_two_sided`, 'p-value')],
    },
  ];
}

function mhRows(mh: MantelHaenszel, s: StratifiedStats): StatRow[] {
  const bdt = s.breslow_day_tarone;
  const P = 'stratified_two_by_two';
  return [
    {
      label: t.analysis.mhOddsRatio,
      estimate: mh.odds_ratio.estimate,
      ci: mh.odds_ratio.ci_95,
      stats: [
        stat(`${P}.mantel_haenszel.odds_ratio.estimate`, 'Estimate'),
        stat(`${P}.mantel_haenszel.odds_ratio.ci_95.lower`, 'Lower bound'),
        stat(`${P}.mantel_haenszel.odds_ratio.ci_95.upper`, 'Upper bound'),
      ],
    },
    {
      label: t.analysis.mhRiskRatio,
      estimate: mh.risk_ratio.estimate,
      ci: mh.risk_ratio.ci_95,
      stats: [
        stat(`${P}.mantel_haenszel.risk_ratio.estimate`, 'Estimate'),
        stat(`${P}.mantel_haenszel.risk_ratio.ci_95.lower`, 'Lower bound'),
        stat(`${P}.mantel_haenszel.risk_ratio.ci_95.upper`, 'Upper bound'),
      ],
    },
    {
      label: t.analysis.mhChiUncorrected,
      estimate: mh.chi_square.uncorrected.statistic,
      p: mh.chi_square.uncorrected.p_value,
      stats: [
        stat(`${P}.mantel_haenszel.chi_square.uncorrected.statistic`, 'Statistic'),
        stat(`${P}.mantel_haenszel.chi_square.uncorrected.p_value`, 'p-value'),
      ],
    },
    {
      label: t.analysis.mhChiCorrected,
      estimate: mh.chi_square.corrected.statistic,
      p: mh.chi_square.corrected.p_value,
      stats: [
        stat(`${P}.mantel_haenszel.chi_square.corrected.statistic`, 'Statistic'),
        stat(`${P}.mantel_haenszel.chi_square.corrected.p_value`, 'p-value'),
      ],
    },
    {
      label: t.analysis.bdt,
      estimate: bdt.statistic,
      stats: [stat(`${P}.breslow_day_tarone.statistic`, 'Statistic')],
    },
    {
      label: t.analysis.bdtDf,
      estimate: bdt.df,
      stats: [stat(`${P}.breslow_day_tarone.df`, 'df')],
    },
    {
      label: t.analysis.bdtP,
      p: bdt.p_value,
      stats: [stat(`${P}.breslow_day_tarone.p_value`, 'p-value')],
    },
  ];
}

/** The analysis-side `suggests` resolver (v1.0 Task 23, spec §6.1):
 *  statistic ids anchor to the row that displays them. */
const linkFor = (suggests: string): NoteLink => statLink(suggests);

/** Notes whose block has no matching table render once at the end. */
function tableNotes(
  groups: Map<string, Note[]>,
  keys: string[],
): { found: Note[]; rest: Note[] } {
  const found: Note[] = [];
  const rest: Note[] = [];
  for (const [key, list] of groups) {
    if (keys.includes(key)) {
      found.push(...list);
    } else {
      rest.push(...list);
    }
  }
  return { found, rest };
}

export function renderTables(
  result: TablesResult,
  notes: Note[] = [],
  onExplain?: ExplainHandler | undefined,
): ReactNode {
  const groups = groupNotes(notes);
  if ('rx_c' in result) {
    const r = result.rx_c;
    const { found, rest } = tableNotes(groups, ['rxc']);
    return (
      <>
        <table>
          <caption>{t.analysis.countsCaption}</caption>
          <thead>
            <tr>
              <td></td>
              {r.col_labels.map((label, i) => (
                <th scope="col" key={i}>
                  {label}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {r.row_labels.map((label, i) => (
              <tr key={i}>
                <th scope="row">{label}</th>
                {(r.counts[i] ?? []).map((count, j) => (
                  <td key={j}>{count}</td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
        <StatsTable
          caption={t.analysis.chiTableCaption}
          onExplain={onExplain}
          rows={[
            {
              label: t.analysis.chiUncorrected,
              estimate: r.stats.chi_square.statistic,
              p: r.stats.chi_square.p_value,
              stats: [
                stat('rxc.chi_square.statistic', 'Statistic'),
                stat('rxc.chi_square.p_value', 'p-value'),
              ],
            },
            {
              label: t.analysis.chiDf,
              estimate: r.stats.chi_square.df,
              stats: [stat('rxc.chi_square.df', 'df')],
            },
          ]}
        />
        {r.stats.low_expected && <p>{t.analysis.lowExpected}</p>}
        <NotesList notes={found} linkFor={linkFor} />
        <NotesList notes={rest} linkFor={linkFor} />
      </>
    );
  }

  const r = result.two_by_two;
  const [exposed, unexposed] = r.labels.exposure;
  const [ill, well] = r.labels.outcome;
  const s = r.stratified ?? null;
  const excluded = s?.stats.excluded_strata ?? [];
  const stratified = groups.get('stratified_two_by_two') ?? [];
  const leftover = tableNotes(groups, [
    'two_by_two',
    'stratified_two_by_two',
  ]).rest;
  return (
    <>
      <p>
        {t.analysis.levelsLine(
          exposed ?? '',
          unexposed ?? '',
          ill ?? '',
          well ?? '',
        )}
      </p>
      {/* Variable names are not in the result; the card title names them. */}
      <StatsTable
        caption={t.analysis.statsCaption}
        onExplain={onExplain}
        rows={crudeRows(r.crude)}
      />
      <NotesList notes={groups.get('two_by_two') ?? []} linkFor={linkFor} />
      {s !== null && (
        <>
          <p>{t.analysis.strata(s.strata_labels)}</p>
          {excluded.length > 0 && (
            <p>
              {t.analysis.excludedStrata(
                excluded.map((i) => s.strata_labels[i] ?? String(i)),
              )}
            </p>
          )}
          <StatsTable
            caption={t.analysis.stratifiedStatsCaption}
            onExplain={onExplain}
            rows={crudeRows(s.stats.crude, 'stratified_two_by_two.crude')}
          />
          <StatsTable
            caption={t.analysis.mhCaption}
            onExplain={onExplain}
            rows={mhRows(s.stats.mantel_haenszel, s.stats)}
          />
          <NotesList notes={stratified} linkFor={linkFor} />
        </>
      )}
      <NotesList notes={leftover} linkFor={linkFor} />
    </>
  );
}

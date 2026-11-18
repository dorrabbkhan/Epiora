// Result rendering (v0.3 plan Task 12, spec §8.2): `FreqResult` and
// `TablesResult` become semantic `<table>` markup — captions and
// scoped headers; an undefined `Estimate` renders "—" with the reason
// in a visually hidden span; every statistic goes through the one
// `formatNumber` helper (exactly 4 significant digits, trailing
// zeros kept, p-values below 0.0001 as "< 0.0001"). Visual styling
// comes from the caller's
// `.result` scope in analysis.module.css so this module stays free of
// stylesheet imports.
import type { CSSProperties, ReactNode } from 'react';
import type { CategoryStats } from '@engine-types/CategoryStats';
import type { Ci } from '@engine-types/Ci';
import type { Estimate } from '@engine-types/Estimate';
import type { FreqResult } from '@engine-types/FreqResult';
import type { FreqTable } from '@engine-types/FreqTable';
import type { MantelHaenszel } from '@engine-types/MantelHaenszel';
import type { ProportionCi } from '@engine-types/ProportionCi';
import type { StratifiedStats } from '@engine-types/StratifiedStats';
import type { TablesResult } from '@engine-types/TablesResult';
import type { TwoByTwoStats } from '@engine-types/TwoByTwoStats';
import { Button } from '../../components';
import { et, type ExplainStat } from '../../../explain';
import { formatNumber } from '../../format';
import { t } from '../../strings';

// `renderMeans` lives in ./render-means; re-exported here so callers
// keep a single entry point.
export { renderMeans } from './render-means';

const VISUALLY_HIDDEN: CSSProperties = {
  position: 'absolute',
  width: '1px',
  height: '1px',
  padding: 0,
  margin: '-1px',
  overflow: 'hidden',
  clip: 'rect(0 0 0 0)',
  whiteSpace: 'nowrap',
  border: 0,
};

export function estimate(e: Estimate, kind: 'estimate' | 'p' = 'estimate'): ReactNode {
  if (typeof e === 'number') return formatNumber(e, kind);
  const reason = t.analysis.undefinedReason[e.undefined];
  return (
    <>
      {'—'}
      <span style={VISUALLY_HIDDEN}> ({reason})</span>
    </>
  );
}

function interval(c: Ci): ReactNode {
  return (
    <>
      {estimate(c.lower)}
      {' – '}
      {estimate(c.upper)}
    </>
  );
}

/** Explain (v1.0 plan Task 16, spec §6): a row may expose the
 *  concrete `explainProcedure` paths of the leaves it displays —
 *  the Estimate first, then CI bounds and the p-value — so the
 *  panel's picker can switch between them. */
export interface StatRow {
  label: string;
  estimate?: Estimate;
  ci?: Ci;
  p?: Estimate;
  stats?: ExplainStat[];
}

/** The explain affordance one row passes to `StatsTable`. */
export type ExplainHandler = (row: StatRow) => void;

function stat(path: string, label: string): ExplainStat {
  return { path, label };
}

export function StatsTable({
  caption,
  rows,
  onExplain,
}: {
  caption: string;
  rows: StatRow[];
  onExplain?: ExplainHandler | undefined;
}) {
  const explainable = onExplain !== undefined;
  return (
    <table>
      <caption>{caption}</caption>
      <thead>
        <tr>
          <th scope="col">{t.analysis.statisticCol}</th>
          <th scope="col">{t.analysis.estimateCol}</th>
          <th scope="col">{t.analysis.ciCol}</th>
          <th scope="col">{t.analysis.pValueCol}</th>
          {explainable && <th scope="col">{et.explain}</th>}
        </tr>
      </thead>
      <tbody>
        {rows.map((r, i) => (
          <tr key={i}>
            <th scope="row">{r.label}</th>
            <td>{r.estimate === undefined ? '' : estimate(r.estimate)}</td>
            <td>{r.ci === undefined ? '' : interval(r.ci)}</td>
            <td>{r.p === undefined ? '' : estimate(r.p, 'p')}</td>
            {explainable && (
              <td>
                {r.stats !== undefined && r.stats.length > 0 && (
                  <Button onPress={() => onExplain(r)}>{et.explain}</Button>
                )}
              </td>
            )}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** The leaves a CI row covers, with picker labels. */
function ciStats(prefix: string, base: string): ExplainStat[] {
  return [
    stat(`${prefix}.${base}.lower`, 'Lower bound'),
    stat(`${prefix}.${base}.upper`, 'Upper bound'),
  ];
}

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

function categoryCi(cat: CategoryStats, method: ProportionCi): Ci {
  return method === 'wilson' ? cat.ci_95_wilson : cat.ci_95_clopper_pearson;
}

/** The explainable leaves of one frequency category. `freq.*` paths
 *  name no table, so the engine always resolves them against the
 *  run's first table — `renderFreq` therefore only offers the
 *  explain affordance on `tables[0]` (the one a trace reads). */
function freqCatStats(i: number, methods: ProportionCi[]): ExplainStat[] {
  const base = `freq.categories[${i}]`;
  return [
    stat(`${base}.count`, 'Count'),
    stat(`${base}.percent`, 'Percent'),
    stat(`${base}.cumulative_percent`, 'Cumulative percent'),
    ...methods.flatMap((m) => [
      stat(`${base}.ci_95_${m}.lower`, `${m} — lower bound`),
      stat(`${base}.ci_95_${m}.upper`, `${m} — upper bound`),
    ]),
  ];
}

function FreqTableView({
  table,
  onExplain,
}: {
  table: FreqTable;
  onExplain?: ExplainHandler | undefined;
}) {
  // Every statistic v0.2 returns: both per-category CIs are shown;
  // the procedure's `ci_method` interval comes first.
  const methods: ProportionCi[] = [
    table.ci_method,
    table.ci_method === 'wilson' ? 'clopper_pearson' : 'wilson',
  ];
  const explainable = onExplain !== undefined;
  return (
    <>
      <table>
        <caption>{t.analysis.freqCaption(table.variable, table.stratum ?? null)}</caption>
        <thead>
          <tr>
            <th scope="col">{t.analysis.valueCol}</th>
            <th scope="col">{t.analysis.countCol}</th>
            <th scope="col">{t.analysis.percentCol}</th>
            <th scope="col">{t.analysis.cumPercentCol}</th>
            {methods.map((m) => (
              <th scope="col" key={m}>
                {t.analysis.ciMethodLabel[m]} {t.analysis.ciCol}
              </th>
            ))}
            {explainable && <th scope="col">{et.explain}</th>}
          </tr>
        </thead>
        <tbody>
          {table.labels.map((label, i) => {
            const cat = table.categories[i];
            return (
              <tr key={i}>
                <th scope="row">{label}</th>
                <td>{cat === undefined ? '' : cat.count}</td>
                <td>{cat === undefined ? '' : estimate(cat.percent)}</td>
                <td>
                  {cat === undefined ? '' : estimate(cat.cumulative_percent)}
                </td>
                {methods.map((m) => (
                  <td key={m}>
                    {cat === undefined ? '' : interval(categoryCi(cat, m))}
                  </td>
                ))}
                {explainable && (
                  <td>
                    <Button
                      onPress={() =>
                        onExplain({
                          label,
                          stats: cat === undefined ? [] : freqCatStats(i, methods),
                        })
                      }
                    >
                      {et.explain}
                    </Button>
                  </td>
                )}
              </tr>
            );
          })}
        </tbody>
      </table>
      <p>{t.analysis.freqSummary(table.total, table.missing)}</p>
    </>
  );
}

export function renderFreq(result: FreqResult, onExplain?: ExplainHandler | undefined): ReactNode {
  const excluded = Object.entries(result.exclusions);
  return (
    <>
      {result.tables.map((table, i) => (
        <FreqTableView
          key={i}
          table={table}
          onExplain={i === 0 ? onExplain : undefined}
        />
      ))}
      {excluded.length > 0 && <p>{t.analysis.exclusions(excluded)}</p>}
    </>
  );
}

export function renderTables(result: TablesResult, onExplain?: ExplainHandler | undefined): ReactNode {
  if ('rx_c' in result) {
    const r = result.rx_c;
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
      </>
    );
  }

  const r = result.two_by_two;
  const [exposed, unexposed] = r.labels.exposure;
  const [ill, well] = r.labels.outcome;
  const s = r.stratified ?? null;
  const excluded = s?.stats.excluded_strata ?? [];
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
        </>
      )}
    </>
  );
}

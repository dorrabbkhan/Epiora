// FREQ result rendering (v0.3 plan Task 12, spec §8.2): one table
// per `FreqTable` — caption, scoped headers, both per-category CIs
// (the procedure's `ci_method` interval first) — then the total and
// missing summary and any exclusions. `render-result.tsx` re-exports
// `renderFreq` so callers keep a single entry point. Every category
// row carries its `freq.categories[i].*` explainable leaves (v1.0
// plan Task 16, spec §6).
import type { ReactNode } from 'react';
import type { CategoryStats } from '@engine-types/CategoryStats';
import type { Ci } from '@engine-types/Ci';
import type { FreqResult } from '@engine-types/FreqResult';
import type { FreqTable } from '@engine-types/FreqTable';
import type { ProportionCi } from '@engine-types/ProportionCi';
import { Button } from '../../components';
import { et, type ExplainStat } from '../../../explain';
import { t } from '../../strings';
import { estimate, interval, stat, type ExplainHandler } from './stats-table';

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

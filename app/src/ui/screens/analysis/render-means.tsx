// MEANS result rendering (v1.0 plan Task 15, spec §4.2/§7.3): the
// descriptives table and test blocks share the helpers in
// `render-result.tsx` — `estimate` for undefined-aware numbers and
// `StatsTable` for the test rows — so markup and formatting rules stay
// identical. `render-result.tsx` re-exports `renderMeans` so callers
// keep a single entry point.
import type { ReactNode } from 'react';
import type { MeansResult } from '@engine-types/MeansResult';
import { t } from '../../strings';
import { estimate, StatsTable } from './render-result';

/** MEANS (v1.0 plan Task 15, spec §4.2/§7.3): a descriptives table —
 *  one row per group, then overall — followed by each test block the
 *  engine returned (never an empty table for an absent test). */
export function renderMeans(result: MeansResult): ReactNode {
  const excluded = Object.entries(result.exclusions);
  const tt = result.t_test ?? null;
  const a = result.anova ?? null;
  const b = result.bartlett ?? null;
  const kw = result.kruskal_wallis ?? null;
  return (
    <>
      <table>
        <caption>
          {t.analysis.meansCaption(result.variable, result.group_by ?? null)}
        </caption>
        <thead>
          <tr>
            <th scope="col">{t.analysis.groupCol}</th>
            <th scope="col">{t.analysis.observationsCol}</th>
            <th scope="col">{t.analysis.totalCol}</th>
            <th scope="col">{t.analysis.meanCol}</th>
            <th scope="col">{t.analysis.varianceCol}</th>
            <th scope="col">{t.analysis.stdDevCol}</th>
            <th scope="col">{t.analysis.minCol}</th>
            <th scope="col">{t.analysis.p25Col}</th>
            <th scope="col">{t.analysis.medianCol}</th>
            <th scope="col">{t.analysis.p75Col}</th>
            <th scope="col">{t.analysis.maxCol}</th>
            <th scope="col">{t.analysis.modeCol}</th>
          </tr>
        </thead>
        <tbody>
          {[...result.groups, result.overall].map((g, i) => (
            <tr key={i}>
              <th scope="row">{g.label}</th>
              <td>{g.observations}</td>
              <td>{estimate(g.total)}</td>
              <td>{estimate(g.mean)}</td>
              <td>{estimate(g.variance)}</td>
              <td>{estimate(g.std_dev)}</td>
              <td>{estimate(g.min)}</td>
              <td>{estimate(g.p25)}</td>
              <td>{estimate(g.median)}</td>
              <td>{estimate(g.p75)}</td>
              <td>{estimate(g.max)}</td>
              <td>{estimate(g.mode)}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {tt !== null && (
        <StatsTable
          caption={t.analysis.tTestCaption}
          rows={[
            { label: t.analysis.pooledT, estimate: tt.pooled.t },
            { label: t.analysis.pooledDf, estimate: tt.pooled.df },
            { label: t.analysis.pooledP, p: tt.pooled.p_value },
            {
              label: t.analysis.pooledDiff,
              estimate: tt.pooled.diff,
              ci: tt.pooled.diff_ci_95,
            },
            { label: t.analysis.welchT, estimate: tt.welch.t },
            { label: t.analysis.welchDf, estimate: tt.welch.df },
            { label: t.analysis.welchP, p: tt.welch.p_value },
            {
              label: t.analysis.welchDiff,
              estimate: tt.welch.diff,
              ci: tt.welch.diff_ci_95,
            },
          ]}
        />
      )}
      {a !== null && (
        <StatsTable
          caption={t.analysis.anovaCaption}
          rows={[
            { label: t.analysis.ssBetween, estimate: a.ss_between },
            { label: t.analysis.ssWithin, estimate: a.ss_within },
            { label: t.analysis.ssTotal, estimate: a.ss_total },
            { label: t.analysis.dfBetween, estimate: a.df_between },
            { label: t.analysis.dfWithin, estimate: a.df_within },
            { label: t.analysis.dfTotal, estimate: a.df_total },
            { label: t.analysis.msBetween, estimate: a.ms_between },
            { label: t.analysis.msWithin, estimate: a.ms_within },
            { label: t.analysis.fStat, estimate: a.f, p: a.p_value },
          ]}
        />
      )}
      {b !== null && (
        <StatsTable
          caption={t.analysis.bartlettCaption}
          rows={[
            { label: t.analysis.bartlettChi, estimate: b.statistic, p: b.p_value },
            { label: t.analysis.dfLabel, estimate: b.df },
          ]}
        />
      )}
      {kw !== null && (
        <StatsTable
          caption={t.analysis.kwCaption}
          rows={[
            { label: t.analysis.kwH, estimate: kw.h, p: kw.p_value },
            { label: t.analysis.dfLabel, estimate: kw.df },
          ]}
        />
      )}
      {excluded.length > 0 && <p>{t.analysis.exclusions(excluded)}</p>}
    </>
  );
}

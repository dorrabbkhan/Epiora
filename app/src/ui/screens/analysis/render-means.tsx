// MEANS result rendering (v1.0 plan Task 15, spec §4.2/§7.3): the
// descriptives table and test blocks share the helpers in
// `render-result.tsx` — `estimate` for undefined-aware numbers and
// `StatsTable` for the test rows — so markup and formatting rules stay
// identical. `render-result.tsx` re-exports `renderMeans` so callers
// keep a single entry point. Every row carries its `means.*`
// explainable leaves (v1.0 plan Task 16, spec §6).
import type { ReactNode } from 'react';
import type { MeansResult } from '@engine-types/MeansResult';
import type { Note } from '@engine-types/Note';
import { Button } from '../../components';
import { et, type ExplainStat } from '../../../explain';
import { groupNotes, statLink } from '../../../explain/notes';
import { NotesList } from '../NotesList';
import { t } from '../../strings';
import { estimate, StatsTable, type ExplainHandler } from './render-result';

function st(path: string, label: string): ExplainStat {
  return { path, label };
}

/** The eleven leaves of one descriptives row (`groups[i]` or
 *  `overall`). */
function groupStats(base: string): ExplainStat[] {
  return [
    st(`${base}.observations`, 'Observations'),
    st(`${base}.total`, 'Total'),
    st(`${base}.mean`, 'Mean'),
    st(`${base}.variance`, 'Variance'),
    st(`${base}.std_dev`, 'Standard deviation'),
    st(`${base}.min`, 'Minimum'),
    st(`${base}.p25`, 'Lower quartile'),
    st(`${base}.median`, 'Median'),
    st(`${base}.p75`, 'Upper quartile'),
    st(`${base}.max`, 'Maximum'),
    st(`${base}.mode`, 'Mode'),
  ];
}

/** The diff row covers the estimate and both CI bounds. */
function tDiffStats(base: string): ExplainStat[] {
  return [
    st(`${base}.diff`, 'Estimate'),
    st(`${base}.diff_ci_95.lower`, 'Lower bound'),
    st(`${base}.diff_ci_95.upper`, 'Upper bound'),
  ];
}

/** MEANS (v1.0 plan Task 15, spec §4.2/§7.3): a descriptives table —
 *  one row per group, then overall — followed by each test block the
 *  engine returned (never an empty table for an absent test). */
export function renderMeans(
  result: MeansResult,
  notes: Note[] = [],
  onExplain?: ExplainHandler | undefined,
): ReactNode {
  const excluded = Object.entries(result.exclusions);
  const tt = result.t_test ?? null;
  const a = result.anova ?? null;
  const b = result.bartlett ?? null;
  const kw = result.kruskal_wallis ?? null;
  // Assumption notes (v1.0 Task 23, spec §6.1) sit under the test
  // block their first flagged statistic lives in.
  const groups = groupNotes(notes);
  const known = [
    'means.t_test',
    'means.anova',
    'means.bartlett',
    'means.kruskal_wallis',
  ];
  const leftover = [...groups.keys()]
    .filter((key) => !known.includes(key))
    .flatMap((key) => groups.get(key) ?? []);
  const explainable = onExplain !== undefined;
  const groupRows = [...result.groups, result.overall];
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
            {explainable && <th scope="col">{et.explain}</th>}
          </tr>
        </thead>
        <tbody>
          {groupRows.map((g, i) => (
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
              {explainable && (
                <td>
                  <Button
                    onPress={() =>
                      onExplain({
                        label: g.label,
                        stats: groupStats(
                          i < result.groups.length
                            ? `means.groups[${i}]`
                            : 'means.overall',
                        ),
                      })
                    }
                  >
                    {et.explain}
                  </Button>
                </td>
              )}
            </tr>
          ))}
        </tbody>
      </table>
      {tt !== null && (
        <StatsTable
          caption={t.analysis.tTestCaption}
          onExplain={onExplain}
          rows={[
            {
              label: t.analysis.pooledT,
              estimate: tt.pooled.t,
              stats: [st('means.t_test.pooled.t', 'Statistic')],
            },
            {
              label: t.analysis.pooledDf,
              estimate: tt.pooled.df,
              stats: [st('means.t_test.pooled.df', 'df')],
            },
            {
              label: t.analysis.pooledP,
              p: tt.pooled.p_value,
              stats: [st('means.t_test.pooled.p_value', 'p-value')],
            },
            {
              label: t.analysis.pooledDiff,
              estimate: tt.pooled.diff,
              ci: tt.pooled.diff_ci_95,
              stats: tDiffStats('means.t_test.pooled'),
            },
            {
              label: t.analysis.welchT,
              estimate: tt.welch.t,
              stats: [st('means.t_test.welch.t', 'Statistic')],
            },
            {
              label: t.analysis.welchDf,
              estimate: tt.welch.df,
              stats: [st('means.t_test.welch.df', 'df')],
            },
            {
              label: t.analysis.welchP,
              p: tt.welch.p_value,
              stats: [st('means.t_test.welch.p_value', 'p-value')],
            },
            {
              label: t.analysis.welchDiff,
              estimate: tt.welch.diff,
              ci: tt.welch.diff_ci_95,
              stats: tDiffStats('means.t_test.welch'),
            },
          ]}
        />
      )}
      <NotesList
        notes={groups.get('means.t_test') ?? []}
        linkFor={statLink}
      />
      {a !== null && (
        <StatsTable
          caption={t.analysis.anovaCaption}
          onExplain={onExplain}
          rows={[
            {
              label: t.analysis.ssBetween,
              estimate: a.ss_between,
              stats: [st('means.anova.ss_between', 'Sum of squares')],
            },
            {
              label: t.analysis.ssWithin,
              estimate: a.ss_within,
              stats: [st('means.anova.ss_within', 'Sum of squares')],
            },
            {
              label: t.analysis.ssTotal,
              estimate: a.ss_total,
              stats: [st('means.anova.ss_total', 'Sum of squares')],
            },
            {
              label: t.analysis.dfBetween,
              estimate: a.df_between,
              stats: [st('means.anova.df_between', 'df')],
            },
            {
              label: t.analysis.dfWithin,
              estimate: a.df_within,
              stats: [st('means.anova.df_within', 'df')],
            },
            {
              label: t.analysis.dfTotal,
              estimate: a.df_total,
              stats: [st('means.anova.df_total', 'df')],
            },
            {
              label: t.analysis.msBetween,
              estimate: a.ms_between,
              stats: [st('means.anova.ms_between', 'Mean square')],
            },
            {
              label: t.analysis.msWithin,
              estimate: a.ms_within,
              stats: [st('means.anova.ms_within', 'Mean square')],
            },
            {
              label: t.analysis.fStat,
              estimate: a.f,
              p: a.p_value,
              stats: [
                st('means.anova.f', 'Statistic'),
                st('means.anova.p_value', 'p-value'),
              ],
            },
          ]}
        />
      )}
      <NotesList
        notes={groups.get('means.anova') ?? []}
        linkFor={statLink}
      />
      {b !== null && (
        <StatsTable
          caption={t.analysis.bartlettCaption}
          onExplain={onExplain}
          rows={[
            {
              label: t.analysis.bartlettChi,
              estimate: b.statistic,
              p: b.p_value,
              stats: [
                st('means.bartlett.statistic', 'Statistic'),
                st('means.bartlett.p_value', 'p-value'),
              ],
            },
            {
              label: t.analysis.dfLabel,
              estimate: b.df,
              stats: [st('means.bartlett.df', 'df')],
            },
          ]}
        />
      )}
      <NotesList
        notes={groups.get('means.bartlett') ?? []}
        linkFor={statLink}
      />
      {kw !== null && (
        <StatsTable
          caption={t.analysis.kwCaption}
          onExplain={onExplain}
          rows={[
            {
              label: t.analysis.kwH,
              estimate: kw.h,
              p: kw.p_value,
              stats: [
                st('means.kruskal_wallis.h', 'Statistic'),
                st('means.kruskal_wallis.p_value', 'p-value'),
              ],
            },
            {
              label: t.analysis.dfLabel,
              estimate: kw.df,
              stats: [st('means.kruskal_wallis.df', 'df')],
            },
          ]}
        />
      )}
      <NotesList
        notes={groups.get('means.kruskal_wallis') ?? []}
        linkFor={statLink}
      />
      <NotesList notes={leftover} linkFor={statLink} />
      {excluded.length > 0 && <p>{t.analysis.exclusions(excluded)}</p>}
    </>
  );
}

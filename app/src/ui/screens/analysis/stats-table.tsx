// Statistic-row model and the generic statistics table (v0.3 plan
// Task 12, spec §8.2; explain affordance v1.0 plan Task 16, spec §6):
// an undefined `Estimate` renders "—" with the reason in a visually
// hidden span; every statistic goes through the one `formatNumber`
// helper (exactly 4 significant digits, trailing zeros kept,
// p-values below 0.0001 as "< 0.0001"). `render-result.tsx`
// re-exports the public pieces so callers keep a single entry point.
// Visual styling comes from the caller's `.result` scope in
// analysis.module.css so this module stays free of stylesheet
// imports.
import type { CSSProperties, ReactNode } from 'react';
import type { Ci } from '@engine-types/Ci';
import type { Estimate } from '@engine-types/Estimate';
import { Button } from '../../components';
import { et, type ExplainStat } from '../../../explain';
import { noteAnchor } from '../../../explain/notes';
import { formatNumber } from '../../format';
import { t } from '../../strings';

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

export function interval(c: Ci): ReactNode {
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

export function stat(path: string, label: string): ExplainStat {
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
          // The id on a row with explainable leaves anchors a note's
          // suggested alternative (v1.0 Task 23, spec §6.1).
          <tr
            key={i}
            id={
              r.stats !== undefined && r.stats.length > 0
                ? noteAnchor(r.stats[0]!.path)
                : undefined
            }
          >
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
export function ciStats(prefix: string, base: string): ExplainStat[] {
  return [
    stat(`${prefix}.${base}.lower`, 'Lower bound'),
    stat(`${prefix}.${base}.upper`, 'Upper bound'),
  ];
}

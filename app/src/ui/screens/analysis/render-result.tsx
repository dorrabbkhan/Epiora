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

/** The one number format (spec §8.2): exactly 4 significant digits
 *  with trailing zeros kept, never exponent notation, locale-
 *  independent via `toPrecision` (ECMA-262) plus `expandExponential`
 *  for the magnitudes it renders in exponential form. `kind: 'p'`
 *  adds the open bound: p-values in [0, 0.0001) render "< 0.0001",
 *  including an exact 0 (or -0) produced when an upper tail
 *  underflows; estimates of the same magnitude keep their digits.
 *  Non-finite input renders "—". */
export function formatNumber(
  x: number,
  kind: 'estimate' | 'p' = 'estimate',
): string {
  if (!Number.isFinite(x)) return '—';
  if (kind === 'p' && x >= 0 && x < 0.0001) return '< 0.0001';
  return expandExponential(x.toPrecision(4));
}

const EXPONENTIAL = /^(-?)(\d)(?:\.(\d+))?e([+-]\d+)$/;

/** `toPrecision` switches to exponential form outside [1e-6, 1e4);
 *  expanding it by hand keeps the "no exponent" rule for every
 *  finite input ("1.235e+4" → "12350", "3.000e-7" → "0.0000003000"). */
function expandExponential(s: string): string {
  const m = EXPONENTIAL.exec(s);
  if (m === null) return s;
  const [, sign, head, tail = '', expText] = m;
  const digits = head + tail;
  const point = Number(expText) + 1; // '.' sits `point` digits into `digits`
  if (point <= 0) return `${sign}0.${'0'.repeat(-point)}${digits}`;
  if (point >= digits.length) {
    return `${sign}${digits}${'0'.repeat(point - digits.length)}`;
  }
  return `${sign}${digits.slice(0, point)}.${digits.slice(point)}`;
}

function estimate(e: Estimate, kind: 'estimate' | 'p' = 'estimate'): ReactNode {
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

interface StatRow {
  label: string;
  estimate?: Estimate;
  ci?: Ci;
  p?: Estimate;
}

function StatsTable({ caption, rows }: { caption: string; rows: StatRow[] }) {
  return (
    <table>
      <caption>{caption}</caption>
      <thead>
        <tr>
          <th scope="col">{t.analysis.statisticCol}</th>
          <th scope="col">{t.analysis.estimateCol}</th>
          <th scope="col">{t.analysis.ciCol}</th>
          <th scope="col">{t.analysis.pValueCol}</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((r, i) => (
          <tr key={i}>
            <th scope="row">{r.label}</th>
            <td>{r.estimate === undefined ? '' : estimate(r.estimate)}</td>
            <td>{r.ci === undefined ? '' : interval(r.ci)}</td>
            <td>{r.p === undefined ? '' : estimate(r.p, 'p')}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** Every statistic v0.2 returns for a 2×2 (spec §8.2). */
function crudeRows(s: TwoByTwoStats): StatRow[] {
  const or = s.odds_ratio;
  return [
    { label: t.analysis.attackExposed, estimate: s.risk.exposed },
    { label: t.analysis.attackUnexposed, estimate: s.risk.unexposed },
    {
      label: t.analysis.riskDifference,
      estimate: s.risk.difference,
      ci: s.risk.difference_ci_95_wald,
    },
    {
      label: t.analysis.riskRatio,
      estimate: s.risk_ratio.estimate,
      ci: s.risk_ratio.ci_95_wald,
    },
    {
      label: t.analysis.riskRatioSmall,
      estimate: s.risk_ratio.small_sample.estimate,
      ci: {
        lower: s.risk_ratio.small_sample.lower,
        upper: s.risk_ratio.small_sample.upper,
      },
    },
    { label: t.analysis.oddsRatio, estimate: or.estimate, ci: or.ci_95_wald },
    { label: t.analysis.oddsRatioCmle, estimate: or.cmle },
    { label: t.analysis.oddsRatioFisherCi, ci: or.ci_95_fisher_exact },
    { label: t.analysis.oddsRatioMidpCi, ci: or.ci_95_midp_exact },
    {
      label: t.analysis.chiUncorrected,
      estimate: s.chi_square.uncorrected.statistic,
      p: s.chi_square.uncorrected.p_value,
    },
    {
      label: t.analysis.chiYates,
      estimate: s.chi_square.yates.statistic,
      p: s.chi_square.yates.p_value,
    },
    {
      label: t.analysis.chiMantelHaenszel,
      estimate: s.chi_square.mantel_haenszel.statistic,
      p: s.chi_square.mantel_haenszel.p_value,
    },
    { label: t.analysis.chiDf, estimate: s.chi_square.df },
    { label: t.analysis.fisherLower, p: s.fisher_exact.p_lower },
    { label: t.analysis.fisherUpper, p: s.fisher_exact.p_upper },
    { label: t.analysis.fisherTwoSided, p: s.fisher_exact.p_two_sided },
    { label: t.analysis.midpTwoSided, p: s.midp_exact.p_two_sided },
  ];
}

function mhRows(mh: MantelHaenszel, s: StratifiedStats): StatRow[] {
  const bdt = s.breslow_day_tarone;
  return [
    {
      label: t.analysis.mhOddsRatio,
      estimate: mh.odds_ratio.estimate,
      ci: mh.odds_ratio.ci_95,
    },
    {
      label: t.analysis.mhRiskRatio,
      estimate: mh.risk_ratio.estimate,
      ci: mh.risk_ratio.ci_95,
    },
    {
      label: t.analysis.mhChiUncorrected,
      estimate: mh.chi_square.uncorrected.statistic,
      p: mh.chi_square.uncorrected.p_value,
    },
    {
      label: t.analysis.mhChiCorrected,
      estimate: mh.chi_square.corrected.statistic,
      p: mh.chi_square.corrected.p_value,
    },
    { label: t.analysis.bdt, estimate: bdt.statistic },
    { label: t.analysis.bdtDf, estimate: bdt.df },
    { label: t.analysis.bdtP, p: bdt.p_value },
  ];
}

function categoryCi(cat: CategoryStats, method: ProportionCi): Ci {
  return method === 'wilson' ? cat.ci_95_wilson : cat.ci_95_clopper_pearson;
}

function FreqTableView({ table }: { table: FreqTable }) {
  // Every statistic v0.2 returns: both per-category CIs are shown;
  // the procedure's `ci_method` interval comes first.
  const methods: ProportionCi[] = [
    table.ci_method,
    table.ci_method === 'wilson' ? 'clopper_pearson' : 'wilson',
  ];
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
              </tr>
            );
          })}
        </tbody>
      </table>
      <p>{t.analysis.freqSummary(table.total, table.missing)}</p>
    </>
  );
}

export function renderFreq(result: FreqResult): ReactNode {
  const excluded = Object.entries(result.exclusions);
  return (
    <>
      {result.tables.map((table, i) => (
        <FreqTableView key={i} table={table} />
      ))}
      {excluded.length > 0 && <p>{t.analysis.exclusions(excluded)}</p>}
    </>
  );
}

export function renderTables(result: TablesResult): ReactNode {
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
          rows={[
            {
              label: t.analysis.chiUncorrected,
              estimate: r.stats.chi_square.statistic,
              p: r.stats.chi_square.p_value,
            },
            { label: t.analysis.chiDf, estimate: r.stats.chi_square.df },
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
            rows={crudeRows(s.stats.crude)}
          />
          <StatsTable
            caption={t.analysis.mhCaption}
            rows={mhRows(s.stats.mantel_haenszel, s.stats)}
          />
        </>
      )}
    </>
  );
}

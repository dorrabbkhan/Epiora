// Result rendering unit tests (v0.3 plan Task 12): semantic tables
// (caption + scoped headers), the "—" + hidden reason for undefined
// estimates, and the fixed `formatNumber` significant-digit rule.
import { render, screen, within } from '@testing-library/react';
import '@testing-library/jest-dom/vitest';
import { describe, expect, it } from 'vitest';
import type { Ci } from '@engine-types/Ci';
import type { Estimate } from '@engine-types/Estimate';
import type { FreqResult } from '@engine-types/FreqResult';
import type { TablesResult } from '@engine-types/TablesResult';
import type { TwoByTwoStats } from '@engine-types/TwoByTwoStats';
import { formatNumber, renderFreq, renderTables } from './render-result';

const num = (x: number): Estimate => x;
const ci = (lower: number, upper: number): Ci => ({ lower, upper });

const OSWEGO_2X2: TablesResult = {
  two_by_two: {
    labels: { exposure: ['1', '0'], outcome: ['1', '0'] },
    crude: {
      risk: {
        exposed: num(0.7962962962962963),
        unexposed: num(0.14285714285714285),
        difference: num(0.6534391534391535),
        difference_ci_95_wald: ci(0.469215414740773, 0.837662892137534),
      },
      risk_ratio: {
        estimate: num(5.574074074074074),
        ci_95_wald: ci(1.938339835538607, 16.029336658930724),
        small_sample: {
          estimate: num(4.37962962962963),
          lower: num(1.5229812993517629),
          upper: num(12.594478803445574),
        },
      },
      odds_ratio: {
        estimate: num(23.454545454545453),
        cmle: num(22.149035514258824),
        ci_95_wald: ci(4.936758493025653, 111.40555555555556),
        ci_95_fisher_exact: ci(4.6, 137.0),
        ci_95_midp_exact: ci(5.0, 120.0),
      },
      chi_square: {
        df: num(1),
        uncorrected: { statistic: num(25.0), p_value: num(0.00000057) },
        yates: { statistic: num(22.0), p_value: num(0.0000027) },
        mantel_haenszel: { statistic: num(24.9), p_value: num(0.00000059) },
      },
      fisher_exact: {
        p_lower: num(1),
        p_upper: num(0.0000012),
        p_two_sided: num(0.00000095),
      },
      midp_exact: { p_two_sided: num(0.0000008) },
    },
    stratified: null,
  },
};

const FREQ_ILL: FreqResult = {
  tables: [
    {
      variable: 'ILL',
      stratum: null,
      labels: ['0', '1'],
      ci_method: 'clopper_pearson',
      total: 75,
      missing: 0,
      categories: [
        {
          count: 29,
          percent: num(38.666666666666664),
          cumulative_percent: num(38.666666666666664),
          ci_95_clopper_pearson: ci(27.7, 50.6),
          ci_95_wilson: ci(28.4, 49.9),
        },
        {
          count: 46,
          percent: num(61.333333333333336),
          cumulative_percent: num(100),
          ci_95_clopper_pearson: ci(49.4, 72.3),
          ci_95_wilson: ci(50.1, 71.6),
        },
      ],
    },
  ],
  exclusions: { ILL: 0 },
};

const oswego2x2 = (() => {
  if (!('two_by_two' in OSWEGO_2X2)) throw new Error('fixture');
  return OSWEGO_2X2.two_by_two;
})();

function twoByTwoWith(over: Partial<TwoByTwoStats>): TablesResult {
  return {
    two_by_two: { ...oswego2x2, crude: { ...oswego2x2.crude, ...over } },
  };
}

describe('renderTables', () => {
  it('tables_2x2_has_caption_and_scoped_headers', () => {
    const { container } = render(<>{renderTables(OSWEGO_2X2)}</>);
    const tables = container.querySelectorAll('table');
    expect(tables.length).toBeGreaterThan(0);
    for (const table of tables) {
      expect(table.querySelector('caption')).not.toBeNull();
      expect(table.querySelectorAll('th[scope="col"]').length).toBeGreaterThan(0);
    }
    // The statistics table marks each row's statistic name as a row header.
    const statsTable = screen.getByRole('table', { name: /statistics/i });
    expect(
      statsTable.querySelectorAll('th[scope="row"]').length,
    ).toBeGreaterThan(0);
    // The odds ratio is rendered through formatNumber (4 significant digits).
    expect(screen.getByRole('cell', { name: '23.45' })).toBeInTheDocument();
  });

  it('undefined_estimate_renders_dash_with_reason', () => {
    const result = twoByTwoWith({
      risk_ratio: {
        ...oswego2x2.crude.risk_ratio,
        estimate: { undefined: 'zero_cell' },
      },
    });
    const { container } = render(<>{renderTables(result)}</>);
    expect(screen.getByText('—')).toBeInTheDocument();
    expect(container.textContent).toContain('a table cell is zero');
  });
});

describe('renderFreq', () => {
  it('freq_table_has_caption_and_counts', () => {
    const { container } = render(<>{renderFreq(FREQ_ILL)}</>);
    const table = screen.getByRole('table', { name: /ILL/ });
    expect(table.querySelector('caption')).not.toBeNull();
    expect(table.querySelectorAll('th[scope="row"]').length).toBe(2);
    expect(screen.getByRole('cell', { name: '29' })).toBeInTheDocument();
    expect(container.textContent).toContain('61.33');
  });

  it('freq_renders_both_ci_methods', () => {
    const { container } = render(<>{renderFreq(FREQ_ILL)}</>);
    // Both intervals v0.2 returns appear under their own labelled,
    // scoped column headers; the `ci_method` one comes first.
    const table = container.querySelector('table') as HTMLElement;
    const headers = within(table).getAllByRole('columnheader');
    const names = headers.map((h) => h.textContent);
    expect(names).toContain('Clopper–Pearson 95% CI');
    expect(names).toContain('Wilson 95% CI');
    expect(names.indexOf('Clopper–Pearson 95% CI')).toBeLessThan(
      names.indexOf('Wilson 95% CI'),
    );
    for (const h of headers) {
      expect(h).toHaveAttribute('scope', 'col');
    }
    // Row 0 (label '0'): Clopper–Pearson 27.7–50.6, Wilson 28.4–49.9.
    expect(
      within(table).getByRole('cell', { name: '27.7 – 50.6' }),
    ).toBeInTheDocument();
    expect(
      within(table).getByRole('cell', { name: '28.4 – 49.9' }),
    ).toBeInTheDocument();
    expect(
      within(table).getByRole('cell', { name: '49.4 – 72.3' }),
    ).toBeInTheDocument();
    expect(
      within(table).getByRole('cell', { name: '50.1 – 71.6' }),
    ).toBeInTheDocument();
  });
});

describe('formatNumber', () => {
  it('small_p_value_formatting', () => {
    expect(formatNumber(0.00004)).toBe('< 0.0001');
    expect(formatNumber(0.0001)).toBe('0.0001');
    expect(formatNumber(0)).toBe('0');
    expect(formatNumber(23.454545454545453)).toBe('23.45');
    expect(formatNumber(0.7962962962962963)).toBe('0.7963');
    expect(formatNumber(1)).toBe('1');
    expect(formatNumber(100)).toBe('100');
  });
});

// `loadValidationData` (v1.0 plan Task 24, spec §7.4): the
// `/validation` page's data source — the CI-generated
// `validation.json` when the build bundled one, the wasm binding's
// `validationRegistry()` entries otherwise.

import { describe, expect, it } from 'vitest';
import {
  loadValidationData,
  type RegistryEntry,
  type ValidationReport,
} from './data';

const REGISTRY: RegistryEntry[] = [
  {
    id: 'two_by_two.odds_ratio.estimate',
    method: 'fixtures/R/twobytwo.R',
    class: 'iterative',
    cases: 7,
  },
  {
    id: 'calc.two_by_two.odds_ratio.estimate',
    method: 'fixtures/R/calc.R',
    class: 'closed_form',
    cases: 3,
  },
];

describe('loadValidationData', () => {
  it('missing_json_falls_back_to_registry', async () => {
    const data = await loadValidationData(
      async () => null,
      async () => REGISTRY,
    );
    expect(data.measured).toBe(false);
    expect(data.statistics).toEqual(REGISTRY);
    // No measured numbers exist — every rendered error is absent so
    // the page can show "observed errors not measured in this build".
    expect(data.statistics.every((s) => !('worst_error' in s))).toBe(true);
    expect(data.engine_version).toBeUndefined();
    expect(data.run_url).toBeUndefined();
  });

  it('present_json_is_used_verbatim', async () => {
    const report: ValidationReport = {
      engine_version: '1.0.0',
      commit: 'abc123',
      run_url: 'https://example.test/runs/1',
      statistics: [
        {
          id: 'freq.categories[].percent',
          method: 'fixtures/R/freq.R',
          class: 'closed_form',
          references: { r: 'R 4.4: base' },
          cases: 4,
          worst_error: 1.5e-10,
        },
      ],
    };
    const data = await loadValidationData(
      async () => report,
      async () => {
        throw new Error('registry must not be consulted');
      },
    );
    expect(data.measured).toBe(true);
    expect(data.engine_version).toBe('1.0.0');
    expect(data.commit).toBe('abc123');
    expect(data.run_url).toBe('https://example.test/runs/1');
    expect(data.statistics).toEqual(report.statistics);
  });
});

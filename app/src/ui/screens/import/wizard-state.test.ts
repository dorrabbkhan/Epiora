// The wizard reducer (v0.3 plan Task 11): the plan fixes the gates —
// an unusable key choice cannot advance, schema changes need the
// accept box, a row-position key warns, an unchanged re-import ends
// without a version.
import { describe, expect, it } from 'vitest';
import type { DiffSummary } from '@import-types/DiffSummary';
import type { ImportSettings } from '@import-types/ImportSettings';
import type { Inspection } from '@import-types/Inspection';
import type { KeyCandidate } from '@import-types/KeyCandidate';
import {
  canAdvance,
  hasSchemaChanges,
  initialWizardState,
  positionalKeyWarning,
  wizard,
  type WizardState,
} from './wizard-state';

const baseSettings = (over: Partial<ImportSettings> = {}): ImportSettings => ({
  encoding: null,
  delimiter: null,
  header: true,
  decimal: null,
  missing_codes: ['', '.', 'NA'],
  date_format: 'iso',
  sheet: null,
  column_types: {},
  key: 'row_position',
  ...over,
});

const inspection = (settings: ImportSettings): Inspection => ({
  detection: {
    container: 'text',
    encoding: 'utf8',
    delimiter: ',',
    decimal: 'dot',
    header_renames: [],
    sheets: [],
  },
  settings,
  columns: [],
  preview_rows: [],
  row_count: 2,
});

const usable = (column: string): KeyCandidate => ({
  column,
  usable: true,
  duplicates: 0,
  missing: 0,
});

/** choose → inspected lands on `preview`. */
function inspectedState(
  settings: ImportSettings,
  keyCandidates: KeyCandidate[],
  datasetId = 'ds1',
): WizardState {
  let s = wizard(initialWizardState, {
    type: 'file',
    fileName: 'f.csv',
    datasetId,
  });
  s = wizard(s, {
    type: 'inspected',
    sessionId: 'sess',
    inspection: inspection(settings),
    keyCandidates,
  });
  return s;
}

/** choose → preview → settings. */
function settingsState(
  settings: ImportSettings,
  keyCandidates: KeyCandidate[],
  datasetId = 'ds1',
): WizardState {
  const s = inspectedState(settings, keyCandidates, datasetId);
  expect(s.step).toBe('preview');
  return wizard(s, { type: 'next' });
}

const DIFF_WITH_SCHEMA_CHANGE: DiffSummary = {
  added: 1,
  removed: 0,
  changed: 0,
  schema: { added_columns: ['X'], removed_columns: [], type_changes: [] },
  unchanged: false,
  samples: {},
};

describe('wizard', () => {
  it('cannot_advance_with_duplicate_key', () => {
    // Re-import keyed on a column with duplicates: `usable` is false,
    // so Next must not move off the settings step.
    const s = settingsState(
      baseSettings({ key: { column: { name: 'ID' } } }),
      [{ column: 'ID', usable: false, duplicates: 1, missing: 0 }],
    );
    expect(s.step).toBe('settings');
    expect(canAdvance(s)).toBe(false);
    const next = wizard(s, { type: 'next' });
    expect(next.step).toBe('settings');
    expect(next.pending).toBeNull();
  });

  it('schema_change_needs_accept', () => {
    let s = settingsState(baseSettings(), [usable('ID')]);
    s = wizard(s, { type: 'next' });
    expect(s.pending).toBe('diff');
    s = wizard(s, { type: 'diffed', diff: DIFF_WITH_SCHEMA_CHANGE });
    expect(s.step).toBe('diff');
    expect(hasSchemaChanges(s.diff)).toBe(true);
    expect(canAdvance(s)).toBe(false);
    expect(wizard(s, { type: 'next' }).step).toBe('diff');

    s = wizard(s, { type: 'acceptSchema', accept: true });
    expect(canAdvance(s)).toBe(true);
    expect(wizard(s, { type: 'next' }).step).toBe('confirm');
  });

  it('row_position_key_shows_warning', () => {
    let s = settingsState(baseSettings(), [usable('ID')]);
    expect(positionalKeyWarning(s)).toBe(true);

    s = wizard(s, {
      type: 'settings',
      settings: baseSettings({ key: { column: { name: 'ID' } } }),
    });
    expect(positionalKeyWarning(s)).toBe(false);
  });

  it('unchanged_reimport_ends_without_version', () => {
    let s = settingsState(baseSettings(), [usable('ID')]);
    s = wizard(s, { type: 'next' });
    s = wizard(s, {
      type: 'diffed',
      diff: {
        added: 0,
        removed: 0,
        changed: 0,
        schema: { added_columns: [], removed_columns: [], type_changes: [] },
        unchanged: true,
        samples: {},
      },
    });
    s = wizard(s, { type: 'next' });
    expect(s.step).toBe('confirm');
    s = wizard(s, { type: 'next' });
    expect(s.step).toBe('progress');
    s = wizard(s, {
      type: 'committed',
      result: { datasetId: 'ds1', version: null },
    });
    expect(s.step).toBe('done');
    expect(s.result?.version).toBeNull();
  });

  it('close_after_done_shows_no_error', () => {
    // The wizard discards its session on close; the commit already
    // destroyed it, so `session_not_found` there must stay invisible.
    let s = settingsState(baseSettings(), [usable('ID')], 'ds1');
    s = wizard(s, { type: 'next' });
    s = wizard(s, {
      type: 'diffed',
      diff: {
        added: 1,
        removed: 0,
        changed: 0,
        schema: { added_columns: [], removed_columns: [], type_changes: [] },
        unchanged: false,
        samples: {},
      },
    });
    s = wizard(s, { type: 'next' });
    s = wizard(s, { type: 'next' });
    s = wizard(s, {
      type: 'committed',
      result: { datasetId: 'ds1', version: 2 },
    });
    expect(s.step).toBe('done');

    s = wizard(s, { type: 'close' });
    expect(s.closed).toBe(true);
    expect(s.error).toBeNull();
    expect(s.step).toBe('done');
    // Once closed the machine is inert.
    const again = wizard(s, { type: 'next' });
    expect(again).toBe(s);
  });
});

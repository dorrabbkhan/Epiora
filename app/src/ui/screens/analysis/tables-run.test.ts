// Run-gating unit tests (v0.3 plan Task 12): production builds may
// never let a TABLES run start while a two-level Text variable's
// level picks are still empty — the engine refuses the guess, so the
// dialog does too. Under `VITE_TEST_HOOKS=1` Run is allowed with
// empty pickers so `ambiguous_levels` stays reachable end to end.
import { describe, expect, it } from 'vitest';
import { canRunTables, levelsMissing, type VarDraft } from './tables-run';

const numeric = (name: string | null): VarDraft => ({
  name,
  needsLevels: false,
  levels: [null, null],
});

const twoLevelText = (
  name: string | null,
  first: string | null = null,
  second: string | null = null,
): VarDraft => ({ name, needsLevels: true, levels: [first, second] });

describe('canRunTables', () => {
  it('disabled until both variables are chosen, in either build', () => {
    const none = { exposure: numeric(null), outcome: numeric(null) };
    expect(canRunTables(none, false)).toBe(false);
    expect(canRunTables(none, true)).toBe(false);
    const half = { exposure: numeric('VANILLA'), outcome: numeric(null) };
    expect(canRunTables(half, false)).toBe(false);
    expect(canRunTables(half, true)).toBe(false);
  });

  it('enabled for non-text variables once both are chosen', () => {
    const draft = { exposure: numeric('VANILLA'), outcome: numeric('ILL') };
    expect(canRunTables(draft, false)).toBe(true);
    expect(canRunTables(draft, true)).toBe(true);
  });

  it('production run is gated until every two-level Text pick is made', () => {
    const empty = {
      exposure: twoLevelText('SEX'),
      outcome: numeric('ILL'),
    };
    expect(canRunTables(empty, false)).toBe(false);
    const partial = {
      exposure: twoLevelText('SEX', 'Female', null),
      outcome: numeric('ILL'),
    };
    expect(canRunTables(partial, false)).toBe(false);
    const filled = {
      exposure: twoLevelText('SEX', 'Female', 'Male'),
      outcome: numeric('ILL'),
    };
    expect(canRunTables(filled, false)).toBe(true);
  });

  it('production run is gated when the outcome is the two-level Text one', () => {
    const draft = {
      exposure: numeric('VANILLA'),
      outcome: twoLevelText('ILL'),
    };
    expect(canRunTables(draft, false)).toBe(false);
    expect(levelsMissing(draft)).toBe(true);
  });

  it('test hooks allow empty pickers so ambiguous_levels is reachable', () => {
    const empty = {
      exposure: twoLevelText('SEX'),
      outcome: numeric('ILL'),
    };
    expect(canRunTables(empty, true)).toBe(true);
    expect(levelsMissing(empty)).toBe(true);
  });
});

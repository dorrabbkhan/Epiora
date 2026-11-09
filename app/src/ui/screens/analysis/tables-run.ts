// TABLES Run gating (v0.3 plan Task 12; v0.2 spec §3 — a two-level
// Text variable without explicit levels is an engine error, never a
// guess). The dialog must not start a run until the user has picked
// both levels for every two-level Text variable. Only the
// `VITE_TEST_HOOKS=1` e2e build may run with empty pickers, so the
// plan's `engine_error_shows_inline_with_retry` still reaches the
// engine's `ambiguous_levels` error through the real dialog.
//
// This is a standalone module because unit tests cannot import
// TablesDialog.tsx: it pulls in `worker/client`, which spawns a
// Worker at module load.

/** One variable's dialog state. `needsLevels` is true while the
 *  column is known (or still being checked) to be two-level Text —
 *  the level pickers are shown and both picks are required. */
export interface VarDraft {
  name: string | null;
  needsLevels: boolean;
  levels: [string | null, string | null];
}

export interface TablesDraft {
  exposure: VarDraft;
  outcome: VarDraft;
}

const levelMissing = (v: VarDraft): boolean =>
  v.needsLevels && (v.levels[0] === null || v.levels[1] === null);

/** True when a required level pick is still open (the dialog shows a
 *  hint; only meaningful in production builds). */
export function levelsMissing(draft: TablesDraft): boolean {
  return levelMissing(draft.exposure) || levelMissing(draft.outcome);
}

/** True when the dialog's Run button may be enabled. `testHooks`
 *  lifts only the level requirement — a run always needs both
 *  variables. */
export function canRunTables(draft: TablesDraft, testHooks: boolean): boolean {
  if (draft.exposure.name === null || draft.outcome.name === null) {
    return false;
  }
  if (testHooks) return true;
  return !levelsMissing(draft);
}

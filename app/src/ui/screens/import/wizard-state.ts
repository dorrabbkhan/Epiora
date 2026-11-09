// The import wizard's step machine (v0.3 plan Task 11, spec §8.1):
// choose → preview → settings → (diff, re-import only) → confirm →
// progress → done | error. The reducer is pure — the route fires the
// RPC each `pending` value names and dispatches the result action.
//
// The plan fixes two gates: a key choice whose `KeyCandidate.usable`
// is false cannot advance past settings, and schema changes cannot be
// passed until the accept box is checked.

import type { DiffSummary } from '@import-types/DiffSummary';
import type { ImportSettings } from '@import-types/ImportSettings';
import type { Inspection } from '@import-types/Inspection';
import type { KeyCandidate } from '@import-types/KeyCandidate';
import type { CommitResult, ImportProgress } from '../../../worker/api';

export type WizardStep =
  | 'choose'
  | 'preview'
  | 'settings'
  | 'diff'
  | 'confirm'
  | 'progress'
  | 'done'
  | 'error';

/** Where the import lands: a new dataset, or an existing one
 *  (re-import — the flow gains the diff step). */
export type ImportTarget =
  | { kind: 'new' }
  | { kind: 'reimport'; datasetId: string };

/** The RPC the route must run next; `null` when idle. */
export type PendingCall = 'inspect' | 'preview' | 'diff' | 'commit' | null;

export interface WizardError {
  /** The stable RPC detail code (`ragged_row`, `quota_exceeded`, …). */
  detail: string;
  message: string;
}

export interface WizardState {
  step: WizardStep;
  target: ImportTarget;
  fileName: string | null;
  sessionId: string | null;
  inspection: Inspection | null;
  keyCandidates: KeyCandidate[];
  settings: ImportSettings | null;
  diff: DiffSummary | null;
  acceptSchema: boolean;
  progress: ImportProgress | null;
  result: CommitResult | null;
  error: WizardError | null;
  pending: PendingCall;
  /** Set by `close`; the machine is inert afterwards. */
  closed: boolean;
}

export const initialWizardState: WizardState = {
  step: 'choose',
  target: { kind: 'new' },
  fileName: null,
  sessionId: null,
  inspection: null,
  keyCandidates: [],
  settings: null,
  diff: null,
  acceptSchema: false,
  progress: null,
  result: null,
  error: null,
  pending: null,
  closed: false,
};

export type WizardAction =
  | { type: 'file'; fileName: string; datasetId: string | null }
  | {
      type: 'inspected';
      sessionId: string;
      inspection: Inspection;
      keyCandidates: KeyCandidate[];
    }
  | {
      type: 'previewed';
      inspection: Inspection;
      keyCandidates: KeyCandidate[];
    }
  | { type: 'settings'; settings: ImportSettings }
  | { type: 'diffed'; diff: DiffSummary }
  | { type: 'acceptSchema'; accept: boolean }
  | { type: 'back' }
  | { type: 'next' }
  | { type: 'progress'; progress: ImportProgress }
  | { type: 'committed'; result: CommitResult }
  | { type: 'failed'; detail: string; message: string }
  | { type: 'restart' }
  | { type: 'close' };

/** Schema changes needing the explicit accept (spec §6.4). */
export function hasSchemaChanges(diff: DiffSummary | null): boolean {
  if (diff === null) return false;
  const s = diff.schema;
  return (
    s.added_columns.length > 0 ||
    s.removed_columns.length > 0 ||
    s.type_changes.length > 0
  );
}

/** The chosen record key may advance: `row_position` always can; a
 *  column only when its candidate is `usable` (spec §4.4). */
function keyUsable(state: WizardState): boolean {
  const key = state.settings?.key;
  if (key === undefined || key === null || key === 'row_position') return true;
  const candidate = state.keyCandidates.find((c) => c.column === key.column.name);
  return candidate?.usable === true;
}

/** `true` while the row-position warning shows (spec §4.4: re-import
 *  becomes a positional diff; the preview warns). */
export function positionalKeyWarning(state: WizardState): boolean {
  return state.settings?.key === 'row_position';
}

/** Whether Next/Import may move the wizard on from `state.step`. */
export function canAdvance(state: WizardState): boolean {
  if (state.pending !== null) return false;
  switch (state.step) {
    case 'preview':
      return state.inspection !== null;
    case 'settings':
      return keyUsable(state);
    case 'diff':
      return !hasSchemaChanges(state.diff) || state.acceptSchema;
    case 'confirm':
      return true;
    default:
      return false;
  }
}

export function wizard(state: WizardState, action: WizardAction): WizardState {
  if (state.closed) return state;
  switch (action.type) {
    case 'file':
      // While a call is in flight a new file choice is ignored: the
      // pending inspect belongs to the file already chosen, and the
      // route disables the controls anyway.
      if (state.pending !== null) return state;
      return {
        ...initialWizardState,
        fileName: action.fileName,
        target:
          action.datasetId === null
            ? { kind: 'new' }
            : { kind: 'reimport', datasetId: action.datasetId },
        pending: 'inspect',
      };
    case 'inspected':
      return {
        ...state,
        step: 'preview',
        pending: null,
        sessionId: action.sessionId,
        inspection: action.inspection,
        keyCandidates: action.keyCandidates,
        settings: action.inspection.settings,
        error: null,
      };
    case 'previewed':
      return {
        ...state,
        pending: null,
        inspection: action.inspection,
        keyCandidates: action.keyCandidates,
        settings: action.inspection.settings,
      };
    case 'settings':
      // Any settings edit re-reads the file so the preview, the
      // coerced counts and the key candidates stay honest. While a
      // call is in flight the edit is ignored: the pending call
      // answers for the settings the form shows, so a later result
      // can never overwrite a newer edit or arm a commit the user
      // has not reviewed.
      if (state.pending !== null) return state;
      return { ...state, settings: action.settings, pending: 'preview' };
    case 'diffed':
      return { ...state, step: 'diff', diff: action.diff, pending: null };
    case 'acceptSchema':
      return { ...state, acceptSchema: action.accept };
    case 'back':
      switch (state.step) {
        case 'preview':
          return { ...state, step: 'choose' };
        case 'settings':
          return { ...state, step: 'preview' };
        case 'diff':
          return { ...state, step: 'settings' };
        case 'confirm':
          return {
            ...state,
            step: state.target.kind === 'reimport' ? 'diff' : 'settings',
          };
        default:
          return state;
      }
    case 'next':
      if (!canAdvance(state)) return state;
      switch (state.step) {
        case 'preview':
          return { ...state, step: 'settings' };
        case 'settings':
          return state.target.kind === 'reimport'
            ? { ...state, pending: 'diff' }
            : { ...state, step: 'confirm' };
        case 'diff':
          return { ...state, step: 'confirm' };
        case 'confirm':
          return {
            ...state,
            step: 'progress',
            pending: 'commit',
            progress: null,
            result: null,
          };
        default:
          return state;
      }
    case 'progress':
      return { ...state, progress: action.progress };
    case 'committed':
      return {
        ...state,
        step: 'done',
        result: action.result,
        pending: null,
        progress: null,
      };
    case 'failed':
      return {
        ...state,
        step: 'error',
        error: { detail: action.detail, message: action.message },
        pending: null,
      };
    case 'restart':
      return { ...initialWizardState, target: state.target };
    case 'close':
      return { ...state, closed: true };
  }
}

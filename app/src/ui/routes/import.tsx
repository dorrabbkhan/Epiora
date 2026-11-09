// The import wizard route (v0.3 plan Task 11, spec §8.1): the reducer
// in screens/import/wizard-state.ts owns the step machine; this screen
// fires the RPC each `pending` value names and dispatches the result.
//
// Session lifecycle: `inspectFile` opens a session the commit closes
// itself; anything earlier needs `discardSession` on leave — except a
// session already gone (committed, discarded or failed), where
// `session_not_found` is a no-op, never an error. During a running
// commit only `cancelImport` may be called: `discardSession` would
// free the wasm handle mid-write.
import { useEffect, useReducer, useRef } from 'react';
import { getRouteApi, Link } from '@tanstack/react-router';
import { useQueryClient } from '@tanstack/react-query';
import type { ImportSettings } from '@import-types/ImportSettings';
import type { ValueType } from '@import-types/ValueType';
import { Button } from '../components';
import { useDatasets } from '../queries';
import { ChooseFile } from '../screens/import/ChooseFile';
import { Confirm } from '../screens/import/Confirm';
import { DiffReview } from '../screens/import/DiffReview';
import { Preview } from '../screens/import/Preview';
import { Progress } from '../screens/import/Progress';
import { Settings } from '../screens/import/Settings';
import {
  canAdvance,
  initialWizardState,
  wizard,
} from '../screens/import/wizard-state';
import { dataApi } from '../../worker/client';
import { RpcError } from '../../worker/errors';
import { t } from '../strings';
import styles from './routes.module.css';

const routeApi = getRouteApi('/p/$project/import');

/** A discard/cancel landing after the session is already gone is a
 *  no-op, not an error (commit destroys the session itself). */
const swallowGoneSession = (e: unknown) => {
  if (e instanceof RpcError && e.detail === 'session_not_found') return;
  // Any other failure here is unactionable: the wizard is closing.
};

function asFailed(e: unknown): { detail: string; message: string } {
  if (e instanceof RpcError) return { detail: e.detail, message: e.message };
  return {
    detail: 'internal_error',
    message: e instanceof Error ? e.message : String(e),
  };
}

export function ImportScreen() {
  const { project } = routeApi.useParams();
  const datasets = useDatasets(project);
  const qc = useQueryClient();
  const [state, dispatch] = useReducer(wizard, initialWizardState);
  const stateRef = useRef(state);
  stateRef.current = state;
  const bytesRef = useRef<ArrayBuffer | null>(null);
  const inFlight = useRef(false);

  // Leave: release the session. A commit in flight must not see its
  // handle freed, so during `progress` we cancel instead of discard.
  useEffect(
    () => () => {
      const s = stateRef.current;
      if (s.sessionId === null) return;
      void (s.step === 'progress'
        ? dataApi.cancelImport(s.sessionId)
        : dataApi.discardSession(s.sessionId)
      ).catch(swallowGoneSession);
    },
    [],
  );

  // The one place `pending` becomes an RPC call.
  useEffect(() => {
    const pending = state.pending;
    if (pending === null) {
      inFlight.current = false;
      return;
    }
    if (inFlight.current) return;
    inFlight.current = true;
    const fail = (e: unknown) => {
      const f = asFailed(e);
      dispatch({ type: 'failed', detail: f.detail, message: f.message });
    };
    const s = stateRef.current;

    if (pending === 'inspect') {
      const bytes = bytesRef.current;
      const name = s.fileName;
      if (bytes === null || name === null) {
        fail(new Error('no file chosen'));
        return;
      }
      void (async () => {
        try {
          const inspected = await dataApi.inspectFile(name, bytes);
          // Re-import: saved settings apply automatically (spec §6.4);
          // `DatasetInfo.settings` is already unwrapped by the worker.
          const target = s.target;
          if (target.kind === 'reimport') {
            const info = datasets.data?.find((d) => d.id === target.datasetId);
            if (info?.settings != null) {
              const again = await dataApi.previewWithSettings(
                inspected.sessionId,
                info.settings,
              );
              dispatch({
                type: 'inspected',
                sessionId: inspected.sessionId,
                inspection: again.inspection,
                keyCandidates: again.keyCandidates,
              });
              return;
            }
          }
          dispatch({
            type: 'inspected',
            sessionId: inspected.sessionId,
            inspection: inspected.inspection,
            keyCandidates: inspected.keyCandidates,
          });
        } catch (e) {
          fail(e);
        }
      })();
    } else if (pending === 'preview') {
      if (s.sessionId === null || s.settings === null) return;
      void dataApi
        .previewWithSettings(s.sessionId, s.settings)
        .then(({ inspection, keyCandidates }) =>
          dispatch({ type: 'previewed', inspection, keyCandidates }),
        )
        .catch(fail);
    } else if (pending === 'diff') {
      if (s.sessionId === null || s.settings === null || s.target.kind !== 'reimport') {
        return;
      }
      void dataApi
        .diffAgainst(s.sessionId, s.target.datasetId, s.settings)
        .then((diff) => dispatch({ type: 'diffed', diff }))
        .catch(fail);
    } else {
      // 'commit'
      if (s.sessionId === null || s.settings === null) return;
      const target =
        s.target.kind === 'reimport'
          ? { datasetId: s.target.datasetId }
          : { projectId: project, datasetName: s.fileName ?? 'dataset' };
      void dataApi
        .commitImport(s.sessionId, target, s.settings, s.acceptSchema, (p) =>
          dispatch({ type: 'progress', progress: p }),
        )
        .then((result) => {
          dispatch({ type: 'committed', result });
          void qc.invalidateQueries();
        })
        .catch(fail);
    }
  }, [state, project, datasets.data, qc]);

  const onFile = async (file: File, datasetId: string | null) => {
    const prior = stateRef.current.sessionId;
    if (prior !== null) {
      void dataApi.discardSession(prior).catch(swallowGoneSession);
    }
    bytesRef.current = await file.arrayBuffer();
    dispatch({ type: 'file', fileName: file.name, datasetId });
  };

  const onEdit = (settings: ImportSettings) =>
    dispatch({ type: 'settings', settings });

  const onTypeChange = (name: string, type: ValueType) => {
    const s = stateRef.current.settings;
    if (s === null) return;
    onEdit({ ...s, column_types: { ...s.column_types, [name]: type } });
  };

  const onCancel = () => {
    const s = stateRef.current;
    if (s.sessionId !== null) {
      void dataApi.cancelImport(s.sessionId).catch(swallowGoneSession);
    }
  };

  const onRestart = () => {
    const s = stateRef.current;
    if (s.sessionId !== null) {
      void dataApi.discardSession(s.sessionId).catch(swallowGoneSession);
    }
    bytesRef.current = null;
    dispatch({ type: 'restart' });
  };

  const reimportName = (() => {
    const target = state.target;
    if (target.kind !== 'reimport') return null;
    return (
      datasets.data?.find((d) => d.id === target.datasetId)?.name ?? null
    );
  })();

  const showBack = ['preview', 'settings', 'diff', 'confirm'].includes(state.step);
  const showNext = ['preview', 'settings', 'diff'].includes(state.step);

  return (
    <div className={styles.page}>
      <h1>{t.import.title}</h1>
      {state.step === 'choose' && (
        <ChooseFile
          state={state}
          datasets={datasets.data ?? []}
          onFile={onFile}
        />
      )}
      {state.step === 'preview' && (
        <Preview state={state} onTypeChange={onTypeChange} />
      )}
      {state.step === 'settings' && (
        <Settings state={state} onEdit={onEdit} />
      )}
      {state.step === 'diff' && (
        <DiffReview
          state={state}
          onAccept={(accept) => dispatch({ type: 'acceptSchema', accept })}
        />
      )}
      {state.step === 'confirm' && (
        <Confirm
          state={state}
          datasetName={reimportName}
          busy={state.pending !== null}
          onImport={() => dispatch({ type: 'next' })}
        />
      )}
      {state.step === 'progress' && (
        <Progress state={state} onCancel={onCancel} />
      )}
      {state.step === 'done' && (
        <section>
          <h2>{t.import.doneTitle}</h2>
          <p>
            {state.result?.version === null
              ? t.import.doneNoVersion
              : t.import.doneVersion(state.result?.version ?? 0)}
          </p>
          <div className={styles.actions}>
            {state.result !== null && (
              <Link
                to="/p/$project/d/$dataset"
                params={{ project, dataset: state.result.datasetId }}
              >
                {t.import.viewDataset}
              </Link>
            )}
            <Link to="/p/$project" params={{ project }}>
              {t.import.backToProject}
            </Link>
          </div>
        </section>
      )}
      {state.step === 'error' && (
        <section>
          <h2>{t.import.errorTitle}</h2>
          <div role="alert">
            <p>
              {(t.errors as Record<string, string>)[
                state.error?.detail ?? 'internal_error'
              ] ?? t.errors.internal_error}
            </p>
            <p>{state.error?.message}</p>
          </div>
          <div className={styles.actions}>
            <Button onPress={onRestart}>{t.import.startOver}</Button>
            <Link to="/p/$project" params={{ project }}>
              {t.import.backToProject}
            </Link>
          </div>
        </section>
      )}
      {(showBack || showNext) && (
        <div className={styles.actions}>
          {showBack && (
            <Button
              onPress={() => dispatch({ type: 'back' })}
              isDisabled={state.pending !== null}
            >
              {t.import.back}
            </Button>
          )}
          {showNext && (
            <Button
              onPress={() => dispatch({ type: 'next' })}
              isDisabled={!canAdvance(state)}
            >
              {t.import.next}
            </Button>
          )}
        </div>
      )}
    </div>
  );
}

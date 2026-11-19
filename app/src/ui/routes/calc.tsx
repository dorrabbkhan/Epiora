// Generic calculator route (v1.0 plan Task 14, spec §7.1–7.2):
// `/calc/$id` decodes the link's inputs from its query string, marks
// malformed values and unknown keys instead of blanking, computes a
// complete valid link on load, and writes inputs back to the URL on
// calculate so every result is a shareable link. `random_numbers`
// without a `seed` draws one from the worker's
// `crypto.getRandomValues` and writes it into the URL with replace —
// the receipt then seals exactly what the link shows.

import { useEffect, useMemo, useRef, useState } from 'react';
import { Link, useNavigate, useSearch } from '@tanstack/react-router';
import { useQuery } from '@tanstack/react-query';
import type { CalculatorDef } from '@engine-types/CalculatorDef';
import type { InputDef } from '@engine-types/InputDef';
import { Button } from '../components';
import { calcApi } from '../../calc/client';
import type { CalcOutput } from '../../calc/api';
import { ct } from '../../calc/strings';
import { RpcError } from '../../worker/errors';
import { t } from '../strings';
import {
  collectInputs,
  decodeInputs,
  defaultDraft,
  draftsFromParams,
  emptyTableDraft,
  encodeInputs,
  missingErrors,
  type Drafts,
} from '../../calc/url-state';
import { CalculatorForm } from '../screens/calc/CalculatorForm';
import { CalculatorResult } from '../screens/calc/CalculatorResult';
import { CalcReceipt } from '../screens/calc/CalcReceipt';
import styles from '../screens/calc/calc.module.css';

function isTableInput(input: InputDef): boolean {
  const kind = input.kind;
  return typeof kind === 'object' && 'table' in kind;
}

/** Drafts for a fresh form: empty tables, declared defaults, then the
 *  link's raw text over the top so bad values stay visible. */
function initialDrafts(def: CalculatorDef, params: URLSearchParams): Drafts {
  const drafts: Drafts = {};
  for (const input of def.inputs) {
    if (isTableInput(input)) {
      drafts[input.name] = emptyTableDraft(input);
    } else {
      const d = defaultDraft(input);
      if (d !== undefined) drafts[input.name] = d;
    }
  }
  return { ...drafts, ...draftsFromParams(def, params) };
}

/** A link computes on load when every required input and every "one
 *  of" group has at least one parsed value. `random_numbers`'s `seed`
 *  is exempt — `run` draws it from the worker and writes it back. */
function inputsComplete(
  def: CalculatorDef,
  values: Record<string, unknown>,
): boolean {
  for (const input of def.inputs) {
    if (
      input.default === null &&
      values[input.name] === undefined &&
      !(def.id === 'random_numbers' && input.name === 'seed')
    ) {
      return false;
    }
  }
  for (const group of def.groups) {
    if (!group.members.some((m) => values[m] !== undefined)) return false;
  }
  return true;
}

interface RunResult extends CalcOutput {
  /** The inputs the run consumed — the explain calls' arguments. */
  inputs: Record<string, unknown>;
}

export function CalcScreen({ id }: { id: string }) {
  const navigate = useNavigate();
  const search = useSearch({ strict: false }) as Record<string, unknown>;
  const params = useMemo(() => {
    const p = new URLSearchParams();
    for (const [key, value] of Object.entries(search)) {
      if (typeof value === 'string') p.set(key, value);
    }
    return p;
  }, [search]);

  const registry = useQuery<CalculatorDef[]>({
    queryKey: ['calc-registry'],
    queryFn: () => calcApi().listCalculators(),
    staleTime: Number.POSITIVE_INFINITY,
  });
  const def = registry.data?.find((d) => d.id === id);

  const [drafts, setDrafts] = useState<Drafts>({});
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [output, setOutput] = useState<RunResult | null>(null);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [copied, setCopied] = useState(false);
  /** Canonical inputs of the run already triggered for this location —
   *  keeps the seed replace and the submit push from re-running. */
  const ranKey = useRef<string | null>(null);

  const run = async (values: Record<string, unknown>, replace: boolean) => {
    if (def === undefined) return;
    setBusy(true);
    setSubmitError(null);
    try {
      let v = values;
      if (def.id === 'random_numbers' && v.seed === undefined) {
        const seed = await calcApi().newSeed();
        v = { ...v, seed };
        setDrafts((d) => ({ ...d, seed: String(seed) }));
      }
      ranKey.current = JSON.stringify(v);
      void navigate({
        to: '/calc/$id',
        params: { id },
        search: Object.fromEntries(encodeInputs(def, v)),
        replace,
      });
      const out = await calcApi().run(id, v);
      setOutput({ ...out, inputs: v });
    } catch (e) {
      setOutput(null);
      setSubmitError(e instanceof RpcError ? e.userMessage : ct.runFailed);
    } finally {
      setBusy(false);
    }
  };

  // Re-decode on every location change: drafts re-init (bad text
  // stays), errors refresh, a complete link computes once.
  useEffect(() => {
    if (def === undefined) return;
    const decoded = decodeInputs(def, params);
    setDrafts(initialDrafts(def, params));
    setErrors(decoded.errors);
    setSubmitError(null);
    const fieldErrors = Object.keys(decoded.errors).filter(
      (k) => k !== '_unknown',
    );
    const canon = JSON.stringify(decoded.values);
    if (canon === ranKey.current) return;
    if (fieldErrors.length === 0 && inputsComplete(def, decoded.values)) {
      void run(decoded.values, true);
    } else {
      setOutput(null);
    }
  }, [def, params]);

  const submit = () => {
    if (def === undefined) return;
    const collected = collectInputs(def, drafts);
    const all = {
      ...missingErrors(def, collected.values, drafts),
      ...collected.errors,
    };
    if (def.id === 'random_numbers' && collected.values.seed === undefined) {
      delete all.seed;
    }
    setErrors(all);
    if (Object.keys(all).length > 0) {
      setOutput(null);
      return;
    }
    void run(collected.values, false);
  };

  const unknown = errors._unknown;
  return (
    <div className={styles.layout}>
      <h1 className={styles.title}>{ct.calculator(id)}</h1>
      <p className={styles.intro}>
        <Link to="/calc">{ct.backToIndex}</Link>
      </p>
      {registry.isPending && <p>{t.common.loading}</p>}
      {registry.data !== undefined && def === undefined && (
        <p className={styles.error} role="alert">
          {ct.unknownCalculator}
        </p>
      )}
      {unknown !== undefined && (
        <p className={styles.notice} role="alert">
          {ct.unknownKeys(unknown)}
        </p>
      )}
      {def !== undefined && (
        <CalculatorForm
          def={def}
          drafts={drafts}
          errors={errors}
          busy={busy}
          submitError={submitError}
          onDraft={(name, draft) => {
            setDrafts((d) => ({ ...d, [name]: draft }));
            setErrors((e) => {
              const next = { ...e };
              delete next[name];
              return next;
            });
          }}
          onSubmit={submit}
        />
      )}
      {output !== null && (
        <>
          <div className={styles.actions}>
            <Button
              onPress={async () => {
                await navigator.clipboard.writeText(window.location.href);
                setCopied(true);
              }}
            >
              {ct.copyLink}
            </Button>
            {copied && <span>{ct.linkCopied}</span>}
          </div>
          <CalculatorResult
            calcId={id}
            inputs={output.inputs}
            result={output.result}
            notes={output.notes}
            receipt={output.receipt}
          />
          <CalcReceipt receipt={output.receipt} />
        </>
      )}
    </div>
  );
}

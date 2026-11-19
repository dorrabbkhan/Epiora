// The explain dialog (v1.0 plan Task 16, spec §6): one reusable
// panel for calculator and Analysis result rows. Words first — the
// template's plain-language sentence — then "Show the maths"
// reveals the formula rendered as MathML with the run's own trace
// values substituted; the inputs and the result render as a table.
// Method-kind traces have no closed form, so they show the method
// words and the inputs/result table only. Every displayed number
// comes from the engine's trace; nothing is recomputed for
// display. All explain text is pending adviser review.

import { useState } from 'react';
import type { Estimate } from '@engine-types/Estimate';
import type { Explanation } from '@engine-types/Explanation';
import type { ExplainStep } from '@engine-types/ExplainStep';
import { useQuery } from '@tanstack/react-query';
import {
  Button,
  Dialog,
  Disclosure,
  DisclosurePanel,
  Heading,
  ListBox,
  ListBoxItem,
  Modal,
  ModalOverlay,
  Popover,
  Select,
  SelectValue,
} from '../ui/components';
import { RpcError } from '../worker/errors';
import { et, templateFor, type ExplainStat } from './index';
import { formatNumber } from '../ui/format';
import styles from './ExplainPanel.module.css';

export type { ExplainStat };

/** Builds the `Record<string, string>` the template's `mathml`
 *  consumes: every trace value formatted by the one number helper,
 *  plus `result` for the final step result. */
function formattedValues(step: ExplainStep): Record<string, string> {
  const v: Record<string, string> = {};
  for (const [k, n] of Object.entries(step.values)) {
    v[k] = formatNumber(n);
  }
  v.result = typeof step.result === 'number' ? formatNumber(step.result) : '—';
  return v;
}

function EstimateCell({ value }: { value: Estimate }) {
  if (typeof value === 'number') return <>{formatNumber(value)}</>;
  return <>{'—'}</>;
}

function StepView({ step, closed }: { step: ExplainStep; closed: boolean }) {
  const tpl = templateFor(step.formula);
  const mathml =
    closed && tpl !== undefined ? tpl.mathml(formattedValues(step)) : '';
  const entries = Object.entries(step.values);
  return (
    <section aria-label={et.howComputed} className={styles.step}>
      <p className={styles.words}>{tpl !== undefined ? tpl.words : step.formula}</p>
      {mathml !== '' && (
        <Disclosure className={styles.maths ?? ''}>
          <Heading level={3}>
            <Button slot="trigger">{et.showMaths}</Button>
          </Heading>
          <DisclosurePanel>
            <div
              className={styles.mathml}
              // The string comes from our own MathML builders, not
              // user or engine text — numbers only, no markup input.
              dangerouslySetInnerHTML={{ __html: mathml }}
            />
          </DisclosurePanel>
        </Disclosure>
      )}
      <table className={styles.inputs}>
        <caption>{et.inputs}</caption>
        <thead>
          <tr>
            <th scope="col">{et.inputCol}</th>
            <th scope="col">{et.valueCol}</th>
          </tr>
        </thead>
        <tbody>
          {entries.map(([k, n]) => (
            <tr key={k}>
              <th scope="row">
                <code>{k}</code>
              </th>
              <td>{formatNumber(n)}</td>
            </tr>
          ))}
          <tr>
            <th scope="row">{et.resultCol}</th>
            <td>
              <EstimateCell value={step.result} />
            </td>
          </tr>
        </tbody>
      </table>
      {tpl !== undefined && (
        <p className={styles.source}>
          {et.source}: {tpl.source}
        </p>
      )}
    </section>
  );
}

export function ExplainPanel({
  title,
  stats,
  fetchTrace,
  keyParts,
  onClose,
}: {
  /** The row label — the dialog's accessible name. */
  title: string;
  /** The statistic leaves this row covers; ≥ 1. */
  stats: ExplainStat[];
  /** Fetches the engine trace for one concrete statistic path. */
  fetchTrace: (statistic: string) => Promise<Explanation>;
  /** Identifies the trace source (run id, or calculator id + inputs)
   *  so two rows with the same statistic path never share a cached
   *  trace. */
  keyParts: readonly unknown[];
  onClose: () => void;
}) {
  const [selected, setSelected] = useState<string>(stats[0]?.path ?? '');
  const query = useQuery({
    queryKey: ['explain', ...keyParts, selected],
    queryFn: () => fetchTrace(selected),
    staleTime: Number.POSITIVE_INFINITY,
    retry: false,
    enabled: selected !== '',
  });
  const data = query.data;
  const closed = data?.kind === 'closed_form';
  return (
    <ModalOverlay
      isOpen
      onOpenChange={(open) => !open && onClose()}
      isDismissable
    >
      <Modal>
        <Dialog className={styles.dialog ?? ''}>
          <Heading slot="title">{et.explainTitle(title)}</Heading>
          <p className={styles.hint}>{et.pendingReview}</p>
          {stats.length > 1 && (
            <Select
              selectedKey={selected}
              onSelectionChange={(key) => setSelected(String(key))}
              aria-label={et.statistic}
              className={styles.picker ?? ''}
            >
              <Button>
                <SelectValue />
              </Button>
              <Popover>
                <ListBox items={stats.map((s) => ({ id: s.path, label: s.label }))}>
                  {(item) => <ListBoxItem id={item.id}>{item.label}</ListBoxItem>}
                </ListBox>
              </Popover>
            </Select>
          )}
          {query.isPending && <p>{et.loading}</p>}
          {query.isError && (
            <p className={styles.error} role="alert">
              {query.error instanceof RpcError
                ? query.error.userMessage
                : et.error}
            </p>
          )}
          {data !== undefined &&
            data.steps.map((step, i) => (
              <StepView key={i} step={step} closed={closed} />
            ))}
        </Dialog>
      </Modal>
    </ModalOverlay>
  );
}

// Generic calculator result (v1.0 plan Task 14, spec §7.2, §5.4):
// the `result` tree flattens into semantic tables — leaf statistics
// as labelled rows under section captions, scalar arrays as lists —
// every leaf carries an Explain affordance whose dialog shows the
// engine's trace (pending-review text, steps of formula/values/
// result), and the assumption notes render verbatim.

import { useState, type CSSProperties, type ReactNode } from 'react';
import type { Estimate } from '@engine-types/Estimate';
import type { Explanation } from '@engine-types/Explanation';
import type { Note } from '@engine-types/Note';
import { useQuery } from '@tanstack/react-query';
import {
  Button,
  Dialog,
  Heading,
  Modal,
  ModalOverlay,
} from '../../components';
import { RpcError } from '../../../worker/errors';
import { calcApi } from '../../../calc/client';
import { ct } from '../../../calc/strings';
import { t } from '../../strings';
import { formatNumber } from '../analysis/render-result';
import styles from './calc.module.css';

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

interface Leaf {
  /** Concrete statistic path inside the result (`strata[0].odds_ratio.est`). */
  path: string;
  label: string;
  value: Estimate;
}

interface Section {
  caption: string;
  rows: Leaf[];
  /** A scalar array renders as a list rather than a table. */
  list: Estimate[] | null;
}

function isEstimate(v: unknown): v is Estimate {
  return (
    typeof v === 'number' ||
    (typeof v === 'object' && v !== null && 'undefined' in v)
  );
}

const P_TAIL = /(^|_)p(_|$)|_p_[12]t$|fisher_[12]t$|midp(_|$)|fisher(_|$)/;

function isP(path: string): boolean {
  const last = path.split('.').pop() ?? '';
  return last === 'p' || P_TAIL.test(last);
}

function renderEstimate(value: Estimate, p: boolean): ReactNode {
  if (typeof value === 'number') return formatNumber(value, p ? 'p' : 'estimate');
  const reason =
    t.analysis.undefinedReason[
      value.undefined as keyof typeof t.analysis.undefinedReason
    ] ?? value.undefined;
  return (
    <>
      {'—'}
      <span style={VISUALLY_HIDDEN}> ({reason})</span>
    </>
  );
}

function flattenObject(obj: Record<string, unknown>, prefix: string, rel: string, out: Leaf[]): void {
  for (const [key, value] of Object.entries(obj)) {
    const path = prefix === '' ? key : `${prefix}.${key}`;
    const label = rel === '' ? key : `${rel}.${key}`;
    if (isEstimate(value)) {
      out.push({ path, label: ct.statistic(label), value });
    } else if (Array.isArray(value)) {
      value.forEach((item, i) => {
        if (isEstimate(item)) {
          out.push({
            path: `${path}[${i}]`,
            label: ct.statistic(`${label}[${i}]`),
            value: item,
          });
        } else if (typeof item === 'object' && item !== null) {
          flattenObject(
            item as Record<string, unknown>,
            `${path}[${i}]`,
            `${label}[${i}]`,
            out,
          );
        }
      });
    } else if (typeof value === 'object' && value !== null) {
      flattenObject(value as Record<string, unknown>, path, label, out);
    }
  }
}

/** The result tree → a root rows list plus one section per nested
 *  object (arrays of objects become one section per element). */
function sectionsOf(result: unknown): { root: Leaf[]; sections: Section[] } {
  const root: Leaf[] = [];
  const sections: Section[] = [];
  if (typeof result !== 'object' || result === null) return { root, sections };
  for (const [key, value] of Object.entries(result)) {
    if (isEstimate(value)) {
      root.push({ path: key, label: ct.statistic(key), value });
    } else if (Array.isArray(value)) {
      if (value.every((v) => isEstimate(v))) {
        sections.push({ caption: ct.section(key), rows: [], list: value });
      } else {
        value.forEach((item, i) => {
          if (typeof item === 'object' && item !== null) {
            const rows: Leaf[] = [];
            flattenObject(
              item as Record<string, unknown>,
              `${key}[${i}]`,
              '',
              rows,
            );
            sections.push({ caption: ct.item(key, i + 1), rows, list: null });
          }
        });
      }
    } else if (typeof value === 'object' && value !== null) {
      const rows: Leaf[] = [];
      flattenObject(value as Record<string, unknown>, key, '', rows);
      sections.push({ caption: ct.section(key), rows, list: null });
    }
  }
  return { root, sections };
}

function StatTable({
  caption,
  rows,
  onExplain,
}: {
  caption: string | null;
  rows: Leaf[];
  onExplain: (leaf: Leaf) => void;
}) {
  if (rows.length === 0) return null;
  return (
    <table>
      {caption !== null && <caption>{caption}</caption>}
      <thead>
        <tr>
          <th scope="col">{ct.statisticCol}</th>
          <th scope="col">{ct.valueCol}</th>
          <th scope="col">{ct.explain}</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((row) => (
          <tr key={row.path}>
            <th scope="row">{row.label}</th>
            <td>{renderEstimate(row.value, isP(row.path))}</td>
            <td>
              <Button onPress={() => onExplain(row)}>
                {ct.explain}
              </Button>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function ExplainDialog({
  calcId,
  inputs,
  leaf,
  onClose,
}: {
  calcId: string;
  inputs: Record<string, unknown>;
  leaf: Leaf;
  onClose: () => void;
}) {
  const statistic = `calc.${calcId}.${leaf.path}`;
  const explain = useQuery({
    queryKey: ['calc-explain', calcId, statistic, JSON.stringify(inputs)],
    queryFn: () => calcApi().explain(calcId, inputs, statistic),
    staleTime: Number.POSITIVE_INFINITY,
    retry: false,
  });
  const data = explain.data as Explanation | undefined;
  return (
    <ModalOverlay isOpen onOpenChange={(open) => !open && onClose()} isDismissable>
      <Modal>
        <Dialog>
          <Heading slot="title">{ct.explainTitle(leaf.label)}</Heading>
          <p className={styles.hint}>{ct.pendingReview}</p>
          {explain.isPending && <p>{ct.explainLoading}</p>}
          {explain.isError && (
            <p className={styles.error} role="alert">
              {explain.error instanceof RpcError
                ? explain.error.userMessage
                : t.errors.internal_error}
            </p>
          )}
          {data !== undefined && (
            <table>
              <caption>{ct.steps}</caption>
              <thead>
                <tr>
                  <th scope="col">{ct.formulaCol}</th>
                  <th scope="col">{ct.valuesCol}</th>
                  <th scope="col">{ct.resultCol}</th>
                </tr>
              </thead>
              <tbody>
                {data.steps.map((step, i) => (
                  <tr key={i}>
                    <th scope="row">
                      <code>{step.formula}</code>
                    </th>
                    <td>
                      <code>{JSON.stringify(step.values)}</code>
                    </td>
                    <td>{renderEstimate(step.result, false)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </Dialog>
      </Modal>
    </ModalOverlay>
  );
}

export function CalculatorResult({
  calcId,
  inputs,
  result,
  notes,
}: {
  calcId: string;
  /** The canonical inputs the run consumed (for explain traces). */
  inputs: Record<string, unknown>;
  result: unknown;
  notes: Note[];
}) {
  const [explaining, setExplaining] = useState<Leaf | null>(null);
  const { root, sections } = sectionsOf(result);
  return (
    <div className={styles.result}>
      <h2>{ct.results}</h2>
      <StatTable caption={null} rows={root} onExplain={setExplaining} />
      {sections.map((section) =>
        section.list !== null ? (
          <table key={section.caption}>
            <caption>{section.caption}</caption>
            <tbody>
              <tr>
                <td>
                  <ol className={styles.notes}>
                    {section.list.map((v, i) => (
                      <li key={i}>{renderEstimate(v, false)}</li>
                    ))}
                  </ol>
                </td>
              </tr>
            </tbody>
          </table>
        ) : (
          <StatTable
            key={section.caption}
            caption={section.caption}
            rows={section.rows}
            onExplain={setExplaining}
          />
        ),
      )}
      {notes.length > 0 && (
        <ul className={styles.notes}>
          {notes.map((note, i) => (
            <li key={i}>
              <code>{note.code}</code>
              {note.suggests ? ` — ${note.suggests}` : ''}
            </li>
          ))}
        </ul>
      )}
      {explaining !== null && (
        <ExplainDialog
          calcId={calcId}
          inputs={inputs}
          leaf={explaining}
          onClose={() => setExplaining(null)}
        />
      )}
    </div>
  );
}

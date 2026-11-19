// Generic calculator result (v1.0 plan Task 14, spec §7.2, §5.4):
// the `result` tree flattens into semantic tables — leaf statistics
// as labelled rows under section captions, scalar arrays as lists —
// every leaf carries an Explain affordance whose dialog shows the
// engine's trace (pending-review text, steps of formula/values/
// result). Assumption notes (v1.0 plan Task 23, spec §6.1) render in
// words beneath the section their statistics belong to — `crude`
// notes under the Crude table, a note carrying `stratum` under that
// stratum's table — with the suggested alternative anchored to its
// statistic row.

import { useState, type CSSProperties, type ReactNode } from 'react';
import type { Estimate } from '@engine-types/Estimate';
import type { Note } from '@engine-types/Note';
import type { Receipt } from '@engine-types/Receipt';
import { Button } from '../../components';
import { ExplainPanel } from '../../../explain/ExplainPanel';
import { groupNotes, noteAnchor } from '../../../explain/notes';
import { calcApi } from '../../../calc/client';
import { ct } from '../../../calc/strings';
import { t } from '../../strings';
import { formatNumber } from '../../format';
import { NotesList, type NoteLink } from '../NotesList';
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
  /** The top-level result key the section renders (`crude`,
   *  `strata[0]`, `ci`) — the block notes attach to. */
  key: string;
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
        sections.push({ key, caption: ct.section(key), rows: [], list: value });
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
            sections.push({
              key: `${key}[${i}]`,
              caption: ct.item(key, i + 1),
              rows,
              list: null,
            });
          }
        });
      }
    } else if (typeof value === 'object' && value !== null) {
      const rows: Leaf[] = [];
      flattenObject(value as Record<string, unknown>, key, '', rows);
      sections.push({ key, caption: ct.section(key), rows, list: null });
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
          // The row id anchors a note's suggested alternative to the
          // statistic that provides it.
          <tr key={row.path} id={noteAnchor(row.path)}>
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

/** A note's `suggests` — a statistic id inside this run (`calc.<id>.<block>.<leaf>`,
 *  `strata[]` a wildcard for the stratum the note flagged) or a
 *  calculator id — resolved to a link: the row anchor that displays
 *  the statistic, or the calculator's route. */
function suggestLink(calcId: string, suggests: string): NoteLink | null {
  if (!suggests.includes('.')) {
    return { href: `/calc/${suggests}`, label: ct.calculator(suggests) };
  }
  const prefix = `calc.${calcId}.`;
  const path = (
    suggests.startsWith(prefix) ? suggests.slice(prefix.length) : suggests
  ).replace('[]', '[0]');
  // The label drops the block segment so it matches the row's own
  // label (`crude.fisher_exact.p_2t` → "Fisher exact — p (2-sided)").
  const label = ct.statistic(path.split('.').slice(1).join('.'));
  return { href: `#${noteAnchor(path)}`, label };
}

function ExplainDialog({
  calcId,
  inputs,
  leaf,
  notes,
  onClose,
}: {
  calcId: string;
  inputs: Record<string, unknown>;
  leaf: Leaf;
  notes: Note[];
  onClose: () => void;
}) {
  return (
    <ExplainPanel
      title={leaf.label}
      stats={[{ path: `calc.${calcId}.${leaf.path}`, label: leaf.label }]}
      fetchTrace={(statistic) => calcApi().explain(calcId, inputs, statistic)}
      keyParts={['calc', calcId, JSON.stringify(inputs)]}
      notes={notes}
      onClose={onClose}
    />
  );
}

export function CalculatorResult({
  calcId,
  inputs,
  result,
  notes,
  receipt,
}: {
  calcId: string;
  /** The canonical inputs the run consumed (for explain traces). */
  inputs: Record<string, unknown>;
  result: unknown;
  notes: Note[];
  /** The run's sealed receipt — feeds `receipt.json` in the export. */
  receipt: Receipt;
}) {
  const [explaining, setExplaining] = useState<Leaf | null>(null);
  const [exporting, setExporting] = useState(false);
  const [exportError, setExportError] = useState<string | null>(null);
  const { root, sections } = sectionsOf(result);
  // Notes under the section their statistics belong to; a block with
  // no matching section renders once at the end.
  const notesByBlock = groupNotes(notes);
  const sectionKeys = new Set(sections.map((s) => s.key));
  const leftover = [...notesByBlock.keys()]
    .filter((key) => !sectionKeys.has(key))
    .flatMap((key) => notesByBlock.get(key) ?? []);
  const linkFor = (suggests: string): NoteLink | null =>
    suggestLink(calcId, suggests);
  // "Export to R" (v1.0 Task 17, spec §8.2): the zip is built in the
  // calc worker; the page only turns its bytes into a download.
  const exportToR = async () => {
    setExporting(true);
    setExportError(null);
    try {
      const { name, bytes } = await calcApi().exportR(calcId, inputs, receipt);
      const url = URL.createObjectURL(
        new Blob([bytes], { type: 'application/zip' }),
      );
      const a = document.createElement('a');
      a.href = url;
      a.download = name;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      setExportError(e instanceof Error ? e.message : ct.exportRFailed);
    } finally {
      setExporting(false);
    }
  };
  return (
    <div className={styles.result}>
      <h2>{ct.results}</h2>
      <div>
        <Button onPress={() => void exportToR()} isDisabled={exporting}>
          {ct.exportR}
        </Button>
        {exportError !== null && (
          <p className={styles.error} role="alert">
            {exportError}
          </p>
        )}
      </div>
      <StatTable caption={null} rows={root} onExplain={setExplaining} />
      {sections.map((section) => (
        <div key={section.key}>
          {section.list !== null ? (
            <table>
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
              caption={section.caption}
              rows={section.rows}
              onExplain={setExplaining}
            />
          )}
          <NotesList
            notes={notesByBlock.get(section.key) ?? []}
            linkFor={linkFor}
          />
        </div>
      ))}
      <NotesList notes={leftover} linkFor={linkFor} />
      {explaining !== null && (
        <ExplainDialog
          calcId={calcId}
          inputs={inputs}
          leaf={explaining}
          notes={notes}
          onClose={() => setExplaining(null)}
        />
      )}
    </div>
  );
}

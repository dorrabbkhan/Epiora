// One `/validation` table row (v1.0 plan Task 24, spec §7.4): the
// registry statistic's id — which doubles as the `/validation#<id>`
// anchor — its method, tolerance class, reference software, covering
// case count and worst observed error. Every number the cells render
// is a value the CI report's `statistics[]` entry carries; `—` is the
// only placeholder, never an invented measurement.

import { Cell, Row } from '../../components';
import { formatNumber } from '../../format';
import type { ValidationEntry } from '../../../validation/data';
import { t } from '../../strings';
import styles from './validation.module.css';

/** `references` as display text: the R version/package line plus
 *  "Epi Info 7" / "OpenEpi" when the report records them. */
function references(entry: ValidationEntry): string {
  const refs = entry.references;
  if (refs === undefined) return '';
  return [refs.r, refs.ei7, refs.openepi].filter((r) => r !== undefined).join('; ');
}

/** The tolerance class's display name — the class, not its numeric
 *  tolerance, which is not in the report JSON. */
function classLabel(entry: ValidationEntry): string {
  return entry.class === 'closed_form'
    ? t.validation.classClosedForm
    : t.validation.classIterative;
}

export function ValidationEntryRow({ entry }: { entry: ValidationEntry }) {
  const refs = references(entry);
  return (
    <Row>
      <Cell>
        {/* The `/validation#<id>` anchor (spec §7.4) — a real DOM id
            inside the cell, since RAC Row's `id` is a collection key. */}
        <span id={entry.id} className={styles.id}>
          {entry.id}
        </span>
      </Cell>
      <Cell>{entry.method}</Cell>
      <Cell>{classLabel(entry)}</Cell>
      <Cell>{refs === '' ? t.validation.notAvailable : refs}</Cell>
      <Cell>{entry.cases}</Cell>
      <Cell>
        {typeof entry.worst_error === 'number'
          ? formatNumber(entry.worst_error)
          : t.validation.notAvailable}
      </Cell>
    </Row>
  );
}

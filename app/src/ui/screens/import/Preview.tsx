// Wizard step 2 (spec §6.1–6.2): the detected format, a per-column
// table with inferred and chosen types (a `Select` per column),
// missing and coerced-to-missing counts, and the first 100 rows.
import type { PreviewColumn } from '@import-types/PreviewColumn';
import type { ValueType } from '@import-types/ValueType';
import {
  Button,
  Cell,
  Column,
  ListBox,
  ListBoxItem,
  Popover,
  Row,
  Select,
  SelectValue,
  Table,
  TableBody,
  TableHeader,
} from '../../components';
import { t } from '../../strings';
import type { WizardState } from './wizard-state';
import styles from './import.module.css';

const VALUE_TYPES: ValueType[] = ['integer', 'real', 'text', 'boolean', 'date'];

const ENCODING_LABEL: Record<string, string> = {
  utf8: 'UTF-8',
  utf16le: 'UTF-16 LE',
  utf16be: 'UTF-16 BE',
  windows1252: 'Windows-1252',
};

function TypeSelect({
  column,
  disabled,
  onTypeChange,
}: {
  column: PreviewColumn;
  disabled: boolean;
  onTypeChange: (name: string, type: ValueType) => void;
}) {
  return (
    <Select
      selectedKey={column.chosen}
      onSelectionChange={(key) => {
        if (typeof key === 'string') {
          onTypeChange(column.name, key as ValueType);
        }
      }}
      aria-label={t.import.typeFor(column.name)}
      isDisabled={disabled}
    >
      <Button>
        <SelectValue />
      </Button>
      <Popover>
        <ListBox items={VALUE_TYPES.map((v) => ({ id: v }))}>
          {(item) => <ListBoxItem id={item.id}>{item.id}</ListBoxItem>}
        </ListBox>
      </Popover>
    </Select>
  );
}

export function Preview({
  state,
  onTypeChange,
}: {
  state: WizardState;
  onTypeChange: (name: string, type: ValueType) => void;
}) {
  const inspection = state.inspection;
  if (inspection === null) return null;
  const { detection } = inspection;
  // A type change re-reads the file (`pending: 'preview'`); while that
  // call is in flight the reducer ignores further edits, so the type
  // pickers disable rather than swallow input.
  const busy = state.pending !== null;
  return (
    <section
      className={styles.section}
      aria-label={t.import.previewTitle}
      aria-busy={busy}
    >
      <h2>{t.import.previewTitle}</h2>
      <div className={styles.detection}>
        {detection.encoding !== null && (
          <p>
            {t.import.encoding}:{' '}
            {ENCODING_LABEL[detection.encoding] ?? detection.encoding}
          </p>
        )}
        {detection.delimiter !== null && (
          <p>
            {t.import.delimiter}:{' '}
            {detection.delimiter === '\t' ? 'Tab' : detection.delimiter}
          </p>
        )}
        <p>
          {t.import.decimal}: {detection.decimal}
        </p>
        <p>{t.import.rowsCount(inspection.row_count)}</p>
      </div>
      {detection.header_renames.length > 0 && (
        <p>{t.import.renamedHeaders(detection.header_renames)}</p>
      )}
      <Table caption={t.import.columnsTitle} aria-label={t.import.columnsTitle}>
        <TableHeader>
          <Column isRowHeader>{t.import.columnName}</Column>
          <Column>{t.import.inferredType}</Column>
          <Column>{t.import.chosenType}</Column>
          <Column>{t.import.missingCount}</Column>
          <Column>{t.import.coercedCount}</Column>
        </TableHeader>
        <TableBody items={inspection.columns}>
          {(col) => (
            <Row id={col.name}>
              <Cell>{col.name}</Cell>
              <Cell>{col.inferred}</Cell>
              <Cell>
                <TypeSelect
                  column={col}
                  disabled={busy}
                  onTypeChange={onTypeChange}
                />
              </Cell>
              <Cell>{col.missing}</Cell>
              <Cell>
                {col.coerced_to_missing === 0
                  ? '0'
                  : t.import.coercedValues(col.coerced_to_missing)}
              </Cell>
            </Row>
          )}
        </TableBody>
      </Table>
      <Table
        caption={t.import.previewRowsTitle}
        aria-label={t.import.previewRowsTitle}
      >
        <TableHeader columns={inspection.columns.map((c) => ({ id: c.name }))}>
          {(c) => <Column id={c.id}>{c.id}</Column>}
        </TableHeader>
        <TableBody
          items={inspection.preview_rows.map((row, i) => ({ id: i, row }))}
        >
          {(item) => (
            <Row id={item.id}>
              {item.row.map((cell, i) => (
                <Cell key={i}>{cell ?? ''}</Cell>
              ))}
            </Row>
          )}
        </TableBody>
      </Table>
      {busy && <p>{t.import.applying}</p>}
    </section>
  );
}

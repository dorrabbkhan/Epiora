// Wizard step 3 (spec §6.1–6.2, §4.4): encoding, delimiter, header,
// decimal, missing codes, date format, sheet and the record key.
// Text-only controls hide on spreadsheets; the sheet picker shows
// only when the workbook lists sheets. Every edit re-reads the file
// (the route turns `pending: 'preview'` into `previewWithSettings`).
import { useState } from 'react';
import type { DateFormat } from '@import-types/DateFormat';
import type { DecimalSep } from '@import-types/DecimalSep';
import type { Encoding } from '@import-types/Encoding';
import type { ImportSettings } from '@import-types/ImportSettings';
import type { RecordKey } from '@import-types/RecordKey';
import {
  Button,
  Checkbox,
  Input,
  Label,
  ListBox,
  ListBoxItem,
  Popover,
  Select,
  SelectValue,
  TextField,
} from '../../components';
import { t } from '../../strings';
import { positionalKeyWarning, type WizardState } from './wizard-state';
import styles from './import.module.css';

const ENCODINGS: { id: Encoding; label: string }[] = [
  { id: 'utf8', label: 'UTF-8' },
  { id: 'utf16le', label: 'UTF-16 LE' },
  { id: 'utf16be', label: 'UTF-16 BE' },
  { id: 'windows1252', label: 'Windows-1252' },
];

const DELIMITERS = [
  { id: ',', label: 'Comma (,)' },
  { id: ';', label: 'Semicolon (;)' },
  { id: '\t', label: 'Tab' },
  { id: '|', label: 'Pipe (|)' },
];

const DECIMALS: { id: DecimalSep; label: string }[] = [
  { id: 'dot', label: 'Dot (.)' },
  { id: 'comma', label: 'Comma (,)' },
];

const DATE_FORMATS: { id: DateFormat; label: string }[] = [
  { id: 'iso', label: t.import.dateIso },
  { id: 'dmy', label: t.import.dateDmy },
  { id: 'mdy', label: t.import.dateMdy },
];

function WizardSelect({
  label,
  items,
  selectedKey,
  disabled,
  onChange,
}: {
  label: string;
  items: { id: string; label: string }[];
  selectedKey: string;
  disabled: boolean;
  onChange: (id: string) => void;
}) {
  return (
    <Select
      selectedKey={selectedKey}
      onSelectionChange={(key) => {
        if (typeof key === 'string') onChange(key);
      }}
      aria-label={label}
      isDisabled={disabled}
    >
      <Label>{label}</Label>
      <Button>
        <SelectValue />
      </Button>
      <Popover>
        <ListBox items={items}>
          {(item) => <ListBoxItem id={item.id}>{item.label}</ListBoxItem>}
        </ListBox>
      </Popover>
    </Select>
  );
}

export function Settings({
  state,
  onEdit,
}: {
  state: WizardState;
  onEdit: (settings: ImportSettings) => void;
}) {
  const settings = state.settings;
  const detection = state.inspection?.detection;
  const [codes, setCodes] = useState(
    () => settings?.missing_codes.join(', ') ?? '',
  );
  if (settings === null || detection === undefined) {
    return null;
  }
  const isText = detection.container === 'text';
  // While a call is in flight every control is disabled: the reducer
  // ignores edits then, so leaving them enabled would swallow input
  // the screen shows no sign of losing.
  const busy = state.pending !== null;
  const edit = (over: Partial<ImportSettings>) =>
    onEdit({ ...settings, ...over });

  const keyItems = [
    { id: 'row_position', label: t.import.keyRowPosition },
    ...state.keyCandidates.map((c) => ({
      id: `column:${c.column}`,
      label: c.usable
        ? c.column
        : `${c.column} (${t.import.keyCandidateIssues(c.duplicates, c.missing)})`,
    })),
  ];
  const recordKey = settings.key;
  const chosenKey =
    recordKey === 'row_position'
      ? 'row_position'
      : `column:${recordKey.column.name}`;
  const chosenCandidate =
    recordKey === 'row_position'
      ? undefined
      : state.keyCandidates.find((c) => c.column === recordKey.column.name);

  return (
    <section
      className={styles.section}
      aria-label={t.import.settingsTitle}
      aria-busy={busy}
    >
      <h2>{t.import.settingsTitle}</h2>
      <div className={styles.form}>
        {isText && (
          <>
            <WizardSelect
              label={t.import.encoding}
              disabled={busy}
              items={[
                { id: 'auto', label: t.import.detectAuto },
                ...ENCODINGS.map((e) => ({ id: e.id, label: e.label })),
              ]}
              selectedKey={settings.encoding ?? 'auto'}
              onChange={(id) =>
                edit({ encoding: id === 'auto' ? null : (id as Encoding) })
              }
            />
            <WizardSelect
              label={t.import.delimiter}
              disabled={busy}
              items={[
                { id: 'auto', label: t.import.detectAuto },
                ...DELIMITERS,
              ]}
              selectedKey={settings.delimiter ?? 'auto'}
              onChange={(id) => edit({ delimiter: id === 'auto' ? null : id })}
            />
            <WizardSelect
              label={t.import.decimal}
              disabled={busy}
              items={[
                { id: 'auto', label: t.import.detectAuto },
                ...DECIMALS.map((d) => ({ id: d.id, label: d.label })),
              ]}
              selectedKey={settings.decimal ?? 'auto'}
              onChange={(id) =>
                edit({ decimal: id === 'auto' ? null : (id as DecimalSep) })
              }
            />
          </>
        )}
        <Checkbox
          isSelected={settings.header}
          onChange={(header) => edit({ header })}
          isDisabled={busy}
        >
          {t.import.headerRow}
        </Checkbox>
        <TextField
          value={codes}
          onChange={setCodes}
          isDisabled={busy}
          onBlur={() =>
            edit({
              missing_codes: codes
                .split(',')
                .map((s) => s.trim())
                .filter((s) => s !== ''),
            })
          }
        >
          <Label>{t.import.missingCodes}</Label>
          <Input />
          <p>{t.import.missingCodesHelp}</p>
        </TextField>
        <WizardSelect
          label={t.import.dateFormat}
          disabled={busy}
          items={DATE_FORMATS.map((d) => ({ id: d.id, label: d.label }))}
          selectedKey={settings.date_format}
          onChange={(id) => edit({ date_format: id as DateFormat })}
        />
        {detection.sheets.length > 0 && (
          <WizardSelect
            label={t.import.sheet}
            disabled={busy}
            items={detection.sheets.map((s) => ({ id: s, label: s }))}
            selectedKey={settings.sheet ?? detection.sheets[0]!}
            onChange={(id) => edit({ sheet: id })}
          />
        )}
        <WizardSelect
          label={t.import.recordKey}
          disabled={busy}
          items={keyItems}
          selectedKey={chosenKey}
          onChange={(id) => {
            const key: RecordKey =
              id === 'row_position'
                ? 'row_position'
                : { column: { name: id.slice('column:'.length) } };
            edit({ key });
          }}
        />
        {chosenCandidate !== undefined && !chosenCandidate.usable && (
          <p className={styles.warning} role="alert">
            {t.import.keyUnusable}
          </p>
        )}
        {positionalKeyWarning(state) && (
          <p className={styles.warning}>{t.import.positionalKeyWarning}</p>
        )}
        {busy && <p>{t.import.applying}</p>}
      </div>
    </section>
  );
}

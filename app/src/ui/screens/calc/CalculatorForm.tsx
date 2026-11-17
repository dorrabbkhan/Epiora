// Generic calculator form (v1.0 plan Task 14, spec §7.2): every field
// is rendered from the registry `CalculatorDef` — scalar inputs as
// text fields, choices as selects, tables as the draft grid — so a
// link's declared-but-malformed values stay visible and marked, and
// "one of" input groups carry their hint and shared error.

import type { CalculatorDef } from '@engine-types/CalculatorDef';
import type { InputDef } from '@engine-types/InputDef';
import {
  Button,
  FieldError,
  Input,
  Label,
  ListBox,
  ListBoxItem,
  Popover,
  Select,
  SelectValue,
  TextField,
} from '../../components';
import { ct } from '../../../calc/strings';
import { emptyTableDraft, type Drafts } from '../../../calc/url-state';
import { TableInput } from './TableInput';
import styles from './calc.module.css';

function isTable(input: InputDef): boolean {
  const kind = input.kind;
  return typeof kind === 'object' && 'table' in kind;
}

function optionsOf(input: InputDef): string[] | null {
  const kind = input.kind;
  return typeof kind === 'object' && 'choice' in kind
    ? kind.choice.options
    : null;
}

/** The error keyed to `input` — its own name, or the joined member
 *  names of a "one of" group it belongs to. */
function errorFor(
  def: CalculatorDef,
  input: InputDef,
  errors: Record<string, string>,
): string | undefined {
  if (errors[input.name] !== undefined) return errors[input.name];
  const group = def.groups.find((g) => g.members.includes(input.name));
  if (group !== undefined) return errors[group.members.join('/')];
  return undefined;
}

export function CalculatorForm({
  def,
  drafts,
  errors,
  busy,
  submitError,
  onDraft,
  onSubmit,
}: {
  def: CalculatorDef;
  drafts: Drafts;
  errors: Record<string, string>;
  busy: boolean;
  submitError: string | null;
  onDraft: (name: string, draft: Drafts[string]) => void;
  onSubmit: () => void;
}) {
  const groupsSeen = new Set<string>();
  return (
    <form
      className={styles.form}
      onSubmit={(e) => {
        e.preventDefault();
        onSubmit();
      }}
    >
      <div className={styles.fields}>
        {def.inputs.map((input) => {
          const reason = errorFor(def, input, errors);
          const message =
            reason !== undefined
              ? (ct.errors[reason] ?? ct.errors.out_of_range)
              : undefined;
          const group = def.groups.find((g) =>
            g.members.includes(input.name),
          );
          let hint: string | null = null;
          if (group !== undefined && !groupsSeen.has(group.members.join('/'))) {
            groupsSeen.add(group.members.join('/'));
            hint = ct.groupHint(group.members);
          }
          if (isTable(input)) {
            return (
              <div key={input.name}>
                <TableInput
                  input={input}
                  draft={
                    (drafts[input.name] as string[][][] | undefined) ??
                    emptyTableDraft(input)
                  }
                  invalid={message !== undefined}
                  disabled={busy}
                  onChange={(tables) => onDraft(input.name, tables)}
                />
                {message !== undefined && (
                  <p className={styles.error} role="alert">
                    {message}
                  </p>
                )}
              </div>
            );
          }
          const options = optionsOf(input);
          if (options !== null) {
            return (
              <div key={input.name} className={styles.field}>
                <Select
                  aria-label={ct.input(input.name)}
                  selectedKey={
                    typeof drafts[input.name] === 'string' &&
                    drafts[input.name] !== ''
                      ? (drafts[input.name] as string)
                      : null
                  }
                  isInvalid={message !== undefined}
                  isDisabled={busy}
                  onSelectionChange={(key) =>
                    onDraft(input.name, String(key))
                  }
                >
                  <Label>{ct.input(input.name)}</Label>
                  <Button>
                    <SelectValue />
                  </Button>
                  <Popover>
                    <ListBox items={options.map((o) => ({ id: o, label: o }))}>
                      {(item) => (
                        <ListBoxItem id={item.id}>{item.label}</ListBoxItem>
                      )}
                    </ListBox>
                  </Popover>
                </Select>
                {hint !== null && <p className={styles.hint}>{hint}</p>}
                {message !== undefined && (
                  <p className={styles.error} role="alert">
                    {message}
                  </p>
                )}
              </div>
            );
          }
          return (
            <div key={input.name} className={styles.field}>
              <TextField
                value={
                  typeof drafts[input.name] === 'string'
                    ? (drafts[input.name] as string)
                    : ''
                }
                isInvalid={message !== undefined}
                isDisabled={busy}
                onChange={(text) => onDraft(input.name, text)}
              >
                <Label>
                  {ct.input(input.name)}
                  {input.unit !== null ? ` (${input.unit})` : ''}
                </Label>
                <Input />
                {message !== undefined && (
                  <FieldError>{message}</FieldError>
                )}
              </TextField>
              {hint !== null && <p className={styles.hint}>{hint}</p>}
            </div>
          );
        })}
      </div>
      {submitError !== null && (
        <p className={styles.error} role="alert">
          {submitError}
        </p>
      )}
      <div className={styles.actions}>
        <Button type="submit" isDisabled={busy}>
          {busy ? ct.calculating : ct.calculate}
        </Button>
      </div>
    </form>
  );
}

// Table input (v1.0 plan Task 14, spec §7.2): the generic grid for a
// `table` InputKind — a `2×2` (or `2×3`, `r×c`, `2×(M+1)`) block per
// stratum, rows/cells as raw text drafts so a malformed link keeps
// its bad text visible and marked. Row/column/stratum add-remove
// buttons respect the declared bounds; the whole grid is marked
// invalid when the draft fails to parse.

import { useId } from 'react';
import type { InputDef } from '@engine-types/InputDef';
import { Button, Input, TextField } from '../../components';
import { ct } from '../../../calc/strings';
import styles from './calc.module.css';

interface TableSpec {
  cols: number;
  min_rows: number;
  max_rows: number;
  strata: number | null;
}

function specOf(input: InputDef): TableSpec {
  const kind = input.kind;
  if (typeof kind !== 'object' || !('table' in kind)) {
    throw new Error(`TableInput on non-table input ${input.name}`);
  }
  return kind.table;
}

export function TableInput({
  input,
  draft,
  invalid,
  disabled,
  onChange,
}: {
  input: InputDef;
  /** List of tables of raw cell text (one table for flat inputs). */
  draft: string[][][];
  invalid: boolean;
  disabled: boolean;
  onChange: (tables: string[][][]) => void;
}) {
  const spec = specOf(input);
  const baseId = useId();
  const label = ct.input(input.name);
  const isStrata = spec.strata !== null;
  const width = draft[0]?.[0]?.length ?? (spec.cols === 0 ? 2 : spec.cols);
  const rows = draft[0]?.length ?? spec.min_rows;

  const setCell = (t: number, r: number, c: number, text: string) => {
    onChange(
      draft.map((table, ti) =>
        table.map((row, ri) =>
          row.map((cell, ci) => (ti === t && ri === r && ci === c ? text : cell)),
        ),
      ),
    );
  };
  const addRow = () => {
    if (rows >= spec.max_rows) return;
    onChange(
      draft.map((table) => [...table, Array<string>(width).fill('')]),
    );
  };
  const removeRow = () => {
    if (rows <= spec.min_rows) return;
    onChange(draft.map((table) => table.slice(0, -1)));
  };
  const addColumn = () => {
    if (spec.cols !== 0 || width >= 10) return;
    onChange(draft.map((table) => table.map((row) => [...row, ''])));
  };
  const removeColumn = () => {
    if (spec.cols !== 0 || width <= 2) return;
    onChange(draft.map((table) => table.map((row) => row.slice(0, -1))));
  };
  const addStratum = () => {
    if (!isStrata || draft.length >= (spec.strata ?? 1)) return;
    onChange([...draft, Array.from({ length: spec.min_rows }, () => Array<string>(width).fill(''))]);
  };
  const removeStratum = () => {
    if (!isStrata || draft.length <= 1) return;
    onChange(draft.slice(0, -1));
  };

  return (
    <div className={styles.tableWrap} aria-invalid={invalid || undefined}>
      <fieldset>
        <legend>{label}</legend>
        <div className={styles.strata}>
          {draft.map((table, ti) => (
            <table key={`${baseId}-${ti}`} className={styles.grid}>
              {isStrata && <caption>{ct.stratum(ti + 1)}</caption>}
              <thead>
                <tr>
                  <th scope="col">{ct.rowCol}</th>
                  {table[0]?.map((_, ci) => (
                    <th key={ci} scope="col">
                      {ci + 1}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {table.map((row, ri) => (
                  <tr key={ri}>
                    <th scope="row">{ri + 1}</th>
                    {row.map((cell, ci) => (
                      <td key={ci}>
                        <TextField
                          aria-label={ct.cellLabel(
                            isStrata
                              ? `${label} ${ct.stratum(ti + 1)}`
                              : label,
                            ri + 1,
                            ci + 1,
                          )}
                          value={cell}
                          isInvalid={invalid}
                          isDisabled={disabled}
                          onChange={(text) => setCell(ti, ri, ci, text)}
                        >
                          <Input />
                        </TextField>
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          ))}
        </div>
        <div className={styles.tableActions}>
          {rows < spec.max_rows && (
            <Button onPress={addRow} isDisabled={disabled}>
              {ct.addRow}
            </Button>
          )}
          {rows > spec.min_rows && (
            <Button onPress={removeRow} isDisabled={disabled}>
              {ct.removeRow}
            </Button>
          )}
          {spec.cols === 0 && width < 10 && (
            <Button onPress={addColumn} isDisabled={disabled}>
              {ct.addColumn}
            </Button>
          )}
          {spec.cols === 0 && width > 2 && (
            <Button onPress={removeColumn} isDisabled={disabled}>
              {ct.removeColumn}
            </Button>
          )}
          {isStrata && draft.length < (spec.strata ?? 1) && (
            <Button onPress={addStratum} isDisabled={disabled}>
              {ct.addStratum}
            </Button>
          )}
          {isStrata && draft.length > 1 && (
            <Button onPress={removeStratum} isDisabled={disabled}>
              {ct.removeStratum}
            </Button>
          )}
        </div>
      </fieldset>
    </div>
  );
}

// The Epi Info 7 field-type tables and the grid child-dataset builder
// for the golden imports (v1.1 Task 4) — extracted from
// ei7-project.mjs to keep that file under the ~400-line cap. A grid
// field's rows live in `<page table><FieldName>`; its columns are the
// metaGridColumns rows for the FieldId, ordered by GridColumnId, with
// UniqueKey prepended when the physical table has it.

import { ci, convertCell, isIntegralColumn } from './legacy-values.mjs';

export const FIELD_TYPE = {
  1: 'Text', 2: 'LabelTitle', 3: 'TextUppercase', 4: 'Multiline', 5: 'Number',
  6: 'PhoneNumber', 7: 'Date', 8: 'Time', 9: 'DateTime', 10: 'Checkbox',
  11: 'YesNo', 12: 'Option', 13: 'CommandButton', 14: 'Image', 15: 'Mirror',
  16: 'Grid', 17: 'LegalValues', 18: 'Codes', 19: 'CommentLegal', 20: 'Relate',
  21: 'Group', 22: 'RecStatus', 23: 'UniqueKey', 24: 'ForeignKey', 25: 'GUID',
  26: 'GlobalRecordId', 27: 'List', 28: 'UniqueRowId', 99: 'Unknown',
};

// Field-type id → how the field lands in the dataset. 'meta' = form
// metadata only (report entry); 'link' = no column, a project link;
// 'grid' = a linked child dataset.
export const TYPE_KIND = {
  1: 'text', 3: 'text', 4: 'text', 6: 'text', 5: 'number', 7: 'date', 8: 'time',
  9: 'date_time', 10: 'boolean', 11: 'boolean', 12: 'option', 15: 'mirror',
  17: 'coded', 18: 'codes', 19: 'coded', 25: 'text', 26: 'system_text',
  22: 'system_int', 23: 'system_int', 24: 'system_fkey', 28: 'system_text',
  2: 'meta', 13: 'meta', 14: 'binary', 21: 'meta', 20: 'link', 16: 'grid',
};

// One grid field (metaFields kind 'grid') → the child dataset plus the
// project link that reaches it. `rowsOf`/`colOf`/`claimed`/`add`/
// `labelSetFor` are the mapper's helpers, passed in.
export function buildGridDataset({ f, viewName, tableName, gridCols, rowsOf, colOf, claimed, add, labelSetFor }) {
  const gridTable = `${tableName}${f.PageId}${f.Name}`;
  claimed.add(gridTable.toLowerCase());
  const gFields = gridCols.filter((g) => String(g.FieldId) === String(f.FieldId));
  const gRows = rowsOf(gridTable) || [];
  if (rowsOf(gridTable) === null) add('view_data_missing', viewName, `grid table ${gridTable} is not in the file`);
  const gCols = [];
  // UniqueKey is the grid table's identity column but never a
  // metaGridColumns row — prepend it when present.
  if (colOf(gridTable, 'UniqueKey')) {
    gCols.push({
      name: 'UniqueKey',
      type: 'integer',
      values: gRows.map((r) => convertCell(ci(r, 'UniqueKey'), 'integer')),
      meta: { legacy_type: 'UniqueKey' },
    });
  }
  for (const g of gFields.sort((a, b) => Number(a.GridColumnId ?? 0) - Number(b.GridColumnId ?? 0))) {
    const gt = Number(g.FieldTypeId);
    const gk = TYPE_KIND[gt];
    const col = colOf(gridTable, g.Name);
    const gvals = gRows.map((r) => (col ? ci(r, col) : null));
    const type =
      gk === 'system_int'
        ? 'integer'
        : gk === 'number'
          ? String(g.Pattern || '').includes('.') || !isIntegralColumn(gvals)
            ? 'real'
            : 'integer'
          : gk === 'boolean'
            ? 'boolean'
            : gk === 'date' || gk === 'time' || gk === 'date_time'
              ? gk
              : 'text';
    const gmeta = { prompt: g.Text || undefined, legacy_type: FIELD_TYPE[gt] || `type${gt}` };
    if (gk === 'coded' || gk === 'codes' || gk === 'option') {
      const lid = labelSetFor(g);
      if (lid) gmeta.label_set = lid;
    }
    gCols.push({
      name: g.Name,
      type,
      values: gvals.map((x) => convertCell(x, type)),
      meta: gmeta,
    });
  }
  return {
    dataset: { id: gridTable, version: 1, columns: gCols, file: `${gridTable}.dataset.json`, label_sets: [] },
    link: { parent: viewName, child: gridTable, child_key: 'FKEY', parent_key: 'GlobalRecordId', kind: 'grid' },
  };
}

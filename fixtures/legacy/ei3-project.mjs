// The Epi Info 3.5.x layout → the same golden project model as EI7
// (v1.1 Task 4, spec §8.2). EI3 stores one `view<Name>` field-
// definition table per view; a `SOURCE` row names the data table
// (`Datatable` of the row named like DATA1); field rows carry
// PageNumber > 0. Record identity is `UniqueKey`, deleted rows have
// RECSTATUS 0. Implemented independently of `epiora-legacy`.

import { ci, convertCell, unknownLayoutEntries } from './legacy-values.mjs';

const EI3_TYPE = {
  TEXTBOX: 'text',
  NUMBER: 'number',
  DATE: 'date',
  PHONENUMBER: 'text',
  CHECKBOX: 'boolean',
  'YES/NO': 'boolean',
  COMBO: 'coded',
  LISTBOX: 'coded',
  MIRROR: 'mirror',
};

// `Lists` cell forms: "C(SRC,col*col[,col2*col2])", "W(SRC,col)",
// "L(SRC,...)" — SRC is a SOURCE row name (CODE1, DATA1, …) whose
// Datatable is the code table; columns are stored*display pairs.
// Returns {source, cols:[[stored,display],...]}.
function parseEi3List(list) {
  const m = String(list || '').match(/^\s*([A-Z])\s*\(\s*([^,)]+)\s*,\s*([^)]+)\)/i);
  if (!m) return null;
  const cols = m[3].split(',').map((p) => {
    const [a, b] = p.split('*').map((s) => s.trim());
    return [a, b || a];
  });
  return { source: m[2].trim(), cols };
}

const isPseudoRow = (r) => {
  const type = String(ci(r, 'Type') || '').trim();
  const name = String(ci(r, 'Name') || '').trim();
  const page = Number(ci(r, 'PageNumber'));
  return (
    type === '' ||
    type === 'SOURCE' ||
    type === 'RECORD' ||
    /^Background$/i.test(type) ||
    ['RECORD', 'VIEW', 'GENERAL'].includes(name.toUpperCase()) ||
    !(page > 0)
  );
};

export function buildEi3Project(db, opts) {
  const report = [];
  const add = (kind, item, reason) => report.push({ kind, item, reason });
  const tableSet = new Set(db.tables().map((t) => t.toLowerCase()));
  const viewTables = db
    .tables()
    .filter((t) => /^view/i.test(t) && db.columns(t).some((c) => /^Type$/i.test(c.name)));

  if (opts.jet === 'JET3') {
    add('code_page_assumed', opts.stem, 'JET3 text decoded as windows-1252 (MDB_JET3_CHARSET)');
  }

  const claimed = new Set(viewTables.map((t) => t.toLowerCase()));
  const datasets = [];
  const views = [];
  const links = [];
  const checkCode = [];

  if (viewTables.length === 0) {
    for (const e of unknownLayoutEntries(db, opts.stem)) add(e.kind, e.item, e.reason);
    return {
      name: opts.stem,
      legacy_id: null,
      datasets,
      views,
      links,
      programs: [],
      check_code: [],
      report,
    };
  }

  const labelSets = new Map();
  const rowsOf = (t) => (tableSet.has(t.toLowerCase()) ? db.rows(t) : null);
  const colOf = (t, name) => {
    const hit = (db.columns(t) || []).find((c) => c.name.toLowerCase() === name.toLowerCase());
    return hit ? hit.name : null;
  };

  for (const vt of viewTables) {
    const defRows = db.rows(vt);
    const viewName = vt.replace(/^view/i, '');
    // SOURCE rows name external sources; the view's data table is the
    // DATA1 row (CODE* rows point at code tables).
    const src =
      defRows.find(
        (r) =>
          String(ci(r, 'Type') || '').toUpperCase() === 'SOURCE' &&
          /^data\d*$/i.test(String(ci(r, 'Name') || '').trim())
      ) ||
      defRows.find(
        (r) =>
          String(ci(r, 'Type') || '').toUpperCase() === 'SOURCE' &&
          !/^code/i.test(String(ci(r, 'Datatable') || ''))
      );
    const dataTable = src ? String(ci(src, 'Datatable') || viewName) : viewName;
    claimed.add(dataTable.toLowerCase());
    // SOURCE-name → Datatable, for `X(SRC,cols)` field references.
    const sourceMap = new Map(
      defRows
        .filter((r) => String(ci(r, 'Type') || '').toUpperCase() === 'SOURCE')
        .map((r) => [String(ci(r, 'Name') || '').trim().toUpperCase(), String(ci(r, 'Datatable') || '')])
    );
    const dataRows = rowsOf(dataTable);
    if (dataRows === null) {
      add('view_data_missing', viewName, `data table ${dataTable} is not in the file`);
    }
    const data = dataRows || [];
    const fieldRows = defRows.filter((r) => !isPseudoRow(r));

    const columns = [];
    for (const fr of fieldRows) {
      const name = String(ci(fr, 'Name') || '').trim();
      const type = String(ci(fr, 'Type') || '').trim().toUpperCase();
      const kind = EI3_TYPE[type];
      const meta = {
        prompt: ci(fr, 'PromptText') || undefined,
        legacy_type: type,
      };
      if (!kind) {
        add('field_not_imported', `${viewName}/${name}`, `EI3 field type ${type || '(empty)'} is form metadata`);
        continue;
      }
      const colName = colOf(dataTable, name);
      if (colName === null && kind !== 'mirror') {
        add('field_column_missing', `${viewName}/${name}`, `no column in ${dataTable}`);
      }
      if (colName && colName !== name) meta.legacy_name = colName;
      const raw = data.map((r) => (colName ? ci(r, colName) : null));
      let etype = 'text';
      if (kind === 'number') {
        const hasDecimals = Number(ci(fr, 'Decimals')) > 0 || String(ci(fr, 'Formatstring') || '').includes('.');
        const integral = raw.every((v) => v === null || v === undefined || v === '' || Number.isInteger(Number(v)));
        etype = hasDecimals || !integral ? 'real' : 'integer';
      } else if (kind === 'boolean' || kind === 'date') {
        etype = kind;
      } else if (kind === 'mirror') {
        const srcName = String(ci(fr, 'Lists') ?? ci(fr, 'List') ?? '').trim();
        const srcField = fieldRows.find((x) => String(ci(x, 'Name') || '').toLowerCase() === srcName.toLowerCase());
        const srcCol = srcField ? colOf(dataTable, String(ci(srcField, 'Name'))) : null;
        if (!srcCol) {
          add('field_column_missing', `${viewName}/${name}`, `mirror source ${srcName || '(unset)'} not found`);
        } else {
          raw.splice(0, raw.length, ...data.map((r) => ci(r, srcCol)));
          add('field_imported_changed', `${viewName}/${name}`, `MIRROR field imported as the value of ${srcName}`);
        }
      } else if (kind === 'coded') {
        const spec = parseEi3List(ci(fr, 'Lists') ?? ci(fr, 'List'));
        if (spec) {
          const table = sourceMap.get(spec.source.toUpperCase()) || spec.source;
          const rows = rowsOf(table);
          if (rows === null) {
            add('code_table_missing', table, `field ${name} references a code table that is not in the file`);
          } else {
            claimed.add(table.toLowerCase());
            const [stored, display] = spec.cols[0];
            const id = spec.cols.length > 1 ? `${table}#${stored}->${display}` : table;
            if (!labelSets.has(id)) {
              const entries = [];
              const seen = new Set();
              for (const r of rows) {
                const v = ci(r, stored);
                if (v === null || v === undefined || v === '') continue;
                const key = `${v}`;
                if (seen.has(key)) continue;
                seen.add(key);
                entries.push([{ type: 'text', value: String(v) }, String(ci(r, display) ?? v)]);
              }
              labelSets.set(id, { id, entries });
            }
            meta.label_set = id;
          }
        }
      }
      columns.push({
        name,
        type: etype,
        values: raw.map((x) => convertCell(x, etype)),
        meta: Object.fromEntries(Object.entries(meta).filter(([, x]) => x !== undefined)),
      });
    }

    // System columns travel verbatim as fields: UniqueKey, RECSTATUS,
    // FKEY when present.
    for (const sys of ['UniqueKey', 'RECSTATUS', 'FKEY']) {
      const c = colOf(dataTable, sys);
      if (!c) continue;
      const type = sys === 'FKEY' ? 'text' : 'integer';
      columns.push({
        name: c,
        type,
        values: data.map((r) => convertCell(ci(r, c), type)),
        meta: { legacy_type: sys },
      });
    }

    const recCol = colOf(dataTable, 'RECSTATUS');
    const deleted = recCol ? data.filter((r) => Number(ci(r, recCol)) === 0).length : 0;
    datasets.push({ id: viewName, version: 1, columns, file: `${viewName}.dataset.json`, label_sets: [] });
    views.push({ name: viewName, dataset: `${viewName}.dataset.json`, records: data.length, deleted_records: deleted });

    const blocks = fieldRows.filter((r) => String(ci(r, 'Checkcode') || '').trim() !== '').length;
    if (blocks > 0) checkCode.push({ view: viewName, blocks });
  }

  // Programs table → saved programs (EI3 programs live in a `Programs`
  // table with Name + a code column).
  const programs = [];
  if (db.hasTable('Programs')) {
    claimed.add('programs');
    for (const r of db.rows('Programs')) {
      const name = ci(r, 'Name') ?? ci(r, 'ProgramName') ?? '';
      const text = ci(r, 'Code') ?? ci(r, 'Program') ?? ci(r, 'Content') ?? '';
      if (name !== '') programs.push({ name: String(name), content: String(text) });
    }
  }

  for (const t of [...db.tables()].sort((a, b) => a.toLowerCase().localeCompare(b.toLowerCase()))) {
    const lt = t.toLowerCase();
    if (claimed.has(lt) || lt.startsWith('meta')) continue;
    if (/^code/i.test(lt) || labelSets.has(t)) {
      add('table_not_imported', t, 'unreferenced code table');
    } else {
      add('table_not_imported', t, 'no view or code table claims it');
    }
  }

  return {
    name: opts.stem,
    legacy_id: null,
    datasets,
    views,
    links,
    programs,
    check_code: checkCode,
    report,
    label_sets: [...labelSets.values()],
  };
}

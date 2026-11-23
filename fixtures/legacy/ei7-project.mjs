// The Epi Info 7 layout → golden project model (v1.1 Task 4, spec
// §8.2). Implemented from the Epi.Core source at the D2 commit —
// `MetadataDbProvider.GetDataTableName` (a view's base table is the
// first non-empty `metaFields.DataTableName`), `Page.TableName`
// (`view.TableName + PageId`), `MetaFieldType.cs` (the field-type ids)
// — not from `epiora-legacy`, which does not exist yet.

import { ci, convertCell, countBlocks, isIntegralColumn } from './legacy-values.mjs';
import { FIELD_TYPE, TYPE_KIND, buildGridDataset } from './ei7-grid.mjs';

const CHECK_COLS = [
  'CheckCode', 'CheckCodeBefore', 'CheckCodeAfter', 'RecordCheckCodeBefore',
  'RecordCheckCodeAfter', 'CheckCodeVariableDefinitions',
];
const KNOWN_META = new Set([
  'metaviews', 'metapages', 'metafields', 'metafieldtypes', 'metadatatypes',
  'metadbinfo', 'metaprograms', 'metagridcolumns', 'metalinks', 'metabackgrounds',
  'metaimages', 'metalayers', 'metamaps', 'metamaplayers', 'metamappoints',
  'metapatterns', 'metalayerrendertypes',
]);

export function buildEi7Project(db, opts) {
  const report = [];
  const add = (kind, item, reason) => report.push({ kind, item, reason });
  const tableSet = new Set(db.tables().map((t) => t.toLowerCase()));
  const claimed = new Set();

  const info = db.hasTable('metaDbInfo') ? db.rows('metaDbInfo')[0] || {} : {};
  const views = db.hasTable('metaViews') ? db.rows('metaViews') : [];
  const pages = db.hasTable('metaPages') ? db.rows('metaPages') : [];
  const fields = db.hasTable('metaFields') ? db.rows('metaFields') : [];
  const programs = db.hasTable('metaPrograms') ? db.rows('metaPrograms') : [];
  const gridCols = db.hasTable('metaGridColumns') ? db.rows('metaGridColumns') : [];
  const linksMeta = db.hasTable('metaLinks') ? db.rows('metaLinks') : [];

  if (opts.jet === 'JET3') {
    add('code_page_assumed', opts.stem, 'JET3 text decoded as windows-1252 (MDB_JET3_CHARSET)');
  }

  const tableRows = {}; // lazy per-table row cache
  const rowsOf = (t) => {
    if (!(t in tableRows)) {
      try {
        tableRows[t] = tableSet.has(t.toLowerCase()) ? db.rows(t) : null;
      } catch {
        tableRows[t] = null;
      }
    }
    return tableRows[t];
  };
  const colOf = (t, name) => {
    const cols = db.columns(t);
    const hit = cols.find((c) => c.name.toLowerCase() === name.toLowerCase());
    return hit ? hit.name : null;
  };

  const fieldById = new Map(fields.map((f) => [String(f.FieldId), f]));
  const pageById = new Map(pages.map((p) => [String(p.PageId), p]));
  const viewById = new Map(views.map((v) => [String(v.ViewId), v]));

  // ---- label sets ---------------------------------------------------
  const labelSets = new Map(); // id → {id, entries:[{type,value},label]}
  const labelSetFor = (f) => {
    const src = String(f.SourceTableName || '').trim();
    if (!src) {
      // Option / inline Legal Values keep their domain in `List`.
      const list = (f.List || '').split(/\r?\n/).map((s) => s.trim()).filter(Boolean);
      if (list.length === 0) return null;
      const id = `list:${f.Name}`;
      if (!labelSets.has(id)) {
        labelSets.set(id, {
          id,
          entries: list.map((v) => [{ type: 'text', value: v }, v]),
        });
      }
      return id;
    }
    const rows = rowsOf(src);
    if (rows === null) {
      add('code_table_missing', src, `field ${f.Name} references a code table that is not in the file`);
      return null;
    }
    claimed.add(src.toLowerCase());
    const kind = TYPE_KIND[Number(f.FieldTypeId)];
    const textCol = String(f.TextColumnName || '').trim() || String(f.CodeColumnName || '').trim();
    const codeCol = String(f.CodeColumnName || '').trim();
    const usePair = kind === 'codes' && codeCol && textCol && codeCol !== textCol;
    const id = usePair ? `${src}#${codeCol}->${textCol}` : src;
    if (!labelSets.has(id)) {
      const entries = [];
      const seen = new Set();
      for (const r of rows) {
        const v = usePair ? ci(r, codeCol) : ci(r, textCol);
        const l = usePair ? ci(r, textCol) : ci(r, textCol);
        if (v === null || v === undefined || v === '') continue;
        const key = `${v}→${l}`;
        if (seen.has(key)) continue;
        seen.add(key);
        entries.push([{ type: 'text', value: String(v) }, String(l ?? v)]);
      }
      labelSets.set(id, { id, entries });
    }
    return id;
  };

  // ---- one dataset per view ----------------------------------------
  const outViews = [];
  const datasets = [];
  const links = [];
  const checkCode = [];

  for (const v of views) {
    const vId = String(v.ViewId);
    const vFields = fields.filter((f) => String(f.ViewId) === vId);
    const vPages = pages
      .filter((p) => String(p.ViewId) === vId)
      .sort((a, b) => Number(a.Position) - Number(b.Position));

    // view.TableName: first non-empty DataTableName in metaFields order.
    const tableName =
      vFields.map((f) => f.DataTableName).find((s) => s && String(s).trim() !== '') || v.Name;
    claimed.add(String(tableName).toLowerCase());

    const spineRows = rowsOf(tableName);
    if (spineRows === null) {
      add('view_data_missing', v.Name, `base table ${tableName} is not in the file`);
    }
    const spine = spineRows || [];

    // Columns in order: system fields first (metaFields order), then
    // page fields by (page position, metaFields order).
    const sysFields = vFields.filter((f) => !f.PageId || String(f.PageId) === '');
    const dataFields = vFields
      .filter((f) => f.PageId && String(f.PageId) !== '')
      .sort((a, b) => {
        const pa = pageById.get(String(a.PageId));
        const pb = pageById.get(String(b.PageId));
        const pos = Number(pa?.Position ?? 0) - Number(pb?.Position ?? 0);
        return pos !== 0 ? pos : Number(a.FieldId) - Number(b.FieldId);
      });

    const pageRowIndex = (pageId) => {
      const t = `${tableName}${pageId}`;
      claimed.add(t.toLowerCase());
      const rows = rowsOf(t);
      if (rows === null) {
        add('view_data_missing', v.Name, `page table ${t} is not in the file`);
        return null;
      }
      const idx = new Map();
      for (const r of rows) {
        const g = ci(r, 'GlobalRecordId');
        if (g !== null && g !== undefined) idx.set(String(g), r);
      }
      return idx;
    };
    const pageData = new Map(); // pageId → Map<grid, row>
    for (const p of vPages) pageData.set(String(p.PageId), pageRowIndex(p.PageId));

    const columns = [];
    const metas = [];
    const pushColumn = (name, type, values, meta) => {
      columns.push({ name, type, values });
      metas.push(meta);
    };
    const spineCol = (name) => colOf(tableName, name);
    const readField = (f, pageIdx) => {
      // The page-table column for a field: matched case-insensitively
      // by name (EI7 keeps the column name in step with the field).
      // pageIdx is null when the page table is missing — already
      // reported by pageRowIndex, so no per-field noise.
      const t = `${tableName}${f.PageId}`;
      const colName = pageIdx === null ? null : colOf(t, f.Name);
      if (pageIdx !== null && colName === null) {
        add('field_column_missing', `${v.Name}/${f.Name}`, `no column in ${t}`);
        return { values: spine.map(() => null), colName: null };
      }
      const values = spine.map((s) => {
        const row = pageIdx?.get(String(ci(s, 'GlobalRecordId')));
        return row ? ci(row, colName) : null;
      });
      return { values, colName };
    };

    const pending = []; // (f, kind) data fields handled after system cols
    for (const f of sysFields) {
      const t = Number(f.FieldTypeId);
      const kind = TYPE_KIND[t] || 'meta';
      const colName = spineRows === null ? null : spineCol(f.Name);
      const raw = spine.map((r) => (colName ? ci(r, colName) : null));
      if (spineRows !== null && colName === null) {
        add('field_column_missing', `${v.Name}/${f.Name}`, `no column in ${tableName}`);
      }
      const type = kind === 'system_int' ? 'integer' : 'text';
      pushColumn(f.Name, type, raw.map((x) => convertCell(x, type)), {
        prompt: f.PromptText || undefined,
        legacy_type: FIELD_TYPE[t] || `type${t}`,
        legacy_name: colName && colName !== f.Name ? colName : undefined,
      });
    }
    for (const f of dataFields) pending.push(f);

    for (const f of pending) {
      const t = Number(f.FieldTypeId);
      const kind = TYPE_KIND[t];
      const pIdx = pageData.has(String(f.PageId)) ? pageData.get(String(f.PageId)) : new Map();
      const meta = {
        prompt: f.PromptText || undefined,
        legacy_type: FIELD_TYPE[t] || `type${t}`,
      };
      if (kind === 'meta' || kind === 'binary') {
        add('field_not_imported', `${v.Name}/${f.Name}`, `${FIELD_TYPE[t] || 'type' + t} is form metadata${kind === 'binary' ? ' (binary OLE data)' : ''}`);
        continue;
      }
      if (kind === 'link') {
        const child = viewById.get(String(f.RelatedViewId));
        links.push({
          parent: v.Name,
          child: child ? child.Name : `view${f.RelatedViewId}`,
          child_key: 'FKEY',
          parent_key: 'GlobalRecordId',
          kind: 'relate',
        });
        continue;
      }
      if (kind === 'grid') {
        // The grid's rows live in <pageTable><FieldName>; its columns
        // are the metaGridColumns rows for this FieldId.
        const g = buildGridDataset({ f, viewName: v.Name, tableName, gridCols, rowsOf, colOf, claimed, add, labelSetFor });
        datasets.push(g.dataset);
        links.push(g.link);
        continue;
      }
      if (kind === 'mirror') {
        const src = fieldById.get(String(f.SourceFieldId));
        if (!src) {
          add('field_column_missing', `${v.Name}/${f.Name}`, 'mirror source field not found');
          pushColumn(f.Name, 'text', spine.map(() => null), { ...meta });
          continue;
        }
        const srcIdx = pageData.get(String(src.PageId)) || new Map();
        const { values } = readField(src, srcIdx);
        const sk = TYPE_KIND[Number(src.FieldTypeId)] || 'text';
        const type = sk === 'number' ? 'real' : sk === 'coded' || sk === 'codes' ? 'text' : sk;
        pushColumn(f.Name, type, values.map((x) => convertCell(x, type)), {
          ...meta,
          label_set: undefined,
        });
        add('field_imported_changed', `${v.Name}/${f.Name}`, `Mirror field imported as the value of ${src.Name}`);
        continue;
      }
      const { values, colName } = readField(f, pIdx);
      meta.legacy_name = colName && colName !== f.Name ? colName : undefined;
      let type = 'text';
      if (kind === 'number') {
        const pattern = String(f.Pattern || '');
        type = pattern.includes('.') || !isIntegralColumn(values) ? 'real' : 'integer';
      } else if (kind === 'boolean') {
        type = 'boolean';
      } else if (kind === 'date' || kind === 'time' || kind === 'date_time') {
        type = kind;
      }
      const labelId = kind === 'coded' || kind === 'codes' || kind === 'option' ? labelSetFor(f) : null;
      if (labelId) meta.label_set = labelId;
      if (kind === 'codes' && f.CodeColumnName && f.TextColumnName && f.CodeColumnName !== f.TextColumnName) {
        add('field_imported_changed', `${v.Name}/${f.Name}`,
          `Codes field stores the ${f.TextColumnName} display value; the ${f.CodeColumnName} code is kept in its label set`);
      }
      if (!kind) {
        add('field_not_imported', `${v.Name}/${f.Name}`, `unknown field type ${t} imported as text`);
      }
      pushColumn(f.Name, kind ? type : 'text', values.map((x) => convertCell(x, kind ? type : 'text')), meta);
    }

    // Columns that no field claims → report.
    for (const t of [tableName, ...vPages.map((p) => `${tableName}${p.PageId}`)]) {
      for (const c of db.columns(t)) {
        const used =
          [...sysFields, ...dataFields].some((f) => f.Name.toLowerCase() === c.name.toLowerCase());
        if (!used && !/^(recstatus|uniquekey|globalrecordid|fkey|uniquerowid)$/i.test(c.name)) {
          add('column_not_imported', `${t}/${c.name}`, 'no metaFields entry');
        }
      }
    }

    const recCol = spineCol('RecStatus') || spineCol('RECSTATUS');
    const deleted = recCol ? spine.filter((r) => Number(ci(r, recCol)) === 0).length : 0;
    const ds = {
      id: v.Name,
      version: 1,
      columns,
      file: `${v.Name}.dataset.json`,
      label_sets: [],
    };
    datasets.push(ds);
    for (let i = 0; i < columns.length; i++) {
      const m = metas[i];
      if (m && (m.prompt || m.legacy_type || m.legacy_name || m.label_set)) {
        ds.columns[i].meta = Object.fromEntries(Object.entries(m).filter(([, x]) => x !== undefined));
      }
    }
    const blocks = CHECK_COLS.reduce((n, c) => n + countBlocks(v[c]), 0);
    const extra =
      vPages.reduce((n, p) => n + (p.CheckCodeBefore ? 1 : 0) + (p.CheckCodeAfter ? 1 : 0), 0) +
      vFields.reduce((n, f) => n + (f.CheckCodeBefore ? 1 : 0) + (f.CheckCodeAfter ? 1 : 0), 0);
    if (blocks + extra > 0) checkCode.push({ view: v.Name, blocks: blocks + extra });
    outViews.push({
      name: v.Name,
      dataset: ds.file,
      records: spine.length,
      deleted_records: deleted,
    });
  }

  for (const r of linksMeta) {
    add('record_links_not_imported', `${r.FromViewId}→${r.ToViewId}`,
      'metaLinks record-level link (EWE); not a schema link');
  }
  if (db.hasTable('Programs') && rowsOf('Programs') && rowsOf('Programs').length > 0) {
    add('table_not_imported', 'Programs', 'EI3-era programs table; metaPrograms is the EI7 program store');
    claimed.add('programs');
  }
  // Sorted so the report order is identical across Access and SQLite
  // sources (their table listings come in different orders).
  for (const t of [...db.tables()].sort((a, b) => a.toLowerCase().localeCompare(b.toLowerCase()))) {
    const lt = t.toLowerCase();
    if (claimed.has(lt) || KNOWN_META.has(lt)) continue;
    if (lt.startsWith('meta')) {
      add('table_not_imported', t, 'unhandled metadata table');
    } else if (lt.startsWith('code')) {
      add('table_not_imported', t, 'unreferenced code table');
    } else if (lt.startsWith('view')) {
      add('table_not_imported', t, 'EI3-era view-definition table');
    } else {
      add('table_not_imported', t, 'no view, field or code table claims it');
    }
  }

  return {
    name: String(info.ProjectName || opts.stem),
    legacy_id: info.ProjectId ? String(info.ProjectId).replace(/[{}]/g, '') : null,
    datasets,
    views: outViews,
    links,
    programs: programs.map((p) => ({ name: String(p.Name), content: String(p.Content ?? '') })),
    check_code: checkCode,
    report,
    label_sets: [...labelSets.values()],
  };
}

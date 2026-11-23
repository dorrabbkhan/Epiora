// Shared value helpers for the golden mappers (v1.1 Task 4): raw
// dump cells → Epiora values, and the EI7 check-code block count.

export const ci = (o, name) => {
  const k = Object.keys(o || {}).find((k) => k.toLowerCase() === name.toLowerCase());
  return k === undefined ? undefined : o[k];
};

// "YYYY-MM-DD HH:MM:SS" (and the -D "YYYY-MM-DD" form) → parts.
function parseDateTime(v) {
  if (typeof v !== 'string') return null;
  const m = v.match(/^(\d{4})-(\d{2})-(\d{2})(?:[ T](\d{2}):(\d{2})(?::(\d{2}))?)?\s*$/);
  if (!m) return null;
  return {
    date: `${m[1]}-${m[2]}-${m[3]}`,
    time: `${m[4] || '00'}:${m[5] || '00'}:${m[6] || '00'}`,
    full: `${m[1]}-${m[2]}-${m[3]} ${m[4] || '00'}:${m[5] || '00'}:${m[6] || '00'}`,
  };
}

// Converts one raw dumped cell to the Epiora value for `type`.
export function convertCell(raw, type) {
  if (raw === null || raw === undefined || raw === '') return null;
  switch (type) {
    case 'integer': {
      const n = Number(raw);
      return Number.isFinite(n) ? Math.trunc(n) : null;
    }
    case 'real': {
      const n = Number(raw);
      return Number.isFinite(n) ? n : null;
    }
    case 'boolean': {
      const n = Number(raw);
      return Number.isFinite(n) ? n !== 0 : null;
    }
    case 'text':
      return String(raw);
    case 'date': {
      const d = parseDateTime(String(raw));
      return d ? d.date : null;
    }
    case 'time': {
      const d = parseDateTime(String(raw));
      return d ? d.time : String(raw).slice(0, 8);
    }
    case 'date_time': {
      const d = parseDateTime(String(raw));
      return d ? d.full : null;
    }
    default:
      return null;
  }
}

export function isIntegralColumn(values) {
  return values.every((v) => v === null || v === undefined || v === '' || Number.isInteger(Number(v)));
}

// Check-code "blocks" = block headers in the EI7 text form:
// `DefineVariables`, `Form`/`View`/`Record`, `Page [x]`, `Field y`.
const BLOCK_RE = /^\s*(DefineVariables\b|Form\b|View\b|Record\b|Page\s*\[[^\]]*\]|Field\s+\S+)/gim;
export function countBlocks(text) {
  if (!text) return 0;
  return (String(text).match(BLOCK_RE) || []).length;
}

// Report entries for a source with no recognizable view-definition
// tables (an EI7 file without metaViews, an EI3 file without view*
// tables). `item` is the project stem, never a path: report fields
// describe content, and the source path already lives in project.json
// `source` where it is excluded from cross-backend comparison.
export function unknownLayoutEntries(db, stem) {
  const entries = [
    { kind: 'legacy_layout_unknown', item: stem, reason: 'no recognized view-definition tables' },
  ];
  for (const t of [...db.tables()].sort((a, b) => a.localeCompare(b))) {
    if (!/^meta/i.test(t) && !/^sqlite_/i.test(t)) {
      entries.push({ kind: 'table_not_imported', item: t, reason: 'layout_unknown: plain data table' });
    }
  }
  return entries;
}

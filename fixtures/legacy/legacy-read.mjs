// Source readers for the golden imports (v1.1 Task 4, spec §8.2). One
// uniform interface over the two dump routes: `mdb-tables`/`mdb-schema`/
// `mdb-export` for Access files and `sqlite3` for EI7 SQLite files.
// Nothing here shares code with `epiora-legacy` or `epiora-mdb` — the
// golden is produced independently of the mapper by construction.
//
// Tool paths come from PATH or these env overrides (tests point them
// at a locally extracted mdbtools):
//   MDB_TABLES, MDB_SCHEMA, MDB_EXPORT, MDB_VER, SQLITE3
// JET3 files are decoded with MDB_JET3_CHARSET (default cp1252 — the
// JET3 code page; every JET3 read reports the assumption upstream).

import { execFileSync } from 'node:child_process';
import { csvObjects } from './csv.mjs';

const tools = {
  mdbTables: process.env.MDB_TABLES || 'mdb-tables',
  mdbSchema: process.env.MDB_SCHEMA || 'mdb-schema',
  mdbExport: process.env.MDB_EXPORT || 'mdb-export',
  mdbVer: process.env.MDB_VER || 'mdb-ver',
  sqlite3: process.env.SQLITE3 || 'sqlite3',
};

const MDB_ENV = {
  ...process.env,
  MDB_JET3_CHARSET: process.env.MDB_JET3_CHARSET || 'cp1252',
};

function run(cmd, args) {
  try {
    return execFileSync(cmd, args, {
      encoding: 'utf8',
      maxBuffer: 512 * 1024 * 1024,
      env: cmd.startsWith(tools.sqlite3) ? process.env : MDB_ENV,
    });
  } catch (e) {
    const detail = e.stderr ? String(e.stderr).trim() : e.message;
    throw new Error(`${cmd} ${args.join(' ')} failed: ${detail}`);
  }
}

// Access column type names normalised to a small set the mapper maps
// to Epiora types: text | int | real | datetime | bool | guid | blob.
const ACCESS_TYPES = [
  [/^\s*long integer/i, 'int'],
  [/^\s*integer/i, 'int'],
  [/^\s*byte/i, 'int'],
  [/^\s*double/i, 'real'],
  [/^\s*single/i, 'real'],
  [/^\s*currency/i, 'real'],
  [/^\s*numeric/i, 'real'],
  [/^\s*datetime/i, 'datetime'],
  [/^\s*boolean/i, 'bool'],
  [/^\s*replication id/i, 'guid'],
  [/^\s*text/i, 'text'],
  [/^\s*memo/i, 'text'],
  [/^\s*ole/i, 'blob'],
  [/^\s*binary/i, 'blob'],
];

function accessType(decl) {
  for (const [re, t] of ACCESS_TYPES) if (re.test(decl)) return t;
  return 'blob';
}

// Parses `mdb-schema` output into {table: [{name, decl, type}]}.
// Column types like `Memo/Hyperlink` carry non-alpha characters, so
// each `[name] type…` entry is split on top-level commas first.
export function parseMdbSchema(text) {
  const tables = {};
  const re = /CREATE TABLE \[([^\]]+)\]\s*\(([^;]*?)\);/gs;
  let m;
  while ((m = re.exec(text))) {
    const cols = [];
    const colRe = /\[\s*([^\]]+)\]\s*([^,\]]+?)(?:\s*,|$)/g;
    let c;
    while ((c = colRe.exec(m[2]))) {
      const decl = c[2].trim();
      if (/^constraint|primary key|references/i.test(decl)) continue;
      cols.push({ name: c[1], decl, type: accessType(decl) });
    }
    tables[m[1]] = cols;
  }
  return tables;
}

// SQLite declared-type → the same normalised set.
function sqliteType(decl) {
  const d = decl.toLowerCase();
  if (/int|yesno|bit/.test(d)) return 'int';
  if (/real|doub|floa|numeric|decimal|currency/.test(d)) return 'real';
  if (/datetime|^date$|^time$/.test(d)) return 'datetime';
  if (/blob/.test(d)) return 'blob';
  return 'text';
}

// One readable source. `kind` is 'mdb' or 'sqlite'.
export function openSource(path, kind) {
  if (kind === 'mdb') {
    const schemaText = run(tools.mdbSchema, [path]);
    const schemas = parseMdbSchema(schemaText);
    const tableList = run(tools.mdbTables, [path]).trim().split(/\s+/).filter(Boolean);
    let jet = 'unknown';
    try {
      jet = run(tools.mdbVer, [path]).trim();
    } catch {
      /* a missing mdb-ver does not block the dump */
    }
    return {
      kind,
      jet,
      tables: () => tableList,
      hasTable: (t) => t in schemas || tableList.includes(t),
      columns: (t) => schemas[t] || [],
      // rows: {name → string|null}, every value still the raw dump
      // text; -b hex renders OLE/binary cells as hex strings. Access
      // Replication IDs hold ASCII GUID text, so guid columns are
      // hex-decoded back to "{…}" strings.
      rows: (t) => {
        const guidCols = (schemas[t] || []).filter((c) => c.type === 'guid').map((c) => c.name);
        const rows = csvObjects(
          run(tools.mdbExport, ['-T', '%Y-%m-%d %H:%M:%S', '-D', '%Y-%m-%d', '-b', 'hex', path, t])
        );
        if (guidCols.length > 0) {
          for (const r of rows) {
            for (const g of guidCols) {
              if (typeof r[g] === 'string' && /^[0-9A-Fa-f]+$/.test(r[g])) {
                r[g] = Buffer.from(r[g], 'hex').toString('utf8');
              }
            }
          }
        }
        return rows;
      },
    };
  }
  const tablesText = run(tools.sqlite3, [path, '.tables']);
  const tableList = tablesText.trim().split(/\s+/).filter(Boolean);
  const pragmaCache = {};
  const pragma = (t) => {
    if (!(t in pragmaCache)) {
      const out = run(tools.sqlite3, ['-json', path, `PRAGMA table_info(${sqlName(t)})`]);
      const info = out.trim() === '' ? [] : JSON.parse(out);
      pragmaCache[t] = info.map((c) => ({ name: c.name, decl: c.type || 'TEXT', type: sqliteType(c.type || 'TEXT') }));
    }
    return pragmaCache[t];
  };
  return {
    kind,
    jet: 'sqlite',
    tables: () => tableList,
    hasTable: (t) => tableList.includes(t),
    columns: pragma,
    // sqlite3 -json gives typed values; dates arrive as the text EI7
    // stored ("YYYY-MM-DD HH:MM:SS").
    // sqlite3 -json prints nothing for an empty result.
    rows: (t) => {
      const out = run(tools.sqlite3, ['-json', path, `SELECT * FROM ${sqlName(t)}`]);
      return out.trim() === '' ? [] : JSON.parse(out);
    },
  };
}

function sqlName(t) {
  return '"' + t.replace(/"/g, '""') + '"';
}

// A verbatim table copy into a SQLite file: same names, same rows,
// mechanical type mapping (copy-tables.mjs shares this with nothing —
// it writes SQL the sqlite3 CLI executes).
export function accessToSqliteDecl(col) {
  switch (col.type) {
    case 'int':
    case 'bool':
      return 'INTEGER';
    case 'real':
      return 'REAL';
    case 'blob':
      return 'BLOB';
    default:
      return 'TEXT';
  }
}

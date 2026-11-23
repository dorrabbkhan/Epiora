#!/usr/bin/env node
// Verbatim table copies for the golden cross-backend test (v1.1 Task
// 4): the same rows under every reader. Nothing is mapped — table
// names, column names and cell values travel exactly as the source
// dump emits them.
//
//   node fixtures/legacy/copy-tables.mjs --mdb <file.mdb> --sqlite <out.db>
//   node fixtures/legacy/copy-tables.mjs --mdb <file.mdb> --tsql <out.sql>
//
// --tsql emits a T-SQL script Task 9 loads into a SQL Server
// container; the golden generator does not run it.

import { execFileSync } from 'node:child_process';
import * as fs from 'node:fs';
import { openSource, accessToSqliteDecl } from './legacy-read.mjs';

const args = process.argv.slice(2);
const opt = (name) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : null;
};

const mdb = opt('--mdb');
const sqliteOut = opt('--sqlite');
const tsqlOut = opt('--tsql');
if (!mdb || (!sqliteOut && !tsqlOut)) {
  console.error('usage: copy-tables.mjs --mdb <file> [--sqlite out.db | --tsql out.sql]');
  process.exit(2);
}

const db = openSource(mdb, 'mdb');
const SQLITE3 = process.env.SQLITE3 || 'sqlite3';

const sqlLit = (v) => (v === null || v === undefined ? 'NULL' : `'${String(v).replace(/'/g, "''")}'`);
// The sqlite3 shell reads scripts line-wise and strips \r even inside
// quoted literals, so text cells go in as hex and CAST back — the
// stored bytes are then identical to the source cell.
const hexTextLit = (v) => (v === null || v === undefined ? 'NULL' : `CAST(X'${Buffer.from(String(v), 'utf8').toString('hex')}' AS TEXT)`);
const numLit = (v, type) => {
  if (v === null || v === undefined || v === '') return 'NULL';
  const n = Number(v);
  if (!Number.isFinite(n)) {
    throw new Error(`non-numeric value ${JSON.stringify(v)} in a ${type} column`);
  }
  return String(n);
};

function cellSql(v, type, dialect) {
  switch (type) {
    case 'int':
    case 'real':
      return numLit(v, type);
    case 'bool':
      return v === null || v === '' ? 'NULL' : Number(v) ? '1' : '0';
    case 'blob':
      // mdb-export -b hex renders OLE/binary cells as hex text.
      if (v === null || v === undefined) return 'NULL';
      return dialect === 'sqlite' ? `X'${v}'` : `0x${v}`;
    default:
      return dialect === 'sqlite' ? hexTextLit(v) : sqlLit(v);
  }
}

const ddl = (col, dialect) => {
  if (dialect === 'sqlite') return `"${col.name.replace(/"/g, '""')}" ${accessToSqliteDecl(col)}`;
  const t = { int: 'INT', bool: 'BIT', real: 'FLOAT', datetime: 'NVARCHAR(32)', guid: 'NVARCHAR(64)', text: 'NVARCHAR(MAX)', blob: 'NVARCHAR(MAX)' }[col.type] || 'NVARCHAR(MAX)';
  return `[${col.name}] ${t}`;
};

function statements(dialect) {
  const stmts = [];
  for (const t of db.tables()) {
    const cols = db.columns(t);
    const name = dialect === 'sqlite' ? `"${t.replace(/"/g, '""')}"` : `[${t}]`;
    if (cols.length === 0) continue;
    stmts.push(`CREATE TABLE ${name} (\n  ${cols.map((c) => ddl(c, dialect)).join(',\n  ')}\n);`);
    const rows = db.rows(t);
    const colNames = cols.map((c) => (dialect === 'sqlite' ? `"${c.name.replace(/"/g, '""')}"` : `[${c.name}]`));
    for (const r of rows) {
      const vals = cols.map((c) => cellSql(r[c.name] ?? null, c.type, dialect));
      stmts.push(`INSERT INTO ${name} (${colNames.join(', ')}) VALUES (${vals.join(', ')});`);
    }
  }
  return stmts;
}

if (sqliteOut) {
  if (fs.existsSync(sqliteOut)) fs.unlinkSync(sqliteOut);
  const script = statements('sqlite').join('\n');
  execFileSync(SQLITE3, [sqliteOut], { input: script, stdio: ['pipe', 'inherit', 'inherit'] });
  console.log(`sqlite copy written: ${sqliteOut}`);
}
if (tsqlOut) {
  const header = '-- Verbatim Access→SQL Server copy produced by fixtures/legacy/copy-tables.mjs\nSET NOCOUNT ON;\n';
  fs.writeFileSync(tsqlOut, header + statements('tsql').join('\n') + '\n');
  console.log(`t-sql script written: ${tsqlOut}`);
}

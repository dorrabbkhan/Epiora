// CSV reader for `mdb-export` output (v1.1 Task 4). mdb-export wraps
// text-like fields in double quotes, escapes a quote as `""`, lets
// fields contain newlines, writes NULL as an unquoted empty field and
// an empty string as a quoted empty one — so the parse has to keep
// track of whether a field was quoted.

export function parseCsv(text) {
  const rows = [];
  let row = [];
  let field = '';
  let inQuotes = false;
  let wasQuoted = false;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (inQuotes) {
      if (c === '"') {
        if (text[i + 1] === '"') {
          field += '"';
          i++;
        } else {
          inQuotes = false;
        }
      } else {
        field += c;
      }
    } else if (c === '"') {
      inQuotes = true;
      wasQuoted = true;
    } else if (c === ',') {
      row.push({ value: field, quoted: wasQuoted });
      field = '';
      wasQuoted = false;
    } else if (c === '\n') {
      row.push({ value: field, quoted: wasQuoted });
      rows.push(row);
      row = [];
      field = '';
      wasQuoted = false;
    } else if (c === '\r') {
      // skipped; row endings are \n or \r\n
    } else {
      field += c;
    }
  }
  if (field !== '' || row.length > 0 || wasQuoted) {
    row.push({ value: field, quoted: wasQuoted });
    rows.push(row);
  }
  return rows;
}

// Rows as objects keyed by the header line. `null` marks an unquoted
// empty field (SQL NULL); a quoted empty field stays the empty string.
export function csvObjects(text) {
  const rows = parseCsv(text);
  if (rows.length === 0) return [];
  const header = rows[0].map((h) => h.value);
  return rows.slice(1).map((r) => {
    const o = {};
    for (let i = 0; i < header.length; i++) {
      const cell = r[i] ?? { value: '', quoted: false };
      o[header[i]] = cell.value === '' && !cell.quoted ? null : cell.value;
    }
    return o;
  });
}

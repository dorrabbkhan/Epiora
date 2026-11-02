// parse.mjs — extract every labelled number from an Epi Info 7 ROUTEOUT
// html file into the fixtures/ei7/oswego.json shape (spec §6.4, v0.2 Task 3).
//
//   { "source": { "build", "zip_sha256", "sample_mdb_sha256", "run_url" },
//     "outputs": [ { "command": "TABLES VANILLA ILL",
//                    "values": [ { "label", "key", "text", "value", "decimals" } ] } ] }
//
// Library use:   parseHtml(html, labels) -> { outputs }
// CLI use:       node parse.mjs --html in.html --labels labels.json --out oswego.json
//                  --build 7.2.7.0 --zip-sha256 <h> --sample-mdb-sha256 <h> --run-url <url>
//
// Label grammar (labels.json maps these strings to Task 2 key paths or null):
//   stat table:  "<context>: <row label> <column header> (<method letter>)"
//   crosstab:    "<context>: <ROWVAR>=<level> <COLVAR>=<level>[ Row%| Col%]"
// <context> joins two caption slots with a space and is omitted when both are
// empty. A caption is the stripped text between two tables (Epi Info wraps
// them in <b>, <h4> or <center><strong>): a "VAR=value" caption (e.g.
// "SEX=Female") fills the stratum slot and resets the block slot; any other
// caption fills the block slot and keeps the stratum, except a caption
// containing "summary" which resets the stratum. Parenthesised captions
// (method legends) are ignored. A table may also carry a caption cell before
// its first <tr> (e.g. "Exact 95% Conf Limits"), which is appended as a third
// context part. The method-letter suffix comes from Epi Info's own
// "(T)"/"(M)"/"(F)"/"(R)" cell markers. A data row with an empty label cell
// inherits the previous row's label (Epi Info prints extra CI methods as
// continuation rows, e.g. the Fisher "(F)" limits under "Odds Ratio (MLE)").
// No dependencies — the html is parsed with a small tag scanner.

import { readFileSync, writeFileSync } from 'node:fs';

const ENTITIES = { nbsp: ' ', amp: '&', lt: '<', gt: '>', quot: '"', apos: "'" };

function decodeEntities(s) {
  return s.replace(/&(#\d+|#\w+|\w+);/g, (m, e) =>
    e[0] === '#' ? String.fromCodePoint(Number(e.slice(1))) : (ENTITIES[e] ?? m));
}

function stripTags(s) {
  return decodeEntities(s.replace(/<[^>]*>/g, ' ')).replace(/\s+/g, ' ').trim();
}

// A number as Epi Info prints it: optional sign, optional thousands commas,
// optional fraction, a single trailing comma (CI lists print "lower,"),
// optional "%" suffix, optional " (X)" method marker.
const NUM_RE = /^([+-]?(?:\d{1,3}(?:,\d{3})+|\d+)(?:\.\d+)?),?(%?)(\s*\(([A-Za-z]+)\))?$/;

function parseNumber(text) {
  const m = NUM_RE.exec(text);
  if (!m) return null;
  const mantissa = m[1].replace(/,/g, '');
  const frac = mantissa.split('.')[1] ?? '';
  return { text, value: Number(mantissa), decimals: frac.length, marker: m[4] ?? null };
}

// Cells that hold a displayed token rather than a number ("Undefined" when a
// statistic is undefined for the stratum, "NaN" for Breslow-Day variants Epi
// Info does not report). They mark a row as data (never a header) but emit no
// value — there is no number to store.
const PLACEHOLDER_RE = /^(Undefined|NaN)\b/i;

// Method marker at the end of a cell, e.g. "(T)", "(M)", "(F)", "(R)".
const MARKER_RE = /\(([A-Za-z]+)\)\s*$/;

// Split html into balanced <table>...</table> blocks (handles nesting).
function tableBlocks(html) {
  const blocks = [];
  const re = /<\/?table\b[^>]*>/gi;
  let depth = 0, start = -1, m;
  while ((m = re.exec(html))) {
    if (m[0][1] === '/') {
      if (--depth === 0) blocks.push([start, re.lastIndex]);
    } else if (depth++ === 0) start = m.index;
  }
  return blocks;
}

// A table block -> { caption, rows } where caption is the stripped cell text
// before the first <tr> (Epi Info puts e.g. "Exact 95% Conf Limits" there)
// and rows are lists of cells { text, span }. Cells are delimited by opening
// <td>/<th> tags: Epi Info's html leaves some cells unclosed.
function parseTable(block) {
  const rows = [];
  const trRe = /<tr\b[^>]*>/gi;
  const starts = [];
  let tm;
  while ((tm = trRe.exec(block))) starts.push([tm.index, trRe.lastIndex]);
  const caption = starts.length ? stripTags(block.slice(0, starts[0][0])) : '';
  for (let r = 0; r < starts.length; r++) {
    const body = block.slice(starts[r][1], r + 1 < starts.length ? starts[r + 1][0] : block.length);
    const cells = [];
    const cellRe = /<(td|th)\b([^>]*)>/gi;
    let cm, prev = null;
    const flush = upto => {
      if (prev) cells.push({ text: stripTags(body.slice(prev.end, upto)), span: prev.span });
    };
    while ((cm = cellRe.exec(body))) {
      flush(cm.index);
      const span = /colspan\s*=\s*"?(\d+)/i.exec(cm[2]);
      prev = { end: cellRe.lastIndex, span: span ? Number(span[1]) : 1 };
    }
    flush(body.length);
    if (cells.length) rows.push(cells);
  }
  return { caption, rows };
}

// Expand a row's cells into per-column texts (a colspan writes into every
// covered column).
function rowColumns(cells, width) {
  const cols = new Array(width).fill('');
  let i = 0;
  for (const c of cells) {
    for (let k = 0; k < c.span && i < width; k++, i++) cols[i] = c.text;
  }
  return cols;
}

function emit(values, ctx, label, parsed) {
  const prefix = [ctx.stratum, ctx.block, ctx.caption].filter(Boolean).join(' ');
  const full = (prefix ? `${prefix}: ${label}` : label).replace(/\s+/g, ' ').trim();
  values.push({ label: full, text: parsed.text, value: parsed.value, decimals: parsed.decimals });
}

// The stripped text between two tables updates the caption context.
function updateContext(ctx, caption) {
  const c = caption.trim();
  if (!c || c.startsWith('(')) return ctx;
  if (/^[\w.\-]+\s*=/.test(c)) return { stratum: c, block: '' };
  if (/summary/i.test(c)) return { stratum: '', block: c };
  return { stratum: ctx.stratum, block: c };
}

// A table is a crosstab when one of its rows is a Row%/Col% marker row.
function isCrosstab(rows) {
  return rows.some(r => r.length && /^(row|col)%$/i.test(r[0].text));
}

function parseCrosstab(table, ctx, values) {
  const rows = table.rows;
  ctx = { ...ctx, caption: table.caption };
  // Locate the header row carrying "Total" — its first cell is the row
  // variable name and cells 1..n are the column levels; the column variable
  // name is the non-empty cell of the row above it (may be colspan'd).
  let headIdx = rows.findIndex(r => r.some(c => /^total$/i.test(c.text)));
  if (headIdx < 0) headIdx = 1;
  const head = rows[headIdx];
  const rowVar = head[0]?.text || 'row';
  const colLevels = [];
  for (const c of head.slice(1)) for (let k = 0; k < c.span; k++) colLevels.push(c.text);
  let colVar = 'col';
  for (let i = headIdx - 1; i >= 0; i--) {
    const cell = rows[i].find(c => c.text);
    if (cell) { colVar = cell.text; break; }
  }
  let level = '';
  for (const row of rows.slice(headIdx + 1)) {
    const first = row[0]?.text ?? '';
    const markerM = /^(row|col)%$/i.exec(first);
    if (!markerM) level = /^total$/i.test(first) ? 'Total' : (first || level);
    let col = 1;
    for (const c of row.slice(1)) {
      const n = parseNumber(c.text);
      if (n && level) {
        const lv = colLevels[col - 1] ?? `c${col}`;
        emit(values, ctx,
          `${rowVar}=${level} ${colVar}=${lv}${markerM ? ` ${first}` : ''}`, n);
      }
      col += c.span;
    }
  }
}

function parseStatTable(table, ctx, values) {
  const rows = table.rows;
  ctx = { ...ctx, caption: table.caption };
  const width = Math.max(...rows.map(r => r.reduce((a, c) => a + c.span, 0)));
  const headers = new Array(width).fill('');
  let lastLabel = '';
  for (const row of rows) {
    const cols = rowColumns(row, width);
    const parsed = cols.map(parseNumber);
    const numericIdx = parsed.map((p, i) => (p ? i : -1)).filter(i => i >= 0);
    const hasPlaceholder = cols.some(c => PLACEHOLDER_RE.test(c));
    if (numericIdx.length === 0 && !hasPlaceholder) {
      // header / group / footnote row: overwrite headers at non-empty cells
      cols.forEach((t, i) => { if (t) headers[i] = t; });
      continue;
    }
    const rowLabel = cols[0] || lastLabel;
    if (cols[0]) lastLabel = cols[0];
    const markers = [...new Set(cols.slice(1).map(c => MARKER_RE.exec(c)?.[1]).filter(Boolean))];
    const suffix = markers.length ? ` (${markers.join(' ')})` : '';
    for (const i of numericIdx) {
      const colName = headers[i] || `col${i}`;
      emit(values, ctx, `${rowLabel} ${colName}${suffix}`, parsed[i]);
    }
  }
}

export function parseHtml(html, labels = {}) {
  const outputs = [];
  // Sections are delimited by <h3> command captions.
  const h3re = /<h3\b[^>]*>(.*?)<\/h3>/gi;
  const marks = [];
  let m;
  while ((m = h3re.exec(html))) marks.push({ command: stripTags(m[1]), start: m.index, end: h3re.lastIndex });
  for (let s = 0; s < marks.length; s++) {
    const segEnd = s + 1 < marks.length ? marks[s + 1].start : html.length;
    const seg = html.slice(marks[s].end, segEnd);
    const values = [];
    let pos = 0;
    let ctx = { stratum: '', block: '', caption: '' };
    for (const [ts, te] of tableBlocks(seg)) {
      ctx = updateContext(ctx, stripTags(seg.slice(pos, ts)));
      const table = parseTable(seg.slice(ts, te));
      if (isCrosstab(table.rows)) parseCrosstab(table, ctx, values);
      else parseStatTable(table, ctx, values);
      pos = te;
    }
    outputs.push({ command: marks[s].command, values: values.map(v => ({ ...v, key: labels[v.label] ?? null })) });
  }
  return { outputs };
}

export function buildResult(outputs, source) {
  return { source, outputs };
}

function arg(flag) {
  const i = process.argv.indexOf(flag);
  return i === -1 ? undefined : process.argv[i + 1];
}

if (process.argv[1] && process.argv[1].endsWith('parse.mjs')) {
  const htmlPath = arg('--html'), labelsPath = arg('--labels'), outPath = arg('--out');
  if (!htmlPath || !outPath) {
    console.error('usage: node parse.mjs --html <in.html> [--labels labels.json] --out <out.json> ' +
      '[--build B] [--zip-sha256 H] [--sample-mdb-sha256 H] [--run-url U]');
    process.exit(2);
  }
  const labels = labelsPath ? JSON.parse(readFileSync(labelsPath, 'utf8')) : {};
  const { outputs } = parseHtml(readFileSync(htmlPath, 'utf8'), labels);
  const source = {
    build: arg('--build') ?? 'unknown',
    zip_sha256: arg('--zip-sha256') ?? 'unknown',
    sample_mdb_sha256: arg('--sample-mdb-sha256') ?? 'unknown',
    run_url: arg('--run-url') ?? 'unknown',
  };
  const unmapped = [...new Set(outputs.flatMap(o => o.values.filter(v => v.key === null).map(v => v.label)))];
  console.error(`parse.mjs: ${outputs.length} command block(s), ${unmapped.length} unmapped label(s)`);
  for (const l of unmapped) console.error(`  unmapped: ${l}`);
  writeFileSync(outPath, JSON.stringify(buildResult(outputs, source), null, 2));
}

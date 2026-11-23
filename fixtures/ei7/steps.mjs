// steps.mjs — the "--steps" corpus half of parse.mjs (v1.1 Task 3):
// correlate a rendered program's source lines with its ROUTEOUT html into
// per-command steps for the corpus json shape
//   { "source": {...}, "steps": [ { "line", "command", "rows", "results" } ] }.
//
// Epi Info emits exactly one <h3> section per output-producing statement and
// none for DEFINE/ASSIGN/RECODE rules/IF bodies/ROUTEOUT/CLOSEOUT/QUIT, so the
// two lists walk in lockstep: a section with a caption binds to the next
// unbound source line whose normalised text matches it, an empty-caption
// section (READ/SELECT/SORT/RELATE/MERGE metadata) binds to the next unbound
// source line, and lines never reached by a section keep rows:null.
//
// Split out of parse.mjs in fix round 1 to keep both files under the
// ~400-line rule (plan Global Constraints); parse.mjs keeps the
// labelled-values parser and the CLI and imports this module for its
// --steps branch.

import {
  htmlSections,
  parseNumber,
  parseTable,
  rowColumns,
  sectionValues,
  stripTags,
  tableBlocks,
} from './parse.mjs';

const normCommand = s => s.replace(/\s+/g, ' ').trim().toUpperCase();

// The non-comment, non-blank lines of a .pgm program (the commands Epi Info
// executes; `***` lines are comments).
function programCommands(pgmText) {
  const cmds = [];
  pgmText.split(/\r?\n/).forEach((raw, i) => {
    const t = raw.trim();
    if (t && !t.startsWith('***')) cmds.push({ line: i + 1, command: t });
  });
  return cmds;
}

// A displayed cell -> its json value: numbers become numbers (a bare
// integer or decimal cell), "Missing" and empty cells become null, and
// everything else (labels, percent cells, datetimes) stays displayed text.
function cellValue(text) {
  if (text === '' || text === 'Missing') return null;
  const n = parseNumber(text);
  if (n && !text.includes('%') && !n.marker) return n.value;
  return text;
}

// The first table of a section as an array of cell arrays, header row
// included, colspans expanded.
function firstTableRows(seg) {
  const blocks = tableBlocks(seg);
  if (!blocks.length) return null;
  const table = parseTable(seg.slice(blocks[0][0], blocks[0][1]));
  const width = Math.max(...table.rows.map(r => r.reduce((a, c) => a + c.span, 0)));
  return table.rows.map(r => rowColumns(r, width).map(cellValue));
}

// A FREQ table's header carries a "Frequency" column.
function isFreqTable(rows) {
  return !!rows && rows.length > 0 && rows[0].some(c => c === 'Frequency');
}

// FREQ blocks hold two tables: the frequency table itself and the "Exact 95%
// Conf Limits" table. Categories map to freq.categories[k] in display order.
function freqResults(seg, results) {
  const blocks = tableBlocks(seg);
  for (let t = 0; t < blocks.length; t++) {
    const table = parseTable(seg.slice(blocks[t][0], blocks[t][1]));
    const width = Math.max(...table.rows.map(r => r.reduce((a, c) => a + c.span, 0)));
    const rows = table.rows.map(r => rowColumns(r, width));
    if (t === 0) {
      let k = 0;
      for (const cols of rows.slice(1)) {
        if (/^total$/i.test(cols[0])) {
          const total = parseNumber(cols[1]);
          if (total) results['freq.total'] = { value: total.value, decimals: total.decimals };
          continue;
        }
        for (const [col, suffix] of [[1, 'count'], [2, 'percent'], [3, 'cumulative_percent']]) {
          const n = parseNumber(cols[col] ?? '');
          if (n) results[`freq.categories[${k}].${suffix}`] = { value: n.value, decimals: n.decimals };
        }
        k++;
      }
    } else if (/conf limits/i.test(table.caption)) {
      let k = 0;
      for (const cols of rows) {
        const lo = parseNumber(cols[1] ?? '');
        const hi = parseNumber(cols[2] ?? '');
        if (!lo || !hi) continue;
        results[`freq.categories[${k}].ci_95_clopper_pearson.lower`] = { value: lo.value, decimals: lo.decimals };
        results[`freq.categories[${k}].ci_95_clopper_pearson.upper`] = { value: hi.value, decimals: hi.decimals };
        k++;
      }
    }
  }
}

// One command + its bound section -> a corpus step. LIST and DISPLAY sections
// carry their data in rows; FREQ gets positional freq.* results; other
// captioned sections (MEANS, TABLES, ...) map values through labels.json;
// table-less metadata sections pin the working-view record count and the
// deleted-records processing flag.
function stepFor(cmd, block, labels) {
  const step = { line: cmd.line, command: cmd.command, rows: null, results: {} };
  if (!block) return step;
  step.rows = firstTableRows(block.seg);
  if (step.rows) {
    if (isFreqTable(step.rows)) freqResults(block.seg, step.results);
    else if (!/^(LIST|DISPLAY)\b/i.test(cmd.command)) {
      for (const v of sectionValues(block.seg)) {
        const key = labels[v.label];
        if (key != null) step.results[key] = { value: v.value, decimals: v.decimals };
      }
    }
  } else {
    const text = stripTags(block.seg);
    const rc = /Record Count:\s*([\d,]+)/i.exec(text);
    if (rc) step.results[labels['Record Count'] ?? 'selection.record_count'] =
      { value: Number(rc[1].replace(/,/g, '')), decimals: 0 };
    step.results['selection.deleted_records_included'] =
      { value: /Deleted Records Included/i.test(text) ? 1 : 0, decimals: 0 };
    // DELETE reports "Number of records deleted: 29"; UNDELETE reports
    // "0 items marked as deleted" (a no-op on raw .mdb reads — the working
    // set is not an Epi7 project).
    const del = /Number of records deleted:\s*([\d,]+)/i.exec(text);
    if (del) step.results['delete.records_deleted'] =
      { value: Number(del[1].replace(/,/g, '')), decimals: 0 };
    const undel = /([\d,]+)\s+items marked as deleted/i.exec(text);
    if (undel) step.results['undelete.records_restored'] =
      { value: Number(undel[1].replace(/,/g, '')), decimals: 0 };
  }
  return step;
}

// Correlate program lines with the html's <h3> sections into corpus steps.
// Returns { steps, warnings } — a warning is recorded for every captioned
// section that matched no upcoming source line (it then binds in order).
export function parseSteps(html, pgmText, labels = {}) {
  const cmds = programCommands(pgmText);
  const blocks = htmlSections(html);
  const warnings = [];
  const bound = new Map();
  let ci = 0;
  for (const block of blocks) {
    if (ci >= cmds.length) { warnings.push(`extra section ${block.command || '(metadata)'} has no source line`); break; }
    if (!block.command) { bound.set(ci, block); ci++; continue; }
    const n = normCommand(block.command);
    let j = cmds.findIndex((c, k) => k >= ci && normCommand(c.command) === n);
    if (j === -1) { warnings.push(`section ${block.command} matched no source line — bound in order`); j = ci; }
    bound.set(j, block);
    ci = j + 1;
  }
  const steps = cmds.map((c, i) => stepFor(c, bound.get(i), labels));
  return { steps, warnings };
}

export function buildStepsResult(steps, source) {
  return { source, steps };
}

// The --steps source block: the same build/zip_sha256/run_url provenance as
// the outputs corpus plus the .pgm's basename and the "--input name=sha256"
// pairs naming every data file the program read.
export function stepsSource(pgmPath, build, zipSha256, runUrl, inputArgs) {
  const source = {
    build: build ?? 'unknown',
    zip_sha256: zipSha256 ?? 'unknown',
    run_url: runUrl ?? 'unknown',
    program: pgmPath.replace(/\\/g, '/').split('/').pop(),
    inputs: {},
  };
  for (const kv of inputArgs) {
    const eq = kv.indexOf('=');
    if (eq > 0) source.inputs[kv.slice(0, eq)] = kv.slice(eq + 1);
  }
  return source;
}

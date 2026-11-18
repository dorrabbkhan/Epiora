// Calculator URL state (v1.0 plan Task 14, spec §7.1–7.2): the
// `/calc/$id` search params carry one calculator's inputs — scalars as
// plain values, tables as `;`-joined rows of `,`-joined cells, a
// strata list as `|`-joined tables, choices by option name.
//
// `draftsFromParams` splits a URL into raw text drafts (no validation —
// the bad text stays visible in the form), `collectInputs` validates
// drafts against the registry definition (reporting, never throwing),
// `decodeInputs` = split + collect + unknown-key reporting, and
// `encodeInputs` writes canonical values back to params. The
// sub-codes mirror the engine's `CalcInputOutOfRange` reasons
// (crates/epiora-core/src/calc/input.rs) so a link marks the same
// inputs the engine would reject.

import type { CalculatorDef } from '@engine-types/CalculatorDef';
import type { CellKind } from '@engine-types/CellKind';
import type { DefaultValue } from '@engine-types/DefaultValue';
import type { InputDef } from '@engine-types/InputDef';
import type { InputKind } from '@engine-types/InputKind';

/** One form draft: raw text for scalars and choices, a list of string
 *  tables for a `table` input (single item for a flat table, one per
 *  stratum for a strata input). */
export type Draft = string | string[][][];

export type Drafts = Record<string, Draft>;

export interface DecodedInputs {
  values: Record<string, unknown>;
  /** Input name → stable sub-code; `_unknown` lists unknown keys. */
  errors: Record<string, string>;
}

const MAX_COUNT = 2 ** 53;

function tableKind(
  input: InputDef,
): { cols: number; min_rows: number; max_rows: number; cell: CellKind; strata: number | null } | null {
  const kind = input.kind;
  if (typeof kind === 'object' && 'table' in kind) return kind.table;
  return null;
}

/** The raw-text draft for `input` under `params`, or `undefined` when
 *  the link does not carry it. */
function draftFor(input: InputDef, params: URLSearchParams): Draft | undefined {
  const raw = params.get(input.name);
  if (raw === null) return undefined;
  if (tableKind(input) !== null) {
    return raw.split('|').map((t) => t.split(';').map((r) => r.split(',')));
  }
  return raw;
}

/** Splits `params` into raw-text drafts for every declared input the
 *  link carries — never validates, never throws. */
export function draftsFromParams(
  def: CalculatorDef,
  params: URLSearchParams,
): Drafts {
  const drafts: Drafts = {};
  for (const input of def.inputs) {
    const draft = draftFor(input, params);
    if (draft !== undefined) drafts[input.name] = draft;
  }
  return drafts;
}

function parseNumber(text: string): number | null {
  const t = text.trim();
  if (t === '') return null;
  const n = Number(t);
  return Number.isFinite(n) ? n : null;
}

function parseCell(kind: CellKind, text: string): number | string {
  const n = parseNumber(text);
  if (kind === 'count') {
    if (n === null || !Number.isInteger(n) || n < 0 || n >= MAX_COUNT) {
      return 'not_a_count';
    }
    return n;
  }
  if (n === null) return 'not_finite';
  return n;
}

/** One flat table of `min_rows..=max_rows` rows, each `cols` cells wide
 *  (or a shared 2..=10 width when `cols` is 0), of `cell` kind. */
function flatTable(
  spec: { cols: number; min_rows: number; max_rows: number; cell: CellKind },
  rows: string[][],
): number[][] | string {
  if (rows.length < spec.min_rows || rows.length > spec.max_rows) {
    return 'bad_table_shape';
  }
  const width = rows[0]?.length ?? 0;
  const out: number[][] = [];
  for (const row of rows) {
    const widthOk =
      spec.cols === 0
        ? row.length >= 2 && row.length <= 10 && row.length === width
        : row.length === spec.cols;
    if (!widthOk) return 'bad_table_shape';
    const parsed: number[] = [];
    for (const text of row) {
      const cell = parseCell(spec.cell, text);
      if (typeof cell === 'string') return cell;
      parsed.push(cell);
    }
    out.push(parsed);
  }
  return out;
}

function parseTable(
  input: InputDef,
  tables: string[][][],
): number[][] | number[][][] | string {
  const spec = tableKind(input);
  if (spec === null) return 'bad_table_shape';
  if (spec.strata === null && tables.length !== 1) return 'bad_table_shape';
  if (spec.strata !== null && (tables.length < 1 || tables.length > spec.strata)) {
    return 'bad_table_shape';
  }
  const parsed: number[][][] = [];
  for (const table of tables) {
    const flat = flatTable(spec, table);
    if (typeof flat === 'string') return flat;
    parsed.push(flat);
  }
  if (spec.strata !== null) return parsed;
  return parsed[0] ?? 'bad_table_shape';
}

type Parsed = { ok: true; value: unknown } | { ok: false; reason: string };

function fail(reason: string): Parsed {
  return { ok: false, reason };
}

function parseScalar(input: InputDef, text: string): Parsed {
  const kind: InputKind = input.kind;
  if (kind === 'count') {
    const n = parseNumber(text);
    return n !== null && Number.isInteger(n) && n >= 0 && n < MAX_COUNT
      ? { ok: true, value: n }
      : fail('not_a_count');
  }
  if (typeof kind === 'object' && 'integer' in kind) {
    const n = parseNumber(text);
    if (n === null || !Number.isInteger(n)) return fail('not_an_integer');
    if (n < kind.integer.min) return fail('below_min');
    if (n > kind.integer.max) return fail('above_max');
    return { ok: true, value: n };
  }
  if (typeof kind === 'object' && 'real' in kind) {
    const n = parseNumber(text);
    if (n === null) return fail('not_finite');
    if (n < kind.real.min) return fail('below_min');
    if (n > kind.real.max) return fail('above_max');
    return { ok: true, value: n };
  }
  if (kind === 'proportion') {
    const n = parseNumber(text);
    if (n === null) return fail('not_finite');
    return n > 0 && n < 1 ? { ok: true, value: n } : fail('out_of_range');
  }
  if (kind === 'percent') {
    const n = parseNumber(text);
    if (n === null) return fail('not_finite');
    if (n < 0) return fail('below_min');
    if (n > 100) return fail('above_max');
    return { ok: true, value: n };
  }
  if (kind === 'conf_level') {
    const n = parseNumber(text);
    return n !== null && (n === 90 || n === 95 || n === 99)
      ? { ok: true, value: n }
      : fail('bad_conf_level');
  }
  if (typeof kind === 'object' && 'choice' in kind) {
    return kind.choice.options.includes(text)
      ? { ok: true, value: text }
      : fail('bad_choice');
  }
  return fail('bad_table_shape');
}

/** Validates `drafts` against `def`: parses every present draft by its
 *  declared kind and reports failures per input — never throws. A
 *  "one of" `InputGroup` with more than one member supplied is
 *  `one_of` under the joined member names (missing members are not
 *  flagged here: a truncated link simply does not compute). */
export function collectInputs(def: CalculatorDef, drafts: Drafts): DecodedInputs {
  const values: Record<string, unknown> = {};
  const errors: Record<string, string> = {};
  const present = new Set<string>();
  for (const input of def.inputs) {
    const draft = drafts[input.name];
    if (draft === undefined) continue;
    if (tableKind(input) !== null) {
      const parsed = parseTable(input, draft as string[][][]);
      if (typeof parsed === 'string') {
        errors[input.name] = parsed;
      } else {
        values[input.name] = parsed;
        present.add(input.name);
      }
    } else {
      const parsed = parseScalar(input, draft as string);
      if (parsed.ok) {
        values[input.name] = parsed.value;
        present.add(input.name);
      } else {
        errors[input.name] = parsed.reason;
      }
    }
  }
  for (const group of def.groups) {
    const given = group.members.filter((m) => present.has(m) || drafts[m] !== undefined);
    if (given.length > 1) errors[group.members.join('/')] = 'one_of';
  }
  return { values, errors };
}

/** The required inputs (and "one of" groups) `drafts` leaves
 *  unsatisfied, for the submit path — `missing` under each name. */
export function missingErrors(def: CalculatorDef, values: Record<string, unknown>, drafts: Drafts): Record<string, string> {
  const errors: Record<string, string> = {};
  const present = new Set(
    def.inputs
      .filter((i) => values[i.name] !== undefined || drafts[i.name] !== undefined)
      .map((i) => i.name),
  );
  for (const input of def.inputs) {
    if (input.default === null && !present.has(input.name)) {
      errors[input.name] = 'missing';
    }
  }
  for (const group of def.groups) {
    if (!group.members.some((m) => present.has(m))) {
      errors[group.members.join('/')] = 'missing';
    }
  }
  return errors;
}

/** `draftsFromParams` + `collectInputs` + unknown-key reporting. */
export function decodeInputs(
  def: CalculatorDef,
  params: URLSearchParams,
): DecodedInputs {
  const drafts = draftsFromParams(def, params);
  const decoded = collectInputs(def, drafts);
  const unknown = [...params.keys()].filter(
    (key) => !def.inputs.some((i) => i.name === key),
  );
  if (unknown.length > 0) decoded.errors._unknown = unknown.join(',');
  return decoded;
}

function encodeFlat(rows: readonly (readonly unknown[])[]): string {
  return rows.map((row) => row.map(String).join(',')).join(';');
}

/** A fresh empty table draft: `min_rows` rows of the declared (or
 *  default 2) column width, one stratum. */
export function emptyTableDraft(input: InputDef): string[][][] {
  const spec = tableKind(input);
  const cols = spec?.cols === 0 ? 2 : (spec?.cols ?? 2);
  return [
    Array.from({ length: spec?.min_rows ?? 2 }, () =>
      Array<string>(cols).fill(''),
    ),
  ];
}

/** The declared default value `input` fills when absent. */
export function defaultDraft(input: InputDef): Draft | undefined {
  const d: DefaultValue | null = input.default;
  if (d === null || d === 'omit') return undefined;
  if (typeof d === 'object' && 'number' in d) return String(d.number);
  if (typeof d === 'object' && 'choice' in d) return d.choice;
  return undefined;
}

/** Canonical `values` → search params, in declaration order. A strata
 *  value is a list of tables (`|` between them); a flat table a single
 *  `;`-joined row list; scalars and choices their plain text. */
export function encodeInputs(
  def: CalculatorDef,
  values: Record<string, unknown>,
): URLSearchParams {
  const params = new URLSearchParams();
  for (const input of def.inputs) {
    const v = values[input.name];
    if (v === undefined) continue;
    if (tableKind(input) !== null && Array.isArray(v)) {
      const nested = Array.isArray(v[0]?.[0]);
      const tables = (nested ? v : [v]) as unknown[][][];
      params.set(input.name, tables.map((t) => encodeFlat(t)).join('|'));
    } else {
      params.set(input.name, String(v));
    }
  }
  return params;
}

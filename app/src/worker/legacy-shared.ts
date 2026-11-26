// Shared legacy worker types and pure helpers (v1.1 plan Task 15).
// Kept separate so the import, refresh and plain-table modules each
// stay single-purpose and below the repository file-size rule.

import type { SnapshotColumnInfo } from './snapshots';
import { snapshotColumns } from './snapshots';

/** One `project` member of `legacyMap`'s return — the golden
 *  `project.json` vocabulary (plus `key_column`, `text` and
 *  `datasets`, which the store needs). */
export interface LegacyProjectJson {
  name: string;
  legacy_id: string;
  source_format: string;
  views: {
    name: string;
    records: number;
    deleted_records: number;
    hash: string;
    key_column: string;
    fields: SnapshotColumnInfo[];
  }[];
  datasets: { name: string; records: number; hash: string; fields: SnapshotColumnInfo[] }[];
  links: {
    parent: string;
    child: string;
    child_key: string;
    parent_key: string;
    kind: 'relate' | 'grid';
  }[];
  programs: { name: string; text: string; text_sha256: string }[];
  check_code: { view: string; blocks: number; text: string }[];
  report: { kind: string; item: string; reason: string }[];
}

export interface RefreshTypeChange {
  column: string;
  from: string;
  to: string;
}

export interface RefreshFieldChange {
  record_id: string;
  row: number;
  field: string;
  old: unknown;
  new: unknown;
}

/** The `Diff` nested in a changed refresh outcome, plus the record
 *  ids the worker derives for the added row indices before returning
 *  the preview. */
export interface RefreshDiff {
  added: number[];
  added_records?: (string | null)[];
  removed: string[];
  changed: RefreshFieldChange[];
  schema: {
    added_columns: string[];
    removed_columns: string[];
    type_changes: RefreshTypeChange[];
  };
  unchanged: boolean;
}

/** `RefreshPlanJson['plan']` — every view's refresh decision. */
export interface RefreshPlanJson {
  views: {
    name: string;
    outcome: 'unchanged' | 'missing' | 'new' | 'no_key' | { changed: RefreshDiff };
  }[];
}

export interface LegacyPreview {
  previewId: string;
  project: LegacyProjectJson;
}

export interface RefreshPreview {
  previewId: string;
  plan: RefreshPlanJson;
  project: LegacyProjectJson;
}

/** A picked file — the plan's `File[]` spelling — or the equivalent
 *  `{name, bytes}` plain object tests and pre-read callers use. */
export type FileInput = File | { name: string; bytes: Uint8Array };

export interface SqlServerTableInput {
  name: string;
  batches: Uint8Array[];
}

export interface PlainTableInfo {
  name: string;
  records: number;
  hash: string;
}

export interface PlainPreview {
  previewId: string;
  sourceName: string;
  tables: string[];
}

/** `File`s read via `arrayBuffer()`; `{name, bytes}` inputs pass
 *  through. Discriminates on `arrayBuffer` — modern `File`s also
 *  carry a `bytes()` method, so `'bytes' in f` cannot tell them
 *  apart. */
export async function toNameBytes(
  files: FileInput[],
): Promise<{ name: string; bytes: Uint8Array }[]> {
  return Promise.all(
    files.map(async (f) =>
      'arrayBuffer' in f ? { name: f.name, bytes: new Uint8Array(await f.arrayBuffer()) } : f,
    ),
  );
}

/** Adds the per-column legacy/Epiora type preview the route shows
 *  before writing. The metadata is read from the snapshots the mapper
 *  just produced — no store write and no row decode. */
export function enrichProject(
  project: Omit<LegacyProjectJson, 'views' | 'datasets'> & {
    views: Omit<LegacyProjectJson['views'][number], 'fields'>[];
    datasets: Omit<LegacyProjectJson['datasets'][number], 'fields'>[];
  },
  snapshots: Uint8Array[],
): LegacyProjectJson {
  return {
    ...project,
    views: project.views.map((v, i) => ({ ...v, fields: snapshotColumns(snapshots[i]!) })),
    datasets: project.datasets.map((d, j) => ({
      ...d,
      fields: snapshotColumns(snapshots[project.views.length + j]!),
    })),
  };
}

/** Tables offered for plain import: the same non-metadata list the
 *  legacy detector's `legacy_layout_unknown` contract uses. */
export function plainTableNames(tables: string[]): string[] {
  return tables.filter((t) => {
    const lower = t.toLowerCase();
    return !lower.startsWith('meta') && !lower.startsWith('sqlite_');
  });
}

/** File stem used as the plain-import project name. */
export function fileStem(name: string): string {
  const base = name.split(/[\\/]/).pop() ?? name;
  return base.replace(/\.[^.]+$/, '') || base;
}

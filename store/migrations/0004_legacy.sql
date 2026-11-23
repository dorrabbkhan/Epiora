-- Store schema v4 (v1.1 plan Task 6, spec §4.2): the tables a legacy
-- import writes. `legacy_projects` marks a project as mapped from an
-- Epi Info file so re-import finds it (identified by the legacy id and
-- file name); `dataset_links` holds the parent/child relations the
-- mapper emits (Relate fields and grids, `FKEY` → `GlobalRecordId`);
-- `programs` keeps the saved `.pgm` documents; `check_code` the
-- per-view check code text with its block count; `migration_reports`
-- one JSON report per import. `IF NOT EXISTS` keeps the file
-- idempotent — the conformance rewind tests replay it over a database
-- whose v4 tables still stand.

CREATE TABLE IF NOT EXISTS legacy_projects (
  project_id    TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
  legacy_id     TEXT NOT NULL,
  source_format TEXT NOT NULL,
  file_name     TEXT NOT NULL,
  imported_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS dataset_links (
  project_id     TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  parent_dataset TEXT NOT NULL,
  child_dataset  TEXT NOT NULL,
  child_key      TEXT NOT NULL,
  parent_key     TEXT NOT NULL,
  kind           TEXT NOT NULL CHECK (kind IN ('relate', 'grid')),
  PRIMARY KEY (project_id, parent_dataset, child_dataset, child_key, parent_key)
);

CREATE TABLE IF NOT EXISTS programs (
  id         TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  name       TEXT NOT NULL,
  text       TEXT NOT NULL,
  source     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS check_code (
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  dataset_id TEXT NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
  text       TEXT NOT NULL,
  blocks     INTEGER NOT NULL,
  PRIMARY KEY (project_id, dataset_id)
);

CREATE TABLE IF NOT EXISTS migration_reports (
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  version    INTEGER NOT NULL,
  json       TEXT NOT NULL,
  PRIMARY KEY (project_id, version)
);

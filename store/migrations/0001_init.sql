-- Epiora store schema v1 (v0.3 spec §4.1). Shared by both backends:
-- sqlite-wasm opfs-sahpool in the browser and native
-- SQLite3MultipleCiphers on desktop. Migrations are numbered,
-- forward-only and applied in one transaction by migrate(); the applied
-- version is recorded in meta.schema_version. `PRAGMA foreign_keys = ON`
-- is set per connection by the backend, not here.

CREATE TABLE projects (
  id         TEXT PRIMARY KEY,
  name       TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE datasets (
  id              TEXT PRIMARY KEY,
  project_id      TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  name            TEXT NOT NULL,
  record_key      TEXT NOT NULL,
  settings        TEXT NOT NULL,
  current_version INTEGER NOT NULL DEFAULT 0,
  created_at      TEXT NOT NULL
);

CREATE TABLE dataset_versions (
  dataset_id         TEXT NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
  version            INTEGER NOT NULL,
  source_name        TEXT NOT NULL,
  source_sha256      TEXT NOT NULL,
  row_count          INTEGER NOT NULL,
  content_hash       TEXT NOT NULL,
  snapshot           BLOB,
  snapshot_available INTEGER NOT NULL DEFAULT 1,
  PRIMARY KEY (dataset_id, version)
);

CREATE TABLE records (
  dataset_id    TEXT NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
  record_id     TEXT NOT NULL,
  record_values TEXT NOT NULL,
  version       INTEGER NOT NULL,
  PRIMARY KEY (dataset_id, record_id)
);

CREATE TABLE changes (
  seq          INTEGER PRIMARY KEY AUTOINCREMENT,
  change_id    TEXT NOT NULL,
  dataset_id   TEXT NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
  record_id    TEXT NOT NULL,
  kind         TEXT NOT NULL CHECK (kind IN ('insert', 'update', 'delete')),
  field        TEXT,
  value        TEXT,
  base_version INTEGER,
  author       TEXT NOT NULL,
  device_id    TEXT NOT NULL,
  lamport      INTEGER NOT NULL,
  import_id    TEXT
);
CREATE INDEX changes_dataset_seq ON changes (dataset_id, seq);

CREATE TABLE imports (
  id         TEXT PRIMARY KEY,
  dataset_id TEXT NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
  version    INTEGER NOT NULL,
  settings   TEXT NOT NULL,
  coercions  TEXT NOT NULL,
  encoding   TEXT,
  delimiter  TEXT,
  sheet      TEXT,
  created_at TEXT NOT NULL
);

CREATE TABLE runs (
  id              TEXT PRIMARY KEY,
  project_id      TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  procedure       TEXT NOT NULL,
  options         TEXT NOT NULL,
  dataset_id      TEXT NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
  dataset_version INTEGER NOT NULL,
  result          TEXT NOT NULL,
  receipt         TEXT NOT NULL,
  receipt_hash    TEXT NOT NULL,
  created_at      TEXT NOT NULL,
  deleted_at      TEXT
);
CREATE INDEX runs_project_created ON runs (project_id, created_at);

CREATE TABLE meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

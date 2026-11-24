-- Store schema v5 (v1.1 plan Task 11, spec §5.2): the tables the
-- Analysis session keeps. `sessions` is one row per cleaning session:
-- the dataset the recipe replays from, the recipe's canonical text
-- and the as-of instant it captured. `permanent_vars` holds a
-- project's PERMANENT-scope DEFINEs — a session's recipe opens with
-- them (spec §6.1). `IF NOT EXISTS` keeps the file idempotent — the
-- conformance rewind tests replay it over a database whose v5 tables
-- still stand.

CREATE TABLE IF NOT EXISTS sessions (
  id             TEXT PRIMARY KEY,
  project_id     TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  source_dataset TEXT NOT NULL,
  source_version INTEGER NOT NULL,
  recipe_text    TEXT NOT NULL,
  as_of          TEXT NOT NULL,
  updated_at     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS permanent_vars (
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  name       TEXT NOT NULL,
  type       TEXT NOT NULL,
  expr_text  TEXT,
  PRIMARY KEY (project_id, name)
);

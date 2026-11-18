-- Store schema v2 (v1.0 plan Task 15, spec §7.3): every result card
-- shows the equivalent Epi Info command. New runs store it at insert;
-- rows written before this migration keep NULL — the worker generates
-- the command on load and never writes it back.

ALTER TABLE runs ADD COLUMN command TEXT;

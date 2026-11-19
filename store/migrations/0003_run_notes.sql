-- Store schema v3 (v1.0 plan Task 23, spec §6.1): every result card
-- shows the run's assumption notes beneath the table they concern.
-- New runs store the engine's `notes` JSON at insert; rows written
-- before this migration keep NULL — their cards simply show no
-- notes, since the side channel did not exist when they ran.

ALTER TABLE runs ADD COLUMN notes TEXT;

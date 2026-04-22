-- Bitemporal support: add recorded_at (Transaction Time) to observations and edges.
-- recorded_at captures when the system learned this fact, distinct from valid_from (Valid Time).

ALTER TABLE observations ADD COLUMN recorded_at TEXT NOT NULL DEFAULT '';
ALTER TABLE edges ADD COLUMN recorded_at TEXT NOT NULL DEFAULT '';

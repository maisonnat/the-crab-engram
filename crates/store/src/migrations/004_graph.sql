-- Migration 004: Graph edges with temporal columns

CREATE TABLE IF NOT EXISTS edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id INTEGER NOT NULL REFERENCES observations(id),
    target_id INTEGER NOT NULL REFERENCES observations(id),
    relation TEXT NOT NULL,
    weight REAL NOT NULL DEFAULT 1.0,
    valid_from TEXT NOT NULL DEFAULT (datetime('now')),
    valid_until TEXT,
    superseded_by INTEGER REFERENCES edges(id),
    auto_detected INTEGER NOT NULL DEFAULT 0,
    UNIQUE(source_id, target_id, relation, valid_from)
);

CREATE INDEX IF NOT EXISTS idx_edges_source ON edges(source_id);
CREATE INDEX IF NOT EXISTS idx_edges_target ON edges(target_id);
CREATE INDEX IF NOT EXISTS idx_edges_valid_from ON edges(valid_from);
CREATE INDEX IF NOT EXISTS idx_edges_relation ON edges(relation);

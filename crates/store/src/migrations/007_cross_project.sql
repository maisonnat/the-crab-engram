-- Migration 007: Cross-project knowledge transfers

CREATE TABLE IF NOT EXISTS knowledge_transfers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_project TEXT NOT NULL,
    target_project TEXT NOT NULL,
    capsule_id INTEGER REFERENCES knowledge_capsules(id),
    relevance_score REAL NOT NULL DEFAULT 0.0,
    transfer_type TEXT NOT NULL DEFAULT 'direct_applicable',
    accepted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_transfers_target ON knowledge_transfers(target_project);
CREATE INDEX IF NOT EXISTS idx_transfers_source ON knowledge_transfers(source_project);

-- Migration 012: Knowledge boundaries

CREATE TABLE IF NOT EXISTS knowledge_boundaries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    domain TEXT NOT NULL UNIQUE,
    confidence_level TEXT NOT NULL DEFAULT 'unknown',
    evidence TEXT NOT NULL DEFAULT '[]',
    successful_applications INTEGER NOT NULL DEFAULT 0,
    failed_applications INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_boundaries_domain ON knowledge_boundaries(domain);

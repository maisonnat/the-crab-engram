-- Migration 006: Knowledge capsules

CREATE TABLE IF NOT EXISTS knowledge_capsules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    topic TEXT NOT NULL,
    project TEXT,
    summary TEXT NOT NULL DEFAULT '',
    key_decisions TEXT NOT NULL DEFAULT '[]',
    known_issues TEXT NOT NULL DEFAULT '[]',
    anti_patterns TEXT NOT NULL DEFAULT '[]',
    best_practices TEXT NOT NULL DEFAULT '[]',
    source_observations TEXT NOT NULL DEFAULT '[]',
    confidence REAL NOT NULL DEFAULT 0.5,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_consolidated TEXT NOT NULL DEFAULT (datetime('now')),
    version INTEGER NOT NULL DEFAULT 1,
    UNIQUE(topic, project)
);

CREATE INDEX IF NOT EXISTS idx_capsules_project ON knowledge_capsules(project);
CREATE INDEX IF NOT EXISTS idx_capsules_topic ON knowledge_capsules(topic);

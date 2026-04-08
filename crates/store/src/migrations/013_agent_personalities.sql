-- Migration 013: Agent personalities

CREATE TABLE IF NOT EXISTS agent_personalities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    project TEXT NOT NULL,
    working_style TEXT NOT NULL DEFAULT 'balanced',
    preferences TEXT NOT NULL DEFAULT '{}',
    strengths TEXT NOT NULL DEFAULT '[]',
    weaknesses TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(agent_id, project)
);

CREATE INDEX IF NOT EXISTS idx_personalities_agent ON agent_personalities(agent_id);

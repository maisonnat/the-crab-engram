-- Migration 008: Episodic-Semantic memory separation

CREATE TABLE IF NOT EXISTS episodic_memories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    observation_id INTEGER NOT NULL REFERENCES observations(id),
    session_id TEXT NOT NULL,
    what_happened TEXT NOT NULL,
    context TEXT NOT NULL DEFAULT '',
    emotional_valence REAL NOT NULL DEFAULT 0.0,
    surprise_factor REAL NOT NULL DEFAULT 0.0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS semantic_memories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    observation_id INTEGER NOT NULL REFERENCES observations(id),
    knowledge TEXT NOT NULL,
    domain TEXT NOT NULL DEFAULT '',
    confidence REAL NOT NULL DEFAULT 0.5,
    source_episodes TEXT NOT NULL DEFAULT '[]',
    last_validated TEXT NOT NULL DEFAULT (datetime('now')),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_episodic_session ON episodic_memories(session_id);
CREATE INDEX IF NOT EXISTS idx_semantic_domain ON semantic_memories(domain);

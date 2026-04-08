-- Migration 001: Base schema
-- Tables: sessions, observations, prompts

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    project TEXT NOT NULL,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at TEXT,
    summary TEXT
);

CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project);
CREATE INDEX IF NOT EXISTS idx_sessions_started ON sessions(started_at);

CREATE TABLE IF NOT EXISTS observations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    type TEXT NOT NULL,
    scope TEXT NOT NULL DEFAULT 'project',
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    project TEXT NOT NULL,
    topic_key TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    access_count INTEGER NOT NULL DEFAULT 0,
    last_accessed TEXT,
    pinned INTEGER NOT NULL DEFAULT 0,
    normalized_hash TEXT NOT NULL,
    provenance_source TEXT DEFAULT 'llm_reasoning',
    provenance_confidence REAL DEFAULT 0.6,
    provenance_evidence TEXT DEFAULT '[]',
    lifecycle_state TEXT DEFAULT 'active',
    emotional_valence REAL DEFAULT 0.0,
    surprise_factor REAL DEFAULT 0.0,
    effort_invested REAL DEFAULT 0.0
);

CREATE INDEX IF NOT EXISTS idx_obs_session ON observations(session_id);
CREATE INDEX IF NOT EXISTS idx_obs_type ON observations(type);
CREATE INDEX IF NOT EXISTS idx_obs_project ON observations(project);
CREATE INDEX IF NOT EXISTS idx_obs_created ON observations(created_at);
CREATE INDEX IF NOT EXISTS idx_obs_topic ON observations(topic_key);
CREATE INDEX IF NOT EXISTS idx_obs_hash ON observations(normalized_hash);
CREATE INDEX IF NOT EXISTS idx_obs_lifecycle ON observations(lifecycle_state);

CREATE TABLE IF NOT EXISTS prompts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    project TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_prompts_session ON prompts(session_id);

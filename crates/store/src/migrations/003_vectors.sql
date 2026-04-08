-- Vector embeddings table for hybrid search
-- Stub schema — actual vector search requires sqlite-vec extension

CREATE TABLE IF NOT EXISTS embeddings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    observation_id INTEGER NOT NULL REFERENCES observations(id) ON DELETE CASCADE,
    model_name TEXT NOT NULL DEFAULT 'default',
    model_version TEXT NOT NULL DEFAULT '1.0',
    vector_blob BLOB NOT NULL,
    dimensions INTEGER NOT NULL DEFAULT 384,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE(observation_id, model_name)
);

CREATE INDEX IF NOT EXISTS idx_embeddings_observation ON embeddings(observation_id);
CREATE INDEX IF NOT EXISTS idx_embeddings_model ON embeddings(model_name, model_version);

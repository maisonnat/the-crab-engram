-- Migration 011: Observation attachments (multimodal)

CREATE TABLE IF NOT EXISTS observation_attachments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    observation_id INTEGER NOT NULL REFERENCES observations(id) ON DELETE CASCADE,
    attachment_type TEXT NOT NULL,  -- code_diff, terminal_output, error_trace, git_commit
    content TEXT NOT NULL,           -- JSON serialized Attachment
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_attachments_obs ON observation_attachments(observation_id);
CREATE INDEX IF NOT EXISTS idx_attachments_type ON observation_attachments(attachment_type);

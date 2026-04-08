-- Migration 009: Spaced repetition review schedule

CREATE TABLE IF NOT EXISTS review_schedule (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    observation_id INTEGER NOT NULL REFERENCES observations(id),
    interval_days REAL NOT NULL DEFAULT 1.0,
    ease_factor REAL NOT NULL DEFAULT 2.5,
    next_review TEXT NOT NULL,
    review_count INTEGER NOT NULL DEFAULT 0,
    last_result TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(observation_id)
);

CREATE INDEX IF NOT EXISTS idx_review_next ON review_schedule(next_review);
CREATE INDEX IF NOT EXISTS idx_review_obs ON review_schedule(observation_id);

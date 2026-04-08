-- Migration 002: FTS5 full-text search index
-- Virtual table synced via triggers

CREATE VIRTUAL TABLE IF NOT EXISTS observations_fts USING fts5(
    title,
    content,
    content='observations',
    content_rowid='id'
);

-- Trigger: INSERT
CREATE TRIGGER IF NOT EXISTS observations_ai AFTER INSERT ON observations BEGIN
    INSERT INTO observations_fts(rowid, title, content)
    VALUES (new.id, new.title, new.content);
END;

-- Trigger: DELETE
CREATE TRIGGER IF NOT EXISTS observations_ad AFTER DELETE ON observations BEGIN
    INSERT INTO observations_fts(observations_fts, rowid, title, content)
    VALUES ('delete', old.id, old.title, old.content);
END;

-- Trigger: UPDATE
CREATE TRIGGER IF NOT EXISTS observations_au AFTER UPDATE ON observations BEGIN
    INSERT INTO observations_fts(observations_fts, rowid, title, content)
    VALUES ('delete', old.id, old.title, old.content);
    INSERT INTO observations_fts(rowid, title, content)
    VALUES (new.id, new.title, new.content);
END;

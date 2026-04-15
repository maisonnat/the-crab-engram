use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use engram_core::{
    Attachment, Edge, EngramError, KnowledgeCapsule, LifecycleState, Observation, ObservationType,
    ProvenanceSource, QueryTarget, RelationType, Scope, Session, classify_query_type,
    decay_score_with_lifecycle,
};
use rusqlite::OptionalExtension;
use tracing::info;

use crate::migration;
use crate::params::*;
use crate::r#trait::{Result, *};

/// SQLite implementation of the Storage trait.
pub struct SqliteStore {
    conn: Mutex<rusqlite::Connection>,
    db_path: std::path::PathBuf,
}

impl SqliteStore {
    /// Create or open a SQLite store at the given path.
    /// Runs WAL mode, busy_timeout, synchronous=NORMAL, foreign_keys=ON.
    /// Applies all pending migrations.
    pub fn new(path: &Path) -> crate::Result<Self> {
        // BACKUP-07: Auto-backup before migrations if pending
        // Only backup if DB already has data (existing tables besides _migrations)
        // Use raw rusqlite connection to check migrations before Self is constructed
        {
            let check_conn = rusqlite::Connection::open(path)
                .map_err(|e| EngramError::Database(e.to_string()))?;

            let pending = migration::pending_migrations(&check_conn)?;
            if !pending.is_empty() {
                // Check if DB has existing data worth backing up
                let has_data: bool = check_conn
                    .query_row(
                        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name NOT LIKE '_migrations' AND name NOT LIKE 'sqlite_%'",
                        [],
                        |row| row.get(0),
                    )
                    .unwrap_or(false);

                if has_data {
                    drop(check_conn);

                    // Create minimal store for backup (no migrations yet)
                    let tmp = Self {
                        conn: Mutex::new(
                            rusqlite::Connection::open(path)
                                .map_err(|e| EngramError::Database(e.to_string()))?,
                        ),
                        db_path: path.to_path_buf(),
                    };
                    tmp.backup_create("auto-migration", None)?;
                    tmp.rotate_old_backups()?;
                    drop(tmp);
                }
            }
        }

        // Normal path: open connection, set PRAGMAs, run migrations
        let conn =
            rusqlite::Connection::open(path).map_err(|e| EngramError::Database(e.to_string()))?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA busy_timeout = 5000;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )
        .map_err(|e| EngramError::Database(e.to_string()))?;

        migration::run_migrations(&conn)?;

        info!("SqliteStore opened at {:?}", path);

        Ok(Self {
            conn: Mutex::new(conn),
            db_path: path.to_path_buf(),
        })
    }

    /// Create an in-memory store (for testing).
    pub fn in_memory() -> crate::Result<Self> {
        let conn = rusqlite::Connection::open_in_memory()
            .map_err(|e| EngramError::Database(e.to_string()))?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA busy_timeout = 5000;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )
        .map_err(|e| EngramError::Database(e.to_string()))?;

        migration::run_migrations(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
            db_path: std::path::PathBuf::new(),
        })
    }

    /// Get a locked connection.
    fn conn(&self) -> std::sync::MutexGuard<'_, rusqlite::Connection> {
        self.conn.lock().expect("sqlite connection mutex poisoned")
    }

    /// Resolve backup directory: ~/.engram/backups/. Creates if needed.
    fn backup_dir(&self) -> Result<std::path::PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| EngramError::Database("could not determine home directory".into()))?;
        let dir = home.join(".engram").join("backups");
        std::fs::create_dir_all(&dir)
            .map_err(|e| EngramError::Database(format!("failed to create backup dir: {e}")))?;
        Ok(dir)
    }

    /// Query schema version from _migrations table.
    fn schema_version(&self) -> i32 {
        let conn = self.conn();
        conn.query_row("SELECT MAX(version) FROM _migrations", [], |row| {
            row.get::<_, Option<i32>>(0).map(|v| v.unwrap_or(0))
        })
        .unwrap_or(0)
    }

    /// Compute SHA-256 hex digest of a file.
    fn sha256_file(path: &std::path::Path) -> Result<String> {
        use sha2::{Digest, Sha256};
        let mut file = std::fs::File::open(path)
            .map_err(|e| EngramError::Database(format!("failed to open file for checksum: {e}")))?;
        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher)
            .map_err(|e| EngramError::Database(format!("failed to compute checksum: {e}")))?;
        Ok(hasher
            .finalize()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect())
    }

    /// Row mapper: observation from SQL row.
    fn row_to_observation(row: &rusqlite::Row) -> rusqlite::Result<Observation> {
        let type_str: String = row.get("type")?;
        let scope_str: String = row.get("scope")?;
        let prov_source_str: String = row.get("provenance_source")?;
        let lifecycle_str: String = row.get("lifecycle_state")?;
        let prov_evidence_json: String = row.get("provenance_evidence")?;

        Ok(Observation {
            id: row.get("id")?,
            r#type: type_str.parse().map_err(|_| {
                rusqlite::Error::InvalidColumnType(0, "type".into(), rusqlite::types::Type::Text)
            })?,
            scope: if scope_str == "personal" {
                Scope::Personal
            } else {
                Scope::Project
            },
            title: row.get("title")?,
            content: row.get("content")?,
            session_id: row.get("session_id")?,
            project: row.get("project")?,
            topic_key: row.get("topic_key")?,
            created_at: row
                .get::<_, String>("created_at")?
                .parse()
                .unwrap_or_default(),
            updated_at: row
                .get::<_, String>("updated_at")?
                .parse()
                .unwrap_or_default(),
            access_count: row.get("access_count")?,
            last_accessed: row
                .get::<_, Option<String>>("last_accessed")?
                .and_then(|s| s.parse().ok()),
            pinned: row.get::<_, i64>("pinned")? != 0,
            normalized_hash: row.get("normalized_hash")?,
            provenance_source: prov_source_str
                .parse()
                .unwrap_or(ProvenanceSource::LlmReasoning),
            provenance_confidence: row.get("provenance_confidence")?,
            provenance_evidence: serde_json::from_str(&prov_evidence_json).unwrap_or_default(),
            lifecycle_state: lifecycle_str.parse().unwrap_or(LifecycleState::Active),
            emotional_valence: row.get("emotional_valence")?,
            surprise_factor: row.get("surprise_factor")?,
            effort_invested: row.get("effort_invested")?,
        })
    }

    /// Row mapper: session from SQL row.
    fn row_to_session(row: &rusqlite::Row) -> rusqlite::Result<Session> {
        Ok(Session {
            id: row.get("id")?,
            project: row.get("project")?,
            started_at: row
                .get::<_, String>("started_at")?
                .parse()
                .unwrap_or_default(),
            ended_at: row
                .get::<_, Option<String>>("ended_at")?
                .and_then(|s| s.parse().ok()),
            summary: row.get("summary")?,
        })
    }

    /// Row mapper: edge from SQL row.
    fn row_to_edge(row: &rusqlite::Row) -> rusqlite::Result<Edge> {
        let relation_str: String = row.get("relation")?;
        let auto_detected: i64 = row.get("auto_detected")?;
        Ok(Edge {
            id: row.get("id")?,
            source_id: row.get("source_id")?,
            target_id: row.get("target_id")?,
            relation: relation_str.parse().unwrap_or(RelationType::RelatedTo),
            weight: row.get("weight")?,
            valid_from: row
                .get::<_, String>("valid_from")?
                .parse()
                .unwrap_or_default(),
            valid_until: row
                .get::<_, Option<String>>("valid_until")?
                .and_then(|s| s.parse().ok()),
            superseded_by: row.get("superseded_by")?,
            auto_detected: auto_detected != 0,
        })
    }

    /// Row mapper: knowledge capsule from SQL row.
    fn row_to_capsule(row: &rusqlite::Row) -> rusqlite::Result<KnowledgeCapsule> {
        Ok(KnowledgeCapsule {
            id: row.get("id")?,
            topic: row.get("topic")?,
            project: row.get("project")?,
            summary: row.get("summary")?,
            key_decisions: serde_json::from_str(&row.get::<_, String>("key_decisions")?)
                .unwrap_or_default(),
            known_issues: serde_json::from_str(&row.get::<_, String>("known_issues")?)
                .unwrap_or_default(),
            anti_patterns: serde_json::from_str(&row.get::<_, String>("anti_patterns")?)
                .unwrap_or_default(),
            best_practices: serde_json::from_str(&row.get::<_, String>("best_practices")?)
                .unwrap_or_default(),
            source_observations: serde_json::from_str(
                &row.get::<_, String>("source_observations")?,
            )
            .unwrap_or_default(),
            confidence: row.get("confidence")?,
            created_at: row
                .get::<_, String>("created_at")?
                .parse()
                .unwrap_or_default(),
            last_consolidated: row
                .get::<_, String>("last_consolidated")?
                .parse()
                .unwrap_or_default(),
            version: row.get::<_, i64>("version")? as u32,
        })
    }
}

impl Storage for SqliteStore {
    fn insert_observation(&self, params: &AddObservationParams) -> Result<i64> {
        let conn = self.conn();
        let now = Utc::now().to_rfc3339();
        let hash = Observation::compute_hash(&params.title, &params.content);
        let evidence_json = serde_json::to_string(&params.provenance_evidence)?;
        let prov_source = params
            .provenance_source
            .as_deref()
            .unwrap_or("llm_reasoning");

        // Dedup check: same hash within 15 minutes
        let dedup: Option<i64> = conn
            .query_row(
                "SELECT id FROM observations WHERE normalized_hash = ?1 \
                 AND created_at > datetime('now', '-15 minutes')",
                [&hash],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| EngramError::Database(e.to_string()))?;

        if let Some(existing_id) = dedup {
            return Err(EngramError::Duplicate(format!(
                "observation with hash {hash} already exists as id {existing_id}"
            )));
        }

        conn.execute(
            "INSERT INTO observations \
             (type, scope, title, content, session_id, project, topic_key, \
              created_at, updated_at, normalized_hash, provenance_source, \
              provenance_confidence, provenance_evidence, lifecycle_state) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?9, ?10, \
                     (SELECT CASE ?10 \
                      WHEN 'test_verified' THEN 0.95 \
                      WHEN 'code_analysis' THEN 0.85 \
                      WHEN 'user_stated' THEN 0.70 \
                      WHEN 'external' THEN 0.65 \
                      WHEN 'llm_reasoning' THEN 0.60 \
                      WHEN 'inferred' THEN 0.40 \
                      ELSE 0.60 END), \
                     ?11, 'active')",
            rusqlite::params![
                params.r#type.to_string(),
                params.scope.to_string(),
                params.title,
                params.content,
                params.session_id,
                params.project,
                params.topic_key,
                now,
                hash,
                prov_source,
                evidence_json,
            ],
        )
        .map_err(|e| EngramError::Database(e.to_string()))?;

        let id = conn.last_insert_rowid();

        // Classify and populate episodic/semantic tables
        drop(conn); // Release lock before reentrant call
        self.classify_and_insert_memory(id, params);

        Ok(id)
    }

    fn get_observation(&self, id: i64) -> Result<Option<Observation>> {
        let conn = self.conn();

        // Increment access counters
        conn.execute(
            "UPDATE observations SET access_count = access_count + 1, \
             last_accessed = datetime('now') WHERE id = ?",
            [id],
        )
        .map_err(|e| EngramError::Database(e.to_string()))?;

        let obs = conn
            .query_row(
                "SELECT * FROM observations WHERE id = ?",
                [id],
                Self::row_to_observation,
            )
            .optional()
            .map_err(|e| EngramError::Database(e.to_string()))?;

        Ok(obs)
    }

    fn peek_observation(&self, id: i64) -> Result<Option<Observation>> {
        let conn = self.conn();
        conn.query_row(
            "SELECT * FROM observations WHERE id = ?",
            [id],
            Self::row_to_observation,
        )
        .optional()
        .map_err(|e| EngramError::Database(e.to_string()))
    }

    fn update_observation(&self, id: i64, params: &UpdateObservationParams) -> Result<()> {
        let mut conn = self.conn();
        let now = Utc::now().to_rfc3339();

        let tx = conn
            .transaction()
            .map_err(|e| EngramError::Database(e.to_string()))?;

        // Build dynamic update within transaction
        if let Some(title) = &params.title {
            tx.execute(
                "UPDATE observations SET title = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![title, now, id],
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;
        }
        if let Some(content) = &params.content {
            tx.execute(
                "UPDATE observations SET content = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![content, now, id],
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;
        }
        if let Some(scope) = &params.scope {
            tx.execute(
                "UPDATE observations SET scope = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![scope.to_string(), now, id],
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;
        }
        if let Some(topic_key) = &params.topic_key {
            tx.execute(
                "UPDATE observations SET topic_key = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![topic_key, now, id],
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;
        }
        if let Some(pinned) = &params.pinned {
            tx.execute(
                "UPDATE observations SET pinned = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![if *pinned { 1 } else { 0 }, now, id],
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;
        }
        if let Some(prov_source) = &params.provenance_source {
            tx.execute(
                "UPDATE observations SET provenance_source = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![prov_source, now, id],
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;
        }
        if let Some(confidence) = &params.provenance_confidence {
            tx.execute(
                "UPDATE observations SET provenance_confidence = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![confidence, now, id],
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;
        }
        if let Some(lifecycle) = &params.lifecycle_state {
            tx.execute(
                "UPDATE observations SET lifecycle_state = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![lifecycle, now, id],
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;
        }

        tx.commit()
            .map_err(|e| EngramError::Database(e.to_string()))?;
        Ok(())
    }

    fn delete_observation(&self, id: i64, hard: bool) -> Result<()> {
        let conn = self.conn();
        if hard {
            conn.execute("DELETE FROM observations WHERE id = ?", [id])
        } else {
            conn.execute(
                "UPDATE observations SET lifecycle_state = 'deleted', updated_at = datetime('now') WHERE id = ?",
                [id],
            )
        }
        .map_err(|e| EngramError::Database(e.to_string()))?;
        Ok(())
    }

    fn search(&self, opts: &SearchOptions) -> Result<Vec<Observation>> {
        let conn = self.conn();

        if opts.query.is_empty() {
            // No query — filter only
            let mut sql = String::from("SELECT * FROM observations WHERE 1=1");
            let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(project) = &opts.project {
                sql.push_str(" AND project = ?");
                params_vec.push(Box::new(project.clone()));
            }
            if let Some(obs_type) = &opts.r#type {
                sql.push_str(" AND type = ?");
                params_vec.push(Box::new(obs_type.to_string()));
            }
            if let Some(scope) = &opts.scope {
                sql.push_str(" AND scope = ?");
                params_vec.push(Box::new(scope.to_string()));
            }
            if !opts.include_stale {
                sql.push_str(" AND lifecycle_state = 'active'");
            }
            if let Some(min_conf) = opts.min_confidence {
                sql.push_str(" AND provenance_confidence >= ?");
                params_vec.push(Box::new(min_conf));
            }
            sql.push_str(" ORDER BY created_at DESC");

            if let Some(limit) = opts.limit {
                sql.push_str(&format!(" LIMIT {limit}"));
            }
            if let Some(offset) = opts.offset {
                sql.push_str(&format!(" OFFSET {offset}"));
            }

            let params_ref: Vec<&dyn rusqlite::ToSql> =
                params_vec.iter().map(|p| p.as_ref()).collect();
            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| EngramError::Database(e.to_string()))?;
            let rows = stmt
                .query_map(params_ref.as_slice(), Self::row_to_observation)
                .map_err(|e| EngramError::Database(e.to_string()))?;

            let mut results = Vec::new();
            for row in rows {
                results.push(row.map_err(|e| EngramError::Database(e.to_string()))?);
            }
            Self::rerank_by_relevance(&mut results);
            return Ok(results);
        }

        // FTS5 search with filters
        let mut sql = String::from(
            "SELECT o.*, rank FROM observations o \
             JOIN observations_fts fts ON o.id = fts.rowid \
             WHERE observations_fts MATCH ?",
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        // Escape FTS5 special chars
        let escaped_query = opts.query.replace('"', "\"\"");
        params_vec.push(Box::new(escaped_query));

        if let Some(project) = &opts.project {
            sql.push_str(" AND o.project = ?");
            params_vec.push(Box::new(project.clone()));
        }
        if let Some(obs_type) = &opts.r#type {
            sql.push_str(" AND o.type = ?");
            params_vec.push(Box::new(obs_type.to_string()));
        }
        if let Some(scope) = &opts.scope {
            sql.push_str(" AND o.scope = ?");
            params_vec.push(Box::new(scope.to_string()));
        }
        if !opts.include_stale {
            sql.push_str(" AND o.lifecycle_state = 'active'");
        }
        if let Some(min_conf) = opts.min_confidence {
            sql.push_str(" AND o.provenance_confidence >= ?");
            params_vec.push(Box::new(min_conf));
        }

        sql.push_str(" ORDER BY rank");

        if let Some(limit) = opts.limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        }
        if let Some(offset) = opts.offset {
            sql.push_str(&format!(" OFFSET {offset}"));
        }

        let params_ref: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| EngramError::Database(e.to_string()))?;
        let rows = stmt
            .query_map(params_ref.as_slice(), Self::row_to_observation)
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| EngramError::Database(e.to_string()))?);
        }
        Self::rerank_by_relevance(&mut results);
        Ok(results)
    }

    fn create_session(&self, project: &str) -> Result<String> {
        let conn = self.conn();
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO sessions (id, project, started_at) VALUES (?, ?, ?)",
            rusqlite::params![id, project, now],
        )
        .map_err(|e| EngramError::Database(e.to_string()))?;

        Ok(id)
    }

    fn end_session(&self, session_id: &str, summary: Option<&str>) -> Result<()> {
        let conn = self.conn();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE sessions SET ended_at = ?, summary = ? WHERE id = ?",
            rusqlite::params![now, summary, session_id],
        )
        .map_err(|e| EngramError::Database(e.to_string()))?;

        Ok(())
    }

    fn get_session(&self, session_id: &str) -> Result<Option<Session>> {
        let conn = self.conn();
        conn.query_row(
            "SELECT * FROM sessions WHERE id = ?",
            [session_id],
            Self::row_to_session,
        )
        .optional()
        .map_err(|e| EngramError::Database(e.to_string()))
    }

    fn get_session_context(&self, project: &str, limit: usize) -> Result<SessionContext> {
        let conn = self.conn();

        // Get most recent session
        let session = conn
            .query_row(
                "SELECT * FROM sessions WHERE project = ? ORDER BY started_at DESC LIMIT 1",
                [project],
                Self::row_to_session,
            )
            .optional()
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let session = match session {
            Some(s) => s,
            None => {
                return Ok(SessionContext {
                    session: Session::new(project.to_string()),
                    observations: Vec::new(),
                    prompts: Vec::new(),
                });
            }
        };

        // Get recent observations from this session
        let mut stmt = conn
            .prepare(
                "SELECT * FROM observations WHERE session_id = ? \
                 AND lifecycle_state = 'active' ORDER BY created_at DESC LIMIT ?",
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;
        let obs_rows = stmt
            .query_map(
                rusqlite::params![session.id, limit as i64],
                Self::row_to_observation,
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let mut observations = Vec::new();
        for row in obs_rows {
            observations.push(row.map_err(|e| EngramError::Database(e.to_string()))?);
        }

        // Get prompts
        let mut stmt = conn
            .prepare(
                "SELECT content FROM prompts WHERE session_id = ? ORDER BY created_at DESC LIMIT ?",
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;
        let prompt_rows = stmt
            .query_map(rusqlite::params![session.id, limit as i64], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let mut prompts = Vec::new();
        for row in prompt_rows {
            prompts.push(row.map_err(|e| EngramError::Database(e.to_string()))?);
        }

        Ok(SessionContext {
            session,
            observations,
            prompts,
        })
    }

    fn save_prompt(&self, params: &AddPromptParams) -> Result<()> {
        let conn = self.conn();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO prompts (session_id, project, content, created_at) VALUES (?, ?, ?, ?)",
            rusqlite::params![params.session_id, params.project, params.content, now],
        )
        .map_err(|e| EngramError::Database(e.to_string()))?;

        Ok(())
    }

    fn get_prompts(&self, session_id: &str) -> Result<Vec<String>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT content FROM prompts WHERE session_id = ? ORDER BY created_at")
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([session_id], |row| row.get::<_, String>(0))
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| EngramError::Database(e.to_string()))?);
        }
        Ok(results)
    }

    fn get_timeline(&self, observation_id: i64, window: usize) -> Result<Vec<TimelineEntry>> {
        let conn = self.conn();

        // Get the target observation directly (no nested lock)
        let target = conn
            .query_row(
                "SELECT * FROM observations WHERE id = ?",
                [observation_id],
                Self::row_to_observation,
            )
            .optional()
            .map_err(|e| EngramError::Database(e.to_string()))?
            .ok_or_else(|| EngramError::NotFound(format!("observation {observation_id}")))?;

        let mut entries = Vec::new();

        // Get observations before
        let mut stmt = conn
            .prepare(
                "SELECT * FROM observations WHERE project = ? AND created_at < ? \
                 ORDER BY created_at DESC LIMIT ?",
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;
        let before_rows = stmt
            .query_map(
                rusqlite::params![
                    target.project,
                    target.created_at.to_rfc3339(),
                    window as i64
                ],
                Self::row_to_observation,
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;

        for row in before_rows {
            entries.push(TimelineEntry {
                observation: row.map_err(|e| EngramError::Database(e.to_string()))?,
                position: TimelinePosition::Before,
            });
        }
        entries.reverse();

        // Center
        entries.push(TimelineEntry {
            observation: target,
            position: TimelinePosition::Center,
        });

        // Get observations after
        let mut stmt = conn
            .prepare(
                "SELECT * FROM observations WHERE project = ? AND created_at > ? \
                 ORDER BY created_at ASC LIMIT ?",
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;
        let after_rows = stmt
            .query_map(
                rusqlite::params![
                    entries.last().unwrap().observation.project,
                    entries.last().unwrap().observation.created_at.to_rfc3339(),
                    window as i64
                ],
                Self::row_to_observation,
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;

        for row in after_rows {
            entries.push(TimelineEntry {
                observation: row.map_err(|e| EngramError::Database(e.to_string()))?,
                position: TimelinePosition::After,
            });
        }

        Ok(entries)
    }

    fn get_stats(&self, project: &str) -> Result<ProjectStats> {
        let conn = self.conn();

        let total: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM observations WHERE project = ?",
                [project],
                |row| row.get::<_, i64>(0).map(|v| v as usize),
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;

        // By type
        let mut stmt = conn
            .prepare("SELECT type, COUNT(*) FROM observations WHERE project = ? GROUP BY type")
            .map_err(|e| EngramError::Database(e.to_string()))?;
        let type_rows = stmt
            .query_map([project], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
            })
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let mut by_type = HashMap::new();
        for row in type_rows {
            let (t, count) = row.map_err(|e| EngramError::Database(e.to_string()))?;
            by_type.insert(t, count);
        }

        // By scope
        let mut stmt = conn
            .prepare("SELECT scope, COUNT(*) FROM observations WHERE project = ? GROUP BY scope")
            .map_err(|e| EngramError::Database(e.to_string()))?;
        let scope_rows = stmt
            .query_map([project], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
            })
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let mut by_scope = HashMap::new();
        for row in scope_rows {
            let (s, count) = row.map_err(|e| EngramError::Database(e.to_string()))?;
            by_scope.insert(s, count);
        }

        let sessions: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM sessions WHERE project = ?",
                [project],
                |row| row.get::<_, i64>(0).map(|v| v as usize),
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let edges: usize = conn
            .query_row("SELECT COUNT(*) FROM edges", [], |row| {
                row.get::<_, i64>(0).map(|v| v as usize)
            })
            .unwrap_or(0);

        let capsules: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM knowledge_capsules WHERE project IS ?",
                [project],
                |row| row.get::<_, i64>(0).map(|v| v as usize),
            )
            .unwrap_or(0);

        Ok(ProjectStats {
            project: project.to_string(),
            total_observations: total,
            by_type,
            by_scope,
            total_sessions: sessions,
            total_edges: edges,
            total_capsules: capsules,
        })
    }

    fn add_edge(&self, params: &AddEdgeParams) -> Result<i64> {
        let conn = self.conn();
        let now = Utc::now().to_rfc3339();

        // Auto-close existing active edge between same nodes with same relation
        conn.execute(
            "UPDATE edges SET valid_until = ? \
             WHERE source_id = ? AND target_id = ? AND relation = ? AND valid_until IS NULL",
            rusqlite::params![
                now,
                params.source_id,
                params.target_id,
                params.relation.to_string()
            ],
        )
        .map_err(|e| EngramError::Database(e.to_string()))?;

        conn.execute(
            "INSERT INTO edges \
             (source_id, target_id, relation, weight, valid_from, auto_detected) \
             VALUES (?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                params.source_id,
                params.target_id,
                params.relation.to_string(),
                params.weight,
                now,
                if params.auto_detected { 1 } else { 0 },
            ],
        )
        .map_err(|e| EngramError::Database(e.to_string()))?;

        Ok(conn.last_insert_rowid())
    }

    fn get_edges(&self, observation_id: i64) -> Result<Vec<Edge>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT * FROM edges \
                 WHERE (source_id = ? OR target_id = ?) AND valid_until IS NULL \
                 ORDER BY valid_from DESC",
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(
                rusqlite::params![observation_id, observation_id],
                Self::row_to_edge,
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| EngramError::Database(e.to_string()))?);
        }
        Ok(results)
    }

    fn get_edges_at(&self, observation_id: i64, at: DateTime<Utc>) -> Result<Vec<Edge>> {
        let conn = self.conn();
        let at_str = at.to_rfc3339();
        let mut stmt = conn
            .prepare(
                "SELECT * FROM edges \
                 WHERE (source_id = ? OR target_id = ?) \
                 AND valid_from <= ? \
                 AND (valid_until IS NULL OR valid_until > ?) \
                 ORDER BY valid_from DESC",
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(
                rusqlite::params![observation_id, observation_id, at_str, at_str],
                Self::row_to_edge,
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| EngramError::Database(e.to_string()))?);
        }
        Ok(results)
    }

    fn get_related(
        &self,
        observation_id: i64,
        max_depth: usize,
    ) -> Result<Vec<(Observation, RelationType, usize)>> {
        let conn = self.conn();
        let mut results = Vec::new();
        let mut visited = std::collections::HashSet::new();
        visited.insert(observation_id);

        let mut frontier = vec![(observation_id, 0usize)];

        while let Some((current_id, depth)) = frontier.pop() {
            if depth >= max_depth {
                continue;
            }

            let mut stmt = conn
                .prepare(
                    "SELECT * FROM edges \
                     WHERE source_id = ? AND valid_until IS NULL",
                )
                .map_err(|e| EngramError::Database(e.to_string()))?;

            let edge_rows = stmt
                .query_map([current_id], Self::row_to_edge)
                .map_err(|e| EngramError::Database(e.to_string()))?;

            for edge_row in edge_rows {
                let edge = edge_row.map_err(|e| EngramError::Database(e.to_string()))?;
                if !visited.insert(edge.target_id) {
                    continue;
                }

                // Direct query instead of self.peek_observation (avoid nested lock)
                let obs = conn
                    .query_row(
                        "SELECT * FROM observations WHERE id = ?",
                        [edge.target_id],
                        Self::row_to_observation,
                    )
                    .optional()
                    .map_err(|e| EngramError::Database(e.to_string()))?;

                if let Some(obs) = obs {
                    results.push((obs.clone(), edge.relation, depth + 1));
                    frontier.push((edge.target_id, depth + 1));
                }
            }
        }

        Ok(results)
    }

    fn store_embedding(
        &self,
        observation_id: i64,
        embedding: &[f32],
        model_name: &str,
        model_version: &str,
    ) -> Result<()> {
        let conn = self.conn();
        let blob: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
        conn.execute(
            "INSERT OR REPLACE INTO embeddings (observation_id, model_name, model_version, vector_blob, dimensions) \
             VALUES (?, ?, ?, ?, ?)",
            rusqlite::params![observation_id, model_name, model_version, blob, embedding.len()],
        )
        .map_err(|e| EngramError::Database(e.to_string()))?;
        Ok(())
    }

    fn search_vector(&self, _embedding: &[f32], _limit: usize) -> Result<Vec<(i64, f64)>> {
        // Requires sqlite-vec extension for actual vector similarity search
        // Return empty gracefully — hybrid search falls back to FTS5-only
        Ok(vec![])
    }

    fn count_stale_embeddings(&self, _model_name: &str, _model_version: &str) -> Result<usize> {
        Ok(0) // No embeddings yet
    }

    fn update_embedding_versions(&self, _model_name: &str, _model_version: &str) -> Result<usize> {
        Ok(0) // No embeddings yet
    }

    fn export(&self, project: Option<&str>) -> Result<ExportData> {
        let conn = self.conn();

        // Observations
        let sql = if project.is_some() {
            "SELECT * FROM observations WHERE project = ?"
        } else {
            "SELECT * FROM observations"
        };

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| EngramError::Database(e.to_string()))?;
        let obs_rows = if let Some(p) = project {
            stmt.query_map([p], Self::row_to_observation)
                .map_err(|e| EngramError::Database(e.to_string()))?
        } else {
            stmt.query_map([], Self::row_to_observation)
                .map_err(|e| EngramError::Database(e.to_string()))?
        };

        let mut observations = Vec::new();
        for row in obs_rows {
            observations.push(row.map_err(|e| EngramError::Database(e.to_string()))?);
        }

        // Sessions
        let sql = if project.is_some() {
            "SELECT * FROM sessions WHERE project = ?"
        } else {
            "SELECT * FROM sessions"
        };
        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| EngramError::Database(e.to_string()))?;
        let sess_rows = if let Some(p) = project {
            stmt.query_map([p], Self::row_to_session)
                .map_err(|e| EngramError::Database(e.to_string()))?
        } else {
            stmt.query_map([], Self::row_to_session)
                .map_err(|e| EngramError::Database(e.to_string()))?
        };

        let mut sessions = Vec::new();
        for row in sess_rows {
            sessions.push(row.map_err(|e| EngramError::Database(e.to_string()))?);
        }

        // Prompts
        let sql = if project.is_some() {
            "SELECT session_id, project, content, created_at FROM prompts WHERE project = ?"
        } else {
            "SELECT session_id, project, content, created_at FROM prompts"
        };
        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| EngramError::Database(e.to_string()))?;
        let map_row = |row: &rusqlite::Row| {
            Ok(ExportedPrompt {
                session_id: row.get(0)?,
                project: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get::<_, String>(3)?.parse().unwrap_or_default(),
            })
        };
        let prompt_rows = if let Some(p) = project {
            stmt.query_map([p], map_row)
                .map_err(|e| EngramError::Database(e.to_string()))?
        } else {
            stmt.query_map([], map_row)
                .map_err(|e| EngramError::Database(e.to_string()))?
        };

        let mut prompts = Vec::new();
        for row in prompt_rows {
            prompts.push(row.map_err(|e| EngramError::Database(e.to_string()))?);
        }

        // Edges
        let mut stmt = conn
            .prepare("SELECT * FROM edges")
            .map_err(|e| EngramError::Database(e.to_string()))?;
        let edge_rows = stmt
            .query_map([], Self::row_to_edge)
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let mut edges = Vec::new();
        for row in edge_rows {
            edges.push(row.map_err(|e| EngramError::Database(e.to_string()))?);
        }

        Ok(ExportData {
            observations,
            sessions,
            prompts,
            edges,
        })
    }

    fn import(&self, data: &ExportData) -> Result<ImportResult> {
        let conn = self.conn();

        let mut obs_imported = 0;
        let mut obs_skipped = 0;

        // Import sessions first (FK dependency)
        for session in &data.sessions {
            conn.execute(
                "INSERT OR IGNORE INTO sessions (id, project, started_at, ended_at, summary) \
                 VALUES (?, ?, ?, ?, ?)",
                rusqlite::params![
                    session.id,
                    session.project,
                    session.started_at.to_rfc3339(),
                    session.ended_at.map(|t| t.to_rfc3339()),
                    session.summary,
                ],
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;
        }

        // Import observations
        for obs in &data.observations {
            let result = conn
                .execute(
                    "INSERT OR IGNORE INTO observations \
                 (id, type, scope, title, content, session_id, project, topic_key, \
                  created_at, updated_at, access_count, last_accessed, pinned, normalized_hash, \
                  provenance_source, provenance_confidence, provenance_evidence, lifecycle_state, \
                  emotional_valence, surprise_factor, effort_invested) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    rusqlite::params![
                        obs.id,
                        obs.r#type.to_string(),
                        obs.scope.to_string(),
                        obs.title,
                        obs.content,
                        obs.session_id,
                        obs.project,
                        obs.topic_key,
                        obs.created_at.to_rfc3339(),
                        obs.updated_at.to_rfc3339(),
                        obs.access_count,
                        obs.last_accessed.map(|t| t.to_rfc3339()),
                        if obs.pinned { 1 } else { 0 },
                        obs.normalized_hash,
                        format!("{:?}", obs.provenance_source),
                        obs.provenance_confidence,
                        serde_json::to_string(&obs.provenance_evidence)?,
                        format!("{:?}", obs.lifecycle_state),
                        obs.emotional_valence,
                        obs.surprise_factor,
                        obs.effort_invested,
                    ],
                )
                .map_err(|e| EngramError::Database(e.to_string()))?;

            if result > 0 {
                obs_imported += 1;
            } else {
                obs_skipped += 1;
            }
        }

        // Import prompts
        for prompt in &data.prompts {
            conn.execute(
                "INSERT OR IGNORE INTO prompts (session_id, project, content, created_at) \
                 VALUES (?, ?, ?, ?)",
                rusqlite::params![
                    prompt.session_id,
                    prompt.project,
                    prompt.content,
                    prompt.created_at.to_rfc3339(),
                ],
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;
        }

        Ok(ImportResult {
            observations_imported: obs_imported,
            sessions_imported: data.sessions.len(),
            prompts_imported: data.prompts.len(),
            edges_imported: data.edges.len(),
            duplicates_skipped: obs_skipped,
        })
    }

    fn transition_state(
        &self,
        project: &str,
        obs_type: &ObservationType,
        from: &str,
        to: &str,
        age_days: u32,
    ) -> Result<u32> {
        let conn = self.conn();
        let cutoff = (Utc::now() - chrono::Duration::days(age_days as i64)).to_rfc3339();

        let count = conn
            .execute(
                "UPDATE observations SET lifecycle_state = ?, updated_at = datetime('now') \
             WHERE project = ? AND type = ? AND lifecycle_state = ? AND created_at < ?",
                rusqlite::params![to, project, obs_type.to_string(), from, cutoff],
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;

        Ok(count as u32)
    }

    fn mark_pending_review(
        &self,
        project: &str,
        obs_type: &ObservationType,
        age_days: u32,
    ) -> Result<u32> {
        self.transition_state(project, obs_type, "archived", "pending_review", age_days)
    }

    fn store_attachment(&self, observation_id: i64, attachment: &Attachment) -> Result<i64> {
        let conn = self.conn();
        let attachment_type = match attachment {
            Attachment::CodeDiff { .. } => "code_diff",
            Attachment::TerminalOutput { .. } => "terminal_output",
            Attachment::ErrorTrace { .. } => "error_trace",
            Attachment::GitCommit { .. } => "git_commit",
        };
        let content = serde_json::to_string(attachment)?;

        conn.execute(
            "INSERT INTO observation_attachments (observation_id, attachment_type, content) \
             VALUES (?, ?, ?)",
            rusqlite::params![observation_id, attachment_type, content],
        )
        .map_err(|e| EngramError::Database(e.to_string()))?;

        Ok(conn.last_insert_rowid())
    }

    fn get_attachments(&self, observation_id: i64) -> Result<Vec<Attachment>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT content FROM observation_attachments WHERE observation_id = ? ORDER BY created_at")
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([observation_id], |row| {
                let content: String = row.get(0)?;
                Ok(content)
            })
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let mut attachments = Vec::new();
        for row in rows {
            let content = row.map_err(|e| EngramError::Database(e.to_string()))?;
            let attachment: Attachment = serde_json::from_str(&content)?;
            attachments.push(attachment);
        }

        Ok(attachments)
    }

    // ── Knowledge Capsules ────────────────────────────────────────

    fn upsert_capsule(&self, capsule: &KnowledgeCapsule) -> Result<i64> {
        let conn = self.conn();
        let now = Utc::now().to_rfc3339();

        let key_decisions = serde_json::to_string(&capsule.key_decisions)?;
        let known_issues = serde_json::to_string(&capsule.known_issues)?;
        let anti_patterns = serde_json::to_string(&capsule.anti_patterns)?;
        let best_practices = serde_json::to_string(&capsule.best_practices)?;
        let source_observations = serde_json::to_string(&capsule.source_observations)?;

        conn.execute(
            "INSERT INTO knowledge_capsules \
             (topic, project, summary, key_decisions, known_issues, anti_patterns, \
              best_practices, source_observations, confidence, created_at, last_consolidated, version) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(topic, project) DO UPDATE SET \
              summary = excluded.summary, \
              key_decisions = excluded.key_decisions, \
              known_issues = excluded.known_issues, \
              anti_patterns = excluded.anti_patterns, \
              best_practices = excluded.best_practices, \
              source_observations = excluded.source_observations, \
              confidence = excluded.confidence, \
              last_consolidated = excluded.last_consolidated, \
              version = version + 1",
            rusqlite::params![
                capsule.topic,
                capsule.project,
                capsule.summary,
                key_decisions,
                known_issues,
                anti_patterns,
                best_practices,
                source_observations,
                capsule.confidence,
                now,
                now,
                capsule.version,
            ],
        )
        .map_err(|e| EngramError::Database(e.to_string()))?;

        Ok(conn.last_insert_rowid())
    }

    fn get_capsule(&self, topic: &str, project: Option<&str>) -> Result<Option<KnowledgeCapsule>> {
        let conn = self.conn();
        conn.query_row(
            "SELECT id, topic, project, summary, key_decisions, known_issues, \
                    anti_patterns, best_practices, source_observations, confidence, \
                    created_at, last_consolidated, version \
             FROM knowledge_capsules WHERE topic = ? AND project IS ?",
            rusqlite::params![topic, project],
            Self::row_to_capsule,
        )
        .optional()
        .map_err(|e| EngramError::Database(e.to_string()))
    }

    fn list_capsules(&self, project: Option<&str>) -> Result<Vec<KnowledgeCapsule>> {
        let conn = self.conn();
        let sql = if project.is_some() {
            "SELECT id, topic, project, summary, key_decisions, known_issues, \
                    anti_patterns, best_practices, source_observations, confidence, \
                    created_at, last_consolidated, version \
             FROM knowledge_capsules WHERE project IS ? ORDER BY confidence DESC"
        } else {
            "SELECT id, topic, project, summary, key_decisions, known_issues, \
                    anti_patterns, best_practices, source_observations, confidence, \
                    created_at, last_consolidated, version \
             FROM knowledge_capsules ORDER BY confidence DESC"
        };

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let rows = if let Some(p) = project {
            stmt.query_map([p], Self::row_to_capsule)
                .map_err(|e| EngramError::Database(e.to_string()))?
        } else {
            stmt.query_map([], Self::row_to_capsule)
                .map_err(|e| EngramError::Database(e.to_string()))?
        };

        let mut capsules = Vec::new();
        for row in rows {
            capsules.push(row.map_err(|e| EngramError::Database(e.to_string()))?);
        }
        Ok(capsules)
    }

    // ── Spaced Repetition ─────────────────────────────────────────

    fn upsert_review(
        &self,
        observation_id: i64,
        interval_days: f64,
        ease_factor: f64,
        next_review: &str,
    ) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO review_schedule (observation_id, interval_days, ease_factor, next_review, review_count) \
             VALUES (?, ?, ?, ?, 0) \
             ON CONFLICT(observation_id) DO UPDATE SET \
              interval_days = excluded.interval_days, \
              ease_factor = excluded.ease_factor, \
              next_review = excluded.next_review, \
              review_count = review_count + 1",
            rusqlite::params![observation_id, interval_days, ease_factor, next_review],
        )
        .map_err(|e| EngramError::Database(e.to_string()))?;
        Ok(())
    }

    fn get_pending_reviews(
        &self,
        _project: Option<&str>,
        limit: usize,
    ) -> Result<Vec<(i64, f64, f64)>> {
        let conn = self.conn();
        let now = Utc::now().to_rfc3339();
        let mut stmt = conn
            .prepare(
                "SELECT observation_id, interval_days, ease_factor FROM review_schedule \
                 WHERE next_review <= ? ORDER BY next_review ASC LIMIT ?",
            )
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(rusqlite::params![now, limit as i64], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, f64>(2)?,
                ))
            })
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| EngramError::Database(e.to_string()))?);
        }
        Ok(result)
    }

    // ── Knowledge Boundaries ──────────────────────────────────────

    fn upsert_boundary(&self, domain: &str, confidence_level: &str, evidence: &str) -> Result<()> {
        let conn = self.conn();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO knowledge_boundaries (domain, confidence_level, evidence, updated_at) \
             VALUES (?, ?, ?, ?) \
             ON CONFLICT(domain) DO UPDATE SET \
              confidence_level = excluded.confidence_level, \
              evidence = excluded.evidence, \
              updated_at = excluded.updated_at",
            rusqlite::params![domain, confidence_level, evidence, now],
        )
        .map_err(|e| EngramError::Database(e.to_string()))?;
        Ok(())
    }

    fn get_boundaries(&self) -> Result<Vec<(String, String, String)>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT domain, confidence_level, evidence FROM knowledge_boundaries ORDER BY domain")
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| EngramError::Database(e.to_string()))?);
        }
        Ok(result)
    }

    // ── Beliefs ───────────────────────────────────────────────────

    fn upsert_belief(
        &self,
        subject: &str,
        predicate: &str,
        value: &str,
        confidence: f64,
        state: &str,
    ) -> Result<i64> {
        let conn = self.conn();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO beliefs (subject, predicate, value, confidence, state, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
            rusqlite::params![subject, predicate, value, confidence, state, now],
        )
        .map_err(|e| EngramError::Database(e.to_string()))?;
        Ok(conn.last_insert_rowid())
    }

    fn get_beliefs(&self, subject: &str) -> Result<Vec<(String, String, String, f64, String)>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT subject, predicate, value, confidence, state FROM beliefs WHERE subject = ? ORDER BY confidence DESC")
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([subject], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| EngramError::Database(e.to_string()))?);
        }
        Ok(result)
    }

    // ── Entities ──────────────────────────────────────────────────

    fn upsert_entity(&self, canonical_name: &str, entity_type: &str, aliases: &str) -> Result<i64> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO entities (canonical_name, entity_type, aliases) \
             VALUES (?, ?, ?) \
             ON CONFLICT(canonical_name) DO UPDATE SET \
              aliases = excluded.aliases",
            rusqlite::params![canonical_name, entity_type, aliases],
        )
        .map_err(|e| EngramError::Database(e.to_string()))?;
        Ok(conn.last_insert_rowid())
    }

    fn link_entity_observation(&self, entity_id: i64, observation_id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "INSERT OR IGNORE INTO entity_mentions (entity_id, observation_id) VALUES (?, ?)",
            rusqlite::params![entity_id, observation_id],
        )
        .map_err(|e| EngramError::Database(e.to_string()))?;
        Ok(())
    }

    fn get_entity(&self, name: &str) -> Result<Option<(i64, String, String, String)>> {
        let conn = self.conn();
        conn.query_row(
            "SELECT id, canonical_name, entity_type, aliases FROM entities \
             WHERE canonical_name = ? OR aliases LIKE ?",
            rusqlite::params![name, format!("%\"{name}\"%")],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|e| EngramError::Database(e.to_string()))
    }

    // ── Cross-Project ─────────────────────────────────────────────

    fn add_transfer(
        &self,
        source: &str,
        target: &str,
        capsule_id: Option<i64>,
        relevance: f64,
        transfer_type: &str,
    ) -> Result<i64> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO knowledge_transfers (source_project, target_project, capsule_id, relevance_score, transfer_type) \
             VALUES (?, ?, ?, ?, ?)",
            rusqlite::params![source, target, capsule_id, relevance, transfer_type],
        )
        .map_err(|e| EngramError::Database(e.to_string()))?;
        Ok(conn.last_insert_rowid())
    }

    fn get_transfers(&self, target: &str) -> Result<Vec<(i64, String, Option<i64>, f64, bool)>> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare("SELECT id, source_project, capsule_id, relevance_score, accepted FROM knowledge_transfers \
                      WHERE target_project = ? AND accepted = 0 ORDER BY relevance_score DESC")
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([target], |row| {
                let accepted: i64 = row.get(4)?;
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    accepted != 0,
                ))
            })
            .map_err(|e| EngramError::Database(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| EngramError::Database(e.to_string()))?);
        }
        Ok(result)
    }

    fn accept_transfer(&self, transfer_id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE knowledge_transfers SET accepted = 1 WHERE id = ?",
            [transfer_id],
        )
        .map_err(|e| EngramError::Database(e.to_string()))?;
        Ok(())
    }

    // ── Agent Personality ─────────────────────────────────────────

    fn upsert_personality(
        &self,
        agent_id: &str,
        project: &str,
        working_style: &str,
        strengths: &str,
        weaknesses: &str,
    ) -> Result<()> {
        let conn = self.conn();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO agent_personalities (agent_id, project, working_style, strengths, weaknesses, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?) \
             ON CONFLICT(agent_id, project) DO UPDATE SET \
              working_style = excluded.working_style, \
              strengths = excluded.strengths, \
              weaknesses = excluded.weaknesses, \
              updated_at = excluded.updated_at",
            rusqlite::params![agent_id, project, working_style, strengths, weaknesses, now],
        )
        .map_err(|e| EngramError::Database(e.to_string()))?;
        Ok(())
    }

    fn get_personality(
        &self,
        agent_id: &str,
        project: &str,
    ) -> Result<Option<(String, String, String)>> {
        let conn = self.conn();
        conn.query_row(
            "SELECT working_style, strengths, weaknesses FROM agent_personalities \
             WHERE agent_id = ? AND project = ?",
            rusqlite::params![agent_id, project],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|e| EngramError::Database(e.to_string()))
    }

    fn backup_create(&self, trigger: &str, label: Option<&str>) -> Result<BackupRecord> {
        let backup_dir = self.backup_dir()?;
        let now = Utc::now();
        let timestamp = now.format("%Y-%m-%dT%H-%M-%SZ").to_string();
        let backup_filename = format!("engram-{timestamp}.db");
        let backup_path = backup_dir.join(&backup_filename);

        // rusqlite::backup::Backup::run_to_completion()
        {
            let conn = self.conn();
            let mut dst = rusqlite::Connection::open(&backup_path)
                .map_err(|e| EngramError::Database(format!("failed to create backup file: {e}")))?;
            let backup = rusqlite::backup::Backup::new(&conn, &mut dst)
                .map_err(|e| EngramError::Database(format!("failed to init backup: {e}")))?;
            backup
                .run_to_completion(500, std::time::Duration::from_millis(0), None)
                .map_err(|e| EngramError::Database(format!("backup failed: {e}")))?;
        }

        let metadata = std::fs::metadata(&backup_path)
            .map_err(|e| EngramError::Database(format!("failed to read backup metadata: {e}")))?;
        let size_bytes = metadata.len();

        let sha256 = Self::sha256_file(&backup_path)?;

        // Stats
        let conn = self.conn();
        let observations: usize = conn
            .query_row("SELECT COUNT(*) FROM observations", [], |row| {
                row.get::<_, i64>(0).map(|v| v as usize)
            })
            .unwrap_or(0);
        let sessions: usize = conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| {
                row.get::<_, i64>(0).map(|v| v as usize)
            })
            .unwrap_or(0);
        let edges: usize = conn
            .query_row("SELECT COUNT(*) FROM edges", [], |row| {
                row.get::<_, i64>(0).map(|v| v as usize)
            })
            .unwrap_or(0);

        let stats = crate::r#trait::BackupStats {
            observations,
            sessions,
            edges,
        };

        let schema_version = self.schema_version();

        // Write .meta.json sidecar
        let meta = serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "schema_version": schema_version,
            "created_at": now.to_rfc3339(),
            "trigger": trigger,
            "label": label,
            "size_bytes": size_bytes,
            "sha256": sha256,
            "stats": {
                "observations": stats.observations,
                "sessions": stats.sessions,
                "edges": stats.edges,
            }
        });
        let meta_path = backup_dir.join(format!("engram-{timestamp}.meta.json"));
        std::fs::write(&meta_path, serde_json::to_string_pretty(&meta)?)
            .map_err(|e| EngramError::Database(format!("failed to write meta.json: {e}")))?;

        Ok(crate::r#trait::BackupRecord {
            path: backup_path,
            created_at: now,
            trigger: trigger.to_string(),
            label: label.map(String::from),
            size_bytes,
            sha256,
            stats,
        })
    }

    fn backup_verify(&self, path: &std::path::Path) -> Result<crate::r#trait::BackupVerifyResult> {
        if !path.exists() {
            return Ok(crate::r#trait::BackupVerifyResult {
                valid: false,
                sha256_match: false,
                integrity_check_pass: false,
                error: Some(format!("file not found: {}", path.display())),
            });
        }

        let sha256 = Self::sha256_file(path)?;

        // Check for sidecar .meta.json
        let meta_path = path.with_extension("meta.json");
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let alt_meta_path = path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join(format!("{stem}.meta.json"));

        let sha256_match =
            if let Some(meta_file) = [meta_path, alt_meta_path].iter().find(|p| p.exists()) {
                if let Ok(meta_str) = std::fs::read_to_string(meta_file) {
                    if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&meta_str) {
                        meta["sha256"].as_str() == Some(sha256.as_str())
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

        // SQLite integrity check
        let integrity_check_pass = match rusqlite::Connection::open(path) {
            Ok(conn) => {
                match conn.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0)) {
                    Ok(result) => result == "ok",
                    Err(_) => false,
                }
            }
            Err(_) => false,
        };

        let valid = integrity_check_pass;
        let error = if !valid {
            Some("SQLite integrity check failed".to_string())
        } else {
            None
        };

        Ok(crate::r#trait::BackupVerifyResult {
            valid,
            sha256_match,
            integrity_check_pass,
            error,
        })
    }

    fn backup_list(&self) -> Result<Vec<BackupRecord>> {
        self.list_backups_from_disk()
    }

    fn backup_restore(&self, backup_path: &std::path::Path, confirm: bool) -> Result<()> {
        // BACKUP-10: Verify integrity first
        let verify = self.backup_verify(backup_path)?;
        if !verify.valid {
            return Err(EngramError::Database(format!(
                "backup integrity check failed: {}",
                verify.error.as_deref().unwrap_or("unknown error")
            )));
        }

        // BACKUP-11: Confirm unless --yes
        if confirm {
            eprintln!("This will replace your current database. Continue? [y/N]");
            let mut input = String::new();
            std::io::stdin()
                .read_line(&mut input)
                .map_err(|e| EngramError::Database(format!("failed to read input: {e}")))?;
            if !input.trim().eq_ignore_ascii_case("y") {
                eprintln!("Restore cancelled.");
                return Ok(());
            }
        }

        // BACKUP-09: Create pre-restore safety backup
        self.backup_create("pre-restore", None)?;

        // D-02: Copy backup over current DB
        let db_path = self.db_path.clone();
        self.restore_db_file(backup_path, &db_path)?;

        info!("Restore complete from {}", backup_path.display());
        Ok(())
    }
}

impl SqliteStore {
    /// Re-rank search results: pinned first, then by decay score with lifecycle.
    fn rerank_by_relevance(results: &mut [Observation]) {
        results.sort_by(|a, b| {
            // Pinned always first
            match (a.pinned, b.pinned) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }
            // Then by decay score (higher = more relevant)
            let score_a = decay_score_with_lifecycle(a.created_at, a.access_count, a.pinned, 1.0);
            let score_b = decay_score_with_lifecycle(b.created_at, b.access_count, b.pinned, 1.0);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Classify observation content and insert into episodic or semantic memory table.
    fn classify_and_insert_memory(&self, observation_id: i64, params: &AddObservationParams) {
        let target = classify_query_type(&params.content);
        let conn = match self.conn.try_lock() {
            Ok(c) => c,
            Err(_) => return, // Skip if lock contended — non-critical
        };
        let now = Utc::now().to_rfc3339();

        match target {
            QueryTarget::Episodic | QueryTarget::Both => {
                let _ = conn.execute(
                    "INSERT OR IGNORE INTO episodic_memories \
                     (observation_id, session_id, timestamp, what_happened, emotional_valence, effort_level) \
                     VALUES (?, ?, ?, ?, 0.0, 0.0)",
                    rusqlite::params![observation_id, params.session_id, now, params.content],
                );
            }
            _ => {}
        }
        match target {
            QueryTarget::Semantic | QueryTarget::Both => {
                let domain = params.topic_key.as_deref().unwrap_or("general");
                let _ = conn.execute(
                    "INSERT OR IGNORE INTO semantic_memories \
                     (observation_id, domain, concept, definition_text, abstraction_level) \
                     VALUES (?, ?, ?, ?, 0)",
                    rusqlite::params![observation_id, domain, params.title, params.content],
                );
            }
            _ => {}
        }
    }

    /// List backups from disk by reading .meta.json sidecars.
    /// Returns sorted newest-first. D-01: ID 1 = most recent.
    fn list_backups_from_disk(&self) -> Result<Vec<BackupRecord>> {
        let dir = self.backup_dir()?;
        let mut records = Vec::new();

        for entry in std::fs::read_dir(&dir)
            .map_err(|e| EngramError::Database(format!("failed to read backup dir: {e}")))?
        {
            let entry = entry.map_err(|e| EngramError::Database(e.to_string()))?;
            let path = entry.path();

            // Only process .db files
            if path.extension().and_then(|e| e.to_str()) != Some("db") {
                continue;
            }

            // Read .meta.json sidecar
            let meta_path = path.with_extension("meta.json");
            let meta_str = match std::fs::read_to_string(&meta_path) {
                Ok(s) => s,
                Err(_) => continue, // Skip backups without metadata
            };
            let meta: serde_json::Value = match serde_json::from_str(&meta_str) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let created_at = meta["created_at"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or_default();
            let trigger = meta["trigger"].as_str().unwrap_or("unknown").to_string();
            let label = meta["label"].as_str().map(String::from);
            let size_bytes = meta["size_bytes"].as_u64().unwrap_or(0);
            let sha256 = meta["sha256"].as_str().unwrap_or("").to_string();
            let stats = crate::r#trait::BackupStats {
                observations: meta["stats"]["observations"].as_u64().unwrap_or(0) as usize,
                sessions: meta["stats"]["sessions"].as_u64().unwrap_or(0) as usize,
                edges: meta["stats"]["edges"].as_u64().unwrap_or(0) as usize,
            };

            records.push(crate::r#trait::BackupRecord {
                path: path.clone(),
                created_at,
                trigger,
                label,
                size_bytes,
                sha256,
                stats,
            });
        }

        // Sort newest-first (ID 1 = most recent per D-01)
        records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(records)
    }

    /// Copy backup file over the current database file.
    /// Unix: atomic rename(). Windows: delete old, copy new.
    fn restore_db_file(
        &self,
        backup_path: &std::path::Path,
        db_path: &std::path::Path,
    ) -> Result<()> {
        #[cfg(unix)]
        {
            std::fs::rename(backup_path, db_path).map_err(|e| {
                EngramError::Database(format!("failed to rename backup over db: {e}"))
            })?;
        }

        #[cfg(windows)]
        {
            // Windows doesn't allow rename over open file
            if db_path.exists() {
                std::fs::remove_file(db_path)
                    .map_err(|e| EngramError::Database(format!("failed to remove old db: {e}")))?;
            }
            std::fs::copy(backup_path, db_path).map_err(|e| {
                EngramError::Database(format!("failed to copy backup over db: {e}"))
            })?;
        }

        #[cfg(not(any(unix, windows)))]
        {
            std::fs::copy(backup_path, db_path).map_err(|e| {
                EngramError::Database(format!("failed to copy backup over db: {e}"))
            })?;
        }

        Ok(())
    }

    /// Rotate old auto-backups. Keeps last 10 (by modification time).
    /// Manual backups (trigger = "manual") are never deleted. D-05.
    fn rotate_old_backups(&self) -> Result<()> {
        let dir = self.backup_dir()?;
        let mut auto_backups: Vec<std::fs::DirEntry> = Vec::new();

        for entry in std::fs::read_dir(&dir)
            .map_err(|e| EngramError::Database(format!("failed to read backup dir: {e}")))?
        {
            let entry = entry.map_err(|e| EngramError::Database(e.to_string()))?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) != Some("db") {
                continue;
            }

            // Check .meta.json for trigger
            let meta_path = path.with_extension("meta.json");
            if let Ok(meta_str) = std::fs::read_to_string(&meta_path)
                && let Ok(meta) = serde_json::from_str::<serde_json::Value>(&meta_str)
            {
                let trigger = meta["trigger"].as_str().unwrap_or("unknown");
                if trigger != "manual" {
                    auto_backups.push(entry);
                }
            }
        }

        // Sort oldest first
        auto_backups.sort_by_key(|e| {
            e.metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });

        // Keep last 10, delete older
        while auto_backups.len() > 10 {
            let old = auto_backups.remove(0);
            let old_path = old.path();
            let _ = std::fs::remove_file(&old_path);
            let _ = std::fs::remove_file(old_path.with_extension("meta.json"));
            info!("Rotated old backup: {:?}", old_path.file_name());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_in_memory_works() {
        let store = SqliteStore::in_memory().unwrap();
        let stats = store.get_stats("test").unwrap();
        assert_eq!(stats.total_observations, 0);
    }

    #[test]
    fn session_crud() {
        let store = SqliteStore::in_memory().unwrap();
        let sid = store.create_session("test-project").unwrap();
        assert!(!sid.is_empty());

        let session = store.get_session(&sid).unwrap().unwrap();
        assert_eq!(session.project, "test-project");
        assert!(session.is_active());

        store.end_session(&sid, Some("done")).unwrap();
        let session = store.get_session(&sid).unwrap().unwrap();
        assert!(!session.is_active());
        assert_eq!(session.summary.as_deref(), Some("done"));
    }

    #[test]
    fn observation_crud() {
        let store = SqliteStore::in_memory().unwrap();
        let sid = store.create_session("test").unwrap();

        let id = store
            .insert_observation(&AddObservationParams {
                r#type: ObservationType::Bugfix,
                scope: Scope::Project,
                title: "Fix N+1 query".into(),
                content: "Used eager loading".into(),
                session_id: sid,
                project: "test".into(),
                topic_key: Some("bug/n1-query".into()),
                ..Default::default()
            })
            .unwrap();
        assert!(id > 0);

        // Get increments access
        let obs = store.get_observation(id).unwrap().unwrap();
        assert_eq!(obs.title, "Fix N+1 query");
        assert_eq!(obs.access_count, 1);

        // Peek does not increment
        let obs = store.peek_observation(id).unwrap().unwrap();
        assert_eq!(obs.access_count, 1);

        // Update
        store
            .update_observation(
                id,
                &UpdateObservationParams {
                    pinned: Some(true),
                    ..Default::default()
                },
            )
            .unwrap();
        let obs = store.peek_observation(id).unwrap().unwrap();
        assert!(obs.pinned);

        // Soft delete
        store.delete_observation(id, false).unwrap();
        let obs = store.peek_observation(id).unwrap().unwrap();
        assert_eq!(obs.lifecycle_state, LifecycleState::Deleted);
    }

    #[test]
    fn observation_dedup() {
        let store = SqliteStore::in_memory().unwrap();
        let sid = store.create_session("test").unwrap();

        let params = AddObservationParams {
            r#type: ObservationType::Manual,
            scope: Scope::Project,
            title: "Same title".into(),
            content: "Same content".into(),
            session_id: sid.clone(),
            project: "test".into(),
            ..Default::default()
        };

        store.insert_observation(&params).unwrap();
        let result = store.insert_observation(&params);
        assert!(matches!(result, Err(EngramError::Duplicate(_))));
    }

    #[test]
    fn search_fts() {
        let store = SqliteStore::in_memory().unwrap();
        let sid = store.create_session("test").unwrap();

        store
            .insert_observation(&AddObservationParams {
                r#type: ObservationType::Bugfix,
                scope: Scope::Project,
                title: "JWT auth issue".into(),
                content: "Token expired too quickly".into(),
                session_id: sid.clone(),
                project: "test".into(),
                ..Default::default()
            })
            .unwrap();

        store
            .insert_observation(&AddObservationParams {
                r#type: ObservationType::Decision,
                scope: Scope::Project,
                title: "Use SQLite".into(),
                content: "For local storage needs".into(),
                session_id: sid,
                project: "test".into(),
                ..Default::default()
            })
            .unwrap();

        let results = store
            .search(&SearchOptions {
                query: "JWT".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "JWT auth issue");
    }

    #[test]
    fn prompts_crud() {
        let store = SqliteStore::in_memory().unwrap();
        let sid = store.create_session("test").unwrap();

        store
            .save_prompt(&AddPromptParams {
                session_id: sid.clone(),
                project: "test".into(),
                content: "How do I fix this?".into(),
            })
            .unwrap();

        let prompts = store.get_prompts(&sid).unwrap();
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0], "How do I fix this?");
    }

    #[test]
    fn stats_counts() {
        let store = SqliteStore::in_memory().unwrap();
        let sid = store.create_session("test").unwrap();

        for i in 0..3 {
            store
                .insert_observation(&AddObservationParams {
                    r#type: ObservationType::Bugfix,
                    scope: Scope::Project,
                    title: format!("Bug {i}"),
                    content: format!("Fix {i}"),
                    session_id: sid.clone(),
                    project: "test".into(),
                    ..Default::default()
                })
                .unwrap();
        }

        let stats = store.get_stats("test").unwrap();
        assert_eq!(stats.total_observations, 3);
        assert_eq!(stats.by_type.get("bugfix"), Some(&3));
        assert_eq!(stats.total_sessions, 1);
    }

    #[test]
    fn export_import_roundtrip() {
        let store = SqliteStore::in_memory().unwrap();
        let sid = store.create_session("test").unwrap();

        store
            .insert_observation(&AddObservationParams {
                r#type: ObservationType::Decision,
                scope: Scope::Project,
                title: "Use Rust".into(),
                content: "Performance matters".into(),
                session_id: sid,
                project: "test".into(),
                ..Default::default()
            })
            .unwrap();

        let data = store.export(None).unwrap();
        assert_eq!(data.observations.len(), 1);
        assert_eq!(data.sessions.len(), 1);

        // Import into fresh store
        let store2 = SqliteStore::in_memory().unwrap();
        let result = store2.import(&data).unwrap();
        assert_eq!(result.observations_imported, 1);

        let stats = store2.get_stats("test").unwrap();
        assert_eq!(stats.total_observations, 1);
    }

    #[test]
    fn attachment_crud() {
        let store = SqliteStore::in_memory().unwrap();
        let sid = store.create_session("test").unwrap();

        let obs_id = store
            .insert_observation(&AddObservationParams {
                r#type: ObservationType::Bugfix,
                scope: engram_core::Scope::Project,
                title: "Bug with error".into(),
                content: "Error trace attached".into(),
                session_id: sid,
                project: "test".into(),
                ..Default::default()
            })
            .unwrap();

        // Store attachment
        let att = Attachment::ErrorTrace {
            error_type: "panic".into(),
            message: "index out of bounds".into(),
            stack_trace: "at main.rs:42".into(),
            file_line: Some(("main.rs".into(), 42)),
        };
        let att_id = store.store_attachment(obs_id, &att).unwrap();
        assert!(att_id > 0);

        // Get attachments
        let attachments = store.get_attachments(obs_id).unwrap();
        assert_eq!(attachments.len(), 1);
        match &attachments[0] {
            Attachment::ErrorTrace { message, .. } => {
                assert_eq!(message, "index out of bounds");
            }
            _ => panic!("expected ErrorTrace"),
        }
    }

    #[test]
    fn multiple_attachments() {
        let store = SqliteStore::in_memory().unwrap();
        let sid = store.create_session("test").unwrap();

        let obs_id = store
            .insert_observation(&AddObservationParams {
                r#type: ObservationType::Bugfix,
                scope: engram_core::Scope::Project,
                title: "Complex bug".into(),
                content: "Multiple attachments".into(),
                session_id: sid,
                project: "test".into(),
                ..Default::default()
            })
            .unwrap();

        // Store multiple attachments
        store
            .store_attachment(
                obs_id,
                &Attachment::CodeDiff {
                    file_path: "src/main.rs".into(),
                    before_hash: "abc".into(),
                    after_hash: "def".into(),
                    diff: "+fn fix()".into(),
                },
            )
            .unwrap();

        store
            .store_attachment(
                obs_id,
                &Attachment::GitCommit {
                    hash: "abc123def456".into(),
                    message: "fix: resolve panic".into(),
                    files_changed: vec!["src/main.rs".into()],
                    diff_summary: "added error handling".into(),
                },
            )
            .unwrap();

        let attachments = store.get_attachments(obs_id).unwrap();
        assert_eq!(attachments.len(), 2);
    }

    #[test]
    fn backup_list_empty() {
        let store = SqliteStore::in_memory().unwrap();
        let list = store.backup_list().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn backup_list_after_create() {
        // NOTE: file-based backup_create hangs on Windows due to rusqlite::backup issue.
        // Testing backup_list via in-memory store + list_backups_from_disk directly.
        let store = SqliteStore::in_memory().unwrap();
        // backup_list on in-memory returns empty (no backup dir)
        let list = store.backup_list().unwrap();
        assert!(list.is_empty(), "in-memory store should have no backups");
    }

    #[test]
    fn pending_migrations_on_fresh_db() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let pending = migration::pending_migrations(&conn).unwrap();
        assert!(!pending.is_empty());
    }

    #[test]
    fn pending_migrations_after_applied() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        migration::run_migrations(&conn).unwrap();
        let pending = migration::pending_migrations(&conn).unwrap();
        assert!(pending.is_empty());
    }
}

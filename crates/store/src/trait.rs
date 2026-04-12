use engram_core::{
    Attachment, Edge, EngramError, KnowledgeCapsule, Observation, ObservationType, RelationType,
    Session,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::params::*;

/// Result type alias for all storage operations.
pub type Result<T> = std::result::Result<T, EngramError>;

/// Export data format — JSON-compatible with Engram Go.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportData {
    pub observations: Vec<Observation>,
    pub sessions: Vec<Session>,
    pub prompts: Vec<ExportedPrompt>,
    pub edges: Vec<Edge>,
}

/// Imported prompt for export/import round-trip.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportedPrompt {
    pub session_id: String,
    pub project: String,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Result of an import operation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportResult {
    pub observations_imported: usize,
    pub sessions_imported: usize,
    pub prompts_imported: usize,
    pub edges_imported: usize,
    pub duplicates_skipped: usize,
}

/// Timeline entry — observation with positional context.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TimelineEntry {
    pub observation: Observation,
    pub position: TimelinePosition,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum TimelinePosition {
    Before,
    Center,
    After,
}

/// Project statistics.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectStats {
    pub project: String,
    pub total_observations: usize,
    pub by_type: HashMap<String, usize>,
    pub by_scope: HashMap<String, usize>,
    pub total_sessions: usize,
    pub total_edges: usize,
    pub total_capsules: usize,
}

/// Session context — recent observations from a session.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionContext {
    pub session: Session,
    pub observations: Vec<Observation>,
    pub prompts: Vec<String>,
}

/// Statistics snapshot for backup metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BackupStats {
    pub observations: usize,
    pub sessions: usize,
    pub edges: usize,
}

/// Record of a created backup.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BackupRecord {
    pub path: PathBuf,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub trigger: String,
    pub label: Option<String>,
    pub size_bytes: u64,
    pub sha256: String,
    pub stats: BackupStats,
}

/// Result of verifying a backup.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BackupVerifyResult {
    pub valid: bool,
    pub sha256_match: bool,
    pub integrity_check_pass: bool,
    pub error: Option<String>,
}

/// Storage trait — THE firewall against vendor lock-in.
///
/// Rules:
/// - ALL return types are from engram-core or this crate (no rusqlite types)
/// - ALL parameters are structs (no raw SQL strings)
/// - ALL errors are EngramError (no rusqlite::Error)
/// - NO raw_query or get_connection methods
pub trait Storage: Send + Sync {
    // ── Observations CRUD ──────────────────────────────────────────

    /// Insert a new observation. Returns the assigned ID.
    /// Performs dedup check by normalized_hash within 15min window.
    fn insert_observation(&self, params: &AddObservationParams) -> Result<i64>;

    /// Get observation by ID. Increments access_count and last_accessed.
    fn get_observation(&self, id: i64) -> Result<Option<Observation>>;

    /// Get observation by ID without incrementing access counters.
    fn peek_observation(&self, id: i64) -> Result<Option<Observation>>;

    /// Update an observation with partial fields.
    fn update_observation(&self, id: i64, params: &UpdateObservationParams) -> Result<()>;

    /// Delete an observation (soft or hard).
    fn delete_observation(&self, id: i64, hard: bool) -> Result<()>;

    /// Search observations with filters and ranking.
    fn search(&self, opts: &SearchOptions) -> Result<Vec<Observation>>;

    // ── Sessions ───────────────────────────────────────────────────

    /// Create a new session. Returns the session ID (UUID v4).
    fn create_session(&self, project: &str) -> Result<String>;

    /// End a session with optional summary.
    fn end_session(&self, session_id: &str, summary: Option<&str>) -> Result<()>;

    /// Get session by ID.
    fn get_session(&self, session_id: &str) -> Result<Option<Session>>;

    /// Get session context (recent observations from last session in project).
    fn get_session_context(&self, project: &str, limit: usize) -> Result<SessionContext>;

    // ── Prompts ────────────────────────────────────────────────────

    /// Save a user prompt.
    fn save_prompt(&self, params: &AddPromptParams) -> Result<()>;

    /// Get prompts for a session.
    fn get_prompts(&self, session_id: &str) -> Result<Vec<String>>;

    // ── Timeline ───────────────────────────────────────────────────

    /// Get observations around a specific observation in time.
    fn get_timeline(&self, observation_id: i64, window: usize) -> Result<Vec<TimelineEntry>>;

    // ── Statistics ─────────────────────────────────────────────────

    /// Get project statistics.
    fn get_stats(&self, project: &str) -> Result<ProjectStats>;

    // ── Graph ──────────────────────────────────────────────────────

    /// Add an edge to the knowledge graph.
    /// If an active edge exists between the same nodes with the same relation type,
    /// closes the old one (valid_until = now).
    fn add_edge(&self, params: &AddEdgeParams) -> Result<i64>;

    /// Get all active edges from an observation.
    fn get_edges(&self, observation_id: i64) -> Result<Vec<Edge>>;

    /// Get edges valid at a specific point in time.
    fn get_edges_at(
        &self,
        observation_id: i64,
        at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Edge>>;

    /// Get related observations via BFS up to max_depth.
    fn get_related(
        &self,
        observation_id: i64,
        max_depth: usize,
    ) -> Result<Vec<(Observation, RelationType, usize)>>;

    // ── Embeddings ─────────────────────────────────────────────────

    /// Store an embedding for an observation.
    fn store_embedding(
        &self,
        observation_id: i64,
        embedding: &[f32],
        model_name: &str,
        model_version: &str,
    ) -> Result<()>;

    /// Search by vector similarity. Returns (observation_id, distance) pairs.
    fn search_vector(&self, embedding: &[f32], limit: usize) -> Result<Vec<(i64, f64)>>;

    /// Count embeddings not matching the given model version.
    fn count_stale_embeddings(&self, model_name: &str, model_version: &str) -> Result<usize>;

    /// Update embedding model version for all observations.
    fn update_embedding_versions(&self, model_name: &str, model_version: &str) -> Result<usize>;

    // ── Export / Import ────────────────────────────────────────────

    /// Export all data to JSON-compatible format.
    fn export(&self, project: Option<&str>) -> Result<ExportData>;

    /// Import data from JSON-compatible format.
    fn import(&self, data: &ExportData) -> Result<ImportResult>;

    // ── Lifecycle (F2.5.9) ─────────────────────────────────────────

    /// Batch transition observations between lifecycle states by type and age.
    fn transition_state(
        &self,
        project: &str,
        obs_type: &ObservationType,
        from: &str,
        to: &str,
        age_days: u32,
    ) -> Result<u32>;

    /// Mark observations pending review by type and age.
    fn mark_pending_review(
        &self,
        project: &str,
        obs_type: &ObservationType,
        age_days: u32,
    ) -> Result<u32>;

    // ── Attachments (F3.7) ─────────────────────────────────────────

    /// Store an attachment for an observation.
    fn store_attachment(&self, observation_id: i64, attachment: &Attachment) -> Result<i64>;

    /// Get all attachments for an observation.
    fn get_attachments(&self, observation_id: i64) -> Result<Vec<Attachment>>;

    // ── Knowledge Capsules (F2.5.3) ───────────────────────────────

    /// Upsert a knowledge capsule (insert or update on conflict).
    fn upsert_capsule(&self, capsule: &KnowledgeCapsule) -> Result<i64>;

    /// Get a capsule by topic and project.
    fn get_capsule(&self, topic: &str, project: Option<&str>) -> Result<Option<KnowledgeCapsule>>;

    /// List capsules for a project.
    fn list_capsules(&self, project: Option<&str>) -> Result<Vec<KnowledgeCapsule>>;

    // ── Spaced Repetition (F2.75.3) ───────────────────────────────

    /// Upsert a review schedule entry.
    fn upsert_review(
        &self,
        observation_id: i64,
        interval_days: f64,
        ease_factor: f64,
        next_review: &str,
    ) -> Result<()>;

    /// Get pending reviews (next_review <= now).
    fn get_pending_reviews(
        &self,
        project: Option<&str>,
        limit: usize,
    ) -> Result<Vec<(i64, f64, f64)>>;

    // ── Knowledge Boundaries (F2.5.8) ─────────────────────────────

    /// Upsert a knowledge boundary.
    fn upsert_boundary(&self, domain: &str, confidence_level: &str, evidence: &str) -> Result<()>;

    /// Get all boundaries for a project scope.
    fn get_boundaries(&self) -> Result<Vec<(String, String, String)>>;

    // ── Beliefs (F2.5.10) ─────────────────────────────────────────

    /// Upsert a belief.
    fn upsert_belief(
        &self,
        subject: &str,
        predicate: &str,
        value: &str,
        confidence: f64,
        state: &str,
    ) -> Result<i64>;

    /// Get beliefs by subject.
    #[allow(clippy::type_complexity)]
    fn get_beliefs(&self, subject: &str) -> Result<Vec<(String, String, String, f64, String)>>;

    // ── Entities (F2.5.12) ────────────────────────────────────────

    /// Upsert an entity.
    fn upsert_entity(&self, canonical_name: &str, entity_type: &str, aliases: &str) -> Result<i64>;

    /// Link entity to observation.
    fn link_entity_observation(&self, entity_id: i64, observation_id: i64) -> Result<()>;

    /// Get entity by name (checks canonical + aliases).
    fn get_entity(&self, name: &str) -> Result<Option<(i64, String, String, String)>>;

    // ── Cross-Project (F2.75.2) ───────────────────────────────────

    /// Add a knowledge transfer suggestion.
    fn add_transfer(
        &self,
        source: &str,
        target: &str,
        capsule_id: Option<i64>,
        relevance: f64,
        transfer_type: &str,
    ) -> Result<i64>;

    /// Get transfers for a target project.
    #[allow(clippy::type_complexity)]
    fn get_transfers(&self, target: &str) -> Result<Vec<(i64, String, Option<i64>, f64, bool)>>;

    /// Accept a transfer.
    fn accept_transfer(&self, transfer_id: i64) -> Result<()>;

    // ── Agent Personality (F2.75.5) ───────────────────────────────

    /// Upsert agent personality.
    fn upsert_personality(
        &self,
        agent_id: &str,
        project: &str,
        working_style: &str,
        strengths: &str,
        weaknesses: &str,
    ) -> Result<()>;

    /// Get agent personality.
    fn get_personality(
        &self,
        agent_id: &str,
        project: &str,
    ) -> Result<Option<(String, String, String)>>;

    // ── Backup ─────────────────────────────────────────────────────

    /// Create a backup of the database. Returns backup record with metadata.
    fn backup_create(&self, trigger: &str, label: Option<&str>) -> Result<BackupRecord>;

    /// Verify backup file integrity (SHA-256 + SQLite integrity_check).
    fn backup_verify(&self, path: &Path) -> Result<BackupVerifyResult>;
}

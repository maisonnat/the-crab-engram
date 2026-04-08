use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Strict observation type enum matching Engram Go's types.
/// Every observation MUST be one of these types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationType {
    Bugfix,
    Decision,
    Architecture,
    Pattern,
    Discovery,
    Learning,
    Config,
    Convention,
    ToolUse,
    FileChange,
    Command,
    FileRead,
    Search,
    Manual,
}

impl std::fmt::Display for ObservationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Bugfix => "bugfix",
            Self::Decision => "decision",
            Self::Architecture => "architecture",
            Self::Pattern => "pattern",
            Self::Discovery => "discovery",
            Self::Learning => "learning",
            Self::Config => "config",
            Self::Convention => "convention",
            Self::ToolUse => "tool_use",
            Self::FileChange => "file_change",
            Self::Command => "command",
            Self::FileRead => "file_read",
            Self::Search => "search",
            Self::Manual => "manual",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for ObservationType {
    type Err = crate::EngramError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bugfix" => Ok(Self::Bugfix),
            "decision" => Ok(Self::Decision),
            "architecture" => Ok(Self::Architecture),
            "pattern" => Ok(Self::Pattern),
            "discovery" => Ok(Self::Discovery),
            "learning" => Ok(Self::Learning),
            "config" => Ok(Self::Config),
            "convention" => Ok(Self::Convention),
            "tool_use" => Ok(Self::ToolUse),
            "file_change" => Ok(Self::FileChange),
            "command" => Ok(Self::Command),
            "file_read" => Ok(Self::FileRead),
            "search" => Ok(Self::Search),
            "manual" => Ok(Self::Manual),
            _ => Err(crate::EngramError::InvalidObservationType(s.to_string())),
        }
    }
}

/// Observation scope — determines visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Project,
    Personal,
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Project => write!(f, "project"),
            Self::Personal => write!(f, "personal"),
        }
    }
}

/// Core observation — the fundamental unit of memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub id: i64,
    pub r#type: ObservationType,
    pub scope: Scope,
    pub title: String,
    pub content: String,
    pub session_id: String,
    pub project: String,
    pub topic_key: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub access_count: i64,
    pub last_accessed: Option<DateTime<Utc>>,
    pub pinned: bool,
    pub normalized_hash: String,

    // Provenance (F2.5.1)
    pub provenance_source: ProvenanceSource,
    pub provenance_confidence: f64,
    pub provenance_evidence: Vec<String>,

    // Lifecycle (F2.5.9)
    pub lifecycle_state: LifecycleState,

    // Salience (F2.5.7)
    pub emotional_valence: f64,
    pub surprise_factor: f64,
    pub effort_invested: f64,
}

/// How an observation was verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceSource {
    TestVerified,
    CodeAnalysis,
    UserStated,
    External,
    LlmReasoning,
    Inferred,
}

impl ProvenanceSource {
    pub fn default_confidence(&self) -> f64 {
        match self {
            Self::TestVerified => 0.95,
            Self::CodeAnalysis => 0.85,
            Self::UserStated => 0.70,
            Self::External => 0.65,
            Self::LlmReasoning => 0.60,
            Self::Inferred => 0.40,
        }
    }
}

impl std::str::FromStr for ProvenanceSource {
    type Err = crate::EngramError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "test_verified" => Ok(Self::TestVerified),
            "code_analysis" => Ok(Self::CodeAnalysis),
            "user_stated" => Ok(Self::UserStated),
            "external" => Ok(Self::External),
            "llm_reasoning" => Ok(Self::LlmReasoning),
            "inferred" => Ok(Self::Inferred),
            _ => Err(crate::EngramError::Config(format!(
                "invalid provenance source: {s}"
            ))),
        }
    }
}

impl Default for ProvenanceSource {
    fn default() -> Self {
        Self::LlmReasoning
    }
}

/// Observation lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    Active,
    Stale,
    Archived,
    Deleted,
}

impl std::str::FromStr for LifecycleState {
    type Err = crate::EngramError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "stale" => Ok(Self::Stale),
            "archived" => Ok(Self::Archived),
            "deleted" => Ok(Self::Deleted),
            _ => Err(crate::EngramError::Config(format!(
                "invalid lifecycle state: {s}"
            ))),
        }
    }
}

impl Default for LifecycleState {
    fn default() -> Self {
        Self::Active
    }
}

impl Observation {
    /// Create a new observation with defaults.
    pub fn new(
        r#type: ObservationType,
        scope: Scope,
        title: String,
        content: String,
        session_id: String,
        project: String,
        topic_key: Option<String>,
    ) -> Self {
        let now = Utc::now();
        let normalized_hash = Self::compute_hash(&title, &content);
        Self {
            id: 0, // assigned by storage
            r#type,
            scope,
            title,
            content,
            session_id,
            project,
            topic_key,
            created_at: now,
            updated_at: now,
            access_count: 0,
            last_accessed: None,
            pinned: false,
            normalized_hash,
            provenance_source: ProvenanceSource::default(),
            provenance_confidence: ProvenanceSource::default().default_confidence(),
            provenance_evidence: Vec::new(),
            lifecycle_state: LifecycleState::default(),
            emotional_valence: 0.0,
            surprise_factor: 0.0,
            effort_invested: 0.0,
        }
    }

    /// Compute SHA-256 hash of title + content for dedup.
    pub fn compute_hash(title: &str, content: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(title.as_bytes());
        hasher.update(b"\n");
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Generate a UUID v4 session ID.
    pub fn generate_session_id() -> String {
        Uuid::new_v4().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_type_display_roundtrip() {
        let types = [
            ObservationType::Bugfix,
            ObservationType::Decision,
            ObservationType::Architecture,
            ObservationType::Pattern,
            ObservationType::Discovery,
            ObservationType::Learning,
            ObservationType::Config,
            ObservationType::Convention,
            ObservationType::ToolUse,
            ObservationType::FileChange,
            ObservationType::Command,
            ObservationType::FileRead,
            ObservationType::Search,
            ObservationType::Manual,
        ];
        for t in &types {
            let s = t.to_string();
            let parsed: ObservationType = s.parse().unwrap();
            assert_eq!(*t, parsed, "roundtrip failed for {s}");
        }
    }

    #[test]
    fn observation_type_invalid() {
        let result: Result<ObservationType, _> = "invalid_type".parse();
        assert!(result.is_err());
    }

    #[test]
    fn observation_new_has_defaults() {
        let obs = Observation::new(
            ObservationType::Bugfix,
            Scope::Project,
            "Fix N+1 query".into(),
            "Used eager loading".into(),
            "session-1".into(),
            "engram".into(),
            Some("bug/n1-query".into()),
        );
        assert_eq!(obs.access_count, 0);
        assert!(!obs.pinned);
        assert_eq!(obs.lifecycle_state, LifecycleState::Active);
        assert_eq!(obs.provenance_source, ProvenanceSource::LlmReasoning);
        assert!((obs.provenance_confidence - 0.6).abs() < f64::EPSILON);
        assert!(!obs.normalized_hash.is_empty());
    }

    #[test]
    fn hash_is_deterministic() {
        let h1 = Observation::compute_hash("title", "content");
        let h2 = Observation::compute_hash("title", "content");
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_differs_for_different_content() {
        let h1 = Observation::compute_hash("title", "content1");
        let h2 = Observation::compute_hash("title", "content2");
        assert_ne!(h1, h2);
    }
}

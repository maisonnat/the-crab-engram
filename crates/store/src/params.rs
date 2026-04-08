use engram_core::{ObservationType, Scope};

/// Parameters for creating a new observation.
#[derive(Debug, Clone)]
pub struct AddObservationParams {
    pub r#type: ObservationType,
    pub scope: Scope,
    pub title: String,
    pub content: String,
    pub session_id: String,
    pub project: String,
    pub topic_key: Option<String>,
    pub provenance_source: Option<String>,
    pub provenance_evidence: Vec<String>,
}

/// Parameters for updating an existing observation.
#[derive(Debug, Clone, Default)]
pub struct UpdateObservationParams {
    pub title: Option<String>,
    pub content: Option<String>,
    pub scope: Option<Scope>,
    pub topic_key: Option<String>,
    pub pinned: Option<bool>,
    pub provenance_source: Option<String>,
    pub provenance_confidence: Option<f64>,
    pub provenance_evidence: Option<Vec<String>>,
    pub lifecycle_state: Option<String>,
}

/// Search options and filters.
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    pub query: String,
    pub project: Option<String>,
    pub r#type: Option<ObservationType>,
    pub scope: Option<Scope>,
    pub topic_key: Option<String>,
    pub min_confidence: Option<f64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub include_stale: bool,
}

/// Parameters for saving a user prompt.
#[derive(Debug, Clone)]
pub struct AddPromptParams {
    pub session_id: String,
    pub project: String,
    pub content: String,
}

/// Parameters for adding a graph edge.
#[derive(Debug, Clone)]
pub struct AddEdgeParams {
    pub source_id: i64,
    pub target_id: i64,
    pub relation: engram_core::RelationType,
    pub weight: f64,
    pub auto_detected: bool,
}

impl Default for AddObservationParams {
    fn default() -> Self {
        Self {
            r#type: ObservationType::Manual,
            scope: Scope::Project,
            title: String::new(),
            content: String::new(),
            session_id: String::new(),
            project: String::new(),
            topic_key: None,
            provenance_source: None,
            provenance_evidence: Vec::new(),
        }
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Memory type — separates episodic (what happened) from semantic (what is known).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    Episodic,
    Semantic,
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Episodic => write!(f, "episodic"),
            Self::Semantic => write!(f, "semantic"),
        }
    }
}

impl std::str::FromStr for MemoryType {
    type Err = crate::EngramError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "episodic" => Ok(Self::Episodic),
            "semantic" => Ok(Self::Semantic),
            _ => Err(crate::EngramError::Config(format!(
                "invalid memory type: {s}"
            ))),
        }
    }
}

impl Default for MemoryType {
    fn default() -> Self {
        Self::Episodic
    }
}

/// Episodic memory — rich temporal context about what happened.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicMemory {
    pub id: i64,
    pub observation_id: i64,
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub what_happened: String,
    pub context: EpisodicContext,
    pub emotional_valence: f64,
    pub surprise_factor: f64,
}

/// Context for an episodic memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicContext {
    pub where_: Vec<String>,
    pub why: String,
    pub with_whom: Option<String>,
    pub files_before: Option<String>, // git hash reference, not diff inline
}

impl Default for EpisodicContext {
    fn default() -> Self {
        Self {
            where_: Vec::new(),
            why: String::new(),
            with_whom: None,
            files_before: None,
        }
    }
}

/// Semantic memory — dense, general knowledge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticMemory {
    pub id: i64,
    pub observation_id: i64,
    pub knowledge: String,
    pub domain: String,
    pub confidence: f64,
    pub source_episodes: Vec<i64>, // Traceability to source episodes
    pub last_validated: DateTime<Utc>,
}

impl SemanticMemory {
    /// Create a semantic memory from an episodic memory.
    pub fn from_episode(episode: &EpisodicMemory, domain: &str) -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            observation_id: episode.observation_id,
            knowledge: episode.what_happened.clone(),
            domain: domain.to_string(),
            confidence: 0.6, // Initial confidence, increases with validation
            source_episodes: vec![episode.observation_id],
            last_validated: now,
        }
    }
}

/// Classify a search query as targeting episodic, semantic, or both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryTarget {
    Episodic,
    Semantic,
    Both,
}

/// Simple heuristic to classify query type.
pub fn classify_query_type(query: &str) -> QueryTarget {
    let lower = query.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();

    // Episodic indicators: temporal, event-oriented
    let episodic_phrases = [
        "what happened",
        "last time",
        "how did",
        "what went wrong",
        "error that",
        "bug that",
    ];
    let episodic_words = ["when", "yesterday", "session", "crash", "bug"];

    // Semantic indicators: knowledge-oriented
    let semantic_phrases = ["what is", "how to", "how do", "best practice", "why does"];
    let semantic_words = [
        "config",
        "setting",
        "pattern",
        "architecture",
        "decision",
        "approach",
        "explain",
    ];

    let is_episodic = episodic_phrases.iter().any(|p| lower.contains(p))
        || episodic_words.iter().any(|w| words.contains(w));

    let is_semantic = semantic_phrases.iter().any(|p| lower.contains(p))
        || semantic_words.iter().any(|w| words.contains(w));

    match (is_episodic, is_semantic) {
        (true, false) => QueryTarget::Episodic,
        (false, true) => QueryTarget::Semantic,
        _ => QueryTarget::Both,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_type_roundtrip() {
        assert_eq!(MemoryType::Episodic.to_string(), "episodic");
        assert_eq!(MemoryType::Semantic.to_string(), "semantic");

        let e: MemoryType = "episodic".parse().unwrap();
        assert_eq!(e, MemoryType::Episodic);

        let s: MemoryType = "semantic".parse().unwrap();
        assert_eq!(s, MemoryType::Semantic);
    }

    #[test]
    fn memory_type_invalid() {
        let result: Result<MemoryType, _> = "invalid".parse();
        assert!(result.is_err());
    }

    #[test]
    fn classify_episodic_queries() {
        assert_eq!(
            classify_query_type("what happened with the auth bug"),
            QueryTarget::Episodic
        );
        assert_eq!(
            classify_query_type("when did we break the build"),
            QueryTarget::Episodic
        );
        assert_eq!(
            classify_query_type("last time we had this error"),
            QueryTarget::Episodic
        );
    }

    #[test]
    fn classify_semantic_queries() {
        assert_eq!(
            classify_query_type("what is the auth configuration"),
            QueryTarget::Semantic
        );
        assert_eq!(
            classify_query_type("how to set up JWT"),
            QueryTarget::Semantic
        );
        assert_eq!(
            classify_query_type("best practice for error handling"),
            QueryTarget::Semantic
        );
    }

    #[test]
    fn semantic_from_episode() {
        let episode = EpisodicMemory {
            id: 1,
            observation_id: 42,
            session_id: "session-1".into(),
            timestamp: Utc::now(),
            what_happened: "Fixed auth bug".into(),
            context: EpisodicContext::default(),
            emotional_valence: -0.5,
            surprise_factor: 0.7,
        };

        let semantic = SemanticMemory::from_episode(&episode, "auth");
        assert_eq!(semantic.observation_id, 42);
        assert_eq!(semantic.domain, "auth");
        assert_eq!(semantic.source_episodes, vec![42]);
        assert!((semantic.confidence - 0.6).abs() < f64::EPSILON);
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Relationship types in the knowledge graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    CausedBy,
    RelatedTo,
    Supersedes,
    Blocks,
    PartOf,
}

impl std::fmt::Display for RelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::CausedBy => "caused_by",
            Self::RelatedTo => "related_to",
            Self::Supersedes => "supersedes",
            Self::Blocks => "blocks",
            Self::PartOf => "part_of",
        };
        write!(f, "{s}")
    }
}

impl RelationType {
    /// Return all valid relation type strings (derived from enum, single source of truth).
    pub fn all_str() -> &'static [&'static str] {
        &["caused_by", "related_to", "supersedes", "blocks", "part_of"]
    }
}

impl std::str::FromStr for RelationType {
    type Err = crate::EngramError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "caused_by" => Ok(Self::CausedBy),
            "related_to" => Ok(Self::RelatedTo),
            "supersedes" => Ok(Self::Supersedes),
            "blocks" => Ok(Self::Blocks),
            "part_of" => Ok(Self::PartOf),
            _ => Err(crate::EngramError::Config(format!(
                "invalid relation type: {s}"
            ))),
        }
    }
}

/// A temporal edge in the knowledge graph.
/// Edges have validity windows — they can be superseded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: i64,
    pub source_id: i64,
    pub target_id: i64,
    pub relation: RelationType,
    pub weight: f64,
    pub valid_from: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>,
    /// Transaction time: when this edge was recorded in the system (bitemporal).
    pub recorded_at: DateTime<Utc>,
    pub superseded_by: Option<i64>,
    pub auto_detected: bool,
}

impl Edge {
    pub fn new(source_id: i64, target_id: i64, relation: RelationType, weight: f64) -> Self {
        Self {
            id: 0, // assigned by storage
            source_id,
            target_id,
            relation,
            weight,
            valid_from: Utc::now(),
            valid_until: None,
            recorded_at: Utc::now(),
            superseded_by: None,
            auto_detected: false,
        }
    }

    pub fn is_active(&self) -> bool {
        self.valid_until.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_new_is_active() {
        let edge = Edge::new(1, 2, RelationType::CausedBy, 1.0);
        assert!(edge.is_active());
        assert!(edge.valid_until.is_none());
        assert!(!edge.auto_detected);
    }

    #[test]
    fn relation_type_display() {
        assert_eq!(RelationType::CausedBy.to_string(), "caused_by");
        assert_eq!(RelationType::RelatedTo.to_string(), "related_to");
        assert_eq!(RelationType::Supersedes.to_string(), "supersedes");
    }
}

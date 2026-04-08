use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A coding session — groups observations by time and context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String, // UUID v4
    pub project: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub summary: Option<String>,
}

/// Condensed view of a session for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub project: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub observation_count: usize,
    pub summary: Option<String>,
}

impl Session {
    pub fn new(project: String) -> Self {
        Self {
            id: crate::Observation::generate_session_id(),
            project,
            started_at: Utc::now(),
            ended_at: None,
            summary: None,
        }
    }

    pub fn end(&mut self, summary: Option<String>) {
        self.ended_at = Some(Utc::now());
        self.summary = summary;
    }

    pub fn is_active(&self) -> bool {
        self.ended_at.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_new_is_active() {
        let session = Session::new("engram".into());
        assert!(session.is_active());
        assert!(session.ended_at.is_none());
        assert!(!session.id.is_empty());
    }

    #[test]
    fn session_end_sets_timestamp() {
        let mut session = Session::new("engram".into());
        session.end(Some("worked on auth".into()));
        assert!(!session.is_active());
        assert!(session.ended_at.is_some());
        assert_eq!(session.summary.as_deref(), Some("worked on auth"));
    }
}

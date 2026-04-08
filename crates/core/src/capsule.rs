use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Dense synthesis of knowledge by topic.
/// More than raw observations — this is what the system "understands".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeCapsule {
    pub id: i64,
    pub topic: String,
    pub project: Option<String>,
    pub summary: String, // 500-1000 chars, dense
    pub key_decisions: Vec<String>,
    pub known_issues: Vec<String>,
    pub anti_patterns: Vec<String>,
    pub best_practices: Vec<String>,
    pub source_observations: Vec<i64>,
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
    pub last_consolidated: DateTime<Utc>,
    pub version: u32,
}

impl KnowledgeCapsule {
    pub fn new(topic: String, project: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            topic,
            project,
            summary: String::new(),
            key_decisions: Vec::new(),
            known_issues: Vec::new(),
            anti_patterns: Vec::new(),
            best_practices: Vec::new(),
            source_observations: Vec::new(),
            confidence: 0.5,
            created_at: now,
            last_consolidated: now,
            version: 1,
        }
    }

    /// Format as Markdown for display.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str(&format!(
            "## 📌 {} (confidence: {:.0}%, v{})\n\n",
            self.topic,
            self.confidence * 100.0,
            self.version
        ));
        md.push_str(&format!("{}\n\n", self.summary));

        if !self.key_decisions.is_empty() {
            md.push_str("**Key Decisions:**\n");
            for d in &self.key_decisions {
                md.push_str(&format!("- {d}\n"));
            }
            md.push('\n');
        }

        if !self.known_issues.is_empty() {
            md.push_str("**Known Issues:**\n");
            for i in &self.known_issues {
                md.push_str(&format!("- {i}\n"));
            }
            md.push('\n');
        }

        if !self.anti_patterns.is_empty() {
            md.push_str("**Anti-Patterns:**\n");
            for a in &self.anti_patterns {
                md.push_str(&format!("- ⚠️ {a}\n"));
            }
            md.push('\n');
        }

        if !self.best_practices.is_empty() {
            md.push_str("**Best Practices:**\n");
            for b in &self.best_practices {
                md.push_str(&format!("- ✅ {b}\n"));
            }
            md.push('\n');
        }

        md.push_str(&format!(
            "_Sources: {} observations | Last consolidated: {}_\n",
            self.source_observations.len(),
            self.last_consolidated.format("%Y-%m-%d %H:%M")
        ));

        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capsule_new_has_defaults() {
        let capsule = KnowledgeCapsule::new("auth".into(), Some("test".into()));
        assert_eq!(capsule.topic, "auth");
        assert_eq!(capsule.version, 1);
        assert!(capsule.summary.is_empty());
    }

    #[test]
    fn capsule_markdown_format() {
        let mut capsule = KnowledgeCapsule::new("auth".into(), None);
        capsule.summary = "JWT-based auth with RS256".into();
        capsule.confidence = 0.85;
        capsule.key_decisions.push("Use RS256 over HS256".into());
        capsule.best_practices.push("15min token expiry".into());

        let md = capsule.to_markdown();
        assert!(md.contains("auth"));
        assert!(md.contains("85%"));
        assert!(md.contains("RS256"));
        assert!(md.contains("15min"));
    }
}

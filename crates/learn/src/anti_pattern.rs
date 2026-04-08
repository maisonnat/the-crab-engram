use std::collections::HashMap;

use engram_core::{EngramError, ObservationType, RelationType};
use engram_search::Embedder;
use engram_store::{SearchOptions, Storage};

/// Anti-pattern severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "Low"),
            Self::Medium => write!(f, "Medium"),
            Self::High => write!(f, "High"),
            Self::Critical => write!(f, "Critical"),
        }
    }
}

/// Types of anti-patterns detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntiPatternType {
    RecurringBug,
    RevertPattern,
    HotspotFile,
    UnverifiedDecision,
}

impl std::fmt::Display for AntiPatternType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RecurringBug => write!(f, "Recurring Bug"),
            Self::RevertPattern => write!(f, "Revert Pattern"),
            Self::HotspotFile => write!(f, "Hotspot File"),
            Self::UnverifiedDecision => write!(f, "Unverified Decision"),
        }
    }
}

/// A detected anti-pattern.
#[derive(Debug, Clone)]
pub struct AntiPattern {
    pub r#type: AntiPatternType,
    pub description: String,
    pub evidence: Vec<i64>, // observation IDs
    pub severity: Severity,
    pub suggestion: String,
}

impl std::fmt::Display for AntiPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "⚠️ [{}] {} ({} occurrences, severity: {})\n   Suggestion: {}",
            self.r#type,
            self.description,
            self.evidence.len(),
            self.severity,
            self.suggestion
        )
    }
}

/// Detects anti-patterns in the knowledge base.
pub struct AntiPatternDetector {
    pub store: std::sync::Arc<dyn Storage>,
    pub embedder: Option<std::sync::Arc<Embedder>>,
}

impl AntiPatternDetector {
    pub fn new(
        store: std::sync::Arc<dyn Storage>,
        embedder: Option<std::sync::Arc<Embedder>>,
    ) -> Self {
        Self { store, embedder }
    }

    /// Run all detectors and return all found anti-patterns.
    pub fn detect_all(&self, project: &str) -> Result<Vec<AntiPattern>, EngramError> {
        let mut patterns = Vec::new();

        patterns.extend(self.detect_recurring_bugs(project, 3, 0.8)?);
        patterns.extend(self.detect_revert_patterns(project)?);
        patterns.extend(self.detect_hotspot_files(project, 10)?);
        patterns.extend(self.detect_unverified_decisions(project, 0.7)?);

        // Sort by severity descending
        patterns.sort_by(|a, b| b.severity.cmp(&a.severity));

        Ok(patterns)
    }

    /// Detect recurring bugs: 3+ similar bugfixes.
    pub fn detect_recurring_bugs(
        &self,
        project: &str,
        min_occurrences: usize,
        similarity_threshold: f64,
    ) -> Result<Vec<AntiPattern>, EngramError> {
        let embedder = match &self.embedder {
            Some(e) => e,
            None => return Ok(vec![]),
        };

        let bugfixes = self.store.search(&SearchOptions {
            query: String::new(),
            project: Some(project.to_string()),
            r#type: Some(ObservationType::Bugfix),
            limit: Some(1000),
            ..Default::default()
        })?;

        if bugfixes.len() < min_occurrences {
            return Ok(vec![]);
        }

        let texts: Vec<&str> = bugfixes.iter().map(|b| b.content.as_str()).collect();
        let embeddings = embedder
            .embed_batch(&texts)
            .map_err(|e| EngramError::Embedding(e.to_string()))?;

        let mut patterns = Vec::new();
        let mut used = std::collections::HashSet::new();

        for i in 0..bugfixes.len() {
            if used.contains(&bugfixes[i].id) {
                continue;
            }

            let mut cluster = vec![i];

            for j in (i + 1)..bugfixes.len() {
                if used.contains(&bugfixes[j].id) {
                    continue;
                }
                let sim = Embedder::cosine_similarity(&embeddings[i], &embeddings[j]);
                if sim > similarity_threshold {
                    cluster.push(j);
                }
            }

            if cluster.len() >= min_occurrences {
                let severity = match cluster.len() {
                    n if n >= 5 => Severity::Critical,
                    n if n >= 4 => Severity::High,
                    _ => Severity::Medium,
                };

                let evidence: Vec<i64> = cluster.iter().map(|&idx| bugfixes[idx].id).collect();

                patterns.push(AntiPattern {
                    r#type: AntiPatternType::RecurringBug,
                    description: format!(
                        "Recurring bug: '{}' ({} similar occurrences)",
                        bugfixes[cluster[0]].title,
                        cluster.len()
                    ),
                    evidence,
                    severity,
                    suggestion: "Consider root cause analysis. This bug keeps reappearing."
                        .to_string(),
                });

                for &idx in &cluster {
                    used.insert(bugfixes[idx].id);
                }
            }
        }

        Ok(patterns)
    }

    /// Detect revert patterns: A→B→A cycles in supersedes edges.
    pub fn detect_revert_patterns(&self, project: &str) -> Result<Vec<AntiPattern>, EngramError> {
        let observations = self.store.search(&SearchOptions {
            query: String::new(),
            project: Some(project.to_string()),
            limit: Some(1000),
            ..Default::default()
        })?;

        let mut patterns = Vec::new();

        for obs in &observations {
            let edges = self.store.get_edges(obs.id)?;

            for edge in &edges {
                if edge.relation != RelationType::Supersedes || edge.source_id != obs.id {
                    continue;
                }

                // Check if the target (B) supersedes something that supersedes A
                let target_edges = self.store.get_edges(edge.target_id)?;
                for target_edge in &target_edges {
                    if target_edge.relation == RelationType::Supersedes
                        && target_edge.target_id == obs.id
                    {
                        patterns.push(AntiPattern {
                            r#type: AntiPatternType::RevertPattern,
                            description: format!(
                                "Revert pattern: decision #{} was reverted and then re-applied",
                                obs.id
                            ),
                            evidence: vec![obs.id, edge.target_id, target_edge.target_id],
                            severity: Severity::High,
                            suggestion:
                                "Decisions are being changed back and forth. Investigate the root cause."
                                    .to_string(),
                        });
                    }
                }
            }
        }

        Ok(patterns)
    }

    /// Detect hotspot files: mentioned in >threshold observations.
    pub fn detect_hotspot_files(
        &self,
        project: &str,
        threshold: usize,
    ) -> Result<Vec<AntiPattern>, EngramError> {
        let observations = self.store.search(&SearchOptions {
            query: String::new(),
            project: Some(project.to_string()),
            limit: Some(1000),
            ..Default::default()
        })?;

        // Count file mentions (heuristic: paths with / or .rs/.ts/.go extensions)
        let mut file_mentions: HashMap<String, Vec<i64>> = HashMap::new();

        for obs in &observations {
            let content = format!("{} {}", obs.title, obs.content);
            for word in content.split_whitespace() {
                if (word.contains('/') || word.contains('\\'))
                    && (word.ends_with(".rs")
                        || word.ends_with(".ts")
                        || word.ends_with(".go")
                        || word.ends_with(".py")
                        || word.ends_with(".js"))
                {
                    let clean = word.trim_matches(|c: char| {
                        !c.is_alphanumeric() && c != '/' && c != '\\' && c != '.'
                    });
                    file_mentions
                        .entry(clean.to_lowercase())
                        .or_default()
                        .push(obs.id);
                }
            }
        }

        let mut patterns = Vec::new();

        for (file, mentions) in &file_mentions {
            if mentions.len() >= threshold {
                let severity = match mentions.len() {
                    n if n >= 20 => Severity::Critical,
                    n if n >= 15 => Severity::High,
                    _ => Severity::Medium,
                };

                patterns.push(AntiPattern {
                    r#type: AntiPatternType::HotspotFile,
                    description: format!(
                        "Hotspot file: '{}' (mentioned in {} observations)",
                        file,
                        mentions.len()
                    ),
                    evidence: mentions.clone(),
                    severity,
                    suggestion: "This file is frequently modified. Consider refactoring or splitting responsibilities.".to_string(),
                });
            }
        }

        Ok(patterns)
    }

    /// Detect unverified decisions: provenance_confidence < threshold.
    pub fn detect_unverified_decisions(
        &self,
        project: &str,
        min_confidence: f64,
    ) -> Result<Vec<AntiPattern>, EngramError> {
        let observations = self.store.search(&SearchOptions {
            query: String::new(),
            project: Some(project.to_string()),
            r#type: Some(ObservationType::Decision),
            limit: Some(1000),
            ..Default::default()
        })?;

        let unverified: Vec<i64> = observations
            .iter()
            .filter(|o| o.provenance_confidence < min_confidence)
            .map(|o| o.id)
            .collect();

        if unverified.is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![AntiPattern {
            r#type: AntiPatternType::UnverifiedDecision,
            description: format!(
                "{} decisions with low confidence (<{:.0}%)",
                unverified.len(),
                min_confidence * 100.0
            ),
            evidence: unverified,
            severity: Severity::Low,
            suggestion:
                "Some decisions lack verification (test, code review, etc.). Consider validating them."
                    .to_string(),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engram_core::Scope;
    use engram_store::SqliteStore;

    fn setup_store() -> std::sync::Arc<SqliteStore> {
        let store = std::sync::Arc::new(SqliteStore::in_memory().unwrap());
        let sid = store.create_session("test").unwrap();

        // Add some decisions with low confidence
        store
            .insert_observation(&engram_store::AddObservationParams {
                r#type: ObservationType::Decision,
                scope: Scope::Project,
                title: "Use Redis".into(),
                content: "For caching".into(),
                session_id: sid.clone(),
                project: "test".into(),
                provenance_source: Some("llm_reasoning".into()),
                ..Default::default()
            })
            .unwrap();

        store
    }

    #[test]
    fn detector_no_embedder_returns_empty() {
        let store = setup_store();
        let detector = AntiPatternDetector::new(store, None);
        let patterns = detector.detect_all("test").unwrap();
        // Only unverified decisions should be found without embedder
        assert!(
            patterns
                .iter()
                .all(|p| p.r#type == AntiPatternType::UnverifiedDecision)
        );
    }

    #[test]
    fn detect_unverified_decisions() {
        let store = setup_store();
        let detector = AntiPatternDetector::new(store, None);
        let patterns = detector.detect_unverified_decisions("test", 0.7).unwrap();
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].r#type, AntiPatternType::UnverifiedDecision);
        assert!(patterns[0].description.contains("1 decisions"));
    }

    #[test]
    fn detect_hotspot_files() {
        let store = std::sync::Arc::new(SqliteStore::in_memory().unwrap());
        let sid = store.create_session("test").unwrap();

        // Add 11 observations mentioning the same file
        for i in 0..11 {
            store
                .insert_observation(&engram_store::AddObservationParams {
                    r#type: ObservationType::Bugfix,
                    scope: Scope::Project,
                    title: format!("Bug {i}"),
                    content: format!("Fixed in src/auth.rs iteration {i}"),
                    session_id: sid.clone(),
                    project: "test".into(),
                    ..Default::default()
                })
                .unwrap();
        }

        let detector = AntiPatternDetector::new(store, None);
        let patterns = detector.detect_hotspot_files("test", 10).unwrap();
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].r#type, AntiPatternType::HotspotFile);
        assert!(patterns[0].description.contains("auth.rs"));
    }

    #[test]
    fn anti_pattern_display() {
        let pattern = AntiPattern {
            r#type: AntiPatternType::RecurringBug,
            description: "Auth bug recurring".into(),
            evidence: vec![1, 2, 3],
            severity: Severity::High,
            suggestion: "Investigate root cause".into(),
        };
        let display = format!("{pattern}");
        assert!(display.contains("Recurring Bug"));
        assert!(display.contains("3 occurrences"));
        assert!(display.contains("High"));
    }
}

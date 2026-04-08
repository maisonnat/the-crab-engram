use std::sync::Arc;

use chrono::Utc;
use tracing::info;

use engram_core::{EngramError, KnowledgeCapsule, Observation, ObservationType};
use engram_store::{SearchOptions, Storage};

/// Trait for synthesizing knowledge capsules from observations.
/// Implementations can be heuristic, LLM-based, or chained.
pub trait CapsuleSynthesizer: Send + Sync {
    /// Synthesize a capsule from observations on a topic.
    fn synthesize(
        &self,
        observations: &[Observation],
        topic: &str,
    ) -> Result<KnowledgeCapsule, EngramError>;

    /// Is this synthesizer available?
    fn can_synthesize(&self) -> bool;

    /// Name for logging/debugging.
    fn name(&self) -> &str;
}

/// Heuristic synthesizer — always available, no external dependencies.
/// Extracts knowledge by analyzing observation types and content patterns.
pub struct HeuristicSynthesizer;

impl CapsuleSynthesizer for HeuristicSynthesizer {
    fn synthesize(
        &self,
        observations: &[Observation],
        topic: &str,
    ) -> Result<KnowledgeCapsule, EngramError> {
        let mut capsule = KnowledgeCapsule::new(topic.to_string(), None);

        // Collect source IDs
        capsule.source_observations = observations.iter().map(|o| o.id).collect();

        // Extract key decisions
        capsule.key_decisions = observations
            .iter()
            .filter(|o| o.r#type == ObservationType::Decision)
            .map(|o| format!("{}: {}", o.title, truncate(&o.content, 100)))
            .collect();

        // Extract known issues (bugfixes)
        capsule.known_issues = observations
            .iter()
            .filter(|o| o.r#type == ObservationType::Bugfix)
            .map(|o| format!("{}: {}", o.title, truncate(&o.content, 100)))
            .collect();

        // Extract anti-patterns
        capsule.anti_patterns = observations
            .iter()
            .filter(|o| o.r#type == ObservationType::Pattern)
            .map(|o| format!("{}: {}", o.title, truncate(&o.content, 100)))
            .collect();

        // Extract best practices (config + convention)
        capsule.best_practices = observations
            .iter()
            .filter(|o| {
                o.r#type == ObservationType::Config
                    || o.r#type == ObservationType::Convention
                    || o.r#type == ObservationType::Architecture
            })
            .map(|o| format!("{}: {}", o.title, truncate(&o.content, 100)))
            .collect();

        // Generate summary
        capsule.summary = generate_summary(observations, topic);

        // Confidence = min of source provenance confidences
        capsule.confidence = observations
            .iter()
            .map(|o| o.provenance_confidence)
            .fold(1.0f64, f64::min)
            .max(0.1);

        capsule.project = observations.first().map(|o| o.project.clone());
        capsule.last_consolidated = Utc::now();

        Ok(capsule)
    }

    fn can_synthesize(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "heuristic"
    }
}

/// Chained synthesizer: tries primary, falls back to secondary.
pub struct ChainedSynthesizer {
    primary: Box<dyn CapsuleSynthesizer>,
    fallback: Box<dyn CapsuleSynthesizer>,
}

impl ChainedSynthesizer {
    pub fn new(
        primary: Box<dyn CapsuleSynthesizer>,
        fallback: Box<dyn CapsuleSynthesizer>,
    ) -> Self {
        Self { primary, fallback }
    }
}

impl CapsuleSynthesizer for ChainedSynthesizer {
    fn synthesize(
        &self,
        observations: &[Observation],
        topic: &str,
    ) -> Result<KnowledgeCapsule, EngramError> {
        if self.primary.can_synthesize() {
            match self.primary.synthesize(observations, topic) {
                Ok(capsule) => return Ok(capsule),
                Err(e) => {
                    tracing::warn!("Primary synthesizer failed, falling back: {}", e);
                }
            }
        }
        self.fallback.synthesize(observations, topic)
    }

    fn can_synthesize(&self) -> bool {
        true // Always has fallback
    }

    fn name(&self) -> &str {
        "chained"
    }
}

/// Builder that uses a synthesizer to create/update capsules from storage.
pub struct CapsuleBuilder {
    pub store: Arc<dyn Storage>,
    pub synthesizer: Box<dyn CapsuleSynthesizer>,
}

impl CapsuleBuilder {
    pub fn new(store: Arc<dyn Storage>, synthesizer: Box<dyn CapsuleSynthesizer>) -> Self {
        Self { store, synthesizer }
    }

    /// Build or update a capsule for a topic.
    pub fn build_capsule(
        &self,
        project: &str,
        topic: &str,
    ) -> Result<KnowledgeCapsule, EngramError> {
        // Search by topic_key prefix (more reliable than FTS5 for topic matching)
        let all_observations = self.store.search(&SearchOptions {
            query: String::new(),
            project: Some(project.to_string()),
            limit: Some(1000),
            ..Default::default()
        })?;

        // Filter by topic_key containing the topic
        let observations: Vec<Observation> = all_observations
            .into_iter()
            .filter(|o| {
                o.topic_key.as_deref().unwrap_or("").contains(topic)
                    || o.title.to_lowercase().contains(&topic.to_lowercase())
                    || o.content.to_lowercase().contains(&topic.to_lowercase())
            })
            .collect();

        if observations.is_empty() {
            return Err(EngramError::NotFound(format!(
                "no observations found for topic '{topic}'"
            )));
        }

        info!(
            "Building capsule for topic '{}' from {} observations",
            topic,
            observations.len()
        );

        let mut capsule = self.synthesizer.synthesize(&observations, topic)?;
        capsule.project = Some(project.to_string());

        Ok(capsule)
    }
}

// ── Helpers ───────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

fn generate_summary(observations: &[Observation], topic: &str) -> String {
    let decisions = observations
        .iter()
        .filter(|o| o.r#type == ObservationType::Decision)
        .count();
    let bugs = observations
        .iter()
        .filter(|o| o.r#type == ObservationType::Bugfix)
        .count();
    let patterns = observations
        .iter()
        .filter(|o| o.r#type == ObservationType::Pattern)
        .count();
    let configs = observations
        .iter()
        .filter(|o| o.r#type == ObservationType::Config)
        .count();

    format!(
        "Topic '{topic}' has {total} observations: {decisions} decisions, {bugs} bugfixes, \
         {patterns} patterns, {configs} config entries. \
         Based on the accumulated knowledge, this topic covers the following areas: {}",
        observations
            .iter()
            .take(5)
            .map(|o| o.title.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        total = observations.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use engram_core::Scope;
    use engram_store::SqliteStore;

    fn setup_with_observations() -> Arc<SqliteStore> {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let sid = store.create_session("test").unwrap();

        store
            .insert_observation(&engram_store::AddObservationParams {
                r#type: ObservationType::Decision,
                scope: Scope::Project,
                title: "Use JWT auth".into(),
                content: "RS256 with 15min expiry".into(),
                session_id: sid.clone(),
                project: "test".into(),
                topic_key: Some("decision/auth".into()),
                ..Default::default()
            })
            .unwrap();

        store
            .insert_observation(&engram_store::AddObservationParams {
                r#type: ObservationType::Bugfix,
                scope: Scope::Project,
                title: "Fix token expiry".into(),
                content: "Tokens were expiring too quickly".into(),
                session_id: sid.clone(),
                project: "test".into(),
                topic_key: Some("bug/auth".into()),
                ..Default::default()
            })
            .unwrap();

        store
            .insert_observation(&engram_store::AddObservationParams {
                r#type: ObservationType::Config,
                scope: Scope::Project,
                title: "Auth config".into(),
                content: "Set RS256 as algorithm".into(),
                session_id: sid,
                project: "test".into(),
                topic_key: Some("config/auth".into()),
                ..Default::default()
            })
            .unwrap();

        store
    }

    #[test]
    fn heuristic_synthesizer_produces_capsule() {
        let store = setup_with_observations();
        let builder = CapsuleBuilder::new(store, Box::new(HeuristicSynthesizer));

        let capsule = builder.build_capsule("test", "auth").unwrap();
        assert!(!capsule.summary.is_empty());
        assert_eq!(capsule.key_decisions.len(), 1);
        assert_eq!(capsule.known_issues.len(), 1);
        assert_eq!(capsule.best_practices.len(), 1);
        assert!(capsule.confidence > 0.0);
    }

    #[test]
    fn heuristic_synthesizer_always_available() {
        let synth = HeuristicSynthesizer;
        assert!(synth.can_synthesize());
        assert_eq!(synth.name(), "heuristic");
    }

    #[test]
    fn chained_synthesizer_falls_back() {
        struct FailingSynth;
        impl CapsuleSynthesizer for FailingSynth {
            fn synthesize(
                &self,
                _obs: &[Observation],
                _topic: &str,
            ) -> Result<KnowledgeCapsule, EngramError> {
                Err(EngramError::Config("not available".into()))
            }
            fn can_synthesize(&self) -> bool {
                true
            }
            fn name(&self) -> &str {
                "failing"
            }
        }

        let chained =
            ChainedSynthesizer::new(Box::new(FailingSynth), Box::new(HeuristicSynthesizer));

        assert!(chained.can_synthesize());

        let store = setup_with_observations();
        let builder = CapsuleBuilder::new(store, Box::new(chained));
        let capsule = builder.build_capsule("test", "auth").unwrap();
        assert!(!capsule.summary.is_empty());
    }

    #[test]
    fn build_capsule_no_observations_errors() {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let builder = CapsuleBuilder::new(store, Box::new(HeuristicSynthesizer));
        let result = builder.build_capsule("test", "nonexistent");
        assert!(result.is_err());
    }
}

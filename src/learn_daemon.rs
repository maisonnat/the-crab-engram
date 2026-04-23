use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use chrono::Utc;
use tracing::{error, info, warn};

use engram_api::{LearnDaemonStatus, LearnTickStatus};
use engram_core::{EngramError, ObservationType, Scope};
use engram_learn::{
    AntiPatternDetector, CapsuleBuilder, ConsolidationEngine, ConsolidationResult, EvolutionResult,
    GraphEvolver, HeuristicSynthesizer, SmartInjector, infer_salience,
};
use engram_search::Embedder;
use engram_store::{AddObservationParams, SearchOptions, Storage};

#[derive(Debug, Clone)]
pub struct LearnDaemonConfig {
    pub project: String,
    pub interval_seconds: u64,
    pub max_search_observations: usize,
    pub max_capsules_per_tick: usize,
    pub max_reviews_bootstrap: usize,
    pub max_injection_tokens: usize,
    pub write_summary_observations: bool,
    pub enable_consolidation: bool,
    pub enable_evolution: bool,
    pub enable_capsules: bool,
    pub enable_reviews: bool,
    pub enable_anti_patterns: bool,
    pub enable_injection_snapshots: bool,
}

impl Default for LearnDaemonConfig {
    fn default() -> Self {
        Self {
            project: "default".into(),
            interval_seconds: 60,
            max_search_observations: 1000,
            max_capsules_per_tick: 5,
            max_reviews_bootstrap: 25,
            max_injection_tokens: 1200,
            write_summary_observations: true,
            enable_consolidation: true,
            enable_evolution: true,
            enable_capsules: true,
            enable_reviews: true,
            enable_anti_patterns: true,
            enable_injection_snapshots: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LearnTickResult {
    pub consolidation: Option<ConsolidationResult>,
    pub evolution: Option<EvolutionResult>,
    pub capsules_upserted: usize,
    pub reviews_upserted: usize,
    pub anti_patterns_found: usize,
    pub entities_linked: usize,
    pub snapshots_written: usize,
}

pub struct LearnDaemon {
    store: Arc<dyn Storage>,
    config: LearnDaemonConfig,
    embedder: Option<Arc<Embedder>>,
    status: Option<Arc<Mutex<LearnDaemonStatus>>>,
}

impl LearnDaemon {
    pub fn new(
        store: Arc<dyn Storage>,
        config: LearnDaemonConfig,
        embedder: Option<Arc<Embedder>>,
        status: Option<Arc<Mutex<LearnDaemonStatus>>>,
    ) -> Self {
        Self {
            store,
            config,
            embedder,
            status,
        }
    }

    pub fn run_once(&self) -> Result<LearnTickResult, EngramError> {
        self.mark_tick_started();

        let result = (|| {
            let mut result = LearnTickResult {
                entities_linked: self.observe_phase()?,
                ..Default::default()
            };

            if self.config.enable_consolidation {
                let engine = ConsolidationEngine::new(self.store.clone(), self.embedder.clone());
                result.consolidation = Some(engine.run_consolidation(&self.config.project)?);
            }

            if self.config.enable_evolution {
                let evolver = GraphEvolver::new(self.store.clone(), self.embedder.clone());
                result.evolution = Some(evolver.evolve(&self.config.project)?);
            }

            if self.config.enable_capsules {
                result.capsules_upserted = self.capsule_phase()?;
            }

            if self.config.enable_reviews {
                result.reviews_upserted = self.review_phase()?;
            }

            if self.config.enable_anti_patterns {
                result.anti_patterns_found = self.anti_pattern_phase()?;
            }

            if self.config.enable_injection_snapshots {
                result.snapshots_written = self.injection_phase()?;
            }

            Ok(result)
        })();

        match &result {
            Ok(tick) => self.mark_tick_succeeded(tick),
            Err(err) => self.mark_tick_failed(err),
        }

        result
    }

    pub fn run_loop(&self) -> Result<(), EngramError> {
        loop {
            match self.run_once() {
                Ok(result) => info!(
                    project = %self.config.project,
                    entities_linked = result.entities_linked,
                    capsules_upserted = result.capsules_upserted,
                    reviews_upserted = result.reviews_upserted,
                    anti_patterns_found = result.anti_patterns_found,
                    snapshots_written = result.snapshots_written,
                    "learn tick complete"
                ),
                Err(err) => error!(project = %self.config.project, ?err, "learn tick failed"),
            }
            thread::sleep(Duration::from_secs(self.config.interval_seconds));
        }
    }

    fn observe_phase(&self) -> Result<usize, EngramError> {
        let observations = self.project_observations(self.config.max_search_observations)?;
        let mut linked = 0usize;
        let mut seen_pairs = HashSet::new();

        for obs in &observations {
            let text = format!("{} {}", obs.title, obs.content);
            let entities = engram_learn::MemoryStream::extract_entities(&text);
            let _salience = infer_salience(&obs.content, None);

            for entity in entities {
                let entity_id = if let Some((id, _, _, _)) = self.store.get_entity(&entity.name)? {
                    id
                } else {
                    self.store
                        .upsert_entity(&entity.name, &entity.entity_type, "")?
                };

                if seen_pairs.insert((entity_id, obs.id)) {
                    self.store.link_entity_observation(entity_id, obs.id)?;
                    linked += 1;
                }
            }
        }

        Ok(linked)
    }

    fn capsule_phase(&self) -> Result<usize, EngramError> {
        let observations = self.project_observations(self.config.max_search_observations)?;
        let mut counts: HashMap<String, usize> = HashMap::new();

        for obs in &observations {
            if let Some(topic) = obs.topic_key.as_ref().filter(|s| !s.is_empty()) {
                *counts.entry(topic.clone()).or_insert(0) += 1;
            }
        }

        let mut topics: Vec<(String, usize)> = counts.into_iter().collect();
        topics.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

        let builder = CapsuleBuilder::new(self.store.clone(), Box::new(HeuristicSynthesizer));
        let mut upserted = 0usize;

        for (topic, count) in topics
            .into_iter()
            .filter(|(_, count)| *count >= 2)
            .take(self.config.max_capsules_per_tick)
        {
            match builder.build_capsule(&self.config.project, &topic) {
                Ok(capsule) => {
                    self.store.upsert_capsule(&capsule)?;
                    upserted += 1;
                    info!(topic = %topic, source_count = count, "capsule upserted");
                }
                Err(err) => warn!(topic = %topic, ?err, "capsule build skipped"),
            }
        }

        Ok(upserted)
    }

    fn review_phase(&self) -> Result<usize, EngramError> {
        let pending = self.store.get_pending_reviews(
            Some(&self.config.project),
            self.config.max_reviews_bootstrap,
        )?;
        if !pending.is_empty() {
            return Ok(0);
        }

        let observations = self.project_observations(self.config.max_search_observations)?;
        let ranked: Vec<(i64, i64)> = observations
            .iter()
            .map(|o| (o.id, o.access_count))
            .collect();
        let schedules = engram_learn::bootstrap_reviews(&ranked, self.config.max_reviews_bootstrap);
        let now = Utc::now();
        let mut upserted = 0usize;

        for schedule in schedules {
            let next_review =
                now + chrono::Duration::seconds((schedule.interval_days * 86_400.0) as i64);
            self.store.upsert_review(
                schedule.memory_id,
                schedule.interval_days,
                schedule.ease_factor,
                &next_review.to_rfc3339(),
            )?;
            upserted += 1;
        }

        Ok(upserted)
    }

    fn anti_pattern_phase(&self) -> Result<usize, EngramError> {
        let detector = AntiPatternDetector::new(self.store.clone(), self.embedder.clone());
        let patterns = detector.detect_all(&self.config.project)?;

        if self.config.write_summary_observations && !patterns.is_empty() {
            let session_id = self.ensure_session()?;
            for pattern in &patterns {
                let topic = format!(
                    "learn/anti-pattern/{}",
                    pattern.r#type.to_string().to_lowercase().replace(' ', "-")
                );
                let content = format!(
                    "{}\n\nSeverity: {}\nSuggestion: {}\nEvidence: {:?}",
                    pattern.description, pattern.severity, pattern.suggestion, pattern.evidence
                );
                self.store.insert_observation(&AddObservationParams {
                    r#type: ObservationType::Learning,
                    scope: Scope::Project,
                    title: format!("Anti-pattern detected: {}", pattern.r#type),
                    content,
                    session_id: session_id.clone(),
                    project: self.config.project.clone(),
                    topic_key: Some(topic),
                    provenance_source: Some("inferred".into()),
                    provenance_evidence: pattern
                        .evidence
                        .iter()
                        .map(|id| format!("observation:{id}"))
                        .collect(),
                })?;
            }
        }

        Ok(patterns.len())
    }

    fn injection_phase(&self) -> Result<usize, EngramError> {
        let injector = SmartInjector::new(self.store.clone());
        let ctx = injector.build_context(
            &self.config.project,
            "current project state",
            self.config.max_injection_tokens,
        )?;

        if ctx.is_empty() {
            return Ok(0);
        }

        let session_id = self.ensure_session()?;
        self.store.insert_observation(&AddObservationParams {
            r#type: ObservationType::Learning,
            scope: Scope::Project,
            title: "Injection snapshot".into(),
            content: ctx.to_markdown(),
            session_id,
            project: self.config.project.clone(),
            topic_key: Some("learn/injection-snapshot".into()),
            provenance_source: Some("inferred".into()),
            provenance_evidence: vec![format!("generated_at:{}", Utc::now().to_rfc3339())],
        })?;

        Ok(1)
    }

    fn project_observations(
        &self,
        limit: usize,
    ) -> Result<Vec<engram_core::Observation>, EngramError> {
        self.store.search(&SearchOptions {
            query: String::new(),
            project: Some(self.config.project.clone()),
            limit: Some(limit),
            ..Default::default()
        })
    }

    fn ensure_session(&self) -> Result<String, EngramError> {
        self.store.create_session(&self.config.project)
    }

    #[allow(clippy::collapsible_if)]
    fn mark_tick_started(&self) {
        if let Some(status) = &self.status {
            if let Ok(mut status) = status.lock() {
                status.enabled = true;
                status.project = self.config.project.clone();
                status.interval_seconds = self.config.interval_seconds;
                status.last_started_at = Some(Utc::now().to_rfc3339());
                status.last_error = None;
            }
        }
    }

    #[allow(clippy::collapsible_if)]
    fn mark_tick_succeeded(&self, result: &LearnTickResult) {
        if let Some(status) = &self.status {
            if let Ok(mut status) = status.lock() {
                status.enabled = true;
                status.project = self.config.project.clone();
                status.interval_seconds = self.config.interval_seconds;
                status.ticks_run += 1;
                status.last_completed_at = Some(Utc::now().to_rfc3339());
                status.last_error = None;
                status.last_tick = Some(LearnTickStatus {
                    entities_linked: result.entities_linked,
                    capsules_upserted: result.capsules_upserted,
                    reviews_upserted: result.reviews_upserted,
                    anti_patterns_found: result.anti_patterns_found,
                    snapshots_written: result.snapshots_written,
                });
            }
        }
    }

    #[allow(clippy::collapsible_if)]
    fn mark_tick_failed(&self, err: &EngramError) {
        if let Some(status) = &self.status {
            if let Ok(mut status) = status.lock() {
                status.enabled = true;
                status.project = self.config.project.clone();
                status.interval_seconds = self.config.interval_seconds;
                status.last_completed_at = Some(Utc::now().to_rfc3339());
                status.last_error = Some(err.to_string());
            }
        }
    }
}

impl std::fmt::Display for LearnTickResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "learn tick: entities={}, capsules={}, reviews={}, anti_patterns={}, snapshots={}",
            self.entities_linked,
            self.capsules_upserted,
            self.reviews_upserted,
            self.anti_patterns_found,
            self.snapshots_written
        )
    }
}

pub fn format_tick_summary(result: &LearnTickResult) -> String {
    let consolidation = result
        .consolidation
        .as_ref()
        .map(|r| {
            format!(
                "duplicates={}, obsolete={}, conflicts={}, patterns={}",
                r.duplicates_merged, r.obsolete_marked, r.conflicts_found, r.patterns_extracted
            )
        })
        .unwrap_or_else(|| "disabled".into());
    let evolution = result
        .evolution
        .as_ref()
        .map(|r| {
            format!(
                "edges={}, temporal={}, file={}, semantic={}",
                r.edges_created, r.temporal_patterns, r.file_correlations, r.semantic_clusters
            )
        })
        .unwrap_or_else(|| "disabled".into());

    format!(
        "Learn tick complete\n- Observe: {} entity links\n- Consolidate: {}\n- Evolve: {}\n- Capsules upserted: {}\n- Reviews upserted: {}\n- Anti-patterns found: {}\n- Snapshots written: {}",
        result.entities_linked,
        consolidation,
        evolution,
        result.capsules_upserted,
        result.reviews_upserted,
        result.anti_patterns_found,
        result.snapshots_written
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use engram_store::SqliteStore;

    fn seed_store(project: &str) -> Arc<SqliteStore> {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let session_id = store.create_session(project).unwrap();

        for i in 0..4 {
            let id = store
                .insert_observation(&AddObservationParams {
                    r#type: if i % 2 == 0 {
                        ObservationType::Bugfix
                    } else {
                        ObservationType::Decision
                    },
                    scope: Scope::Project,
                    title: format!("Auth issue {i}"),
                    content: format!(
                        "Fixed src/auth.rs in AuthService iteration {i}; topic auth/login and crate::auth::service"
                    ),
                    session_id: session_id.clone(),
                    project: project.into(),
                    topic_key: Some("auth/login".into()),
                    provenance_source: None,
                    provenance_evidence: vec![],
                })
                .unwrap();
            store
                .update_observation(
                    id,
                    &engram_store::UpdateObservationParams {
                        provenance_confidence: Some(0.9),
                        ..Default::default()
                    },
                )
                .unwrap();
        }

        store
    }

    #[test]
    fn run_once_without_embedder_succeeds() {
        let store = seed_store("test");
        let daemon = LearnDaemon::new(
            store,
            LearnDaemonConfig {
                project: "test".into(),
                enable_injection_snapshots: true,
                ..Default::default()
            },
            None,
            None,
        );

        let result = daemon.run_once().unwrap();
        assert!(result.entities_linked > 0);
    }

    #[test]
    fn capsule_phase_upserts_capsule() {
        let store = seed_store("test");
        let daemon = LearnDaemon::new(
            store.clone(),
            LearnDaemonConfig {
                project: "test".into(),
                enable_consolidation: false,
                enable_evolution: false,
                enable_reviews: false,
                enable_anti_patterns: false,
                ..Default::default()
            },
            None,
            None,
        );

        let upserted = daemon.capsule_phase().unwrap();
        assert!(upserted >= 1);
        assert!(!store.list_capsules(Some("test")).unwrap().is_empty());
    }

    #[test]
    fn review_phase_bootstraps_reviews() {
        let store = seed_store("test");
        let daemon = LearnDaemon::new(
            store.clone(),
            LearnDaemonConfig {
                project: "test".into(),
                enable_consolidation: false,
                enable_evolution: false,
                enable_capsules: false,
                enable_anti_patterns: false,
                ..Default::default()
            },
            None,
            None,
        );

        let upserted = daemon.review_phase().unwrap();
        assert!(upserted > 0);
    }

    #[test]
    fn run_once_updates_shared_status() {
        let store = seed_store("test");
        let status = Arc::new(Mutex::new(LearnDaemonStatus::enabled("test".into(), 60)));
        let daemon = LearnDaemon::new(
            store,
            LearnDaemonConfig {
                project: "test".into(),
                ..Default::default()
            },
            None,
            Some(status.clone()),
        );

        let result = daemon.run_once().unwrap();
        assert!(result.entities_linked > 0);

        let snapshot = status.lock().unwrap().clone();
        assert_eq!(snapshot.ticks_run, 1);
        assert!(snapshot.last_started_at.is_some());
        assert!(snapshot.last_completed_at.is_some());
        assert!(snapshot.last_tick.is_some());
    }
}

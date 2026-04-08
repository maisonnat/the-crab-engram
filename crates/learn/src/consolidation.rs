use std::sync::Arc;

use chrono::Utc;
use tracing::{info, instrument};

use engram_core::{EngramError, Observation, ObservationType, Scope};
use engram_search::Embedder;
use engram_store::{AddObservationParams, SearchOptions, Storage};

/// Result of a consolidation run.
#[derive(Debug, Clone, Default)]
pub struct ConsolidationResult {
    pub duplicates_merged: u32,
    pub obsolete_marked: u32,
    pub conflicts_found: u32,
    pub patterns_extracted: u32,
    pub time_taken_ms: u64,
}

impl std::fmt::Display for ConsolidationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Consolidation complete in {}ms:\n  - {} duplicates merged\n  - {} obsolete marked\n  - {} conflicts found\n  - {} patterns extracted",
            self.time_taken_ms,
            self.duplicates_merged,
            self.obsolete_marked,
            self.conflicts_found,
            self.patterns_extracted,
        )
    }
}

/// Engine that consolidates, cleans, and improves the knowledge base.
pub struct ConsolidationEngine {
    pub store: Arc<dyn Storage>,
    pub embedder: Option<Arc<Embedder>>,
}

impl ConsolidationEngine {
    pub fn new(store: Arc<dyn Storage>, embedder: Option<Arc<Embedder>>) -> Self {
        Self { store, embedder }
    }

    /// Run full consolidation pipeline.
    #[instrument(skip(self), fields(project = %project))]
    pub fn run_consolidation(&self, project: &str) -> Result<ConsolidationResult, EngramError> {
        let start = Utc::now();
        let mut result = ConsolidationResult::default();

        info!("Starting consolidation for project: {}", project);

        // 1. Find and merge semantic duplicates
        result.duplicates_merged = self.merge_duplicates(project)?;
        info!("Merged {} duplicates", result.duplicates_merged);

        // 2. Find and mark obsolete observations
        result.obsolete_marked = self.mark_obsolete(project)?;
        info!("Marked {} obsolete", result.obsolete_marked);

        // 3. Find contradictions
        result.conflicts_found = self.find_contradictions(project)?;
        info!("Found {} conflicts", result.conflicts_found);

        // 4. Extract patterns from similar bugfixes
        result.patterns_extracted = self.extract_patterns(project)?;
        info!("Extracted {} patterns", result.patterns_extracted);

        let elapsed = (Utc::now() - start).num_milliseconds() as u64;
        result.time_taken_ms = elapsed;

        info!("{}", result);
        Ok(result)
    }

    /// Find and merge duplicates. Uses semantic similarity when embedder is available,
    /// falls back to hash-based dedup otherwise.
    fn merge_duplicates(&self, project: &str) -> Result<u32, EngramError> {
        // Get all observations for the project
        let observations = self.store.search(&SearchOptions {
            query: String::new(),
            project: Some(project.to_string()),
            limit: Some(1000),
            ..Default::default()
        })?;

        if let Some(embedder) = &self.embedder {
            self.merge_semantic_duplicates(&observations, embedder)
        } else {
            self.merge_hash_duplicates(&observations)
        }
    }

    /// Hash-based dedup: merge observations with same normalized_hash.
    fn merge_hash_duplicates(&self, observations: &[Observation]) -> Result<u32, EngramError> {
        let mut hash_groups: std::collections::HashMap<String, Vec<&Observation>> =
            std::collections::HashMap::new();
        for obs in observations {
            hash_groups
                .entry(obs.normalized_hash.clone())
                .or_default()
                .push(obs);
        }

        let mut merged = 0u32;
        for (_, group) in &hash_groups {
            if group.len() > 1 {
                // Keep the newest (highest id), soft-delete others
                let newest = group.iter().max_by_key(|o| o.id).unwrap();
                for obs in group {
                    if obs.id != newest.id {
                        self.store.delete_observation(obs.id, false)?;
                        merged += 1;
                        info!("Hash-dedup: merged #{} into #{}", obs.id, newest.id);
                    }
                }
            }
        }
        Ok(merged)
    }

    /// Semantic dedup: cosine similarity > 0.92 via embeddings.
    fn merge_semantic_duplicates(
        &self,
        observations: &[Observation],
        embedder: &Embedder,
    ) -> Result<u32, EngramError> {
        let mut merged = 0u32;
        let mut skip_ids = std::collections::HashSet::new();

        // Embed all observations
        let texts: Vec<&str> = observations.iter().map(|o| o.content.as_str()).collect();
        let embeddings = embedder
            .embed_batch(&texts)
            .map_err(|e| EngramError::Embedding(e.to_string()))?;

        // Find pairs with similarity > 0.92
        for i in 0..observations.len() {
            if skip_ids.contains(&observations[i].id) {
                continue;
            }
            for j in (i + 1)..observations.len() {
                if skip_ids.contains(&observations[j].id) {
                    continue;
                }

                let similarity = Embedder::cosine_similarity(&embeddings[i], &embeddings[j]);

                if similarity > 0.92 {
                    // Merge: keep the one with higher access_count
                    let (keep, remove) =
                        if observations[i].access_count >= observations[j].access_count {
                            (&observations[i], &observations[j])
                        } else {
                            (&observations[j], &observations[i])
                        };

                    // Soft-delete the redundant one
                    self.store.delete_observation(remove.id, false)?;
                    skip_ids.insert(remove.id);
                    merged += 1;

                    info!(
                        "Merged observation #{} into #{} (similarity: {:.3})",
                        remove.id, keep.id, similarity
                    );
                }
            }
        }

        Ok(merged)
    }

    /// Mark observations as obsolete if they have a supersedes edge pointing to them.
    fn mark_obsolete(&self, project: &str) -> Result<u32, EngramError> {
        let observations = self.store.search(&SearchOptions {
            query: String::new(),
            project: Some(project.to_string()),
            limit: Some(1000),
            ..Default::default()
        })?;

        let mut marked = 0u32;

        for obs in &observations {
            let edges = self.store.get_edges(obs.id)?;
            for edge in &edges {
                // If another observation supersedes this one
                if edge.relation == engram_core::RelationType::Supersedes
                    && edge.target_id == obs.id
                {
                    self.store.update_observation(
                        obs.id,
                        &engram_store::UpdateObservationParams {
                            lifecycle_state: Some("stale".into()),
                            ..Default::default()
                        },
                    )?;
                    marked += 1;
                    break;
                }
            }
        }

        Ok(marked)
    }

    /// Find contradictions: same topic_key, different content sentiment.
    fn find_contradictions(&self, project: &str) -> Result<u32, EngramError> {
        let observations = self.store.search(&SearchOptions {
            query: String::new(),
            project: Some(project.to_string()),
            limit: Some(1000),
            ..Default::default()
        })?;

        // Group by topic_key
        let mut by_topic: std::collections::HashMap<String, Vec<&Observation>> =
            std::collections::HashMap::new();
        for obs in &observations {
            if let Some(key) = &obs.topic_key {
                by_topic.entry(key.clone()).or_default().push(obs);
            }
        }

        let mut conflicts = 0u32;

        for (_topic, group) in &by_topic {
            if group.len() < 2 {
                continue;
            }

            // Check for different decision types without supersedes edge
            for i in 0..group.len() {
                for j in (i + 1)..group.len() {
                    if group[i].r#type == ObservationType::Decision
                        && group[j].r#type == ObservationType::Decision
                    {
                        // Check if there's a supersedes relationship
                        let edges = self.store.get_edges(group[i].id)?;
                        let has_supersedes = edges.iter().any(|e| {
                            e.relation == engram_core::RelationType::Supersedes
                                && e.target_id == group[j].id
                        });

                        if !has_supersedes {
                            // Potential conflict - different decisions on same topic
                            conflicts += 1;
                            info!(
                                "Potential conflict: decision #{} vs #{} on topic '{}'",
                                group[i].id, group[j].id, _topic
                            );
                        }
                    }
                }
            }
        }

        Ok(conflicts)
    }

    /// Extract patterns from 3+ similar bugfixes.
    fn extract_patterns(&self, project: &str) -> Result<u32, EngramError> {
        let embedder = match &self.embedder {
            Some(e) => e,
            None => return Ok(0),
        };

        // Get all bugfixes
        let bugfixes = self.store.search(&SearchOptions {
            query: String::new(),
            project: Some(project.to_string()),
            r#type: Some(ObservationType::Bugfix),
            limit: Some(1000),
            ..Default::default()
        })?;

        if bugfixes.len() < 3 {
            return Ok(0);
        }

        // Embed bugfix content
        let texts: Vec<&str> = bugfixes.iter().map(|b| b.content.as_str()).collect();
        let embeddings = embedder
            .embed_batch(&texts)
            .map_err(|e| EngramError::Embedding(e.to_string()))?;

        // Find clusters of 3+ similar bugfixes
        let mut extracted = 0u32;
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

                let similarity = Embedder::cosine_similarity(&embeddings[i], &embeddings[j]);

                if similarity > 0.8 {
                    cluster.push(j);
                }
            }

            if cluster.len() >= 3 {
                // Extract pattern from cluster
                let pattern_content = format!(
                    "Recurring bug pattern ({} occurrences):\n{}",
                    cluster.len(),
                    cluster
                        .iter()
                        .map(|&idx| format!("- {}", bugfixes[idx].title))
                        .collect::<Vec<_>>()
                        .join("\n")
                );

                let params = AddObservationParams {
                    r#type: ObservationType::Pattern,
                    scope: Scope::Project,
                    title: format!("Recurring bug pattern ({} occurrences)", cluster.len()),
                    content: pattern_content,
                    session_id: bugfixes[cluster[0]].session_id.clone(),
                    project: project.to_string(),
                    topic_key: Some(format!(
                        "pattern/recurring-bug-{}",
                        bugfixes[cluster[0]]
                            .topic_key
                            .as_deref()
                            .unwrap_or("unknown")
                    )),
                    provenance_source: Some("inferred".into()),
                    ..Default::default()
                };

                match self.store.insert_observation(&params) {
                    Ok(id) => {
                        info!(
                            "Extracted pattern #{} from {} similar bugfixes",
                            id,
                            cluster.len()
                        );
                        extracted += 1;
                    }
                    Err(EngramError::Duplicate(_)) => {
                        // Pattern already exists
                    }
                    Err(e) => return Err(e),
                }

                for &idx in &cluster {
                    used.insert(bugfixes[idx].id);
                }
            }
        }

        Ok(extracted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engram_store::SqliteStore;

    fn setup_test_store() -> (Arc<SqliteStore>, String) {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let sid = store.create_session("test").unwrap();
        (store, sid)
    }

    #[test]
    fn consolidation_empty_project() {
        let (store, _) = setup_test_store();
        let engine = ConsolidationEngine::new(store, None);
        let result = engine.run_consolidation("test").unwrap();
        assert_eq!(result.duplicates_merged, 0);
        assert_eq!(result.obsolete_marked, 0);
    }

    #[test]
    fn consolidation_marks_obsolete() {
        let (store, sid) = setup_test_store();

        let id1 = store
            .insert_observation(&AddObservationParams {
                r#type: ObservationType::Decision,
                scope: Scope::Project,
                title: "Use Redis".into(),
                content: "For caching".into(),
                session_id: sid.clone(),
                project: "test".into(),
                topic_key: Some("decision/cache".into()),
                ..Default::default()
            })
            .unwrap();

        let id2 = store
            .insert_observation(&AddObservationParams {
                r#type: ObservationType::Decision,
                scope: Scope::Project,
                title: "Use Memcached".into(),
                content: "Better for our use case".into(),
                session_id: sid,
                project: "test".into(),
                topic_key: Some("decision/cache".into()),
                ..Default::default()
            })
            .unwrap();

        // Create supersedes edge: id2 supersedes id1
        store
            .add_edge(&engram_store::AddEdgeParams {
                source_id: id2,
                target_id: id1,
                relation: engram_core::RelationType::Supersedes,
                weight: 1.0,
                auto_detected: false,
            })
            .unwrap();

        let engine = ConsolidationEngine::new(store.clone(), None);
        let result = engine.run_consolidation("test").unwrap();

        assert_eq!(result.obsolete_marked, 1);

        // Check that id1 is now stale
        let obs = store.peek_observation(id1).unwrap().unwrap();
        assert_eq!(obs.lifecycle_state, engram_core::LifecycleState::Stale);
    }

    #[test]
    fn consolidation_finds_conflicts() {
        let (store, sid) = setup_test_store();

        // Two decisions on same topic without supersedes
        store
            .insert_observation(&AddObservationParams {
                r#type: ObservationType::Decision,
                scope: Scope::Project,
                title: "Use Postgres".into(),
                content: "For relational data".into(),
                session_id: sid.clone(),
                project: "test".into(),
                topic_key: Some("decision/database".into()),
                ..Default::default()
            })
            .unwrap();

        store
            .insert_observation(&AddObservationParams {
                r#type: ObservationType::Decision,
                scope: Scope::Project,
                title: "Use MongoDB".into(),
                content: "For document data".into(),
                session_id: sid,
                project: "test".into(),
                topic_key: Some("decision/database".into()),
                ..Default::default()
            })
            .unwrap();

        let engine = ConsolidationEngine::new(store, None);
        let result = engine.run_consolidation("test").unwrap();

        assert_eq!(result.conflicts_found, 1);
    }

    #[test]
    fn consolidation_no_embedder_skips_duplicates() {
        let (store, sid) = setup_test_store();

        store
            .insert_observation(&AddObservationParams {
                r#type: ObservationType::Manual,
                scope: Scope::Project,
                title: "Same".into(),
                content: "Same content".into(),
                session_id: sid.clone(),
                project: "test".into(),
                ..Default::default()
            })
            .unwrap();

        let engine = ConsolidationEngine::new(store, None);
        let result = engine.run_consolidation("test").unwrap();

        // Without embedder, duplicate detection is skipped
        assert_eq!(result.duplicates_merged, 0);
    }

    #[test]
    fn consolidation_result_display() {
        let result = ConsolidationResult {
            duplicates_merged: 5,
            obsolete_marked: 3,
            conflicts_found: 1,
            patterns_extracted: 2,
            time_taken_ms: 150,
        };
        let display = format!("{result}");
        assert!(display.contains("5 duplicates merged"));
        assert!(display.contains("3 obsolete marked"));
    }
}

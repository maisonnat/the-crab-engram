use std::collections::HashMap;
use std::sync::Arc;

use engram_core::{EngramError, ExtractedEntity, MemoryEvent, Observation};
use engram_search::Embedder;
use engram_store::{SearchOptions, Storage};

/// Memory stream that detects events from tool calls and observations.
pub struct MemoryStream {
    pub store: Arc<dyn Storage>,
    pub embedder: Option<Arc<Embedder>>,
}

impl MemoryStream {
    pub fn new(store: Arc<dyn Storage>, embedder: Option<Arc<Embedder>>) -> Self {
        Self { store, embedder }
    }

    /// Detect relevant context for a file being edited.
    pub fn detect_file_context(
        &self,
        project: &str,
        file_path: &str,
    ) -> Result<Vec<MemoryEvent>, EngramError> {
        // Use empty query + content filter (FTS5 can't handle file paths with / and .)
        let all_obs = self.store.search(&SearchOptions {
            query: String::new(),
            project: Some(project.to_string()),
            limit: Some(500),
            ..Default::default()
        })?;

        // Filter by file path in content
        let matching: Vec<&Observation> = all_obs
            .iter()
            .filter(|o| {
                let content = format!("{} {}", o.title, o.content).to_lowercase();
                content.contains(&file_path.to_lowercase())
            })
            .take(5)
            .collect();

        if matching.is_empty() {
            return Ok(vec![]);
        }

        let ids: Vec<i64> = matching.iter().map(|o| o.id).collect();
        Ok(vec![MemoryEvent::RelevantFileContext {
            file_path: file_path.to_string(),
            observation_ids: ids,
        }])
    }

    /// Detect DejaVu: current task matches a previous solution.
    pub fn detect_deja_vu(
        &self,
        project: &str,
        task_description: &str,
    ) -> Result<Vec<MemoryEvent>, EngramError> {
        let embedder = match &self.embedder {
            Some(e) => e,
            None => return Ok(vec![]),
        };

        let task_embedding = embedder
            .embed_one(task_description)
            .map_err(|e| EngramError::Embedding(e.to_string()))?;

        let observations = self.store.search(&SearchOptions {
            query: String::new(),
            project: Some(project.to_string()),
            limit: Some(50),
            ..Default::default()
        })?;

        let texts: Vec<&str> = observations.iter().map(|o| o.content.as_str()).collect();
        let embeddings = embedder
            .embed_batch(&texts)
            .map_err(|e| EngramError::Embedding(e.to_string()))?;

        let mut events = Vec::new();

        for (obs, emb) in observations.iter().zip(embeddings.iter()) {
            let similarity = Embedder::cosine_similarity(&task_embedding, emb);
            if similarity > 0.85 && similarity < 0.99 {
                events.push(MemoryEvent::DejaVu {
                    current_task: task_description.to_string(),
                    previous_observation_id: obs.id,
                    similarity,
                });
            }
        }

        Ok(events)
    }

    /// Detect anti-pattern warnings for current work.
    pub fn detect_anti_pattern_warnings(
        &self,
        project: &str,
        current_content: &str,
    ) -> Result<Vec<MemoryEvent>, EngramError> {
        // Find recurring bugfixes
        let bugfixes = self.store.search(&SearchOptions {
            query: String::new(),
            project: Some(project.to_string()),
            r#type: Some(engram_core::ObservationType::Bugfix),
            limit: Some(100),
            ..Default::default()
        })?;

        let mut events = Vec::new();

        // Check if current content mentions files with recurring bugs
        let mut file_bug_count: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for bug in &bugfixes {
            for word in format!("{} {}", bug.title, bug.content).split_whitespace() {
                if word.contains(".rs") || word.contains(".ts") || word.contains(".go") {
                    *file_bug_count.entry(word.to_lowercase()).or_insert(0) += 1;
                }
            }
        }

        for (file, count) in &file_bug_count {
            if *count >= 3
                && current_content
                    .to_lowercase()
                    .contains(&file.to_lowercase())
            {
                events.push(MemoryEvent::AntiPatternWarning {
                    pattern_description: format!(
                        "Recurring bugs in `{file}` ({count} occurrences)"
                    ),
                    suggestion: "Consider root cause analysis".to_string(),
                });
            }
        }

        Ok(events)
    }

    /// Detect pending reviews for spaced repetition.
    pub fn detect_pending_reviews(&self, project: &str) -> Result<Vec<MemoryEvent>, EngramError> {
        // Check observations that are old and heavily accessed (likely need review)
        let observations = self.store.search(&SearchOptions {
            query: String::new(),
            project: Some(project.to_string()),
            r#type: Some(engram_core::ObservationType::Decision),
            limit: Some(10),
            ..Default::default()
        })?;

        let cutoff = chrono::Utc::now() - chrono::Duration::days(7);
        let mut events = Vec::new();

        for obs in &observations {
            if obs.access_count > 5 && obs.created_at < cutoff {
                events.push(MemoryEvent::ReviewDue {
                    observation_id: obs.id,
                    interval_days: (chrono::Utc::now() - obs.created_at).num_days() as f64,
                });
            }
        }

        Ok(events)
    }

    /// Extract entities from event text using NER heuristics.
    ///
    /// Detects:
    /// - File paths (*.rs, *.ts, *.go, etc.)
    /// - PascalCase identifiers (class/type names)
    /// - snake_case identifiers (function/method names)
    /// - Module paths (crate::path::module)
    pub fn extract_entities(text: &str) -> Vec<ExtractedEntity> {
        let mut entities = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for word in
            text.split(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == '(' || c == ')')
        {
            let word = word.trim();
            if word.is_empty() || word.len() < 3 {
                continue;
            }

            // File paths: contains . and common extensions
            if let Some(ext) = word.rsplit('.').next() {
                if matches!(
                    ext,
                    "rs" | "ts" | "js" | "go" | "py" | "toml" | "json" | "yaml" | "yml" | "md"
                ) && seen.insert(word.to_lowercase())
                {
                    entities.push(ExtractedEntity {
                        name: word.to_string(),
                        entity_type: "file".into(),
                        confidence: 0.9,
                    });
                    continue;
                }
            }

            // Module paths: contains ::
            if word.contains("::") && seen.insert(word.to_lowercase()) {
                entities.push(ExtractedEntity {
                    name: word.to_string(),
                    entity_type: "module".into(),
                    confidence: 0.85,
                });
                continue;
            }

            // PascalCase: starts with uppercase, contains lowercase
            let starts_upper = word.chars().next().map_or(false, |c| c.is_uppercase());
            let has_lower = word.chars().any(|c| c.is_lowercase());
            let has_upper_inner = word.chars().skip(1).any(|c| c.is_uppercase());
            if starts_upper && has_lower && (has_upper_inner || word.len() > 5) {
                let clean: String = word.chars().filter(|c| c.is_alphanumeric()).collect();
                if clean.len() >= 3 && seen.insert(clean.to_lowercase()) {
                    entities.push(ExtractedEntity {
                        name: clean,
                        entity_type: "class".into(),
                        confidence: 0.7,
                    });
                    continue;
                }
            }

            // snake_case: contains _ and lowercase
            if word.contains('_') && !word.starts_with('-') {
                let clean: String = word
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if clean.len() >= 5 && seen.insert(clean.to_lowercase()) {
                    entities.push(ExtractedEntity {
                        name: clean,
                        entity_type: "function".into(),
                        confidence: 0.6,
                    });
                }
            }
        }

        entities
    }

    /// Observe topics from text and calculate entropy per topic.
    ///
    /// Extracts topic keywords from text, counts their frequency,
    /// and computes Shannon entropy for each topic.
    pub fn observe_topics(text: &str) -> HashMap<String, f64> {
        let mut topic_counts: HashMap<String, usize> = HashMap::new();

        // Normalize and count word frequencies
        let words: Vec<&str> = text.split_whitespace().filter(|w| w.len() > 3).collect();

        let total = words.len() as f64;
        if total == 0.0 {
            return HashMap::new();
        }

        for word in &words {
            let normalized = word
                .to_lowercase()
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_string();
            if !normalized.is_empty() {
                *topic_counts.entry(normalized).or_insert(0) += 1;
            }
        }

        // Compute Shannon entropy per topic
        let mut topic_entropy: HashMap<String, f64> = HashMap::new();
        for (topic, count) in &topic_counts {
            let p = (*count as f64) / total;
            let entropy = -p * p.log2();
            topic_entropy.insert(topic.clone(), entropy);
        }

        // Return top 10 by entropy (most informative topics)
        let mut sorted: Vec<(String, f64)> = topic_entropy.into_iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        sorted.truncate(10);
        sorted.into_iter().collect()
    }

    /// Detect entities and topics from event text.
    ///
    /// Combines `extract_entities()` and `observe_topics()` into a single
    /// `EntityExtracted` event.
    pub fn detect_entities(&self, event_text: &str) -> Result<Vec<MemoryEvent>, EngramError> {
        let entities = Self::extract_entities(event_text);
        if entities.is_empty() {
            return Ok(vec![]);
        }

        let topic_entropy = Self::observe_topics(event_text);

        Ok(vec![MemoryEvent::EntityExtracted {
            entities,
            topic_entropy,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engram_core::{ObservationType, Scope};
    use engram_store::SqliteStore;

    fn setup_store() -> Arc<SqliteStore> {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let sid = store.create_session("test").unwrap();

        for i in 0..5 {
            store
                .insert_observation(&engram_store::AddObservationParams {
                    r#type: ObservationType::Bugfix,
                    scope: Scope::Project,
                    title: format!("Bug {i}"),
                    content: format!("Fixed error in src/auth.rs iteration {i}"),
                    session_id: sid.clone(),
                    project: "test".into(),
                    ..Default::default()
                })
                .unwrap();
        }

        store
    }

    #[test]
    fn detect_file_context_empty() {
        let store = setup_store();
        let stream = MemoryStream::new(store, None);
        let events = stream
            .detect_file_context("test", "completely_nonexistent_file.xyz")
            .unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn detect_file_context() {
        let store = setup_store();
        let stream = MemoryStream::new(store, None);
        let events = stream.detect_file_context("test", "src/auth.rs").unwrap();
        assert!(!events.is_empty());
        match &events[0] {
            MemoryEvent::RelevantFileContext {
                file_path,
                observation_ids,
            } => {
                assert_eq!(file_path, "src/auth.rs");
                assert!(!observation_ids.is_empty());
            }
            _ => panic!("expected RelevantFileContext"),
        }
    }

    #[test]
    fn detect_anti_pattern_warning() {
        let store = setup_store();
        let stream = MemoryStream::new(store, None);
        let events = stream
            .detect_anti_pattern_warnings("test", "working on src/auth.rs")
            .unwrap();
        assert!(!events.is_empty());
        match &events[0] {
            MemoryEvent::AntiPatternWarning {
                pattern_description,
                ..
            } => {
                assert!(pattern_description.contains("auth.rs"));
            }
            _ => panic!("expected AntiPatternWarning"),
        }
    }

    #[test]
    fn detect_no_events_for_clean_content() {
        let store = setup_store();
        let stream = MemoryStream::new(store, None);
        let events = stream
            .detect_anti_pattern_warnings("test", "working on unrelated code")
            .unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn detect_pending_reviews() {
        let store = setup_store();
        let stream = MemoryStream::new(store, None);
        let _events = stream.detect_pending_reviews("test").unwrap();
        // May be empty since observations were just created (not 7 days old)
        // This validates the function runs without error
    }

    #[test]
    fn extract_entities_from_code_text() {
        let text = "Fixed bug in src/auth.rs where TokenValidator failed. Changed validate_token function.";
        let entities = MemoryStream::extract_entities(text);

        // Should find file
        assert!(entities
            .iter()
            .any(|e| e.name == "src/auth.rs" && e.entity_type == "file"));
        // Should find class
        assert!(entities
            .iter()
            .any(|e| e.name == "TokenValidator" && e.entity_type == "class"));
        // Should find function
        assert!(entities
            .iter()
            .any(|e| e.name == "validate_token" && e.entity_type == "function"));
    }

    #[test]
    fn extract_entities_no_duplicates() {
        let text = "src/auth.rs src/auth.rs TokenValidator TokenValidator";
        let entities = MemoryStream::extract_entities(text);
        let file_entities: Vec<_> = entities
            .iter()
            .filter(|e| e.entity_type == "file")
            .collect();
        assert_eq!(file_entities.len(), 1);
    }

    #[test]
    fn extract_entities_module_path() {
        let text = "Error at engram_core::stream::MemoryEvent";
        let entities = MemoryStream::extract_entities(text);
        assert!(entities.iter().any(|e| e.entity_type == "module"));
    }

    #[test]
    fn extract_entities_empty_text() {
        let entities = MemoryStream::extract_entities("");
        assert!(entities.is_empty());
    }

    #[test]
    fn observe_topics_basic() {
        let text = "auth validation error error error token validation auth";
        let topics = MemoryStream::observe_topics(text);
        // "error" appears 3 times, "validation" 2 times, "auth" 2 times
        assert!(topics.contains_key("error"));
        assert!(topics.contains_key("auth"));
        assert!(topics.contains_key("validation"));
    }

    #[test]
    fn observe_topics_empty() {
        let topics = MemoryStream::observe_topics("");
        assert!(topics.is_empty());
    }

    #[test]
    fn detect_entities_produces_event() {
        let store = setup_store();
        let stream = MemoryStream::new(store, None);
        let events = stream
            .detect_entities("Fixed bug in src/auth.rs using TokenValidator")
            .unwrap();
        assert!(!events.is_empty());
        match &events[0] {
            MemoryEvent::EntityExtracted {
                entities,
                topic_entropy,
            } => {
                assert!(!entities.is_empty());
                assert!(!topic_entropy.is_empty());
            }
            _ => panic!("expected EntityExtracted"),
        }
    }

    #[test]
    fn detect_entities_empty_text() {
        let store = setup_store();
        let stream = MemoryStream::new(store, None);
        let events = stream.detect_entities("ok").unwrap();
        assert!(events.is_empty());
    }
}

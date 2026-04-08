use std::sync::Arc;

use engram_core::{compaction::determine_level, ConfidenceLevel, EngramError, Observation};
use engram_store::{SearchOptions, Storage};

/// Context built for injection into agent sessions.
#[derive(Debug, Clone)]
pub struct InjectionContext {
    pub relevant_memories: Vec<Observation>,
    pub knowledge_capsules: Vec<String>, // Formatted capsule summaries
    pub warnings: Vec<String>,           // Anti-pattern warnings
    pub knowledge_boundaries: Vec<String>, // Domain confidence
    pub review_reminders: Vec<String>,   // Spaced repetition pending
    pub total_tokens: usize,
}

impl Default for InjectionContext {
    fn default() -> Self {
        Self {
            relevant_memories: Vec::new(),
            knowledge_capsules: Vec::new(),
            warnings: Vec::new(),
            knowledge_boundaries: Vec::new(),
            review_reminders: Vec::new(),
            total_tokens: 0,
        }
    }
}

impl InjectionContext {
    pub fn is_empty(&self) -> bool {
        self.relevant_memories.is_empty()
            && self.knowledge_capsules.is_empty()
            && self.warnings.is_empty()
            && self.knowledge_boundaries.is_empty()
            && self.review_reminders.is_empty()
    }

    /// Format as Markdown for system context injection.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        if self.is_empty() {
            return md;
        }

        md.push_str("## Context for current task\n\n");

        if !self.warnings.is_empty() {
            md.push_str("### ⚠️ Active warnings\n\n");
            for w in &self.warnings {
                md.push_str(&format!("- {w}\n"));
            }
            md.push('\n');
        }

        if !self.knowledge_boundaries.is_empty() {
            md.push_str("### 🧠 Knowledge boundaries\n\n");
            for b in &self.knowledge_boundaries {
                md.push_str(&format!("- {b}\n"));
            }
            md.push('\n');
        }

        if !self.knowledge_capsules.is_empty() {
            md.push_str("### 📌 Relevant knowledge\n\n");
            for c in &self.knowledge_capsules {
                md.push_str(&format!("{c}\n"));
            }
            md.push('\n');
        }

        if !self.relevant_memories.is_empty() {
            md.push_str("### 📚 Relevant memories\n\n");
            for obs in &self.relevant_memories {
                md.push_str(&format!(
                    "- **#{}** [{}] {} — {}\n",
                    obs.id,
                    obs.r#type,
                    obs.title,
                    obs.content.chars().take(120).collect::<String>()
                ));
            }
            md.push('\n');
        }

        if !self.review_reminders.is_empty() {
            md.push_str("### 🔄 Review reminders\n\n");
            for r in &self.review_reminders {
                md.push_str(&format!("- {r}\n"));
            }
            md.push('\n');
        }

        md
    }

    /// Approximate token count (1 token ≈ 4 chars).
    pub fn estimate_tokens(&mut self) {
        self.total_tokens = self.to_markdown().len() / 4;
    }
}

/// Builds smart context for injection into agent sessions.
pub struct SmartInjector {
    pub store: Arc<dyn Storage>,
}

impl SmartInjector {
    pub fn new(store: Arc<dyn Storage>) -> Self {
        Self { store }
    }

    /// Build context for a task with token budget.
    pub fn build_context(
        &self,
        project: &str,
        task: &str,
        max_tokens: usize,
    ) -> Result<InjectionContext, EngramError> {
        let mut ctx = InjectionContext::default();
        let _max_chars = max_tokens * 4;

        // Determine abstraction level from task query
        let level = determine_level(task);
        let limit = match level {
            engram_core::CompactionLevel::Raw => 10, // Raw: maximum details
            engram_core::CompactionLevel::Fact => 8, // Facts: more details
            engram_core::CompactionLevel::Pattern => 5, // Patterns: balanced
            engram_core::CompactionLevel::Principle => 3, // Principles: fewer, high-level
        };

        // 1. Find relevant memories
        let memories = self.store.search(&SearchOptions {
            query: task.to_string(),
            project: Some(project.to_string()),
            limit: Some(limit),
            ..Default::default()
        })?;
        ctx.relevant_memories = memories;

        // 2. Find active anti-patterns → warnings
        let bugfixes = self.store.search(&SearchOptions {
            query: String::new(),
            project: Some(project.to_string()),
            r#type: Some(engram_core::ObservationType::Bugfix),
            limit: Some(100),
            ..Default::default()
        })?;

        // Simple heuristic: if 3+ bugfixes mention the same file → warning
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
            if *count >= 3 {
                ctx.warnings.push(format!(
                    "⚠️ Recurring bugs in `{file}` ({count} occurrences) — consider root cause analysis"
                ));
            }
        }

        // 3. Knowledge boundaries from domain counts
        let all_obs = self.store.search(&SearchOptions {
            query: String::new(),
            project: Some(project.to_string()),
            limit: Some(1000),
            ..Default::default()
        })?;

        let mut domain_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for obs in &all_obs {
            if let Some(key) = &obs.topic_key {
                if let Some(domain) = key.split('/').next() {
                    *domain_counts.entry(domain.to_string()).or_insert(0) += 1;
                }
            }
        }
        for (domain, count) in &domain_counts {
            let level = ConfidenceLevel::from_count(*count);
            if level <= ConfidenceLevel::Aware {
                ctx.knowledge_boundaries
                    .push(format!("🟠 {}: {} — limited experience", domain, level));
            }
        }

        ctx.estimate_tokens();

        // Trim if over budget
        while ctx.total_tokens > max_tokens / 4 && ctx.relevant_memories.len() > 1 {
            ctx.relevant_memories.pop();
            ctx.estimate_tokens();
        }

        Ok(ctx)
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

        // Add various observations
        for i in 0..5 {
            store
                .insert_observation(&engram_store::AddObservationParams {
                    r#type: ObservationType::Bugfix,
                    scope: Scope::Project,
                    title: format!("Bug {i}"),
                    content: format!("Fixed error in src/auth.rs iteration {i}"),
                    session_id: sid.clone(),
                    project: "test".into(),
                    topic_key: Some(format!("bug/auth-{i}")),
                    ..Default::default()
                })
                .unwrap();
        }

        store
            .insert_observation(&engram_store::AddObservationParams {
                r#type: ObservationType::Decision,
                scope: Scope::Project,
                title: "Use JWT auth".into(),
                content: "RS256 with 15min expiry for security".into(),
                session_id: sid,
                project: "test".into(),
                topic_key: Some("decision/auth".into()),
                ..Default::default()
            })
            .unwrap();

        store
    }

    #[test]
    fn injection_context_empty() {
        let ctx = InjectionContext::default();
        assert!(ctx.is_empty());
        assert!(ctx.to_markdown().is_empty());
    }

    #[test]
    fn injection_context_with_content() {
        let mut ctx = InjectionContext::default();
        ctx.warnings.push("⚠️ test warning".into());
        ctx.knowledge_boundaries.push("rust: Expert".into());

        let md = ctx.to_markdown();
        assert!(md.contains("Active warnings"));
        assert!(md.contains("Knowledge boundaries"));
        assert!(md.contains("rust: Expert"));
    }

    #[test]
    fn smart_injector_builds_context() {
        let store = setup_store();
        let injector = SmartInjector::new(store);
        let ctx = injector
            .build_context("test", "auth JWT token security", 2000)
            .unwrap();

        // Should find relevant memories or warnings or boundaries
        assert!(
            !ctx.relevant_memories.is_empty()
                || !ctx.warnings.is_empty()
                || !ctx.knowledge_boundaries.is_empty(),
            "context should have some content"
        );
    }

    #[test]
    fn token_estimate() {
        let mut ctx = InjectionContext::default();
        ctx.relevant_memories.push(Observation::new(
            ObservationType::Manual,
            Scope::Project,
            "Test".into(),
            "Some content here".into(),
            "sid".into(),
            "proj".into(),
            None,
        ));
        ctx.estimate_tokens();
        assert!(ctx.total_tokens > 0);
    }
}

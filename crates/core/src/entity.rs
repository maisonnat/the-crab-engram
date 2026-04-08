use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Entity types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Person,
    Vendor,
    Project,
    File,
    Concept,
    Tool,
    Config,
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Person => write!(f, "person"),
            Self::Vendor => write!(f, "vendor"),
            Self::Project => write!(f, "project"),
            Self::File => write!(f, "file"),
            Self::Concept => write!(f, "concept"),
            Self::Tool => write!(f, "tool"),
            Self::Config => write!(f, "config"),
        }
    }
}

/// An entity — resolves different textual references to the same thing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: i64,
    pub canonical_name: String,
    pub aliases: Vec<String>,
    pub entity_type: EntityType,
    pub properties: HashMap<String, String>,
    pub observation_ids: Vec<i64>,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

impl Entity {
    pub fn new(canonical_name: String, entity_type: EntityType) -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            canonical_name,
            aliases: Vec::new(),
            entity_type,
            properties: HashMap::new(),
            observation_ids: Vec::new(),
            first_seen: now,
            last_seen: now,
        }
    }

    /// Check if a text reference matches this entity.
    pub fn matches(&self, text: &str) -> bool {
        let lower = text.to_lowercase();
        let canonical = self.canonical_name.to_lowercase();

        lower.contains(&canonical)
            || canonical.contains(&lower)
            || self.aliases.iter().any(|a| {
                let a_lower = a.to_lowercase();
                lower.contains(&a_lower) || a_lower.contains(&lower)
            })
    }

    /// Add an alias.
    pub fn add_alias(&mut self, alias: String) {
        if !self.aliases.contains(&alias)
            && alias.to_lowercase() != self.canonical_name.to_lowercase()
        {
            self.aliases.push(alias);
        }
    }
}

/// Simple NER heuristic to extract entities from text.
pub fn extract_entities(text: &str) -> Vec<(String, EntityType)> {
    let mut entities = Vec::new();

    for word in text.split_whitespace() {
        let clean: String = word
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '/' || *c == '\\' || *c == '.')
            .collect();

        // File paths
        if (clean.contains('/') || clean.contains('\\')) && has_file_extension(&clean) {
            entities.push((clean.clone(), EntityType::File));
            continue;
        }

        // PascalCase → likely Person/Project/Concept
        if is_pascal_case(&clean) && clean.len() > 2 {
            entities.push((clean.clone(), EntityType::Concept));
        }
    }

    entities
}

fn has_file_extension(s: &str) -> bool {
    [".rs", ".ts", ".go", ".py", ".js", ".toml", ".json", ".yaml"]
        .iter()
        .any(|ext| s.ends_with(ext))
}

fn is_pascal_case(s: &str) -> bool {
    s.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
        && s.chars().any(|c| c.is_lowercase())
        && !s.contains(' ')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_matches_canonical() {
        let e = Entity::new("Alice".into(), EntityType::Person);
        assert!(e.matches("Alice approved the PR"));
    }

    #[test]
    fn entity_matches_alias() {
        let mut e = Entity::new("Alice".into(), EntityType::Person);
        e.add_alias("our CTO".into());
        assert!(e.matches("our CTO approved the PR"));
    }

    #[test]
    fn entity_no_match() {
        let e = Entity::new("Alice".into(), EntityType::Person);
        assert!(!e.matches("Bob did something"));
    }

    #[test]
    fn extract_file_entities() {
        let entities = extract_entities("Changed src/auth.rs");
        let files: Vec<_> = entities
            .iter()
            .filter(|(_, t)| *t == EntityType::File)
            .collect();
        assert!(!files.is_empty(), "should find at least one file entity");
    }

    #[test]
    fn extract_pascal_case() {
        let entities = extract_entities("Using TextEmbedding from fastembed");
        assert!(
            entities
                .iter()
                .any(|(name, _)| name.contains("TextEmbedding"))
        );
    }

    #[test]
    fn add_alias_no_duplicates() {
        let mut e = Entity::new("Alice".into(), EntityType::Person);
        e.add_alias("our CTO".into());
        e.add_alias("our CTO".into()); // duplicate
        assert_eq!(e.aliases.len(), 1);
    }

    #[test]
    fn add_alias_skips_canonical() {
        let mut e = Entity::new("Alice".into(), EntityType::Person);
        e.add_alias("alice".into()); // same as canonical (case-insensitive)
        assert!(e.aliases.is_empty());
    }
}

//! Extraction pipeline — structured knowledge extraction from LLM output.
//!
//! Implements a self-healing loop:
//! 1. LLM inference guided by GBNF grammar → raw text
//! 2. `serde_json::from_str` → typed extraction
//! 3. If parse fails → re-prompt with error context (max 2 retries)
//!
//! Includes semantic validation: weight ranges, relation type consistency,
//! and observation type validity.

use serde::{Deserialize, Serialize};

use crate::inference::InferenceEngine;

/// Maximum self-healing retries before giving up.
const MAX_RETRIES: usize = 2;

/// Grammar file embedded at compile time.
const GBNF_GRAMMAR: &str = include_str!("../resources/kg_extraction.gbnf");

/// A single extracted observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedObservation {
    pub r#type: String,
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub topic_key: Option<String>,
}

/// A single extracted edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEdge {
    pub source_id: i64,
    pub target_id: i64,
    pub relation: String,
    pub weight: f64,
}

/// The full extraction result matching the GBNF grammar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeExtraction {
    pub title: String,
    pub observations: Vec<ExtractedObservation>,
    pub edges: Vec<ExtractedEdge>,
}

/// Validation error for extracted data.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    WeightOutOfRange { index: usize, value: f64 },
    InvalidRelationType { index: usize, relation: String },
    InvalidObservationType { index: usize, obs_type: String },
    InconsistentNodeIds { source: i64, target: i64, max_obs: usize },
}

/// Result of the extraction pipeline.
#[derive(Debug)]
pub struct ExtractionResult {
    pub extraction: KnowledgeExtraction,
    pub retries_used: usize,
    pub validation_errors: Vec<ValidationError>,
}

/// The extraction pipeline with self-healing loop.
pub struct ExtractionPipeline {
    engine: InferenceEngine,
}

impl ExtractionPipeline {
    /// Create a new pipeline backed by the given inference engine.
    pub fn new(engine: InferenceEngine) -> Self {
        Self { engine }
    }

    /// Extract structured knowledge from raw text.
    /// Runs inference → parse → validate, with self-healing retries.
    pub fn extract(&self, input: &str) -> Result<ExtractionResult, ExtractionError> {
        let mut prompt = build_extraction_prompt(input);
        let mut retries = 0;

        loop {
            // Step 1: Inference
            let raw = self
                .engine
                .infer(&prompt)
                .map_err(|e| ExtractionError::InferenceFailed(e.to_string()))?;

            // Step 2: Parse JSON
            match serde_json::from_str::<KnowledgeExtraction>(&clean_json_output(&raw)) {
                Ok(extraction) => {
                    // Step 3: Validate
                    let errors = validate_extraction(&extraction);
                    if errors.is_empty() || retries >= MAX_RETRIES {
                        return Ok(ExtractionResult {
                            extraction,
                            retries_used: retries,
                            validation_errors: errors,
                        });
                    }
                    // If validation errors, retry with error context
                    prompt = build_retry_prompt(input, &format!("{errors:?}"));
                    retries += 1;
                }
                Err(parse_err) => {
                    if retries >= MAX_RETRIES {
                        return Err(ExtractionError::ParseFailed(parse_err.to_string()));
                    }
                    // Self-healing: re-prompt with the parse error
                    prompt = build_retry_prompt(input, &parse_err.to_string());
                    retries += 1;
                }
            }
        }
    }

    /// Get the GBNF grammar string (for use with grammar-based sampling).
    pub fn grammar(&self) -> &'static str {
        GBNF_GRAMMAR
    }
}

/// Errors that can occur during extraction.
#[derive(Debug)]
pub enum ExtractionError {
    InferenceFailed(String),
    ParseFailed(String),
}

/// Validate extracted data for semantic consistency.
fn validate_extraction(extraction: &KnowledgeExtraction) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let valid_relations = [
        "caused_by",
        "related_to",
        "supersedes",
        "blocks",
        "part_of",
    ];
    let valid_types = [
        "bugfix",
        "decision",
        "architecture",
        "pattern",
        "discovery",
        "learning",
        "config",
        "convention",
        "tool_use",
        "file_change",
        "command",
        "file_read",
        "search",
        "manual",
    ];

    // Validate observation types
    for (i, obs) in extraction.observations.iter().enumerate() {
        if !valid_types.contains(&obs.r#type.as_str()) {
            errors.push(ValidationError::InvalidObservationType {
                index: i,
                obs_type: obs.r#type.clone(),
            });
        }
    }

    // Validate edges
    let obs_count = extraction.observations.len() as i64;
    for (i, edge) in extraction.edges.iter().enumerate() {
        // Weight range
        if !(0.0..=1.0).contains(&edge.weight) {
            errors.push(ValidationError::WeightOutOfRange {
                index: i,
                value: edge.weight,
            });
        }

        // Relation type
        if !valid_relations.contains(&edge.relation.as_str()) {
            errors.push(ValidationError::InvalidRelationType {
                index: i,
                relation: edge.relation.clone(),
            });
        }

        // Node IDs consistency (must reference valid observations, 1-indexed)
        if edge.source_id < 1 || edge.source_id > obs_count {
            errors.push(ValidationError::InconsistentNodeIds {
                source: edge.source_id,
                target: edge.target_id,
                max_obs: extraction.observations.len(),
            });
        }
        if edge.target_id < 1 || edge.target_id > obs_count {
            errors.push(ValidationError::InconsistentNodeIds {
                source: edge.source_id,
                target: edge.target_id,
                max_obs: extraction.observations.len(),
            });
        }
    }

    errors
}

/// Build the initial extraction prompt.
fn build_extraction_prompt(input: &str) -> String {
    format!(
        "Extract structured knowledge from the following text as JSON.\n\
         Follow the schema: {{\"title\": string, \"observations\": [{{\"type\": string, \"title\": string, \"content\": string, \"topic_key\": string|null}}], \"edges\": [{{\"source_id\": int, \"target_id\": int, \"relation\": string, \"weight\": float}}]}}\n\
         Valid observation types: bugfix, decision, architecture, pattern, discovery, learning, config, convention, tool_use, file_change, command, file_read, search, manual\n\
         Valid relation types: caused_by, related_to, supersedes, blocks, part_of\n\
         Weight must be between 0.0 and 1.0.\n\n\
         Text:\n{input}\n\n\
         Output only valid JSON:"
    )
}

/// Build a retry prompt injecting the previous error.
fn build_retry_prompt(input: &str, error: &str) -> String {
    format!(
        "The previous extraction attempt produced an error. Fix it.\n\n\
         Error: {error}\n\n\
         Original text:\n{input}\n\n\
         Output only valid JSON matching the schema:"
    )
}

/// Clean up LLM output — extract JSON from markdown code blocks, trim whitespace.
fn clean_json_output(raw: &str) -> String {
    let trimmed = raw.trim();

    // If wrapped in ```json ... ```, extract just the JSON
    if let Some(start) = trimmed.find("```json") {
        if let Some(end) = trimmed.rfind("```") {
            let json_start = start + 7; // len of "```json"
            if json_start < end {
                return trimmed[json_start..end].trim().to_string();
            }
        }
    }

    // If wrapped in ``` ... ```
    if let Some(start) = trimmed.find("```") {
        if let Some(end) = trimmed.rfind("```") {
            let json_start = start + 3;
            if json_start < end {
                return trimmed[json_start..end].trim().to_string();
            }
        }
    }

    // Try to find JSON object bounds
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if start < end {
                return trimmed[start..=end].to_string();
            }
        }
    }

    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn clean_json_from_markdown() {
        let input = "```json\n{\"title\": \"test\", \"observations\": [], \"edges\": []}\n```";
        let cleaned = clean_json_output(input);
        assert!(cleaned.starts_with('{'));
        assert!(cleaned.ends_with('}'));
    }

    #[test]
    fn clean_json_bare() {
        let input = r#"{"title": "test", "observations": [], "edges": []}"#;
        let cleaned = clean_json_output(input);
        assert_eq!(cleaned, input);
    }

    #[test]
    fn clean_json_with_surrounding_text() {
        let input = "Here is the result:\n{\"title\": \"test\", \"observations\": [], \"edges\": []}\nDone.";
        let cleaned = clean_json_output(input);
        assert!(cleaned.starts_with('{'));
        assert!(cleaned.ends_with('}'));
    }

    #[test]
    fn validate_good_extraction() {
        let extraction = KnowledgeExtraction {
            title: "test".into(),
            observations: vec![ExtractedObservation {
                r#type: "decision".into(),
                title: "Use Rust".into(),
                content: "Chose Rust for performance".into(),
                topic_key: None,
            }],
            edges: vec![ExtractedEdge {
                source_id: 1,
                target_id: 1,
                relation: "related_to".into(),
                weight: 0.8,
            }],
        };
        let errors = validate_extraction(&extraction);
        assert!(errors.is_empty());
    }

    #[test]
    fn validate_bad_weight() {
        let extraction = KnowledgeExtraction {
            title: "test".into(),
            observations: vec![],
            edges: vec![ExtractedEdge {
                source_id: 1,
                target_id: 2,
                relation: "related_to".into(),
                weight: 1.5, // out of range
            }],
        };
        let errors = validate_extraction(&extraction);
        assert!(errors.iter().any(|e| matches!(e, ValidationError::WeightOutOfRange { .. })));
    }

    #[test]
    fn validate_bad_relation() {
        let extraction = KnowledgeExtraction {
            title: "test".into(),
            observations: vec![],
            edges: vec![ExtractedEdge {
                source_id: 1,
                target_id: 2,
                relation: "invalid_relation".into(),
                weight: 0.5,
            }],
        };
        let errors = validate_extraction(&extraction);
        assert!(errors.iter().any(|e| matches!(e, ValidationError::InvalidRelationType { .. })));
    }

    #[test]
    fn validate_bad_obs_type() {
        let extraction = KnowledgeExtraction {
            title: "test".into(),
            observations: vec![ExtractedObservation {
                r#type: "nonexistent".into(),
                title: "Test".into(),
                content: "Content".into(),
                topic_key: None,
            }],
            edges: vec![],
        };
        let errors = validate_extraction(&extraction);
        assert!(errors.iter().any(|e| matches!(e, ValidationError::InvalidObservationType { .. })));
    }

    #[test]
    fn validate_inconsistent_node_ids() {
        let extraction = KnowledgeExtraction {
            title: "test".into(),
            observations: vec![ExtractedObservation {
                r#type: "decision".into(),
                title: "A".into(),
                content: "B".into(),
                topic_key: None,
            }],
            edges: vec![ExtractedEdge {
                source_id: 1,
                target_id: 5, // only 1 observation, so 5 is invalid
                relation: "related_to".into(),
                weight: 0.5,
            }],
        };
        let errors = validate_extraction(&extraction);
        assert!(errors.iter().any(|e| matches!(e, ValidationError::InconsistentNodeIds { .. })));
    }

    #[test]
    fn grammar_embedded() {
        assert!(!GBNF_GRAMMAR.is_empty());
        assert!(GBNF_GRAMMAR.contains("knowledge-capsule"));
        assert!(GBNF_GRAMMAR.contains("obs-type"));
        assert!(GBNF_GRAMMAR.contains("relation-type"));
    }

    #[test]
    fn extraction_pipeline_creates() {
        let engine = InferenceEngine::new(PathBuf::from("/tmp/test.gguf"));
        let pipeline = ExtractionPipeline::new(engine);
        assert!(!pipeline.grammar().is_empty());
    }

    #[test]
    fn parse_valid_json() {
        let json = r#"{"title":"test","observations":[{"type":"decision","title":"Use Rust","content":"Chose Rust","topic_key":null}],"edges":[{"source_id":1,"target_id":1,"relation":"related_to","weight":0.8}]}"#;
        let parsed: KnowledgeExtraction = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.title, "test");
        assert_eq!(parsed.observations.len(), 1);
        assert_eq!(parsed.edges.len(), 1);
    }

    #[test]
    fn parse_fails_on_invalid_json() {
        let json = r#"{"title": invalid}"#;
        let result = serde_json::from_str::<KnowledgeExtraction>(json);
        assert!(result.is_err());
    }
}

use std::sync::Mutex;

use anyhow::Result;
use chrono::{DateTime, Utc};
use engram_core::Attachment;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use tracing::{info, warn};

/// Metadata about the embedding model used.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmbeddingMeta {
    pub model_name: String,
    pub model_version: String,
    pub dimensions: usize,
    pub created_at: DateTime<Utc>,
}

impl Default for EmbeddingMeta {
    fn default() -> Self {
        Self {
            model_name: "all-MiniLM-L6-v2".into(),
            model_version: "v2".into(),
            dimensions: 384,
            created_at: Utc::now(),
        }
    }
}

/// Result of hydrating embeddings for an observation.
#[derive(Debug, Clone)]
pub struct HydratedEmbedding {
    pub observation_embedding: Vec<f32>,
    pub attachment_embeddings: Vec<Vec<f32>>,
    /// 1-bit binary hash of the observation embedding (384 bits → 48 bytes).
    pub binary_hash: Vec<u8>,
    pub confidence: f64,
    pub updated_at: DateTime<Utc>,
}

/// Compute confidence based on text length.
///
/// Longer texts produce more reliable embeddings:
/// - < 10 chars: 0.3 (very low)
/// - 10-50 chars: 0.5
/// - 50-200 chars: 0.7
/// - 200-500 chars: 0.85
/// - 500+ chars: 0.95
pub fn confidence_from_text_length(text_len: usize) -> f64 {
    match text_len {
        0..=9 => 0.3,
        10..=49 => 0.5,
        50..=199 => 0.7,
        200..=499 => 0.85,
        _ => 0.95,
    }
}

/// Local embedder using fastembed (all-MiniLM-L6-v2, 384d).
///
/// Features:
/// - Model versioning (detect drift when model changes)
/// - spawn_blocking for CPU-bound work
/// - Fallback warning when mix of models detected
pub struct Embedder {
    model: Mutex<TextEmbedding>,
    meta: EmbeddingMeta,
}

impl Embedder {
    /// Create a new embedder. Downloads model on first use (~80MB).
    pub fn new() -> Result<Self> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
        )?;

        info!("Embedder initialized: all-MiniLM-L6-v2 (384d)");

        Ok(Self {
            model: Mutex::new(model),
            meta: EmbeddingMeta::default(),
        })
    }

    /// Get the model metadata (for storing alongside embeddings).
    pub fn meta(&self) -> &EmbeddingMeta {
        &self.meta
    }

    /// Embed a single text. Returns 384-dimensional vector.
    pub fn embed_one(&self, text: &str) -> Result<Vec<f32>> {
        let mut model = self.model.lock().unwrap();
        let embeddings = model.embed(vec![text], None)?;
        Ok(embeddings.into_iter().next().unwrap_or_default())
    }

    /// Embed multiple texts in batch.
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut model = self.model.lock().unwrap();
        let embeddings = model.embed(texts, None)?;
        Ok(embeddings)
    }

    /// Hyrate embeddings for an observation with its attachments.
    ///
    /// Produces embeddings for both the observation text and each attachment's
    /// `embeddable_text()`. Confidence is calculated from text length — longer
    /// texts produce more reliable embeddings.
    pub fn hydrate_embeddings_enhanced(
        &self,
        observation_text: &str,
        attachments: &[Attachment],
    ) -> Result<HydratedEmbedding> {
        let mut all_texts: Vec<&str> = vec![observation_text];

        let attachment_embeddable: Vec<String> =
            attachments.iter().map(|a| a.embeddable_text()).collect();
        for text in &attachment_embeddable {
            all_texts.push(text.as_str());
        }

        let embeddings = self.embed_batch(&all_texts)?;

        let observation_embedding = embeddings.first().cloned().unwrap_or_default();
        let attachment_embeddings: Vec<Vec<f32>> = embeddings.into_iter().skip(1).collect();

        // Confidence based on combined text length
        let total_text_len: usize =
            observation_text.len() + attachment_embeddable.iter().map(|t| t.len()).sum::<usize>();
        let confidence = confidence_from_text_length(total_text_len);
        let binary_hash = binary_quantize(&observation_embedding);

        Ok(HydratedEmbedding {
            observation_embedding,
            attachment_embeddings,
            binary_hash,
            confidence,
            updated_at: Utc::now(),
        })
    }

    /// Cosine similarity between two vectors.
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let mut dot = 0.0f64;
        let mut norm_a = 0.0f64;
        let mut norm_b = 0.0f64;

        for (va, vb) in a.iter().zip(b.iter()) {
            dot += (*va as f64) * (*vb as f64);
            norm_a += (*va as f64) * (*va as f64);
            norm_b += (*vb as f64) * (*vb as f64);
        }

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot / (norm_a.sqrt() * norm_b.sqrt())
    }

    /// Check if embeddings in the DB match the current model.
    /// Returns count of stale embeddings.
    pub fn detect_drift(&self, stale_count: usize) {
        if stale_count > 0 {
            warn!(
                "Found {} embeddings from different model ({}). \
                 Search quality degraded. Run `the-crab-engram reembed` to fix.",
                stale_count, self.meta.model_name
            );
        }
    }
}

impl std::fmt::Debug for Embedder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Embedder")
            .field("meta", &self.meta)
            .finish()
    }
}

/// Binary quantization: convert f32 embedding to 1-bit packed representation.
/// Each dimension: (val > 0.0) ? 1 : 0, packed 8 bits per byte.
/// For 384 dimensions → 48 bytes.
///
/// Reference: HuggingFace Research — Binary and Scalar Embedding Quantization
pub fn binary_quantize(vec: &[f32]) -> Vec<u8> {
    let byte_count = vec.len().div_ceil(8);
    let mut packed = vec![0u8; byte_count];

    for (i, &val) in vec.iter().enumerate() {
        if val > 0.0 {
            packed[i / 8] |= 1 << (i % 8);
        }
    }

    packed
}

/// Hamming distance between two binary-packed vectors.
/// Returns the number of differing bits.
pub fn hamming_distance(a: &[u8], b: &[u8]) -> u32 {
    let min_len = a.len().min(b.len());
    let mut dist = 0u32;

    for i in 0..min_len {
        // XOR to find differing bits, then count them
        dist += (a[i] ^ b[i]).count_ones();
    }

    // Extra bytes in the longer vector count as all-different
    let extra = a.len().abs_diff(b.len());
    dist += (extra as u32) * 8;

    dist
}

/// Hamming similarity (normalized): 1.0 = identical, 0.0 = completely different.
pub fn hamming_similarity(a: &[u8], b: &[u8]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let total_bits = a.len().max(b.len()) as f64 * 8.0;
    if total_bits == 0.0 {
        return 1.0;
    }
    1.0 - (hamming_distance(a, b) as f64 / total_bits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = Embedder::cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = Embedder::cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_opposite() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        let sim = Embedder::cosine_similarity(&a, &b);
        assert!((sim - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_empty() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        let sim = Embedder::cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn embedding_meta_defaults() {
        let meta = EmbeddingMeta::default();
        assert_eq!(meta.model_name, "all-MiniLM-L6-v2");
        assert_eq!(meta.dimensions, 384);
        assert_eq!(meta.model_version, "v2");
    }

    // Note: actual embedding tests require model download (~80MB)
    // Run with: cargo test -p engram-search -- --ignored
    #[test]
    #[ignore]
    fn embed_one_produces_384d_vector() {
        let embedder = Embedder::new().unwrap();
        let vec = embedder.embed_one("hello world").unwrap();
        assert_eq!(vec.len(), 384);
    }

    #[test]
    #[ignore]
    fn semantic_similarity() {
        let embedder = Embedder::new().unwrap();
        let v1 = embedder.embed_one("N+1 query performance issue").unwrap();
        let v2 = embedder
            .embed_one("database performance optimization")
            .unwrap();
        let v3 = embedder.embed_one("the weather is nice today").unwrap();

        let sim_related = Embedder::cosine_similarity(&v1, &v2);
        let sim_unrelated = Embedder::cosine_similarity(&v1, &v3);

        assert!(
            sim_related > sim_unrelated,
            "related ({sim_related}) should be > unrelated ({sim_unrelated})"
        );
    }

    #[test]
    #[ignore]
    fn hydrate_embeddings_with_attachments() {
        let embedder = Embedder::new().unwrap();
        let attachments = vec![
            Attachment::CodeDiff {
                file_path: "src/auth.rs".into(),
                before_hash: "aaa".into(),
                after_hash: "bbb".into(),
                diff: "+fn validate() {}".into(),
            },
            Attachment::ErrorTrace {
                error_type: "panic".into(),
                message: "index out of bounds".into(),
                stack_trace: "at main.rs:42".into(),
                file_line: Some(("main.rs".into(), 42)),
            },
        ];

        let result = embedder
            .hydrate_embeddings_enhanced("Fixed auth bug", &attachments)
            .unwrap();

        assert_eq!(result.observation_embedding.len(), 384);
        assert_eq!(result.attachment_embeddings.len(), 2);
        assert_eq!(result.attachment_embeddings[0].len(), 384);
        assert!(result.confidence > 0.0);
        assert!(result.updated_at <= chrono::Utc::now());
    }

    #[test]
    fn confidence_from_text_length_short() {
        assert_eq!(confidence_from_text_length(5), 0.3);
    }

    #[test]
    fn confidence_from_text_length_medium() {
        assert_eq!(confidence_from_text_length(30), 0.5);
    }

    #[test]
    fn confidence_from_text_length_long() {
        assert_eq!(confidence_from_text_length(100), 0.7);
    }

    #[test]
    fn confidence_from_text_length_very_long() {
        assert_eq!(confidence_from_text_length(300), 0.85);
    }

    #[test]
    fn confidence_from_text_length_max() {
        assert_eq!(confidence_from_text_length(1000), 0.95);
    }

    #[test]
    fn binary_quantize_all_positive() {
        let vec = vec![1.0, 2.0, 3.0, 0.5, 0.1, 5.0, 1.0, 1.0]; // 8 values, all > 0
        let packed = binary_quantize(&vec);
        assert_eq!(packed.len(), 1); // 8 bits → 1 byte
        assert_eq!(packed[0], 0b11111111); // all 1s
    }

    #[test]
    fn binary_quantize_mixed() {
        // +, -, +, -, +, -, +, -
        let vec = vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
        let packed = binary_quantize(&vec);
        assert_eq!(packed.len(), 1);
        assert_eq!(packed[0], 0b01010101); // bits 0,2,4,6 set
    }

    #[test]
    fn binary_quantize_all_negative() {
        let vec = vec![-1.0, -2.0, -3.0, -0.5, -0.1, -5.0, -1.0, -1.0];
        let packed = binary_quantize(&vec);
        assert_eq!(packed[0], 0b00000000); // all 0s
    }

    #[test]
    fn binary_quantize_384d_produces_48_bytes() {
        let vec = vec![0.5_f32; 384];
        let packed = binary_quantize(&vec);
        assert_eq!(packed.len(), 48); // 384 / 8 = 48
    }

    #[test]
    fn hamming_distance_identical() {
        let a = vec![0b11110000, 0b10101010];
        let b = vec![0b11110000, 0b10101010];
        assert_eq!(hamming_distance(&a, &b), 0);
    }

    #[test]
    fn hamming_distance_different() {
        let a = vec![0b11111111]; // 8 ones
        let b = vec![0b00000000]; // 8 zeros
        assert_eq!(hamming_distance(&a, &b), 8);
    }

    #[test]
    fn hamming_distance_partial() {
        let a = vec![0b11001100];
        let b = vec![0b10101010];
        // XOR = 01100110 → 4 bits differ
        assert_eq!(hamming_distance(&a, &b), 4);
    }

    #[test]
    fn hamming_similarity_identical() {
        let a = vec![0xFF, 0xAA];
        let b = vec![0xFF, 0xAA];
        let sim = hamming_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn hamming_similarity_opposite() {
        let a = vec![0xFF];
        let b = vec![0x00];
        let sim = hamming_similarity(&a, &b);
        assert!(sim.abs() < 1e-6);
    }
}

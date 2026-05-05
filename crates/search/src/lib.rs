pub mod embedder;
pub mod hybrid;

pub use embedder::{
    Embedder, EmbedderError, EmbeddingMeta, FastembedEngine, HydratedEmbedding, binary_quantize,
    hamming_distance, hamming_similarity,
};

#[cfg(feature = "candle")]
pub use embedder::candle_nomic::CandleNomicEmbedder;

pub use hybrid::{compute_relevance_score, reciprocal_rank_fusion, reciprocal_rank_fusion_binary};

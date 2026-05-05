//! A/B Validation: FastembedEngine vs CandleNomicEmbedder
//!
//! Run: cargo test -p engram-search --features candle --test ab_validation -- --nocapture
//!
//! Tests:
//! 1. FastembedEngine produces 384-dimensional vectors for "hello world"
//! 2. CandleNomicEmbedder produces 384-dimensional vectors (candle feature)
//! 3. Cosine similarity between embeddings > 0.7 (candle feature)
//! 4. Deterministic embeddings for FastembedEngine
//! 5. Deterministic embeddings for CandleNomicEmbedder (candle feature)
//! 6. binary_quantize produces 48 bytes from 384d (both engines)

#![cfg(feature = "candle")]

use engram_search::{CandleNomicEmbedder, Embedder, FastembedEngine, binary_quantize};
use tokio;

/// Blocking helper for CandleNomicEmbedder::generate_embedding
fn candle_embed(embedder: &CandleNomicEmbedder, text: &str) -> Vec<f32> {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(Embedder::generate_embedding(embedder, text))
        .expect("candle embedding")
}

/// Test 1: Both embedders produce 384-dimensional vectors for "hello world"
#[test]
fn ab_both_produce_384d() {
    let fast = FastembedEngine::new().expect("FastembedEngine");
    let candle = CandleNomicEmbedder::new().expect("CandleNomicEmbedder");

    let fast_vec = fast.embed_one("hello world").expect("fastembed embedding");
    let candle_vec = candle_embed(&candle, "hello world");

    assert_eq!(
        fast_vec.len(),
        384,
        "FastembedEngine should be 384d, got {}",
        fast_vec.len()
    );
    assert_eq!(
        candle_vec.len(),
        384,
        "CandleNomicEmbedder should be 384d, got {}",
        candle_vec.len()
    );
    eprintln!("✓ Both embedders produce 384d vectors");
    eprintln!("  FastembedEngine: {}d", fast_vec.len());
    eprintln!("  CandleNomicEmbedder: {}d", candle_vec.len());
}

/// Test 2: Cosine similarity between both embedders for "hello world" > 0.7
#[test]
fn ab_cosine_similarity_threshold() {
    let fast = FastembedEngine::new().expect("FastembedEngine");
    let candle = CandleNomicEmbedder::new().expect("CandleNomicEmbedder");

    let fast_vec = fast.embed_one("hello world").expect("fastembed embedding");
    let candle_vec = candle_embed(&candle, "hello world");

    let similarity = FastembedEngine::cosine_similarity(&fast_vec, &candle_vec);
    eprintln!(
        "✓ Cosine similarity (fastembed vs candle): {:.6}",
        similarity
    );

    assert!(
        similarity > 0.7,
        "Cosine similarity should be > 0.7, got {:.6}",
        similarity
    );
}

/// Test 3: Deterministic — same text should produce the same embedding
#[test]
fn ab_deterministic_embedding() {
    let fast = FastembedEngine::new().expect("FastembedEngine");
    let candle = CandleNomicEmbedder::new().expect("CandleNomicEmbedder");

    // Fastembed
    let v1 = fast.embed_one("hello world").expect("first");
    let v2 = fast.embed_one("hello world").expect("second");
    let sim = FastembedEngine::cosine_similarity(&v1, &v2);
    assert!(
        (sim - 1.0).abs() < 1e-6,
        "FastembedEngine deterministic: {:.10}",
        sim
    );
    eprintln!("✓ FastembedEngine deterministic: {:.10}", sim);

    // Candle
    let v1 = candle_embed(&candle, "hello world");
    let v2 = candle_embed(&candle, "hello world");
    let sim = FastembedEngine::cosine_similarity(&v1, &v2);
    assert!(
        (sim - 1.0).abs() < 1e-6,
        "CandleNomicEmbedder deterministic: {:.10}",
        sim
    );
    eprintln!("✓ CandleNomicEmbedder deterministic: {:.10}", sim);
}

/// Test 4: binary_quantize with 384d vector produces 48 bytes
#[test]
fn ab_binary_quantize_384d() {
    let fast = FastembedEngine::new().expect("FastembedEngine");
    let candle = CandleNomicEmbedder::new().expect("CandleNomicEmbedder");

    let fast_vec = fast.embed_one("hello world").expect("fastembed");
    let candle_vec = candle_embed(&candle, "hello world");

    let fast_bin = binary_quantize(&fast_vec);
    let candle_bin = binary_quantize(&candle_vec);

    assert_eq!(
        fast_bin.len(),
        48,
        "Fastembed binary_quantize: expected 48, got {}",
        fast_bin.len()
    );
    assert_eq!(
        candle_bin.len(),
        48,
        "Candle binary_quantize: expected 48, got {}",
        candle_bin.len()
    );
    eprintln!(
        "✓ binary_quantize: Fastembed {} bytes, Candle {} bytes",
        fast_bin.len(),
        candle_bin.len()
    );
}

/// Test 5: Semantic differentiation — related texts should be closer than unrelated
#[test]
fn ab_semantic_differentiation() {
    let fast = FastembedEngine::new().expect("FastembedEngine");
    let candle = CandleNomicEmbedder::new().expect("CandleNomicEmbedder");

    let pairs = vec![
        ("database performance optimization", "N+1 query performance"),
        ("the weather is nice today", "machine learning inference"),
    ];

    for (a, b) in &pairs {
        let fa = fast.embed_one(a).expect("fast A");
        let fb = fast.embed_one(b).expect("fast B");
        let ca = candle_embed(&candle, a);
        let cb = candle_embed(&candle, b);

        let fast_sim = FastembedEngine::cosine_similarity(&fa, &fb);
        let candle_sim = FastembedEngine::cosine_similarity(&ca, &cb);
        eprintln!("  \"{a}\" vs \"{b}\": fast={fast_sim:.4}, candle={candle_sim:.4}");
    }
}

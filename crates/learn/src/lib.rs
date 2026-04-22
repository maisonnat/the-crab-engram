pub mod anti_pattern;
pub mod boundary_tracker;
pub mod capsule_builder;
pub mod consolidation;
pub mod graph_evolver;
pub mod salience_infer;
pub mod smart_injector;
pub mod spaced_review;
pub mod stream_engine;

#[cfg(feature = "inference")]
pub mod inference;
#[cfg(not(feature = "inference"))]
#[path = "inference.rs"]
pub mod inference;

pub mod extraction;

pub use anti_pattern::{AntiPattern, AntiPatternDetector, AntiPatternType, Severity};
pub use boundary_tracker::BoundaryTracker;
pub use capsule_builder::{
    CapsuleBuilder, CapsuleSynthesizer, ChainedSynthesizer, HeuristicSynthesizer,
};
pub use consolidation::{ConsolidationEngine, ConsolidationResult};
pub use graph_evolver::{EvolutionResult, GraphEvolver, NewEdge};
pub use salience_infer::infer_salience;
pub use smart_injector::{InjectionContext, SmartInjector};
pub use spaced_review::{ReviewResult, SpacedRepetition, bootstrap_reviews};
pub use stream_engine::MemoryStream;

pub use inference::{CacheKey, InferenceEngine};

pub use extraction::{
    ExtractionError, ExtractionPipeline, ExtractionResult, ExtractedEdge, ExtractedObservation,
    KnowledgeExtraction, ValidationError,
};

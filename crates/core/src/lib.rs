pub mod attachment;
pub mod belief;
pub mod boundary;
pub mod capsule;
pub mod compaction;
pub mod crypto;
pub mod entity;
pub mod error;
pub mod graph;
pub mod lifecycle;
pub mod memory;
pub mod observation;
pub mod permissions;
pub mod salience;
pub mod score;
pub mod session;
pub mod stream;
pub mod topic;

pub use attachment::{Attachment, MultimodalObservation};
pub use belief::{Belief, BeliefOperation, BeliefState, HistoricalBelief};
pub use boundary::{BoundaryEvidence, ConfidenceLevel, KnowledgeBoundary};
pub use capsule::KnowledgeCapsule;
pub use compaction::{determine_level, CompactionLevel};
pub use crypto::{decrypt, derive_key, encrypt, is_encrypted_file, EncryptionError};
pub use entity::{extract_entities, Entity, EntityType};
pub use error::EngramError;
pub use graph::{Edge, RelationType};
pub use lifecycle::LifecyclePolicy;
pub use memory::{
    classify_query_type, EpisodicContext, EpisodicMemory, MemoryType, QueryTarget, SemanticMemory,
};
pub use observation::{LifecycleState, Observation, ObservationType, ProvenanceSource, Scope};
pub use permissions::{AccessLevel, PermissionEngine, PermissionRule};
pub use salience::MemorySalience;
pub use score::{compute_final_score, decay_score, decay_score_with_lifecycle};
pub use session::{Session, SessionSummary};
pub use stream::{EventThrottle, ExtractedEntity, MemoryEvent, NotificationThrottle};
pub use topic::{slugify, suggest_topic_key};

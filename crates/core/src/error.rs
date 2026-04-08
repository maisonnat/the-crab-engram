/// Engram error type — the only error type used across all crates.
#[derive(Debug, thiserror::Error)]
pub enum EngramError {
    #[error("database error: {0}")]
    Database(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("duplicate: {0}")]
    Duplicate(String),

    #[error("content too long: {0} chars exceeds limit of {1}")]
    TooLong(String, usize),

    #[error("invalid topic key: {0}")]
    InvalidTopicKey(String),

    #[error("invalid observation type: {0}")]
    InvalidObservationType(String),

    #[error("sync error: {0}")]
    Sync(String),

    #[error("embedding error: {0}")]
    Embedding(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

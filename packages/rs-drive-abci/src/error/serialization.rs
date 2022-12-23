/// Serialization errors
#[derive(Debug, thiserror::Error)]
pub enum SerializationError {
    /// Error
    #[error("corrupted serialization error key: {0}")]
    CorruptedSerialization(String),

    /// Error
    #[error("corrupted deserialization error key: {0}")]
    CorruptedDeserialization(String),
}

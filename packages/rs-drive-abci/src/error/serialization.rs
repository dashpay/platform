#[derive(Debug, thiserror::Error)]
pub enum SerializationError {
    #[error("corrupted serialization error key: {0}")]
    CorruptedSerialization(&'static str),

    #[error("corrupted deserialization error key: {0}")]
    CorruptedDeserialization(&'static str),
}

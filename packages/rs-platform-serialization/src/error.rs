/// Errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A serialization Error.
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// A deserialization Error.
    #[error("deserialization error: {0}")]
    DeserializationError(String),
}
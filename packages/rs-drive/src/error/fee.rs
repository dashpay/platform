/// Fee errors
#[derive(Debug, thiserror::Error)]
pub enum FeeError {
    /// Overflow error
    // TODO: Revisit
    #[error("overflow error: {0}")]
    Overflow(&'static str),

    /// The user does not have enough balance
    #[error("insufficient balance error: {0}")]
    InsufficientBalance(&'static str),

    /// Operation not allowed error
    #[error("operation not allowed error: {0}")]
    OperationNotAllowed(&'static str),

    /// Corrupted estimated layer info missing error
    #[error("corrupted estimated layer info missing error: {0}")]
    CorruptedEstimatedLayerInfoMissing(String),

    /// Corrupted code execution error
    #[error("corrupted removed bytes from identities serialization error: {0}")]
    CorruptedRemovedBytesFromIdentitiesSerialization(&'static str),

    /// Corrupted code execution error
    #[error("corrupted code execution error: {0}")]
    CorruptedCodeExecution(&'static str),

    /// Decimal conversion error
    #[error("decimal conversion error: {0}")]
    DecimalConversion(&'static str),

    /// Document type field not found for estimation
    #[error("document type field not found for estimation  error: {0}")]
    DocumentTypeFieldNotFoundForEstimation(&'static str),
}

/// Execution errors
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    /// Error
    #[error("execution error key: {0}")]
    MissingRequiredKey(&'static str),

    /// Error
    #[error("overflow error: {0}")]
    Overflow(&'static str),

    /// Error
    #[error("conversion error: {0}")]
    Conversion(&'static str),

    /// Error
    #[error("platform corrupted code execution error: {0}")]
    CorruptedCodeExecution(&'static str),

    /// Error
    #[error("drive incoherence error: {0}")]
    DriveIncoherence(&'static str),

    /// Error
    #[error("drive missing data error: {0}")]
    DriveMissingData(&'static str),
}

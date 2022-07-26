#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("execution error key: {0}")]
    MissingRequiredKey(&'static str),

    #[error("overflow error: {0}")]
    Overflow(&'static str),

    #[error("conversion error: {0}")]
    Conversion(&'static str),

    #[error("platform corrupted code execution error: {0}")]
    CorruptedCodeExecution(&'static str),

    #[error("drive incoherence error: {0}")]
    DriveIncoherence(&'static str),

    #[error("drive missing data error: {0}")]
    DriveMissingData(&'static str),
}

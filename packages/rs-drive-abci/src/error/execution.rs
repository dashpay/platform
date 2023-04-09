use crate::error::data_trigger::DataTriggerError;
use dashcore::consensus::encode::Error as DashCoreConsensusEncodeError;
use drive::error::Error as DriveError;

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
    #[error("initialization error: {0}")]
    InitializationError(&'static str),

    /// Error
    #[error("drive incoherence error: {0}")]
    DriveIncoherence(&'static str),

    /// Error
    #[error("protocol upgrade incoherence error: {0}")]
    ProtocolUpgradeIncoherence(&'static str),

    /// Error
    #[error("drive missing data error: {0}")]
    DriveMissingData(&'static str),

    /// Error
    #[error("corrupted credits not balanced error: {0}")]
    CorruptedCreditsNotBalanced(String),

    /// Error
    #[error("transaction not present error: {0}")]
    NotInTransaction(&'static str),

    /// Error
    #[error("cannot update proposed app version: {0}")]
    UpdateValidatorProposedAppVersionError(#[from] DriveError),

    /// Drive responded in a way that was impossible
    /// For example we asked for 2 items and it returned 3
    #[error("corrupted drive response error: {0}")]
    CorruptedDriveResponse(String),

    /// An error received from DashCore
    #[error("dash core consensus encode error: {0}")]
    DashCoreConsensusEncodeError(#[from] DashCoreConsensusEncodeError),

    /// An error received for a data trigger
    #[error("data trigger error: {0}")]
    DataTrigger(#[from] DataTriggerError),
}

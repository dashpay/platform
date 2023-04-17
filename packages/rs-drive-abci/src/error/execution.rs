use crate::error::data_trigger::DataTriggerError;
use dpp::dashcore::consensus::encode::Error as DashCoreConsensusEncodeError;
use drive::error::Error as DriveError;

/// Execution errors
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    /// A required key is missing.
    #[error("missing required key: {0}")]
    MissingRequiredKey(&'static str),

    /// The state has not been initialized.
    #[error("state not initialized: {0}")]
    StateNotInitialized(&'static str),

    /// An overflow error occurred.
    #[error("overflow error: {0}")]
    Overflow(&'static str),

    /// A conversion error occurred.
    #[error("conversion error: {0}")]
    Conversion(&'static str),

    /// The platform encountered a corrupted code execution error.
    #[error("platform corrupted code execution error: {0}")]
    CorruptedCodeExecution(&'static str),

    /// The platform encountered a corrupted cache state error.
    #[error("platform corrupted cached state error: {0}")]
    CorruptedCachedState(&'static str),

    /// An error occurred during initialization.
    #[error("initialization error: {0}")]
    InitializationError(&'static str),

    /// A drive incoherence error occurred.
    #[error("drive incoherence error: {0}")]
    DriveIncoherence(&'static str),

    /// A protocol upgrade incoherence error occurred.
    #[error("protocol upgrade incoherence error: {0}")]
    ProtocolUpgradeIncoherence(&'static str),

    /// Data is missing from the drive.
    #[error("drive missing data error: {0}")]
    DriveMissingData(&'static str),

    /// Corrupted credits are not balanced.
    #[error("corrupted credits not balanced error: {0}")]
    CorruptedCreditsNotBalanced(String),

    /// The transaction is not present.
    #[error("transaction not present error: {0}")]
    NotInTransaction(&'static str),

    /// An error occurred while updating the proposed app version.
    #[error("cannot update proposed app version: {0}")]
    UpdateValidatorProposedAppVersionError(#[from] DriveError),

    /// Drive responded in a way that was impossible (e.g., requested 2 items but got 3).
    #[error("corrupted drive response error: {0}")]
    CorruptedDriveResponse(String),

    /// An error received from DashCore during consensus encoding.
    #[error("dash core consensus encode error: {0}")]
    DashCoreConsensusEncodeError(#[from] DashCoreConsensusEncodeError),

    /// DashCore responded with a bad response error.
    #[error("dash core bad response error: {0}")]
    DashCoreBadResponseError(String),

    /// An error received for a data trigger.
    #[error("data trigger error: {0}")]
    DataTrigger(#[from] DataTriggerError),
}

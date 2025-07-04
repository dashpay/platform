use dpp::bls_signatures::BlsError;
use dpp::dashcore::consensus::encode::Error as DashCoreConsensusEncodeError;
use dpp::identity::TimestampMillis;
use dpp::version::FeatureVersion;
use drive::error::Error as DriveError;

// @append_only
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
    Conversion(String),

    /// The platform encountered a corrupted code execution error.
    #[error("platform corrupted code execution error: {0}")]
    CorruptedCodeExecution(&'static str),

    /// Platform expected a specific version but got another.
    #[error("platform corrupted code version mismatch: {0}")]
    CorruptedCodeVersionMismatch(&'static str),

    /// Platform expected some specific versions
    #[error("platform unknown version on {method}, received: {received}")]
    UnknownVersionMismatch {
        /// method
        method: String,
        /// the allowed versions for this method
        known_versions: Vec<FeatureVersion>,
        /// requested core height
        received: FeatureVersion,
    },

    /// Platform expected some specific versions
    #[error("{method} not active for drive version")]
    VersionNotActive {
        /// method
        method: String,
        /// the allowed versions for this method
        known_versions: Vec<FeatureVersion>,
    },

    /// The platform encountered a corrupted cache state error.
    #[error("platform corrupted cached state error: {0}")]
    CorruptedCachedState(String),

    /// The fork is not yet active for core.
    #[error("initialization fork not active: {0}")]
    InitializationForkNotActive(String),

    /// Invalid core chain locked height
    #[error("initial height {initial_height} is not chain locked. latest chainlocked height is {chain_lock_height}")]
    InitializationHeightIsNotLocked {
        /// initial height (requested or fork)
        initial_height: u32,
        /// best core lock height
        chain_lock_height: u32,
    },

    /// Genesis time is in the future.
    #[error("genesis time {genesis_time} for initial height {initial_height} is in the future. current time is {current_time}")]
    InitializationGenesisTimeInFuture {
        /// initial height (requested or fork)
        initial_height: u32,
        /// genesis time
        genesis_time: TimestampMillis,
        /// current time
        current_time: TimestampMillis,
    },

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
    DriveMissingData(String),

    /// Corrupted credits are not balanced.
    #[error("corrupted credits not balanced error: {0}")]
    CorruptedCreditsNotBalanced(String),

    /// Corrupted tokens are not balanced.
    #[error("corrupted tokens not balanced error: {0}")]
    CorruptedTokensNotBalanced(String),

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
    #[error("data trigger execution error: {0}")]
    DataTriggerExecutionError(String),

    /// Error occurred during deserializing a BLS primitive received from core
    #[error("dash core response bls error: {0}")]
    BlsErrorFromDashCoreResponse(BlsError),

    /// General Bls Error
    #[error("bls error: {0}")]
    BlsErrorGeneral(#[from] BlsError),

    /// General IO Error
    #[error("io error: {0}")]
    IOError(#[from] std::io::Error),
}

use crate::drive::contract::MAX_CONTRACT_HISTORY_FETCH_LIMIT;
use dpp::fee::Credits;
use dpp::version::FeatureVersion;

/// Drive errors
#[derive(Debug, thiserror::Error)]
pub enum DriveError {
    /// Error
    /// This error should never occur, it is the equivalent of a panic.
    #[error("corrupted code execution error: {0}")]
    CorruptedCodeExecution(&'static str),

    /// Platform expected some specific versions
    #[error("drive unknown version on {method}, received: {received}")]
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

    /// Error
    /// A critical corrupted state should stall the chain.
    /// This is done on purpose to prevent more harm to the system
    #[error("critical corrupted state error: {0}")]
    CriticalCorruptedState(&'static str),

    /// Error
    #[error("not supported error: {0}")]
    NotSupported(&'static str),

    /// Error
    #[error("not supported private error: {0}")]
    NotSupportedPrivate(&'static str),

    /// Error
    #[error("grovedb insertion error: {0}")]
    GroveDBInsertion(&'static str),

    /// Error
    #[error("updating readonly immutable contract error: {0}")]
    UpdatingReadOnlyImmutableContract(&'static str),
    /// Error
    #[error("updating readonly immutable document error: {0}")]
    UpdatingReadOnlyImmutableDocument(&'static str),

    /// Error
    #[error("invalid deletion of document that keeps history error: {0}")]
    InvalidDeletionOfDocumentThatKeepsHistory(&'static str),

    /// Error
    #[error("deleting document that does not exist error: {0}")]
    DeletingDocumentThatDoesNotExist(&'static str),
    /// Error
    #[error("updating document that did not already exist error: {0}")]
    UpdatingDocumentThatDoesNotExist(&'static str),

    /// Error
    #[error("changing contract to readonly error: {0}")]
    ChangingContractToReadOnly(&'static str),
    /// Error
    #[error("changing contract keeps history error: {0}")]
    ChangingContractKeepsHistory(&'static str),

    /// Error
    #[error("contested index not found error: {0}")]
    ContestedIndexNotFound(&'static str),

    /// Error
    #[error("contested document missing owner error: {0}")]
    ContestedDocumentMissingOwnerId(&'static str),

    /// Error
    #[error("updating contract with history error: {0}")]
    UpdatingContractWithHistoryError(&'static str),

    /// Error
    #[error("changing contract documents keeps history default error: {0}")]
    ChangingContractDocumentsKeepsHistoryDefault(&'static str),
    /// Error
    #[error("changing contract documents mutability default error: {0}")]
    ChangingContractDocumentsMutabilityDefault(&'static str),
    /// Error
    #[error("changing document type mutability error: {0}")]
    ChangingDocumentTypeMutability(&'static str),
    /// Error
    #[error("changing document type keeps history error: {0}")]
    ChangingDocumentTypeKeepsHistory(&'static str),

    /// Error
    #[error("invalid path error: {0}")]
    InvalidPath(&'static str),

    /// Error
    #[error("corrupted contract path error: {0}")]
    CorruptedContractPath(&'static str),
    /// Error
    #[error("corrupted contract indexes error: {0}")]
    CorruptedContractIndexes(String),
    /// Error
    #[error("corrupted document path error: {0}")]
    CorruptedDocumentPath(&'static str),
    /// Error
    #[error("corrupted balance path error: {0}")]
    CorruptedBalancePath(&'static str),
    /// Error
    #[error("corrupted document already exists error: {0}")]
    CorruptedDocumentAlreadyExists(&'static str),
    /// Error
    #[error("corrupted document not an item error: {0}")]
    CorruptedDocumentNotItem(&'static str),
    /// Error
    #[error("corrupted identity not an item error: {0}")]
    CorruptedIdentityNotItem(&'static str),

    /// Error
    #[error("corrupted query returned non item error: {0}")]
    CorruptedQueryReturnedNonItem(&'static str),
    /// Error
    #[error("corrupted withdrawal not an item error: {0}")]
    CorruptedWithdrawalNotItem(&'static str),
    /// Error
    #[error("corrupted withdrawal transaction count invalid length: {0}")]
    CorruptedWithdrawalTransactionsCounterInvalidLength(&'static str),
    /// Error
    #[error("orrupted withdrawal transaction not an item: {0}")]
    CorruptedWithdrawalTransactionsCounterNotItem(&'static str),

    /// Error
    #[error("corrupted element type error: {0}")]
    CorruptedElementType(&'static str),

    /// Error
    #[error("corrupted drive state error: {0}")]
    CorruptedDriveState(String),

    /// Error
    #[error("corrupted cache state error: {0}")]
    CorruptedCacheState(String),

    /// Error
    #[error("corrupted element flags error: {0}")]
    CorruptedElementFlags(&'static str),

    /// Error
    #[error("corrupted serialization error: {0}")]
    CorruptedSerialization(String),

    /// Error
    #[error("corrupted genesis time not an item error")]
    CorruptedGenesisTimeNotItem(),

    /// Error
    #[error("corrupted genesis time invalid item length error: {0}")]
    CorruptedGenesisTimeInvalidItemLength(String),

    /// Error
    #[error("batch is empty: {0}")]
    BatchIsEmpty(String),

    /// Error
    #[error("unexpected element type: {0}")]
    UnexpectedElementType(&'static str),

    /// Error
    #[error("invalid contract history fetch limit: {0}. The limit must be between 1 and {MAX_CONTRACT_HISTORY_FETCH_LIMIT}")]
    InvalidContractHistoryFetchLimit(u16),

    /// Error
    #[error("prefunded specialized balance does not exist: {0}")]
    PrefundedSpecializedBalanceDoesNotExist(String),

    /// Error
    #[error("prefunded specialized balance does not have enough credits: we have {0}, we want to deduct {1}")]
    PrefundedSpecializedBalanceNotEnough(Credits, Credits),

    /// Data Contract not found
    #[error("data contract not found: {0}")]
    DataContractNotFound(String),
}

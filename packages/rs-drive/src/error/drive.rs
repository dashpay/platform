/// Drive errors
#[derive(Debug, thiserror::Error)]
pub enum DriveError {
    /// Error
    // This error should never occur, it is the equivalent of a panic.
    #[error("corrupted code execution error: {0}")]
    CorruptedCodeExecution(&'static str),
    /// Error
    #[error("unsupported error: {0}")]
    Unsupported(&'static str),
    /// Error
    #[error("unsupported private error: {0}")]
    UnsupportedPrivate(&'static str),
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
    #[error("deleting document that does not exist error: {0}")]
    DeletingDocumentThatDoesNotExist(&'static str),
    /// Error
    #[error("changing contract to readonly error: {0}")]
    ChangingContractToReadOnly(&'static str),
    /// Error
    #[error("changing contract keeps history error: {0}")]
    ChangingContractKeepsHistory(&'static str),
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
    #[error("corrupted contract path error: {0}")]
    CorruptedContractPath(&'static str),
    /// Error
    #[error("corrupted contract indexes error: {0}")]
    CorruptedContractIndexes(&'static str),
    /// Error
    #[error("corrupted document path error: {0}")]
    CorruptedDocumentPath(&'static str),
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
    #[error("corrupted element flags error: {0}")]
    CorruptedElementFlags(&'static str),

    /// Error
    #[error("corrupted serialization error: {0}")]
    CorruptedSerialization(&'static str),

    /// Error
    #[error("corrupted genesis time not an item error")]
    CorruptedGenesisTimeNotItem(),

    /// Error
    #[error("corrupted genesis time invalid item length error: {0}")]
    CorruptedGenesisTimeInvalidItemLength(String),

    /// Error
    #[error("batch is empty")]
    BatchIsEmpty(),
}

///DataContract errors
#[derive(Debug, thiserror::Error)]
pub enum DataContractError {
    /// Overflow error
    #[error("overflow error: {0}")]
    Overflow(&'static str),

    /// KeyBoundsExpectedButNotPresent error
    #[error("key bounds expected but not present error: {0}")]
    KeyBoundsExpectedButNotPresent(&'static str),

    /// Data contract missing or cannot be retrieved
    #[error("data contract cannot be retrieved: {0}")]
    MissingContract(String),

    /// Data contract provided is not the one we want
    #[error("data contract provided is incorrect: {0}")]
    ProvidedContractMismatch(String),

    /// Data contract is corrupted
    #[error("data contract is corrupted: {0}")]
    CorruptedDataContract(String),

    /// Data contract storage error when data contract is too big
    #[error("data contract is too big to be stored: {0}")]
    ContractTooBig(String),
}

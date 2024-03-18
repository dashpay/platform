use grovedb::query_result_type::Path;

/// Proof errors
#[derive(Debug, thiserror::Error)]
pub enum ProofError {
    /// Too many elements error
    #[error("too many elements error: {0}")]
    TooManyElements(&'static str),

    /// Wrong element count error
    #[error("wrong element count error expected: {expected} got: {got}")]
    WrongElementCount {
        /// The expected count
        expected: usize,
        /// The count we got
        got: usize,
    },

    /// Overflow error
    #[error("overflow error: {0}")]
    Overflow(&'static str),

    /// An incoherent result is akin to a corrupted code execution, the proof returned is said to
    /// be valid, however data it possesses isn't what was asked for.
    #[error("corrupted error: {0}")]
    CorruptedProof(String),

    /// The proof returned is said to be valid, data is what we asked for, but is not what was
    /// expected, for example if we updated a contract, but we are getting back the old contract
    #[error("incorrect proof error: {0}")]
    IncorrectProof(String),

    /// The transition we are trying to prove was executed is invalid
    #[error("invalid transition error: {0}")]
    InvalidTransition(String),

    /// The transition we are trying to prove has an unknown contract
    #[error("unknown contract in documents batch transition error: {0}")]
    UnknownContract(String),

    /// The metadata we got back from platform is incorrect
    #[error("invalid metadata: {0}")]
    InvalidMetadata(String),

    /// We are trying to callback to retrieve a contract, but there was an error
    #[error("the contract could not be retrieved during verification: {0}")]
    ErrorRetrievingContract(String),

    /// Incomplete proof error
    #[error("incomplete proof error: {0}")]
    IncompleteProof(&'static str),

    /// Incorrect value size
    #[error("incorrect value size error: {0}")]
    IncorrectValueSize(&'static str),

    /// Incorrect element path error
    #[error("incorrect element path error")]
    IncorrectElementPath {
        /// The expected path
        expected: Path,
        /// The actual path
        actual: Path,
    },
}
#[allow(dead_code)]
#[deprecated(note = "This function is marked as unused.")]
#[allow(deprecated)]
fn get_error_code(error: &ProofError) -> u32 {
    match error {
        ProofError::TooManyElements(_) => 6000,
        ProofError::WrongElementCount { .. } => 6001,
        ProofError::Overflow(_) => 6002,
        ProofError::CorruptedProof(_) => 6003,
        ProofError::IncompleteProof(_) => 6004,
        ProofError::IncorrectValueSize(_) => 6005,
        ProofError::IncorrectElementPath { .. } => 6006,
        ProofError::IncorrectProof(_) => 6007,
        ProofError::InvalidTransition(_) => 6008,
        ProofError::UnknownContract(_) => 6009,
        ProofError::ErrorRetrievingContract(_) => 6010,
        ProofError::InvalidMetadata(_) => 6011,
    }
}

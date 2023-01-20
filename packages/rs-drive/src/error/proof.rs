use grovedb::query_result_type::Path;

/// Proof errors
#[derive(Debug, thiserror::Error)]
pub enum ProofError {
    /// Too many elements error
    #[error("too many elements error: {0}")]
    TooManyElements(&'static str),

    /// Wrong element count error
    #[error("wrong element count error: {0}")]
    WrongElementCount(&'static str),

    /// Overflow error
    #[error("overflow error: {0}")]
    Overflow(&'static str),

    /// Corrupted error
    #[error("corrupted error: {0}")]
    CorruptedProof(&'static str),

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

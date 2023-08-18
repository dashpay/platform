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

    /// An incoherent result is akin to a corrupted code execution, the proof returned is said to
    /// be valid, however data it possesses isn't what was asked for.
    #[error("corrupted error: {0}")]
    CorruptedProof(String),

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

fn get_error_code(error: &ProofError) -> u32 {
    match error {
        ProofError::TooManyElements(_) => 6000,
        ProofError::WrongElementCount(_) => 6001,
        ProofError::Overflow(_) => 6002,
        ProofError::CorruptedProof(_) => 6003,
        ProofError::IncompleteProof(_) => 6004,
        ProofError::IncorrectValueSize(_) => 6005,
        ProofError::IncorrectElementPath { .. } => 6006,
    }
}

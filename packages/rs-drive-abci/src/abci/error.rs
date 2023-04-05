use crate::validator_set::ValidatorSetError;
use dpp::bls_signatures::Error as BLSError;

/// Error returned within ABCI server
#[derive(Debug, thiserror::Error)]
pub enum AbciError {
    /// Invalid system state
    #[error("invalid state: {0}")]
    InvalidState(String),
    /// Wrong finalize block received
    #[error("finalize block received before processing from Tenderdash: {0}")]
    FinalizeBlockReceivedBeforeProcessing(String),
    /// Wrong finalize block received
    #[error("wrong finalize block from Tenderdash: {0}")]
    WrongFinalizeBlockReceived(String),
    /// Bad request received from Tenderdash that can't be translated to the correct size
    /// This often happens if a Vec<> can not be translated into a [u8;32]
    #[error("data received from Tenderdash could not be converted: {0}")]
    BadRequestDataSize(String),
    /// Bad request received from Tenderdash
    #[error("bad request received from Tenderdash: {0}")]
    BadRequest(String),

    /// Error returned by Tenderdash-abci library
    #[cfg(feature = "server")]
    #[error("tenderdash: {0}")]
    Tenderdash(#[from] tenderdash_abci::Error),

    /// Error occurred during bls signature verification
    #[error("bls error set: {0}")]
    BLSError(#[from] BLSError),

    /// Error occurred during validator set creation
    #[error("validator set: {0}")]
    ValidatorSet(#[from] ValidatorSetError),

    /// Generic with code should only be used in tests
    #[error("generic with code: {0}")]
    GenericWithCode(u32),
}

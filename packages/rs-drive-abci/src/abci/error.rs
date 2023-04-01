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
    /// Bad request received from Tenderdash
    #[error("bad request received from Tenderdash: {0}")]
    BadRequest(String),
    /// Error returned by tenderdash-abci library
    #[cfg(feature = "server")]
    #[error("tenderdash: {0}")]
    Tenderdash(#[from] tenderdash_abci::Error),
}

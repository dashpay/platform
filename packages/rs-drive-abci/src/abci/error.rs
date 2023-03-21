/// Error returned within ABCI server
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Invalid system state
    #[error("invalid state: {0}")]
    InvalidState(String),
    /// Error returned by tenderdash-abci library
    #[cfg(feature = "server")]
    #[error("tenderdash: {0}")]
    Tenderdash(#[from] tenderdash_abci::Error),
}

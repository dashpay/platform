use tenderdash_abci::Error as TenderdashError;

/// Error returned within ABCI server
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error returned by tenderdash-abci library
    #[error("tenderdash: {0}")]
    Tenderdash(#[from] TenderdashError),
}

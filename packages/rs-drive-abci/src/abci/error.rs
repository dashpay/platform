use tenderdash_abci::Error as TenderdashError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("tenderdash: {0}")]
    Tenderdash(#[from] TenderdashError),
}

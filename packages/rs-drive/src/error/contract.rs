///DataContract errors
#[derive(Debug, thiserror::Error)]
pub enum DataContractError {
    /// Overflow error
    #[error("overflow error: {0}")]
    Overflow(&'static str),
}

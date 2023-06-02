/// Contract errors
#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    /// Overflow error
    #[error("overflow error: {0}")]
    Overflow(&'static str),
}

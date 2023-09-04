///DataContract errors
#[derive(Debug, thiserror::Error)]
pub enum DataContractError {
    /// Overflow error
    #[error("overflow error: {0}")]
    Overflow(&'static str),

    /// KeyBoundsExpectedButNotPresent error
    #[error("key bounds expected but not present error: {0}")]
    KeyBoundsExpectedButNotPresent(&'static str),
}

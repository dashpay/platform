/// Errors returned by the context provider
#[derive(Debug, thiserror::Error)]
pub enum ContextProviderError {
    /// Generic Context provider error
    #[error("Context provider error: {0}")]
    Generic(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Data contract is invalid or not found, or some error occurred during data contract retrieval
    #[error("cannot get data contract: {0}")]
    DataContractFailure(String),

    /// Token configuration is invalid or not found, or some error occurred during token configuration retrieval
    #[error("cannot get token configuration: {0}")]
    TokenConfigurationFailure(String),

    /// Provided quorum is invalid
    #[error("invalid quorum: {0}")]
    InvalidQuorum(String),

    /// Core Fork Error
    #[error("activation fork error: {0}")]
    ActivationForkError(String),

    /// Async error, eg. when tokio runtime fails
    #[error("async error: {0}")]
    AsyncError(String),
}

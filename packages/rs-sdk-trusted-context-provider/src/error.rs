use std::error::Error as StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TrustedContextProviderError {
    #[error("HTTP request error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Quorum not found for type {quorum_type} and hash {quorum_hash}")]
    QuorumNotFound {
        quorum_type: u32,
        quorum_hash: String,
    },

    #[error("Invalid quorum public key format")]
    InvalidPublicKeyFormat,

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Invalid devnet name: {0}")]
    InvalidDevnetName(String),

    #[error("Unsupported network: {0}")]
    UnsupportedNetwork(String),
}

/// An error with every cause it carries, which `Display` alone leaves out.
pub(crate) fn with_causes(error: &dyn StdError) -> String {
    let mut message = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    message
}

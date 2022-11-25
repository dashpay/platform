use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Invalid instant lock proof: ${message}")]
pub struct InvalidInstantAssetLockProofError {
    pub message: String,
}

impl InvalidInstantAssetLockProofError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

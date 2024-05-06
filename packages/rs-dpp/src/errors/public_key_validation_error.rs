use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Public key validation error: {message:?}")]
#[ferment_macro::export]
pub struct PublicKeyValidationError {
    pub message: String,
}

impl PublicKeyValidationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

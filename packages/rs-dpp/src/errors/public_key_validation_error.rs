use thiserror::Error;

#[derive(Error, Debug, Clone)]
#[error("Public key validation error: {message:?}")]
pub struct PublicKeyValidationError {
    message: String
}

impl PublicKeyValidationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}
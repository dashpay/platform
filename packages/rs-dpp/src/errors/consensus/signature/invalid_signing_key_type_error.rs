use thiserror::Error;

use crate::identity::KeyType;
use crate::ProtocolError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Invalid key type for signing")]
pub struct InvalidSigningKeyTypeError {
    public_key_type: KeyType,
}

impl InvalidSigningKeyTypeError {
    pub fn new(public_key_type: KeyType) -> Self {
        Self { public_key_type }
    }

    pub fn public_key_type(&self) -> KeyType {
        self.public_key_type
    }
}

impl From<InvalidSigningKeyTypeError> for ProtocolError {
    fn from(err: InvalidSigningKeyTypeError) -> Self {
        Self::InvalidSigningKeyTypeError(err)
    }
}

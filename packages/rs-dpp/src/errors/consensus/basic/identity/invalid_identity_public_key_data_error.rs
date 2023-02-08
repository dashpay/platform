use crate::identity::KeyID;
use thiserror::Error;

use crate::PublicKeyValidationError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Invalid identity public key {public_key_id:?} data: {message:?}")]
pub struct InvalidIdentityPublicKeyDataError {
    public_key_id: KeyID,
    message: String,
    validation_error: Option<PublicKeyValidationError>,
}

impl InvalidIdentityPublicKeyDataError {
    pub fn new(
        public_key_id: KeyID,
        message: String,
        validation_error: Option<PublicKeyValidationError>,
    ) -> Self {
        Self {
            public_key_id,
            message,
            validation_error,
        }
    }

    pub fn public_key_id(&self) -> KeyID {
        self.public_key_id
    }

    pub fn validation_error(&self) -> &Option<PublicKeyValidationError> {
        &self.validation_error
    }
}

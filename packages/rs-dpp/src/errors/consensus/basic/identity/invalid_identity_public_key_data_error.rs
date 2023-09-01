use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::KeyID;
use crate::PublicKeyValidationError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Invalid identity public key {public_key_id:?} data: {validation_error:?}")]
#[platform_serialize(unversioned)]
pub struct InvalidIdentityPublicKeyDataError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    public_key_id: KeyID,
    validation_error: String,
}

impl InvalidIdentityPublicKeyDataError {
    pub fn new(public_key_id: KeyID, validation_error: PublicKeyValidationError) -> Self {
        Self {
            public_key_id,
            validation_error: validation_error.message().to_string(),
        }
    }

    pub fn public_key_id(&self) -> KeyID {
        self.public_key_id
    }

    pub fn validation_error(&self) -> &str {
        &self.validation_error
    }
}
impl From<InvalidIdentityPublicKeyDataError> for ConsensusError {
    fn from(err: InvalidIdentityPublicKeyDataError) -> Self {
        Self::BasicError(BasicError::InvalidIdentityPublicKeyDataError(err))
    }
}

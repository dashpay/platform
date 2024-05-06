use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::identity::identity_public_key::KeyID;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Identity key {public_key_id} has invalid signature")]
#[platform_serialize(unversioned)]
#[ferment_macro::export]
pub struct InvalidIdentityKeySignatureError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub public_key_id: KeyID,
}

impl InvalidIdentityKeySignatureError {
    pub fn new(public_key_id: KeyID) -> Self {
        Self { public_key_id }
    }

    pub fn public_key_id(&self) -> KeyID {
        self.public_key_id
    }
}

impl From<InvalidIdentityKeySignatureError> for ConsensusError {
    fn from(err: InvalidIdentityKeySignatureError) -> Self {
        Self::BasicError(BasicError::InvalidIdentityKeySignatureError(err))
    }
}

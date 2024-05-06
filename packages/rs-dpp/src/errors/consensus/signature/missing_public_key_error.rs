use thiserror::Error;

use crate::errors::consensus::signature::signature_error::SignatureError;
use crate::errors::consensus::ConsensusError;
use crate::identity::identity_public_key::KeyID;

use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Public key {public_key_id} doesn't exist")]
#[platform_serialize(unversioned)]
#[ferment_macro::export]
pub struct MissingPublicKeyError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub public_key_id: KeyID,
}

impl MissingPublicKeyError {
    pub fn new(public_key_id: KeyID) -> Self {
        Self { public_key_id }
    }

    pub fn public_key_id(&self) -> KeyID {
        self.public_key_id
    }
}

impl From<MissingPublicKeyError> for ConsensusError {
    fn from(err: MissingPublicKeyError) -> Self {
        Self::SignatureError(SignatureError::MissingPublicKeyError(err))
    }
}

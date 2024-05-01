use thiserror::Error;

use crate::consensus::signature::signature_error::SignatureError;
use crate::consensus::ConsensusError;
use crate::identity::KeyType;

use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Unsupported signature type {public_key_type}. Please use ECDSA (0), BLS (1) or ECDSA_HASH160 (2) keys to sign the state transition")]
#[platform_serialize(unversioned)]
pub struct InvalidIdentityPublicKeyTypeError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    public_key_type: KeyType,
}

impl InvalidIdentityPublicKeyTypeError {
    pub fn new(public_key_type: KeyType) -> Self {
        Self { public_key_type }
    }

    pub fn public_key_type(&self) -> KeyType {
        self.public_key_type
    }
}

impl From<InvalidIdentityPublicKeyTypeError> for ConsensusError {
    fn from(err: InvalidIdentityPublicKeyTypeError) -> Self {
        Self::SignatureError(SignatureError::InvalidIdentityPublicKeyTypeError(err))
    }
}

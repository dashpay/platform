use thiserror::Error;

use crate::consensus::signature::signature_error::SignatureError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::KeyID;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};

use bincode::{Decode, DecodeUntrusted, Encode};

#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error("Identity key {public_key_id} is disabled")]
#[platform_serialize(unversioned)]
pub struct PublicKeyIsDisabledError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    public_key_id: KeyID,
}

impl PublicKeyIsDisabledError {
    pub fn new(public_key_id: KeyID) -> Self {
        Self { public_key_id }
    }

    pub fn public_key_id(&self) -> KeyID {
        self.public_key_id
    }
}

impl From<PublicKeyIsDisabledError> for ConsensusError {
    fn from(err: PublicKeyIsDisabledError) -> Self {
        Self::SignatureError(SignatureError::PublicKeyIsDisabledError(err))
    }
}
